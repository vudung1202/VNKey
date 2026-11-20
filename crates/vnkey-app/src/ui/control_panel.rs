//! The Control Panel window: a sidebar (the product, the Vietnamese/English
//! switch, five views) beside the selected view. Without Accessibility
//! permission it shows the two setup steps instead.

use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{Icon, IndexPath, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use vnkey_engine::{CodeTable, ConvertOptions};

use super::hotkey::{HotkeyKind, HotkeyRecorder, chord_text, hotkey_chips};
use super::macro_editor::MacroView;
use super::strings::{
    self, about_tab, convert_tab, convert_view, permission_gate, sidebar, system_tab, system_view, top_bar,
    typing_tab, typing_view,
};
use super::{Palette, field_row, logo, palette, panel, row, section, segmented, switch_row, title_row, transform_line};
use crate::app::UiState;
use crate::commands;
use crate::platform::mac::ffi;
use crate::settings::Settings;

type Choices = SelectState<Vec<SharedString>>;
/// Writes a select's index into its `u8` settings field.
type SetChoice = fn(Settings, u8) -> Settings;

/// No OS notification exists for an Accessibility grant change, so the
/// window polls while open.
const PERMISSION_POLL: Duration = Duration::from_millis(1500);
const SIDEBAR_WIDTH: f32 = 220.;
const SELECT_WIDTH: f32 = 220.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Typing,
    Macros,
    Convert,
    System,
    About,
}

/// Sidebar order; labels are `strings::TABS` in the same order.
const SECTIONS: [(Section, &str); 5] = [
    (Section::Typing, "icons/keyboard.svg"),
    (Section::Macros, "icons/text-cursor-input.svg"),
    (Section::Convert, "icons/arrow-left-right.svg"),
    (Section::System, "icons/settings.svg"),
    (Section::About, "icons/info.svg"),
];

/// Keystrokes and what the engine makes of them, per input type. Measured on
/// the engine (golden probe 2026-11-20), not typed from memory; each pair
/// shows what sets the type apart.
fn previews(input_type: u8) -> &'static [(&'static str, &'static str)] {
    match input_type {
        1 => &[("vie65t", "việt"), ("d9a6y", "đây")],
        2 => &[("vieejt", "việt"), ("w", "w")],
        3 => &[("vieejt", "việt"), ("]", "]")],
        _ => &[("vieejt", "việt"), ("nguwowif", "người")],
    }
}

/// The convert tab's case and mark options applied to a sample sentence.
fn case_preview(s: &Settings) -> String {
    let options = ConvertOptions {
        from_code: CodeTable::Unicode,
        to_code: CodeTable::Unicode,
        ..s.convert_options()
    };
    vnkey_engine::convert_util(convert_view::SAMPLE, &options)
}

/// 0 keeps the case; 1-4 are the four mutually exclusive caps options.
fn case_index(s: &Settings) -> usize {
    [s.convert_to_all_caps, s.convert_to_all_non_caps, s.convert_to_caps_first_letter, s.convert_to_caps_each_word]
        .iter()
        .position(|on| *on)
        .map_or(0, |ix| ix + 1)
}

fn set_case(mut s: Settings, ix: usize) -> Settings {
    s.convert_to_all_caps = ix == 1;
    s.convert_to_all_non_caps = ix == 2;
    s.convert_to_caps_first_letter = ix == 3;
    s.convert_to_caps_each_word = ix == 4;
    s
}

/// A select over fixed labels whose index is a `u8` settings field.
fn choices(labels: &[&'static str], selected: u8, window: &mut Window, cx: &mut App) -> Entity<Choices> {
    let items = labels.iter().map(|label| SharedString::from(*label)).collect();
    cx.new(|cx| SelectState::new(items, Some(IndexPath::new(usize::from(selected))), window, cx))
}

fn label_of(labels: &[&'static str], ix: u8) -> &'static str {
    labels.get(usize::from(ix)).copied().unwrap_or(labels[0])
}

pub struct ControlPanel {
    section: Section,
    /// The CGEventTap only attaches at launch, so a grant made while this
    /// window is open still needs a restart: the setup steps stay up and
    /// switch their call to action instead of unblocking.
    trusted_at_open: bool,
    trusted_now: bool,
    input_type: Entity<Choices>,
    code_table: Entity<Choices>,
    convert_from: Entity<Choices>,
    convert_to: Entity<Choices>,
    switch_recorder: Entity<HotkeyRecorder>,
    convert_recorder: Entity<HotkeyRecorder>,
    macros: Entity<MacroView>,
    _subscriptions: Vec<Subscription>,
    _poll: Task<()>,
}

impl ControlPanel {
    pub fn new(section: Section, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s = cx.global::<UiState>().settings;
        let input_type = choices(&strings::INPUT_TYPES, s.input_type, window, cx);
        let code_table = choices(&strings::CODE_TABLES, s.code_table, window, cx);
        let convert_from = choices(&strings::CODE_TABLES, s.convert_from_code, window, cx);
        let convert_to = choices(&strings::CODE_TABLES, s.convert_to_code, window, cx);

        let mut subscriptions = vec![
            cx.observe_global_in::<UiState>(window, |this, window, cx| {
                this.sync_selects(window, cx);
                cx.notify();
            }),
            super::follow_system_appearance(window, cx),
        ];
        let bindings: [(&Entity<Choices>, SetChoice); 4] = [
            (&input_type, commands::set_input_type_settings),
            (&code_table, commands::set_code_table_settings),
            (&convert_from, |mut s, v| {
                s.convert_from_code = v;
                s
            }),
            (&convert_to, |mut s, v| {
                s.convert_to_code = v;
                s
            }),
        ];
        for (state, set) in bindings {
            subscriptions.push(cx.subscribe(state, move |_, state, _: &SelectEvent<Vec<SharedString>>, cx| {
                let Some(ix) = state.read(cx).selected_index(cx) else { return };
                let Ok(value) = u8::try_from(ix.row) else { return };
                commands::apply_and_broadcast(cx, |s| set(s, value));
            }));
        }

        let trusted = ffi::is_trusted();
        let poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PERMISSION_POLL).await;
                let trusted = ffi::is_trusted();
                let alive = this.update(cx, |this, cx| {
                    if this.trusted_now != trusted {
                        this.trusted_now = trusted;
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });

        Self {
            section,
            trusted_at_open: trusted,
            trusted_now: trusted,
            input_type,
            code_table,
            convert_from,
            convert_to,
            switch_recorder: cx.new(|cx| HotkeyRecorder::new(HotkeyKind::Switch, window, cx)),
            convert_recorder: cx.new(|cx| HotkeyRecorder::new(HotkeyKind::Convert, window, cx)),
            macros: cx.new(|cx| MacroView::new(window, cx)),
            _subscriptions: subscriptions,
            _poll: poll,
        }
    }

    pub fn show(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        cx.notify();
    }

    /// Selects keep their own state; follow changes made elsewhere (tray,
    /// smart-switch, "Khôi phục mặc định").
    fn sync_selects(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = cx.global::<UiState>().settings;
        for (state, value) in [
            (&self.input_type, s.input_type),
            (&self.code_table, s.code_table),
            (&self.convert_from, s.convert_from_code),
            (&self.convert_to, s.convert_to_code),
        ] {
            let ix = IndexPath::new(usize::from(value));
            state.update(cx, |state, cx| {
                if state.selected_index(cx) != Some(ix) {
                    state.set_selected_index(Some(ix), window, cx);
                }
            });
        }
    }

    fn render_sidebar(&self, s: &Settings, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let vietnamese = s.language == 1;
        let chord = chord_text(HotkeyKind::Switch.get(s));
        let hint = if chord.is_empty() {
            sidebar::NO_SWITCH_HOTKEY.to_string()
        } else {
            format!("{chord} {}", sidebar::SWITCH_HINT_SUFFIX)
        };
        v_flex()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .px_3()
            // Clears the traffic lights of the transparent titlebar.
            .pt(px(44.))
            .pb_3()
            .gap_5()
            .child(
                h_flex()
                    .gap_2p5()
                    .px_1()
                    .child(logo(28.))
                    .child(
                        v_flex()
                            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(about_tab::APP_NAME))
                            .child(div().text_xs().text_color(p.muted).child(env!("CARGO_PKG_VERSION"))),
                    ),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .child(
                        language_switch(vietnamese, p),
                    )
                    .child(div().px_1().text_xs().text_color(p.muted).child(hint)),
            )
            .child(v_flex().gap(px(2.)).children(SECTIONS.iter().zip(strings::TABS).map(|((section, icon), label)| {
                let section = *section;
                nav_row(label, icon, self.section == section, p).on_click(cx.listener(move |this, _, _, cx| this.show(section, cx)))
            })))
            .child(div().flex_1())
            .when(!self.trusted_now, |this| {
                this.child(
                    v_flex()
                        .gap_1()
                        .p_2p5()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(p.notice_border)
                        .bg(p.notice_bg)
                        .text_xs()
                        .text_color(p.notice_ink)
                        .child(sidebar::PERMISSION_MISSING)
                        .child(
                            div()
                                .id("grant")
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .child(top_bar::GRANT_PERMISSION)
                                .on_click(|_, _, _| commands::open_accessibility_settings()),
                        ),
                )
            })
            .child(nav_row(sidebar::QUIT, "icons/power.svg", false, p).on_click(|_, _, cx| cx.quit()))
    }

    fn render_typing(&self, s: &Settings, p: &Palette, cx: &App) -> AnyElement {
        let subline = if s.language == 1 {
            format!(
                "{} {} · {}.",
                typing_view::ON_PREFIX,
                label_of(&strings::INPUT_TYPES, s.input_type),
                label_of(&strings::CODE_TABLES, s.code_table)
            )
        } else {
            match chord_text(HotkeyKind::Switch.get(s)) {
                chord if chord.is_empty() => typing_view::OFF.to_string(),
                chord => format!("{} {} {chord} {}", typing_view::OFF, typing_view::OFF_PRESS, typing_view::OFF_TO_TURN_ON),
            }
        };
        let preview = div()
            .py_3()
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(px(6.))
                    .bg(p.raised)
                    .children(previews(s.input_type).iter().map(|(keys, output)| transform_line(keys, *output, p, cx))),
            )
            .into_any_element();
        let beep = HotkeyKind::Switch.get(s).beep;

        v_flex()
            .gap_6()
            .child(title_row(strings::TABS[0], subline, None, p))
            .child(section(
                typing_view::METHOD,
                vec![
                    preview,
                    row(top_bar::INPUT_TYPE, Select::new(&self.input_type).small().w(px(SELECT_WIDTH)), p),
                    row(top_bar::CODE_TABLE, Select::new(&self.code_table).small().w(px(SELECT_WIDTH)), p),
                    row(top_bar::SWITCH_HOTKEY, hotkey_chips(HotkeyKind::Switch, &self.switch_recorder, s), p),
                    switch_row("beep", typing_view::SWITCH_BEEP, beep, |beep, cx| HotkeyKind::Switch.update(cx, |hk| hk.beep = beep), p),
                ],
                p,
            ))
            .child(section(
                typing_view::MARKS,
                vec![
                    field_row("modern_orthography", typing_tab::MODERN_ORTHOGRAPHY, s, |s| &mut s.modern_orthography, p),
                    field_row("check_spelling", typing_tab::CHECK_SPELLING, s, |s| &mut s.check_spelling, p),
                    field_row("restore_if_wrong_spelling", typing_tab::RESTORE_IF_WRONG_SPELLING, s, |s| {
                        &mut s.restore_if_wrong_spelling
                    }, p),
                    field_row("allow_consonant_zfwj", typing_tab::ALLOW_CONSONANT_ZFWJ, s, |s| &mut s.allow_consonant_zfwj, p),
                    field_row("upper_case_first_char", typing_tab::UPPER_CASE_FIRST_CHAR, s, |s| &mut s.upper_case_first_char, p),
                ],
                p,
            ))
            .child(section(
                typing_view::ENGLISH,
                vec![
                    field_row("auto_restore_english", typing_tab::AUTO_RESTORE_ENGLISH, s, |s| &mut s.auto_restore_english, p),
                    field_row("esc_restore_english", typing_tab::ESC_RESTORE_ENGLISH, s, |s| &mut s.esc_restore_english, p),
                ],
                p,
            ))
            .child(section(
                typing_view::PER_APP,
                vec![
                    field_row("use_smart_switch_key", typing_tab::USE_SMART_SWITCH_KEY, s, |s| &mut s.use_smart_switch_key, p),
                    field_row("remember_code", typing_tab::REMEMBER_CODE, s, |s| &mut s.remember_code, p),
                    field_row("other_language", typing_tab::OTHER_LANGUAGE, s, |s| &mut s.other_language, p),
                    field_row("fix_recommend_browser", typing_tab::FIX_RECOMMEND_BROWSER, s, |s| &mut s.fix_recommend_browser, p),
                ],
                p,
            ))
            .child(section(
                typing_view::TEMP_OFF,
                vec![
                    field_row("temp_off_spelling", typing_tab::TEMP_OFF_SPELLING, s, |s| &mut s.temp_off_spelling, p),
                    field_row("temp_off_engine", typing_tab::TEMP_OFF_ENGINE, s, |s| &mut s.temp_off_engine, p),
                ],
                p,
            ))
            .into_any_element()
    }

    fn render_convert(&self, s: &Settings, p: &Palette) -> AnyElement {
        let subline = format!(
            "{} {} {} {}.",
            convert_view::SUBLINE_PREFIX,
            label_of(&strings::CODE_TABLES, s.convert_from_code),
            convert_view::SUBLINE_TO,
            label_of(&strings::CODE_TABLES, s.convert_to_code)
        );
        let convert = Button::new("convert").primary().small().label(convert_view::CONVERT_CLIPBOARD).on_click(|_, window, cx| {
            let settings = cx.global::<UiState>().settings;
            if commands::convert_clipboard(&settings) {
                if !settings.convert_dont_alert {
                    window.push_notification(Notification::success(convert_tab::SUCCESS_SUB).title(convert_tab::SUCCESS_TITLE), cx);
                }
            } else {
                window.push_notification(Notification::error(convert_tab::FAIL_SUB).title(convert_tab::FAIL_TITLE), cx);
            }
        });
        let column = |title: &'static str, state: &Entity<Choices>| {
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(div().text_xs().text_color(p.muted).child(title))
                .child(Select::new(state).small())
        };
        let codes = h_flex()
            .items_end()
            .gap_2()
            .py_3()
            .child(column(convert_tab::FROM_CODE, &self.convert_from))
            .child(Button::new("reverse").ghost().small().label(convert_tab::REVERSE).on_click(|_, _, cx| {
                commands::apply_and_broadcast(cx, |mut s| {
                    std::mem::swap(&mut s.convert_from_code, &mut s.convert_to_code);
                    s
                });
            }))
            .child(column(convert_tab::TO_CODE, &self.convert_to))
            .into_any_element();
        let preview = div()
            .pt_3()
            .pb_2()
            .child(
                v_flex()
                    .gap_1()
                    .p_3()
                    .rounded(px(6.))
                    .bg(p.raised)
                    .child(div().text_sm().text_color(p.muted).child(convert_view::SAMPLE))
                    .child(div().text_size(px(18.)).font_weight(FontWeight::MEDIUM).text_color(p.brand).child(case_preview(s))),
            )
            .into_any_element();
        let cases = div()
            .pb_3()
            .child(
                segmented("case", &convert_view::CASES, case_index(s), |ix, cx| {
                    commands::apply_and_broadcast(cx, |s| set_case(s, ix));
                })
                .w_full(),
            )
            .into_any_element();

        v_flex()
            .gap_6()
            .child(title_row(strings::TABS[2], subline, Some(convert.into_any_element()), p))
            .child(section(convert_view::CODES, vec![codes], p))
            .child(section(
                convert_view::CASE,
                vec![
                    v_flex().child(preview).child(cases).into_any_element(),
                    field_row("remove_mark", convert_tab::REMOVE_MARK, s, |s| &mut s.convert_remove_mark, p),
                    switch_row(
                        "notify_when_done",
                        convert_tab::NOTIFY_WHEN_DONE,
                        !s.convert_dont_alert,
                        |v, cx| {
                            commands::apply_and_broadcast(cx, |mut s| {
                                s.convert_dont_alert = !v;
                                s
                            });
                        },
                        p,
                    ),
                ],
                p,
            ))
            .child(section(
                convert_view::HOTKEY,
                vec![row(convert_tab::QUICK_HOTKEY, hotkey_chips(HotkeyKind::Convert, &self.convert_recorder, s), p)],
                p,
            ))
            .into_any_element()
    }

    fn render_system(&self, s: &Settings, p: &Palette) -> AnyElement {
        let version = env!("CARGO_PKG_VERSION");
        let subline = if self.trusted_now {
            format!("VNKey {version} {}", system_view::GRANTED)
        } else {
            system_view::MISSING.to_string()
        };
        let permission = if self.trusted_now {
            pill(system_view::PILL_GRANTED, p.positive_bg, p.positive_ink).into_any_element()
        } else {
            h_flex()
                .gap_2()
                .child(pill(system_view::PILL_MISSING, p.notice_bg, p.notice_ink))
                .child(
                    Button::new("open-settings")
                        .small()
                        .label(permission_gate::OPEN_SETTINGS)
                        .on_click(|_, _, _| commands::open_accessibility_settings()),
                )
                .into_any_element()
        };
        let theme = segmented("theme", &system_tab::THEMES, usize::from(s.theme).min(2), |ix, cx| {
            commands::apply_and_broadcast(cx, |mut s| {
                s.theme = ix as u8;
                s
            });
        })
        .w(px(SELECT_WIDTH));
        // No release feed is wired up yet, so there is never a newer version.
        let check_update = Button::new("check-update").small().label(system_tab::CHECK_UPDATE).on_click(|_, window, cx| {
            window.push_notification(Notification::info(system_tab::UP_TO_DATE), cx);
        });
        let reset = Button::new("reset").danger().outline().small().label(system_view::RESET_BUTTON).on_click(|_, window, cx| {
            window.open_alert_dialog(cx, |alert, _, _| {
                alert
                    .title(system_view::RESET_CONFIRM_TITLE)
                    .description(system_view::RESET_EXPLAIN)
                    .confirm()
                    .ok_text(system_view::RESET_CONFIRM_OK)
                    .ok_variant(ButtonVariant::Danger)
                    .cancel_text(system_view::RESET_CONFIRM_CANCEL)
                    .on_ok(|_, _, cx| {
                        commands::apply_and_broadcast(cx, |_| Settings::default());
                        true
                    })
            });
        });
        v_flex()
            .gap_6()
            .child(title_row(strings::TABS[3], subline, None, p))
            .child(section(system_view::PERMISSION, vec![row(system_view::PERMISSION_ROW, permission, p)], p))
            .child(section(
                system_view::APPEARANCE,
                vec![
                    row(system_tab::THEME, theme, p),
                    field_row("show_icon_on_dock", system_tab::SHOW_ICON_ON_DOCK, s, |s| &mut s.show_icon_on_dock, p),
                    field_row("gray_icon", system_tab::GRAY_ICON, s, |s| &mut s.gray_icon, p),
                ],
                p,
            ))
            .child(section(
                system_view::STARTUP,
                vec![
                    field_row("run_on_startup", system_tab::RUN_ON_STARTUP, s, |s| &mut s.run_on_startup, p),
                    field_row("show_ui_on_startup", system_tab::SHOW_UI_ON_STARTUP, s, |s| &mut s.show_ui_on_startup, p),
                    switch_row(
                        "check_update",
                        system_tab::CHECK_UPDATE_ON_STARTUP,
                        !s.dont_check_update,
                        |v, cx| {
                            commands::apply_and_broadcast(cx, |mut s| {
                                s.dont_check_update = !v;
                                s
                            });
                        },
                        p,
                    ),
                    h_flex().py_2().child(check_update).into_any_element(),
                ],
                p,
            ))
            .child(section(
                system_view::COMPAT,
                vec![
                    field_row("send_key_step_by_step", system_tab::SEND_KEY_STEP_BY_STEP, s, |s| &mut s.send_key_step_by_step, p),
                    field_row("fix_chromium_browser", system_tab::FIX_CHROMIUM_BROWSER, s, |s| &mut s.fix_chromium_browser, p),
                    field_row("perform_layout_compat", system_tab::PERFORM_LAYOUT_COMPAT, s, |s| &mut s.perform_layout_compat, p),
                ],
                p,
            ))
            // Destructive, so last and on its own.
            .child(section(system_view::RESET, vec![row(system_view::RESET_EXPLAIN, reset, p)], p))
            .into_any_element()
    }
}

fn render_about(p: &Palette) -> AnyElement {
    let version = format!("{} {}", about_tab::VERSION_PREFIX, env!("CARGO_PKG_VERSION"));
    v_flex()
        .gap_6()
        .child(title_row(strings::TABS[4], about_tab::DESCRIPTION.to_string(), None, p))
        .child(panel(
            vec![
                h_flex()
                    .gap_3()
                    .py_3()
                    .child(logo(48.))
                    .child(
                        v_flex()
                            .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(about_tab::APP_NAME))
                            .child(div().text_sm().text_color(p.muted).child(version)),
                    )
                    .into_any_element(),
                div().py_3().text_sm().child(about_tab::LICENSE_NOTE).into_any_element(),
                h_flex()
                    .py_2()
                    .child(
                        Button::new("source-code")
                            .small()
                            .label(about_tab::SOURCE_CODE)
                            .on_click(|_, _, cx| cx.open_url(about_tab::SOURCE_CODE_URL)),
                    )
                    .into_any_element(),
            ],
            p,
        ))
        .into_any_element()
}

/// The first-run screen: the two steps that make VNKey able to type, with
/// the state of the first one polled live.
fn render_permission_gate(granted: bool, p: &Palette) -> impl IntoElement {
    let step = |done: bool, label: &'static str, action: AnyElement| {
        h_flex()
            .min_h(px(52.))
            .gap_3()
            .child(
                Icon::empty()
                    .path(if done { "icons/circle-check.svg" } else { "icons/circle.svg" })
                    .size_4()
                    .text_color(if done { p.brand } else { p.muted }),
            )
            .child(div().flex_1().text_sm().child(label))
            .child(action)
            .into_any_element()
    };
    let grant_action = if granted {
        pill(permission_gate::DONE, p.positive_bg, p.positive_ink).into_any_element()
    } else {
        Button::new("open-settings")
            .small()
            .label(permission_gate::OPEN_SETTINGS)
            .on_click(|_, _, _| commands::open_accessibility_settings())
            .into_any_element()
    };
    let restart = Button::new("restart")
        .small()
        .with_variant(if granted { ButtonVariant::Primary } else { ButtonVariant::Secondary })
        .label(permission_gate::RESTART_SHORT)
        .on_click(|_, _, cx| cx.restart())
        .into_any_element();

    v_flex().size_full().items_center().pt(px(72.)).px_6().child(
        v_flex()
            .w(px(440.))
            .gap_5()
            .child(logo(56.))
            .child(title_row(
                permission_gate::TITLE,
                if granted { permission_gate::GRANTED } else { permission_gate::EXPLAIN }.to_string(),
                None,
                p,
            ))
            .child(panel(
                vec![
                    step(granted, permission_gate::STEP_GRANT, grant_action),
                    step(false, permission_gate::STEP_RESTART, restart),
                ],
                p,
            )),
    )
}

/// Tiếng Việt | English, drawn here rather than with a `TabBar`: a tab's
/// prefix sits outside its selected pill, which pushed the dot off the segment.
fn language_switch(vietnamese: bool, p: &Palette) -> impl IntoElement {
    let p = *p;
    let segment = move |id: &'static str, selected: bool, language: u8| {
        h_flex()
            .id(id)
            .flex_1()
            .h(px(26.))
            .justify_center()
            .gap_1p5()
            .rounded(px(6.))
            .text_sm()
            .when(selected, |this| this.bg(p.surface).text_color(p.text).font_weight(FontWeight::MEDIUM))
            .when(!selected, |this| {
                let text = p.text;
                this.text_color(p.muted).cursor_pointer().hover(move |this| this.text_color(text))
            })
            .on_click(move |_, _, cx| {
                if !selected {
                    commands::set_language(cx, language);
                }
            })
    };
    // The one hue on the screen: Vietnamese is on.
    let dot = div().size(px(6.)).rounded_full().bg(if vietnamese { p.brand } else { p.muted });

    h_flex()
        .w_full()
        .p(px(2.))
        .gap(px(2.))
        .rounded(px(8.))
        .bg(p.active)
        .child(segment("vi", vietnamese, 1).child(dot).child(top_bar::VIETNAMESE))
        .child(segment("en", !vietnamese, 0).child(top_bar::ENGLISH))
}

fn nav_row(label: &'static str, icon: &'static str, active: bool, p: &Palette) -> Stateful<Div> {
    h_flex()
        .id(label)
        .h(px(32.))
        .px_2()
        .gap_2p5()
        .rounded(px(6.))
        .text_sm()
        .text_color(p.text)
        .cursor_pointer()
        .when(active, |this| this.bg(p.active).font_weight(FontWeight::MEDIUM))
        .when(!active, |this| {
            let hover = p.raised;
            this.hover(move |this| this.bg(hover))
        })
        .child(Icon::empty().path(icon).size_4().text_color(if active { p.text } else { p.muted }))
        .child(label)
}

fn pill(text: &'static str, bg: Hsla, ink: Hsla) -> impl IntoElement {
    div()
        .flex_none()
        .h(px(20.))
        .px_2()
        .flex()
        .items_center()
        .rounded_full()
        .bg(bg)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink)
        .child(text)
}

impl Render for ControlPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let root = h_flex().size_full().bg(p.ground).text_color(p.text);
        if !self.trusted_at_open {
            return root.child(render_permission_gate(self.trusted_now, &p));
        }

        let s = cx.global::<UiState>().settings;
        let view = match self.section {
            Section::Typing => self.render_typing(&s, &p, cx),
            Section::Macros => self.macros.clone().into_any_element(),
            Section::Convert => self.render_convert(&s, &p),
            Section::System => self.render_system(&s, &p),
            Section::About => render_about(&p),
        };
        root.child(self.render_sidebar(&s, &p, cx)).child(
            div()
                .id("canvas")
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_y_scroll()
                .child(v_flex().px_6().pt(px(40.)).pb_8().child(view)),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the `gpui_kit` glob would shadow `#[test]` with GPUI's.
    use super::{case_index, case_preview, set_case};
    use crate::settings::Settings;

    #[test]
    fn case_preview_shows_each_option_on_the_sample() {
        let base = Settings::default();
        let preview = |ix: usize, remove_mark: bool| case_preview(&Settings { convert_remove_mark: remove_mark, ..set_case(base, ix) });
        assert_eq!(preview(0, false), "xin chào. hôm nay trời đẹp");
        assert_eq!(preview(1, false), "XIN CHÀO. HÔM NAY TRỜI ĐẸP");
        assert_eq!(preview(3, false), "Xin chào. Hôm nay trời đẹp");
        assert_eq!(preview(4, false), "Xin Chào. Hôm Nay Trời Đẹp");
        assert_eq!(preview(0, true), "xin chao. hom nay troi dep");
    }

    #[test]
    fn case_options_are_mutually_exclusive() {
        let base = Settings::default();
        for ix in 0..5 {
            assert_eq!(case_index(&set_case(base, ix)), ix);
        }
    }
}
