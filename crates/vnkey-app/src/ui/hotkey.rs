//! The hotkey row shared by the switch key (top bar) and the quick-convert key
//! (Convert tab): four modifier chips plus a recorder for the non-modifier key.
//!
//! The packed `u32` layout is shared with the tap side (`platform/mac/hotkey.rs`
//! decodes it), so both ends must agree exactly:
//!   0x100 Control, 0x200 Option, 0x400 Command, 0x800 Shift, 0x8000 beep,
//!   low byte = macOS virtual keycode, top byte = display character.
//! [`MODIFIER_ONLY_KEYCODE`] in both bytes means "no key, modifiers only";
//! [`SPACE_KEYCODE`] is likewise stored in both bytes for Space.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::platform::mac::layout::char_to_qwerty_keycode;
use crate::settings::Settings;

pub const MODIFIER_ONLY_KEYCODE: u8 = 0xFE;
pub const SPACE_KEYCODE: u8 = 0x31;

const CTRL_BIT: u32 = 0x100;
const OPT_BIT: u32 = 0x200;
const CMD_BIT: u32 = 0x400;
const SHIFT_BIT: u32 = 0x800;
const BEEP_BIT: u32 = 0x8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub opt: bool,
    pub cmd: bool,
    pub shift: bool,
    pub beep: bool,
    pub keycode: u8,
    pub char: u8,
}

impl Hotkey {
    pub fn decode(packed: u32) -> Self {
        Self {
            ctrl: packed & CTRL_BIT != 0,
            opt: packed & OPT_BIT != 0,
            cmd: packed & CMD_BIT != 0,
            shift: packed & SHIFT_BIT != 0,
            beep: packed & BEEP_BIT != 0,
            keycode: (packed & 0xFF) as u8,
            char: (packed >> 24) as u8,
        }
    }

    pub fn encode(self) -> u32 {
        let mut packed = u32::from(self.keycode) | (u32::from(self.char) << 24);
        for (on, bit) in
            [(self.ctrl, CTRL_BIT), (self.opt, OPT_BIT), (self.cmd, CMD_BIT), (self.shift, SHIFT_BIT), (self.beep, BEEP_BIT)]
        {
            if on {
                packed |= bit;
            }
        }
        packed
    }
}

/// Non-printable keys a hotkey can use: GPUI key name, macOS virtual keycode
/// (`kVK_*`), display name. Their keycode is also packed as the "char" byte, so
/// display goes through this table instead of reading it as a character.
const NAMED_KEYS: [(&str, u8, &str); 23] = [
    ("enter", 0x24, "Enter"),
    ("tab", 0x30, "Tab"),
    ("escape", 0x35, "Escape"),
    ("left", 0x7B, "←"),
    ("right", 0x7C, "→"),
    ("down", 0x7D, "↓"),
    ("up", 0x7E, "↑"),
    ("home", 0x73, "Home"),
    ("end", 0x77, "End"),
    ("pageup", 0x74, "Page Up"),
    ("pagedown", 0x79, "Page Down"),
    ("f1", 0x7A, "F1"),
    ("f2", 0x78, "F2"),
    ("f3", 0x63, "F3"),
    ("f4", 0x76, "F4"),
    ("f5", 0x60, "F5"),
    ("f6", 0x61, "F6"),
    ("f7", 0x62, "F7"),
    ("f8", 0x64, "F8"),
    ("f9", 0x65, "F9"),
    ("f10", 0x6D, "F10"),
    ("f11", 0x67, "F11"),
    ("f12", 0x6F, "F12"),
];

/// What the recorder shows for a packed display character.
pub fn key_display_text(char: u8) -> String {
    match char {
        SPACE_KEYCODE => "Space".into(),
        MODIFIER_ONLY_KEYCODE => String::new(),
        _ => NAMED_KEYS
            .iter()
            .find(|(_, code, _)| *code == char)
            .map_or_else(|| char::from(char).to_string(), |(_, _, name)| (*name).into()),
    }
}

/// The chord as the user reads it: modifier glyphs, then the key ("⌃ Space",
/// "⌘⇧ Z", "⌃⇧" for modifiers only). Empty when nothing is set.
pub fn chord_text(hotkey: Hotkey) -> String {
    let mods: String = [(hotkey.ctrl, '⌃'), (hotkey.opt, '⌥'), (hotkey.cmd, '⌘'), (hotkey.shift, '⇧')]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, glyph)| *glyph)
        .collect();
    let key = key_display_text(hotkey.char);
    match (mods.is_empty(), key.is_empty()) {
        (_, true) => mods,
        (true, false) => key,
        (false, false) => format!("{mods} {key}"),
    }
}

/// GPUI key name -> the `(keycode, char)` pair to store, or `None` to ignore
/// the key. GPUI gives no raw keycode, so a printable key goes through the same
/// character -> QWERTY-position table the layout-compat path uses.
pub fn record_key(key: &str) -> Option<(u8, u8)> {
    match key {
        "space" => return Some((SPACE_KEYCODE, SPACE_KEYCODE)),
        "backspace" | "delete" => return Some((MODIFIER_ONLY_KEYCODE, MODIFIER_ONLY_KEYCODE)),
        _ => {}
    }
    if let Some((_, code, _)) = NAMED_KEYS.iter().find(|(name, _, _)| *name == key) {
        return Some((*code, *code));
    }
    let mut chars = key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else { return None };
    if !c.is_ascii() {
        return None;
    }
    let keycode = u8::try_from(char_to_qwerty_keycode(c.to_ascii_lowercase())?).ok()?;
    Some((keycode, c.to_ascii_uppercase() as u8))
}

/// Which of the two packed hotkeys in [`Settings`] a row edits.
#[derive(Debug, Clone, Copy)]
pub enum HotkeyKind {
    Switch,
    Convert,
}

impl HotkeyKind {
    pub fn get(self, s: &Settings) -> Hotkey {
        Hotkey::decode(match self {
            Self::Switch => s.switch_key_status,
            Self::Convert => s.convert_hotkey,
        })
    }

    /// Edit this hotkey through the one config-change funnel.
    pub fn update(self, cx: &mut App, edit: impl FnOnce(&mut Hotkey)) {
        crate::commands::apply_and_broadcast(cx, |s| {
            let mut hotkey = self.get(&s);
            edit(&mut hotkey);
            match self {
                Self::Switch => crate::commands::set_switch_hotkey_settings(s, hotkey.encode()),
                Self::Convert => crate::commands::set_convert_hotkey_settings(s, hotkey.encode()),
            }
        });
    }
}

/// Captures the non-modifier key of a chord. While focused the CGEventTap
/// passes every key through untouched (`set_hotkey_recording`), so the chord
/// reaches this window instead of being swallowed or rewritten.
pub struct HotkeyRecorder {
    kind: HotkeyKind,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl HotkeyRecorder {
    pub fn new(kind: HotkeyKind, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let _subscriptions = vec![
            cx.on_focus(&focus, window, |_, _, cx| crate::commands::set_hotkey_recording(cx, true)),
            cx.on_blur(&focus, window, |_, _, cx| crate::commands::set_hotkey_recording(cx, false)),
            // Switching to another app keeps this focused inside its window but
            // must not leave every keystroke there untransformed.
            // (Two recorders share a window: only the focused one may turn it on.)
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    crate::commands::set_hotkey_recording(cx, false);
                } else if this.focus.is_focused(window) {
                    crate::commands::set_hotkey_recording(cx, true);
                }
            }),
            // Closing the window while recording never blurs.
            cx.on_release(|_, cx| crate::commands::set_hotkey_recording(cx, false)),
        ];
        Self { kind, focus, _subscriptions }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        // Keep Tab/Escape from also moving focus or closing a dialog.
        cx.stop_propagation();
        if let Some((keycode, char)) = record_key(&event.keystroke.key) {
            self.kind.update(cx, |hotkey| {
                hotkey.keycode = keycode;
                hotkey.char = char;
            });
        }
    }
}

impl Render for HotkeyRecorder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = cx.global::<crate::app::UiState>().settings;
        let text = key_display_text(self.kind.get(&settings).char);
        let focused = self.focus.is_focused(window);
        let theme = cx.theme();
        let p = super::palette(cx);

        div()
            .id("hotkey-recorder")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .min_w(px(64.))
            .h(px(28.))
            .px_2()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .border_1()
            .border_b_2()
            .border_color(if focused { p.text } else { p.control })
            .bg(p.surface)
            .font_family(theme.mono_font_family.clone())
            .text_sm()
            .text_color(if text.is_empty() { p.muted } else { p.text })
            .child(if text.is_empty() { "…".to_string() } else { text })
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.focus, cx);
            }))
    }
}

const MODIFIERS: [(&str, &str); 4] = [("ctrl", "⌃"), ("opt", "⌥"), ("cmd", "⌘"), ("shift", "⇧")];

fn modifier_flag<'a>(hotkey: &'a mut Hotkey, name: &str) -> &'a mut bool {
    match name {
        "ctrl" => &mut hotkey.ctrl,
        "opt" => &mut hotkey.opt,
        "cmd" => &mut hotkey.cmd,
        _ => &mut hotkey.shift,
    }
}

/// The modifier chips followed by the recorder.
pub fn hotkey_chips(kind: HotkeyKind, recorder: &Entity<HotkeyRecorder>, settings: &Settings) -> impl IntoElement {
    let mut hotkey = kind.get(settings);
    h_flex()
        .gap_1()
        .children(MODIFIERS.map(|(name, glyph)| {
            let on = *modifier_flag(&mut hotkey, name);
            Button::new(SharedString::from(format!("{kind:?}-{name}")))
                .small()
                .label(glyph)
                .when(on, |b| b.primary())
                .when(!on, |b| b.outline())
                .on_click(move |_, _, cx| kind.update(cx, |hk| *modifier_flag(hk, name) ^= true))
        }))
        .child(recorder.clone())
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the `gpui_kit` glob would shadow `#[test]` with GPUI's.
    use super::{
        BEEP_BIT, CMD_BIT, CTRL_BIT, Hotkey, MODIFIER_ONLY_KEYCODE, SPACE_KEYCODE, chord_text, key_display_text, record_key,
    };
    use crate::settings::Settings;

    #[test]
    fn decode_encode_round_trip() {
        let hotkey = Hotkey { ctrl: true, opt: false, cmd: true, shift: false, beep: true, keycode: 0x06, char: b'Z' };
        assert_eq!(Hotkey::decode(hotkey.encode()), hotkey);
        assert_eq!(hotkey.encode(), 0x06 | (u32::from(b'Z') << 24) | CTRL_BIT | CMD_BIT | BEEP_BIT);
    }

    #[test]
    fn decodes_the_default_switch_hotkey() {
        let hotkey = Hotkey::decode(Settings::default().switch_key_status);
        assert_eq!(Hotkey::decode(hotkey.encode()).encode(), Settings::default().switch_key_status);
    }

    #[test]
    fn record_key_maps_printables_to_qwerty_positions() {
        assert_eq!(record_key("z"), Some((6, b'Z')));
        assert_eq!(record_key("1"), Some((18, b'1')));
        assert_eq!(record_key("/"), Some((44, b'/')));
    }

    #[test]
    fn record_key_special_keys() {
        assert_eq!(record_key("space"), Some((SPACE_KEYCODE, SPACE_KEYCODE)));
        assert_eq!(record_key("backspace"), Some((MODIFIER_ONLY_KEYCODE, MODIFIER_ONLY_KEYCODE)));
        assert_eq!(record_key("delete"), Some((MODIFIER_ONLY_KEYCODE, MODIFIER_ONLY_KEYCODE)));
        assert_eq!(record_key("f1"), Some((0x7A, 0x7A)));
        assert_eq!(record_key("enter"), Some((0x24, 0x24)));
        assert_eq!(record_key("insert"), None);
        assert_eq!(record_key("ư"), None);
    }

    #[test]
    fn chord_text_reads_like_the_menu_bar() {
        let space = Hotkey { ctrl: true, opt: false, cmd: false, shift: false, beep: false, keycode: SPACE_KEYCODE, char: SPACE_KEYCODE };
        assert_eq!(chord_text(space), "⌃ Space");
        let mods_only = Hotkey { shift: true, keycode: MODIFIER_ONLY_KEYCODE, char: MODIFIER_ONLY_KEYCODE, ..space };
        assert_eq!(chord_text(mods_only), "⌃⇧");
        let bare = Hotkey { ctrl: false, char: b'Z', keycode: 6, ..space };
        assert_eq!(chord_text(bare), "Z");
        let none = Hotkey { ctrl: false, keycode: MODIFIER_ONLY_KEYCODE, char: MODIFIER_ONLY_KEYCODE, ..space };
        assert_eq!(chord_text(none), "");
    }

    #[test]
    fn display_text() {
        assert_eq!(key_display_text(SPACE_KEYCODE), "Space");
        assert_eq!(key_display_text(MODIFIER_ONLY_KEYCODE), "");
        assert_eq!(key_display_text(0x24), "Enter");
        assert_eq!(key_display_text(0x7A), "F1");
        assert_eq!(key_display_text(b'Z'), "Z");
    }
}
