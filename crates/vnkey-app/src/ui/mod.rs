//! The GPUI window: one Control Panel with a sidebar of five views.
//!
//! The UI is a passive mirror: every change goes through
//! `commands::apply_and_broadcast`, which replaces the `UiState` global, and
//! the window redraws by observing that global. Changes from the tray, the
//! hotkey or smart-switch therefore show up without any per-window sync code.
//!
//! The window is opened on demand and destroyed when closed (macOS keeps the
//! app running with none open); asking for it while open brings it forward and
//! switches to the requested view.
//!
//! Look: the "achromatic ledger" app style (`.taste-skill/notes.md`): ink
//! chrome, white panels on an off-white ground, one hue -- the VNKey icon's
//! green -- kept for the live typing state.

mod control_panel;
mod hotkey;
mod macro_editor;
mod strings;

use std::rc::Rc;

use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, Theme, ThemeMode, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

pub use control_panel::Section;

use crate::app::UiState;
use crate::settings::Settings;

// The default icon set covers gpui-kit's own icons; these are the extra
// Lucide icons the sidebar and the setup steps use.
gpui_kit::assets::icon_assets!(ExtraIcons, [Keyboard, TextCursorInput, ArrowLeftRight, Power, Circle, CircleCheck]);

/// gpui-kit's default assets plus [`ExtraIcons`].
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match ExtraIcons.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        Ok(paths)
    }
}

/// The Control Panel window, if open.
#[derive(Default)]
struct OpenWindows {
    settings: Option<(AnyWindowHandle, WeakEntity<control_panel::ControlPanel>)>,
}

impl Global for OpenWindows {}

pub fn open_settings(cx: &mut App) {
    open_at(None, cx);
}

/// The status-bar menu's "Gõ tắt..." opens the panel on its macro view.
pub fn open_macro(cx: &mut App) {
    open_at(Some(Section::Macros), cx);
}

fn open_at(section: Option<Section>, cx: &mut App) {
    // A menu-bar (Accessory) app is not frontmost when its menu is used.
    cx.activate(true);
    if let Some((handle, panel)) = cx.default_global::<OpenWindows>().settings.clone()
        && handle.update(cx, |_, window, _| window.activate_window()).is_ok()
    {
        if let Some(section) = section {
            let _ = panel.update(cx, |panel, cx| panel.show(section, cx));
        }
        return;
    }

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(760.), px(560.)), cx))),
        // The sidebar runs up under the traffic lights, so the window is one
        // ground in either theme instead of a system-coloured bar over it.
        titlebar: Some(TitlebarOptions {
            title: Some(strings::SETTINGS_WINDOW_TITLE.into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        window_min_size: Some(size(px(680.), px(480.))),
        ..Default::default()
    };
    let section = section.unwrap_or(Section::Typing);
    match gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| control_panel::ControlPanel::new(section, window, cx))
    }) {
        Ok((handle, panel)) => cx.default_global::<OpenWindows>().settings = Some((handle, panel.downgrade())),
        Err(err) => eprintln!("VNKey: failed to open window: {err}"),
    }
}

/// `Settings::theme`: 0 follows macOS, 1 light, 2 dark.
pub fn apply_theme(theme: u8, cx: &mut App) {
    match theme {
        1 => Theme::change(ThemeMode::Light, None, cx),
        2 => Theme::change(ThemeMode::Dark, None, cx),
        _ => Theme::sync_system_appearance(None, cx),
    }
}

/// Keep following macOS light/dark changes while `theme` is "auto". The
/// window's root view holds the returned subscription.
fn follow_system_appearance<V: 'static>(window: &mut Window, cx: &mut Context<V>) -> Subscription {
    cx.observe_window_appearance(window, |_, window, cx| {
        if cx.global::<UiState>().settings.theme == 0 {
            Theme::sync_system_appearance(Some(window), cx);
        }
    })
}

/// The screen's own colours, beside the gpui-kit theme that styles the
/// controls. Every text tier reaches 4.5:1 on the surface it sits on.
#[derive(Clone, Copy)]
struct Palette {
    ground: Hsla,
    surface: Hsla,
    /// A well inside a panel (the previews).
    raised: Hsla,
    /// The selected sidebar row and table row.
    active: Hsla,
    border: Hsla,
    rule: Hsla,
    control: Hsla,
    text: Hsla,
    muted: Hsla,
    /// The VNKey icon's green, darkened to 4.9:1 on white; the live state only.
    brand: Hsla,
    notice_bg: Hsla,
    notice_border: Hsla,
    notice_ink: Hsla,
    positive_bg: Hsla,
    positive_ink: Hsla,
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

fn palette(cx: &App) -> Palette {
    if cx.theme().mode.is_dark() {
        Palette {
            ground: hex(0x000000),
            surface: hex(0x111111),
            raised: hex(0x1a1a1a),
            active: hex(0x222222),
            border: hex(0x2e2e2e),
            rule: hex(0x1f1f1f),
            control: hex(0x444444),
            text: hex(0xffffff),
            muted: hex(0xb4b4b4),
            brand: hex(0x28b888),
            notice_bg: hex(0x2a2112),
            notice_border: hex(0x5c4515),
            notice_ink: hex(0xf2c46d),
            positive_bg: hex(0x172a1d),
            positive_ink: hex(0x3fc07a),
        }
    } else {
        Palette {
            ground: hex(0xf8f8f8),
            surface: hex(0xffffff),
            raised: hex(0xf4f4f4),
            active: hex(0xebebeb),
            border: hex(0xe6e6e6),
            rule: hex(0xf0f0f0),
            control: hex(0xd4d4d4),
            text: hex(0x0a0a0a),
            muted: hex(0x6b6b6b),
            brand: hex(0x168060),
            notice_bg: hex(0xfffbeb),
            notice_border: hex(0xfde68a),
            notice_ink: hex(0x92400e),
            positive_bg: hex(0xdcfae5),
            positive_ink: hex(0x04250e),
        }
    }
}

/// The view's title and one sentence about the current state, with the
/// view's real actions (if any) at the right.
fn title_row(title: &'static str, subline: String, actions: Option<AnyElement>, p: &Palette) -> impl IntoElement {
    h_flex()
        .items_start()
        .justify_between()
        .gap_4()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(div().text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).text_color(p.text).child(title))
                .child(div().text_sm().text_color(p.muted).child(subline)),
        )
        .children(actions.map(|actions| div().flex_none().pt_1().child(actions)))
}

/// A heading over a white panel whose rows are split by faint rules.
fn section(heading: &'static str, rows: Vec<AnyElement>, p: &Palette) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(p.text).child(heading))
        .child(panel(rows, p))
}

fn panel(rows: Vec<AnyElement>, p: &Palette) -> Div {
    v_flex()
        .px_4()
        .bg(p.surface)
        .border_1()
        .border_color(p.border)
        .rounded(px(8.))
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(ix, row)| div().when(ix > 0, |d| d.border_t_1().border_color(p.rule)).child(row)),
        )
}

/// A labelled row with its control at the right.
fn row(label: &'static str, control: impl IntoElement, p: &Palette) -> AnyElement {
    h_flex()
        .min_h(px(44.))
        .py_2()
        .gap_4()
        .justify_between()
        // `min_w_0` lets a long label wrap instead of pushing the control out.
        .child(div().flex_1().min_w_0().text_sm().text_color(p.text).child(label))
        .child(div().flex_none().child(control))
        .into_any_element()
}

fn switch_row(
    id: &'static str,
    label: &'static str,
    checked: bool,
    on_change: impl Fn(bool, &mut App) + 'static,
    p: &Palette,
) -> AnyElement {
    row(
        label,
        Switch::new(id).small().checked(checked).on_change(move |checked, _, cx| on_change(*checked, cx)),
        p,
    )
}

/// One `bool` field of [`Settings`], for [`field_row`].
type BoolField = fn(&mut Settings) -> &mut bool;

/// A switch row bound to one settings field.
fn field_row(id: &'static str, label: &'static str, settings: &Settings, field: BoolField, p: &Palette) -> AnyElement {
    let mut current = *settings;
    switch_row(
        id,
        label,
        *field(&mut current),
        move |checked, cx| {
            crate::commands::apply_and_broadcast(cx, |mut s| {
                *field(&mut s) = checked;
                s
            });
        },
        p,
    )
}

/// A segmented choice (grey track, white selected segment).
fn segmented(
    id: &'static str,
    labels: &[&'static str],
    selected: usize,
    on_select: impl Fn(usize, &mut App) + 'static,
) -> TabBar {
    let on_select = Rc::new(on_select);
    TabBar::new(id)
        .segmented()
        .small()
        .selected_index(selected)
        .on_click(move |ix: &usize, _, cx| on_select(*ix, cx))
        .children(labels.iter().map(|label| Tab::new().label(*label).flex_1()))
}

/// A key drawn as a keycap: the object VNKey reads.
fn keycap(key: impl Into<SharedString>, p: &Palette, cx: &App) -> impl IntoElement {
    div()
        .min_w(px(22.))
        .h(px(24.))
        .px_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1()
        .border_b_2()
        .border_color(p.control)
        .bg(p.surface)
        .font_family(cx.theme().mono_font_family.clone())
        .text_xs()
        .text_color(p.text)
        .child(key.into())
}

/// Keys as keycaps, an arrow, and what VNKey turns them into.
fn transform_line(keys: &str, output: impl Into<SharedString>, p: &Palette, cx: &App) -> impl IntoElement {
    h_flex()
        .gap_1()
        .children(keys.chars().map(|c| keycap(c.to_string(), p, cx)))
        .child(div().px_2().text_sm().text_color(p.muted).child("→"))
        .child(div().text_size(px(20.)).font_weight(FontWeight::MEDIUM).text_color(p.brand).child(output.into()))
}

/// The app icon, decoded once.
fn logo(size_px: f32) -> impl IntoElement {
    use std::sync::{Arc, LazyLock};
    static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
        Arc::new(Image::from_bytes(ImageFormat::Png, include_bytes!("../../resources/icon.png").to_vec()))
    });
    img(LOGO.clone()).size(px(size_px))
}
