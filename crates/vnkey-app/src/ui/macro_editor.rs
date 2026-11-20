//! The "Gõ tắt" view: the macro table and its options.
//!
//! One pair of fields doubles as the add form and the edit form: clicking a
//! row loads it into the fields, and the primary button reads "Thêm" or "Sửa"
//! depending on whether the typed key already exists. There is no separate
//! edit mode.

use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::strings::{footer, macro_editor as text, macro_tab, macro_view};
use super::{Palette, field_row, palette, panel, section, title_row};
use crate::app::UiState;
use crate::commands;

const ROW_HEIGHT: f32 = 36.;
/// Six and a half rows: the half row says the list goes on (macOS hides
/// scrollbars until scrolling), and the form below stays in view.
const TABLE_MAX_HEIGHT: f32 = ROW_HEIGHT * 6.5;
const KEY_COLUMN: f32 = 140.;

pub struct MacroView {
    macros: Vec<(String, String)>,
    macros_rev: u64,
    key: Entity<InputState>,
    content: Entity<TextareaState>,
    scroll: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl MacroView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let key = cx.new(|cx| InputState::new(window, cx));
        let content = cx.new(|cx| TextareaState::new(window, cx));
        let _subscriptions = vec![
            cx.observe_global::<UiState>(|this, cx| {
                let rev = cx.global::<UiState>().macros_rev;
                if rev != this.macros_rev {
                    this.macros_rev = rev;
                    this.macros = commands::macro_list(cx);
                }
                cx.notify();
            }),
            // The "Thêm"/"Sửa" label and the selected row follow the typed key.
            cx.subscribe(&key, |_, _, _: &InputEvent, cx| cx.notify()),
        ];
        Self {
            macros: commands::macro_list(cx),
            macros_rev: cx.global::<UiState>().macros_rev,
            key,
            content,
            scroll: UniformListScrollHandle::new(),
            _subscriptions,
        }
    }

    fn typed_key(&self, cx: &App) -> String {
        self.key.read(cx).value().to_string()
    }

    fn set_fields(&self, key: &str, content: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.key.update(cx, |state, cx| state.set_value(key.to_string(), window, cx));
        self.content.update(cx, |state, cx| state.set_value(content.to_string(), window, cx));
        cx.notify();
    }

    fn add_or_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.typed_key(cx);
        let content = self.content.read(cx).value().to_string();
        if key.is_empty() || content.is_empty() {
            alert(text::ADD_NEEDS_BOTH_FIELDS, window, cx);
            return;
        }
        commands::macro_add(cx, &key, &content);
        self.set_fields("", "", window, cx);
    }

    fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.typed_key(cx);
        if key.is_empty() {
            alert(text::DELETE_NEEDS_SELECTION, window, cx);
            return;
        }
        commands::macro_delete(cx, &key);
        self.set_fields("", "", window, cx);
    }

    fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |_, cx| {
            let Ok(Ok(Some(paths))) = picked.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let _ = cx.update(|window, cx| ask_keep_or_replace(path, window, cx));
        })
        .detach();
    }

    fn export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let home = dirs::home_dir().unwrap_or_default();
        let picked = cx.prompt_for_new_path(&home, Some(text::EXPORT_DEFAULT_NAME));
        cx.spawn_in(window, async move |_, cx| {
            let Ok(Ok(Some(path))) = picked.await else { return };
            let _ = cx.update(|window, cx| {
                if let Err(err) = commands::macro_export(cx, &path) {
                    alert(&err, window, cx);
                }
            });
        })
        .detach();
    }

    fn render_table(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let p = *p;
        let typed = self.typed_key(cx);
        let mono = cx.theme().mono_font_family.clone();
        let header = h_flex()
            .h(px(36.))
            .text_xs()
            .text_color(p.muted)
            .child(div().w(px(KEY_COLUMN)).child(text::KEY_COLUMN))
            .child(div().flex_1().child(text::CONTENT_COLUMN));

        let body = if self.macros.is_empty() {
            div()
                .pb_3()
                .text_sm()
                .text_color(p.muted)
                .child(macro_view::EMPTY)
                .into_any_element()
        } else {
            // The list is capped in height, so the scrollbar says there is more.
            let list = uniform_list(
                "macros",
                self.macros.len(),
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    range
                        .map(|ix| {
                            let (key, content) = &this.macros[ix];
                            let selected = *key == typed;
                            // Rows bleed 8px past the text column so the
                            // selected fill frames the text, not touches it.
                            h_flex()
                                .id(ix)
                                .h(px(ROW_HEIGHT))
                                .mx(px(-8.))
                                .px_2()
                                .rounded(px(6.))
                                .text_sm()
                                .text_color(p.text)
                                .cursor_pointer()
                                .when(selected, |row| row.bg(p.active))
                                .when(!selected, |row| row.hover(|row| row.bg(p.raised)))
                                .child(div().w(px(KEY_COLUMN)).pr_3().truncate().font_family(mono.clone()).child(key.clone()))
                                .child(div().flex_1().min_w_0().truncate().child(content.clone()))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let (key, content) = this.macros[ix].clone();
                                    this.set_fields(&key, &content, window, cx);
                                }))
                        })
                        .collect()
                }),
            )
            .size_full()
            .track_scroll(&self.scroll);
            // Shrinks to a short table instead of leaving a blank band.
            let height = (self.macros.len() as f32 * ROW_HEIGHT).min(TABLE_MAX_HEIGHT);
            div().relative().h(px(height)).child(list).vertical_scrollbar(&self.scroll).into_any_element()
        };

        v_flex().child(header).child(body)
    }

    fn render_form(&self, is_edit: bool, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let label = |title: &'static str| div().text_xs().text_color(p.muted).child(title);
        h_flex()
            .items_start()
            .gap_3()
            .py_3()
            .child(v_flex().w(px(KEY_COLUMN - 12.)).gap_1().child(label(text::KEY_COLUMN)).child(Input::new(&self.key).small()))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(label(text::CONTENT_COLUMN))
                    .child(Textarea::new(&self.content).small().h(px(56.))),
            )
            .child(
                v_flex()
                    .pt(px(18.))
                    .gap_2()
                    .child(
                        Button::new("add")
                            .primary()
                            .small()
                            .w(px(64.))
                            .label(if is_edit { text::EDIT } else { text::ADD })
                            .on_click(cx.listener(|this, _, window, cx| this.add_or_edit(window, cx))),
                    )
                    // Only an existing key can be deleted.
                    .when(is_edit, |this| {
                        this.child(
                            Button::new("delete")
                                .danger()
                                .outline()
                                .small()
                                .w(px(64.))
                                .label(text::DELETE)
                                .on_click(cx.listener(|this, _, window, cx| this.delete(window, cx))),
                        )
                    }),
            )
    }
}

/// Picked a file: keep the current table (append) or replace it? Escape, the
/// close button and the backdrop abort the import, since replacing wipes every
/// current macro.
fn ask_keep_or_replace(path: PathBuf, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, _| {
        let (keep, replace) = (path.clone(), path.clone());
        dialog.title(text::IMPORT_DIALOG_TITLE).child(text::IMPORT_KEEP_DATA_QUESTION).footer(
            h_flex()
                .gap_2()
                .justify_end()
                .child(Button::new("replace").label(footer::NO).on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    import(&replace, false, window, cx);
                }))
                .child(Button::new("keep").primary().label(footer::YES).on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    import(&keep, true, window, cx);
                })),
        )
    });
}

fn import(path: &std::path::Path, append: bool, window: &mut Window, cx: &mut App) {
    if let Err(err) = commands::macro_import(cx, path, append) {
        alert(&err, window, cx);
    }
}

fn alert(message: &str, window: &mut Window, cx: &mut App) {
    window.push_notification(Notification::error(message.to_string()).title(text::ALERT_TITLE), cx);
}

impl Render for MacroView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = cx.global::<UiState>().settings;
        let p = palette(cx);
        let is_edit = {
            let typed = self.typed_key(cx);
            !typed.is_empty() && self.macros.iter().any(|(key, _)| *key == typed)
        };

        let mut subline = if self.macros.is_empty() {
            macro_view::NONE_YET.to_string()
        } else {
            format!("{} {}", self.macros.len(), macro_view::COUNT_SUFFIX)
        };
        if !s.use_macro {
            subline = format!("{subline} {}", macro_view::DISABLED);
        }
        let actions = h_flex()
            .gap_2()
            .child(
                Button::new("import")
                    .small()
                    .label(text::IMPORT_FILE)
                    .on_click(cx.listener(|this, _, window, cx| this.import(window, cx))),
            )
            .child(
                Button::new("export")
                    .small()
                    .label(text::EXPORT_FILE)
                    .on_click(cx.listener(|this, _, window, cx| this.export(window, cx))),
            )
            .into_any_element();

        let table = self.render_table(&p, cx);
        let form = self.render_form(is_edit, &p, cx);

        v_flex()
            .gap_6()
            .child(title_row(super::strings::TABS[1], subline, Some(actions), &p))
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(p.text).child(macro_view::TABLE))
                    .child(panel(vec![table.into_any_element(), form.into_any_element()], &p)),
            )
            .child(section(
                macro_view::OPTIONS,
                vec![
                    field_row("use_macro", macro_tab::USE_MACRO, &s, |s| &mut s.use_macro, &p),
                    field_row("use_macro_in_english_mode", macro_tab::USE_MACRO_IN_ENGLISH_MODE, &s, |s| {
                        &mut s.use_macro_in_english_mode
                    }, &p),
                    field_row("auto_caps_macro", macro_tab::AUTO_CAPS_MACRO, &s, |s| &mut s.auto_caps_macro, &p),
                ],
                &p,
            ))
            .child(section(
                macro_view::QUICK_TYPING,
                vec![
                    field_row("quick_telex", macro_tab::QUICK_TELEX, &s, |s| &mut s.quick_telex, &p),
                    field_row("quick_start_consonant", macro_tab::QUICK_START_CONSONANT, &s, |s| {
                        &mut s.quick_start_consonant
                    }, &p),
                    field_row("quick_end_consonant", macro_tab::QUICK_END_CONSONANT, &s, |s| &mut s.quick_end_consonant, &p),
                ],
                &p,
            ))
    }
}
