//! Text injection: turning a `HookState` from the engine into the exact
//! sequence of synthetic events to post (backspaces, Unicode strings, key
//! re-emits), including the `sync_key` bookkeeping and per-app quirks.
//!
//! The injection primitives are abstracted behind [`EventSink`] so the whole
//! pipeline (`dispatch`, `SendNewCharString`, `SendBackspace`, encoding for all
//! five code tables, batch chunking, `sync_key` push/pop) is unit-testable with
//! a recording mock - no real `CGEventTapPostEvent` and no Accessibility grant.
//!
//! The one deliberate fix over the original is noted in the tap callback, not
//! here.
//!
//! `collapsible_if` is allowed: several nested `if`s mirror the original's
//! structure verbatim (e.g. `SendBackspace`'s `if (IS_DOUBLE_CODE) { if
//! (sync_key.back() > 1) {...} sync_key.pop_back(); }`), where the outer block
//! also runs code after the inner `if`, so collapsing would obscure the port.
#![allow(clippy::collapsible_if)]

use vnkey_engine::{
    CAPS_MASK, CHAR_CODE_MASK, CodeTable, HookCode, MAX_BUFF, PURE_CHARACTER_MASK, is_double_code,
    key_code_to_character,
};

use super::quirks::{AppQuirks, resolve_quirks};

/// Combining sac/huyen/hoi/nga/nang, used to decode the packed Unicode
/// Compound cells. Duplicated here (small, stable) so the engine crate stays
/// free of platform concerns.
const UNICODE_COMPOUND_MARK: [u16; 5] = [0x0301, 0x0300, 0x0309, 0x0303, 0x0323];

/// Max UTF-16 units per synthetic keyboard event (batched to stay under
/// CoreGraphics' per-event Unicode-string limit).
const CHUNK: usize = 16;

/// The abstraction over "post events to the focused app". The real
/// implementation posts CGEvents; tests use a recording mock.
pub trait EventSink {
    /// One Backspace keystroke (keycode 51, down+up).
    fn backspace(&mut self);
    /// One Shift+Left-Arrow keystroke (selection, down+up).
    fn shift_left_arrow(&mut self);
    /// A UTF-16 string as a synthetic keyboard event (down+up).
    fn unicode_string(&mut self, units: &[u16]);
    /// A raw virtual keycode (down+up), with optional Shift. Used to re-emit
    /// the original key on `Restore` and the trailing key after a macro.
    fn key_stroke(&mut self, keycode: u16, shift: bool);
}

/// The real [`EventSink`]: posts CGEvents from the tap's private source through
/// the callback proxy. Constructed fresh per keystroke inside the tap callback.
pub struct CgSink<'a> {
    source: &'a objc2_core_graphics::CGEventSource,
    proxy: objc2_core_graphics::CGEventTapProxy,
}

impl<'a> CgSink<'a> {
    pub fn new(
        source: &'a objc2_core_graphics::CGEventSource,
        proxy: objc2_core_graphics::CGEventTapProxy,
    ) -> Self {
        Self { source, proxy }
    }
}

impl EventSink for CgSink<'_> {
    fn backspace(&mut self) {
        // SAFETY: `self.proxy` is the live tap-callback proxy.
        unsafe { super::ffi::post_key(self.source, self.proxy, super::ffi::KEYCODE_BACKSPACE, false) }
    }
    fn shift_left_arrow(&mut self) {
        // SAFETY: `self.proxy` is the live tap-callback proxy.
        unsafe {
            super::ffi::post_key(self.source, self.proxy, super::ffi::KEYCODE_LEFT_ARROW, true)
        }
    }
    fn unicode_string(&mut self, units: &[u16]) {
        // SAFETY: `self.proxy` is the live tap-callback proxy.
        unsafe { super::ffi::post_unicode(self.source, self.proxy, units) }
    }
    fn key_stroke(&mut self, keycode: u16, shift: bool) {
        // SAFETY: `self.proxy` is the live tap-callback proxy.
        unsafe { super::ffi::post_key(self.source, self.proxy, keycode, shift) }
    }
}

/// A copy of the engine's `HookState` fields the injector needs, taken under
/// the engine lock so injection itself runs lock-free.
pub struct HookSnapshot {
    pub code: HookCode,
    pub backspace_count: u8,
    pub new_char_count: u8,
    pub ext_code: u8,
    pub char_data: [u32; MAX_BUFF],
    pub macro_data: Vec<u32>,
    /// Original event's virtual keycode (for `Restore`/macro re-emit).
    pub keycode: u16,
    /// `(Shift || CapsLock)` — used for the `Restore` re-emit's caps bit.
    pub caps: bool,
    /// `Shift` only - used for the macro trailing key's caps bit (checks Shift
    /// alone, not CapsLock).
    pub shift: bool,
}

/// Everything the injection pipeline needs besides the sink and the snapshot.
pub struct InjectCtx<'a> {
    pub code_table: CodeTable,
    /// The `sync_key` stack (owned by the tap thread).
    pub sync_key: &'a mut Vec<u8>,
    /// Frontmost app bundle id, read from the shared cache the main-thread
    /// NSWorkspace observer maintains (`workspace.rs`) — the tap thread never
    /// calls `NSWorkspace` itself (closes Task 8 finding 3). Not every dispatch
    /// branch needs quirks (a plain `DoNothing` key never does), so quirks are
    /// resolved lazily: [`InjectCtx::quirks`] runs `resolve_quirks` at most once
    /// per keystroke, only when a branch that reads quirks is reached, and
    /// caches the result in `quirks_cache`.
    pub bundle_id: Option<&'a str>,
    pub quirks_cache: Option<AppQuirks>,
    pub fix_recommend_browser: bool,
    pub fix_chromium_browser: bool,
    pub send_step_by_step: bool,
    /// Whether Spotlight is visible. Deferred to a later task; the callback
    /// passes `false` for now (see module docs / report TODO).
    pub spotlight_visible: bool,
}

impl InjectCtx<'_> {
    /// Per-app quirks for the frontmost app, resolved from the cached
    /// `bundle_id` on first access and cached for the rest of this dispatch
    /// call.
    fn quirks(&mut self) -> AppQuirks {
        if let Some(q) = self.quirks_cache {
            return q;
        }
        let q = resolve_quirks(self.bundle_id);
        self.quirks_cache = Some(q);
        q
    }

    fn should_use_recommend_workaround(&mut self) -> bool {
        if !self.fix_recommend_browser {
            return false;
        }
        if self.spotlight_visible {
            return false;
        }
        // Unknown app -> quirks all-false -> `!recommend_disabled` == true, so
        // an unknown app uses the workaround.
        !self.quirks().recommend_disabled
    }

    fn should_use_selection_replacement(&mut self) -> bool {
        self.spotlight_visible
            || self.quirks().recommend_disabled
            || self.quirks().selection_replacement
    }
}

/// Result of handling one KeyDown: whether to swallow the event (return NULL
/// from the tap) and whether the engine's `start_new_session` must be called
/// afterwards (for `RestoreAndStartNewSession`, which the engine defers to the
/// platform).
pub struct DispatchResult {
    pub swallow: bool,
    pub start_new_session: bool,
}

#[derive(Clone, Copy)]
struct RestoreKey {
    keycode: u16,
    caps: bool,
    rasns: bool,
}

/// Given the engine's decision for one KeyDown, drive the sink and the
/// `sync_key` stack, and report swallow / start-new-session.
pub fn dispatch<S: EventSink>(
    sink: &mut S,
    ctx: &mut InjectCtx,
    snap: &HookSnapshot,
) -> DispatchResult {
    match snap.code {
        HookCode::DoNothing => {
            // Only the 2-byte tables maintain `sync_key`.
            if is_double_code(ctx.code_table) {
                match snap.ext_code {
                    1 => ctx.sync_key.clear(), // break key
                    2 => {
                        // delete key
                        if let Some(&last) = ctx.sync_key.last() {
                            // Short-circuits before `ctx.quirks()` (the lazy
                            // NSWorkspace resolve) whenever `code_table ==
                            // VniWindows`, exactly like the eagerly-resolved
                            // version did -- so a VNI DoNothing-delete still
                            // never touches quirks, only Unicode Compound
                            // does.
                            if last > 1
                                && (ctx.code_table == CodeTable::VniWindows
                                    || !ctx.quirks().unicode_compound_app_prefix)
                            {
                                sink.backspace();
                            }
                            ctx.sync_key.pop();
                        }
                    }
                    3 => ctx.sync_key.push(1), // normal key
                    _ => {}
                }
            }
            DispatchResult {
                swallow: false,
                start_new_session: false,
            }
        }
        HookCode::WillProcess
        | HookCode::Restore
        | HookCode::RestoreAndStartNewSession
        | HookCode::RestoreAndSwallow => {
            let mut bpc = snap.backspace_count;

            // Autocomplete fix.
            if ctx.should_use_recommend_workaround() && snap.ext_code != 4 {
                if ctx.fix_chromium_browser && ctx.quirks().unicode_compound_app_exact {
                    // TODO(Task 9/10): chromium Shift+Left workaround. Off by
                    // default (fix_chromium_browser == false), so this branch is
                    // never taken in this task; kept for structural fidelity.
                    if bpc > 0 {
                        send_shift_left_arrow(sink, ctx);
                        if bpc == 1 {
                            bpc -= 1;
                        }
                    }
                } else {
                    send_empty_character(sink, ctx);
                    bpc += 1;
                }
            }

            // Selection replacement.
            if ctx.should_use_selection_replacement() && bpc > 0 {
                for _ in 0..bpc {
                    send_shift_left_arrow(sink, ctx);
                }
                bpc = 0;
            }

            // Backspaces.
            if bpc > 0 && (bpc as usize) < MAX_BUFF {
                for _ in 0..bpc {
                    send_backspace(sink, ctx);
                }
            }

            // New characters.
            let restore = match snap.code {
                HookCode::Restore => Some(RestoreKey {
                    keycode: snap.keycode,
                    caps: snap.caps,
                    rasns: false,
                }),
                HookCode::RestoreAndStartNewSession => Some(RestoreKey {
                    keycode: snap.keycode,
                    caps: snap.caps,
                    rasns: true,
                }),
                // Deliberately no `RestoreKey`: the ESC undo must not echo its
                // trigger, or a stray ESC reaches the app after the text is
                // already fixed. `start_new` is set explicitly below instead of
                // riding on `RestoreKey::rasns`.
                HookCode::RestoreAndSwallow => None,
                _ => None,
            };

            let mut start_new = false;
            if !ctx.send_step_by_step {
                start_new = send_new_char_string(
                    sink,
                    ctx,
                    &snap.char_data,
                    snap.new_char_count as usize,
                    restore,
                    0,
                );
                if snap.code == HookCode::RestoreAndSwallow {
                    start_new = true;
                }
            } else {
                let n = snap.new_char_count as usize;
                if n > 0 && n <= MAX_BUFF {
                    for i in (0..n).rev() {
                        send_key_code(sink, ctx, snap.char_data[i]);
                    }
                }
                if restore.is_some() {
                    let caps_bit = if snap.caps { CAPS_MASK } else { 0 };
                    send_key_code(sink, ctx, snap.keycode as u32 | caps_bit);
                }
                if matches!(
                    snap.code,
                    HookCode::RestoreAndStartNewSession | HookCode::RestoreAndSwallow
                ) {
                    start_new = true;
                }
            }

            DispatchResult {
                swallow: true,
                start_new_session: start_new,
            }
        }
        HookCode::ReplaceMacro => {
            handle_macro(sink, ctx, snap);
            DispatchResult {
                swallow: true,
                start_new_session: false,
            }
        }
        // BreakWord (== 2) is never emitted by `handle_event`; pass through.
        HookCode::BreakWord => DispatchResult {
            swallow: false,
            start_new_session: false,
        },
    }
}

/// The non-macro path: emit the transformed characters (reversed `char_data`),
/// chunked to `CHUNK` UTF-16 units per event, appending the re-emitted key for
/// `Restore*`. Returns whether `start_new_session` should run (for
/// `RestoreAndStartNewSession`).
fn send_new_char_string<S: EventSink>(
    sink: &mut S,
    ctx: &mut InjectCtx,
    cells: &[u32],
    count: usize,
    restore: Option<RestoreKey>,
    offset: usize,
) -> bool {
    let mut buf: Vec<u16> = Vec::with_capacity(CHUNK + 4);
    let mut will_continue = false;

    if count > 0 {
        // Reversed: `char_data[0]` is the rightmost char, so iterate high->low.
        let mut k = count as isize - 1 - offset as isize;
        while k >= 0 {
            if buf.len() >= CHUNK {
                will_continue = true;
                break;
            }
            encode_cell(ctx, cells[k as usize], &mut buf);
            k -= 1;
        }
    }

    let mut will_send_control_key = false;
    let mut start_new = false;
    if !will_continue {
        if let Some(rk) = restore {
            if key_code_to_character(rk.keycode as u32) != 0 {
                let caps_bit = if rk.caps { CAPS_MASK } else { 0 };
                let ch = key_code_to_character(rk.keycode as u32 | caps_bit) as u16;
                buf.push(ch);
            } else {
                will_send_control_key = true;
            }
            if rk.rasns {
                start_new = true;
            }
        }
    }

    // Post: exactly `CHUNK` units when continuing, else the whole buffer.
    let end = if will_continue {
        CHUNK.min(buf.len())
    } else {
        buf.len()
    };
    sink.unicode_string(&buf[..end]);

    if will_continue {
        start_new |= send_new_char_string(sink, ctx, cells, count, restore, offset + CHUNK);
    }
    if will_send_control_key {
        if let Some(rk) = restore {
            send_key_code(sink, ctx, rk.keycode as u32);
        }
    }
    start_new
}

/// Encode one `char_data`/`macro_data` cell into `buf` (UTF-16 units) for the
/// active code table, pushing `sync_key` lengths.
fn encode_cell(ctx: &mut InjectCtx, temp: u32, buf: &mut Vec<u16>) {
    if temp & PURE_CHARACTER_MASK != 0 {
        buf.push(temp as u16);
        if is_double_code(ctx.code_table) {
            ctx.sync_key.push(1);
        }
    } else if temp & CHAR_CODE_MASK == 0 {
        // A raw keycode: map back to its ASCII character.
        if is_double_code(ctx.code_table) {
            ctx.sync_key.push(1);
        }
        buf.push(key_code_to_character(temp) as u16);
    } else {
        match ctx.code_table {
            CodeTable::Unicode => buf.push(temp as u16),
            CodeTable::Tcvn3 | CodeTable::VniWindows | CodeTable::Cp1258 => {
                let full = temp as u16;
                let hi = (full >> 8) & 0xFF;
                let lo = full & 0xFF;
                buf.push(lo);
                if hi > 32 {
                    if ctx.code_table == CodeTable::VniWindows {
                        ctx.sync_key.push(2);
                    }
                    buf.push(hi);
                } else if ctx.code_table == CodeTable::VniWindows {
                    ctx.sync_key.push(1);
                }
            }
            CodeTable::UnicodeCompound => {
                let full = temp as u16;
                let hi = full >> 13;
                let lo = full & 0x1FFF;
                ctx.sync_key.push(if hi > 0 { 2 } else { 1 });
                buf.push(lo);
                if hi > 0 {
                    buf.push(UNICODE_COMPOUND_MARK[(hi - 1) as usize]);
                }
            }
        }
    }
}

/// Emit a single cell either as a raw keystroke (keycode cell) or as encoded
/// Unicode (char-code cell). Used for step-by-step mode, the `Restore`
/// control-key re-emit, and macros.
fn send_key_code<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx, data: u32) {
    let new_char = data as u16;
    if data & CHAR_CODE_MASK == 0 {
        if is_double_code(ctx.code_table) {
            ctx.sync_key.push(1);
        }
        sink.key_stroke(new_char, data & CAPS_MASK != 0);
        return;
    }
    match ctx.code_table {
        CodeTable::Unicode => sink.unicode_string(&[new_char]),
        CodeTable::Tcvn3 | CodeTable::VniWindows | CodeTable::Cp1258 => {
            let hi = (new_char >> 8) & 0xFF;
            let lo = new_char & 0xFF;
            sink.unicode_string(&[lo]);
            if hi > 32 {
                if ctx.code_table == CodeTable::VniWindows {
                    ctx.sync_key.push(2);
                }
                sink.unicode_string(&[hi]);
            } else if ctx.code_table == CodeTable::VniWindows {
                ctx.sync_key.push(1);
            }
        }
        CodeTable::UnicodeCompound => {
            let hi = new_char >> 13;
            let lo = new_char & 0x1FFF;
            ctx.sync_key.push(if hi > 0 { 2 } else { 1 });
            if hi > 0 {
                sink.unicode_string(&[lo, UNICODE_COMPOUND_MARK[(hi - 1) as usize]]);
            } else {
                sink.unicode_string(&[lo]);
            }
        }
    }
}

/// One backspace, plus a second for a 2-unit character on the 2-byte tables
/// (suppressed for Unicode Compound in a compound-hostile app), then pop
/// `sync_key`.
fn send_backspace<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx) {
    sink.backspace();
    if is_double_code(ctx.code_table) {
        // Guard against an empty stack: `sync_key.back()` would be UB if empty.
        // In normal operation the stack depth tracks what we sent, so it is
        // non-empty exactly when it needs to be read.
        if let Some(&last) = ctx.sync_key.last() {
            if last > 1
                && !(ctx.code_table == CodeTable::UnicodeCompound
                    && ctx.quirks().unicode_compound_app_prefix)
            {
                sink.backspace();
            }
            ctx.sync_key.pop();
        }
    }
}

/// Like `send_backspace` but with Shift+Left (extend selection) instead of
/// Backspace.
fn send_shift_left_arrow<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx) {
    sink.shift_left_arrow();
    if is_double_code(ctx.code_table) {
        if let Some(&last) = ctx.sync_key.last() {
            if last > 1
                && !(ctx.code_table == CodeTable::UnicodeCompound
                    && ctx.quirks().unicode_compound_app_prefix)
            {
                sink.shift_left_arrow();
            }
            ctx.sync_key.pop();
        }
    }
}

/// Send the "empty" character (ZWNJ or NARROW NO-BREAK SPACE per app quirk).
fn send_empty_character<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx) {
    if is_double_code(ctx.code_table) {
        ctx.sync_key.push(1);
    }
    let ch: u16 = if ctx.quirks().nice_space { 0x200C } else { 0x202F };
    sink.unicode_string(&[ch]);
}

/// Send a "pure" character (passed through verbatim), tracking `sync_key`.
fn send_pure_character<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx, ch: u16) {
    sink.unicode_string(&[ch]);
    if is_double_code(ctx.code_table) {
        ctx.sync_key.push(1);
    }
}

/// Expand and emit a macro. Not exercised by default (no macros are loaded in
/// this task), but ported for fidelity.
fn handle_macro<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx, snap: &HookSnapshot) {
    let mut bpc = snap.backspace_count;
    if ctx.should_use_recommend_workaround() {
        send_empty_character(sink, ctx);
        bpc += 1;
    }
    for _ in 0..bpc {
        send_backspace(sink, ctx);
    }
    if !ctx.send_step_by_step {
        send_macro_char_string(sink, ctx, &snap.macro_data);
    } else {
        for &cell in &snap.macro_data {
            if cell & PURE_CHARACTER_MASK != 0 {
                send_pure_character(sink, ctx, cell as u16);
            } else {
                send_key_code(sink, ctx, cell);
            }
        }
    }
    // Trailing key - Shift only, not CapsLock.
    let caps_bit = if snap.shift { CAPS_MASK } else { 0 };
    send_key_code(sink, ctx, snap.keycode as u32 | caps_bit);
}

/// `SendNewCharString` macro path (`dataFromMacro = true`): forward iteration,
/// no key re-emit, chunked to `CHUNK` units, resuming at the exact cell index.
fn send_macro_char_string<S: EventSink>(sink: &mut S, ctx: &mut InjectCtx, data: &[u32]) {
    let mut i = 0;
    while i < data.len() {
        let mut buf: Vec<u16> = Vec::with_capacity(CHUNK + 4);
        while i < data.len() {
            if buf.len() >= CHUNK {
                break;
            }
            encode_cell(ctx, data[i], &mut buf);
            i += 1;
        }
        let end = CHUNK.min(buf.len());
        sink.unicode_string(&buf[..end]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vnkey_engine::{get_character_code, keys};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum SinkOp {
        Backspace,
        ShiftLeft,
        Unicode(Vec<u16>),
        KeyStroke { keycode: u16, shift: bool },
    }

    #[derive(Default)]
    struct MockSink {
        ops: Vec<SinkOp>,
    }
    impl EventSink for MockSink {
        fn backspace(&mut self) {
            self.ops.push(SinkOp::Backspace);
        }
        fn shift_left_arrow(&mut self) {
            self.ops.push(SinkOp::ShiftLeft);
        }
        fn unicode_string(&mut self, units: &[u16]) {
            self.ops.push(SinkOp::Unicode(units.to_vec()));
        }
        fn key_stroke(&mut self, keycode: u16, shift: bool) {
            self.ops.push(SinkOp::KeyStroke { keycode, shift });
        }
    }

    fn ctx<'a>(code_table: CodeTable, sync_key: &'a mut Vec<u8>) -> InjectCtx<'a> {
        InjectCtx {
            code_table,
            sync_key,
            bundle_id: None,
            quirks_cache: None,
            fix_recommend_browser: false,
            fix_chromium_browser: false,
            send_step_by_step: false,
            spotlight_visible: false,
        }
    }

    /// Build a `char_data` cell the way the engine does: `get_character_code`
    /// for the given key+mods against a code table.
    fn cell(code_table: CodeTable, data: u32) -> u32 {
        get_character_code(code_table, data)
    }

    /// The transformed cell for lowercase 'ệ' (e + circumflex + nặng).
    fn e_nang(code_table: CodeTable) -> u32 {
        use vnkey_engine::{MARK5_MASK, TONE_MASK};
        cell(code_table, keys::KEY_E as u32 | TONE_MASK | MARK5_MASK)
    }

    /// The transformed cell for lowercase 'đ'.
    fn d_stroke(code_table: CodeTable) -> u32 {
        use vnkey_engine::TONE_MASK;
        cell(code_table, keys::KEY_D as u32 | TONE_MASK)
    }

    fn only_unicode(ops: &[SinkOp]) -> Vec<Vec<u16>> {
        ops.iter()
            .filter_map(|o| match o {
                SinkOp::Unicode(u) => Some(u.clone()),
                _ => None,
            })
            .collect()
    }

    // -- encoding of 'ệ' across all five code tables (hand-computed) --

    #[test]
    fn encode_e_nang_unicode() {
        let mut sk = Vec::new();
        let mut ctx = ctx(CodeTable::Unicode, &mut sk);
        let mut buf = Vec::new();
        encode_cell(&mut ctx, e_nang(CodeTable::Unicode), &mut buf);
        assert_eq!(buf, vec![0x1EC7]); // ệ, single unit
        assert!(sk.is_empty()); // Unicode never touches _syncKey
    }

    #[test]
    fn encode_e_nang_tcvn3() {
        let mut sk = Vec::new();
        let mut ctx = ctx(CodeTable::Tcvn3, &mut sk);
        let mut buf = Vec::new();
        encode_cell(&mut ctx, e_nang(CodeTable::Tcvn3), &mut buf);
        // KEY_E|TONE|MARK5 row index 9 = 0xD6, HIBYTE 0 -> single unit.
        assert_eq!(buf, vec![0xD6]);
        assert!(sk.is_empty());
    }

    #[test]
    fn encode_e_nang_vni_windows_two_units_and_sync() {
        let mut sk = Vec::new();
        let mut ctx = ctx(CodeTable::VniWindows, &mut sk);
        let mut buf = Vec::new();
        encode_cell(&mut ctx, e_nang(CodeTable::VniWindows), &mut buf);
        // 0xE465: LOBYTE 'e' (0x65) then HIBYTE 0xE4 (>32).
        assert_eq!(buf, vec![0x65, 0xE4]);
        assert_eq!(sk, vec![2]); // VNI pushes length 2 for a 2-unit cell
    }

    #[test]
    fn encode_e_nang_unicode_compound_base_plus_mark() {
        let mut sk = Vec::new();
        let mut ctx = ctx(CodeTable::UnicodeCompound, &mut sk);
        let mut buf = Vec::new();
        encode_cell(&mut ctx, e_nang(CodeTable::UnicodeCompound), &mut buf);
        // 0xA0EA: hi = 5, base = 0x00EA (ê), mark = UNICODE_COMPOUND_MARK[4]=0x0323.
        assert_eq!(buf, vec![0x00EA, 0x0323]);
        assert_eq!(sk, vec![2]);
    }

    #[test]
    fn encode_e_nang_cp1258_two_units_no_sync() {
        let mut sk = Vec::new();
        let mut ctx = ctx(CodeTable::Cp1258, &mut sk);
        let mut buf = Vec::new();
        encode_cell(&mut ctx, e_nang(CodeTable::Cp1258), &mut buf);
        // 0xF2EA: LOBYTE 0xEA then HIBYTE 0xF2 (>32). CP1258 is not a double
        // code, so _syncKey stays empty.
        assert_eq!(buf, vec![0xEA, 0xF2]);
        assert!(sk.is_empty());
    }

    // -- encoding of 'đ' across all five code tables --

    #[test]
    fn encode_d_stroke_all_tables() {
        let cases = [
            (CodeTable::Unicode, vec![0x0111u16], vec![]),
            (CodeTable::Tcvn3, vec![0xAE], vec![]),
            (CodeTable::VniWindows, vec![0xF1], vec![1u8]), // 1-unit -> sync 1
            (CodeTable::UnicodeCompound, vec![0x0111], vec![1u8]),
            (CodeTable::Cp1258, vec![0xF0], vec![]),
        ];
        for (table, want_units, want_sync) in cases {
            let mut sk = Vec::new();
            let mut c = ctx(table, &mut sk);
            let mut buf = Vec::new();
            encode_cell(&mut c, d_stroke(table), &mut buf);
            assert_eq!(buf, want_units, "{table:?} units");
            assert_eq!(sk, want_sync, "{table:?} sync");
        }
    }

    // -- keycode cell (no CHAR_CODE_MASK) round-trips through keyCodeToCharacter --

    #[test]
    fn encode_raw_keycode_cell_maps_to_ascii() {
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut buf = Vec::new();
        // A bare KEY_V keycode cell -> 'v'.
        encode_cell(&mut c, keys::KEY_V as u32, &mut buf);
        assert_eq!(buf, vec![b'v' as u16]);
    }

    // -- full word "việt" via dispatch (Unicode, the default path) --

    #[test]
    fn dispatch_viet_unicode_backspaces_then_string() {
        // char_data is reversed: [0]=t (rightmost) .. [3]=v (leftmost).
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = cell(CodeTable::Unicode, keys::KEY_T as u32); // t (raw-ish)
        char_data[1] = e_nang(CodeTable::Unicode); // ệ
        char_data[2] = cell(CodeTable::Unicode, keys::KEY_I as u32); // i
        char_data[3] = cell(CodeTable::Unicode, keys::KEY_V as u32); // v
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 2,
            new_char_count: 4,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        let res = dispatch(&mut sink, &mut c, &snap);
        assert!(res.swallow);
        assert!(!res.start_new_session);
        // 2 backspaces, then one Unicode event "việt" (left-to-right).
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Backspace,
                SinkOp::Backspace,
                SinkOp::Unicode("việt".encode_utf16().collect()),
            ]
        );
    }

    // -- batch chunking: >16 units splits into 16 + remainder --

    #[test]
    fn chunking_splits_long_word_into_16_plus_remainder() {
        // 20 single-unit Unicode cells 'a'..'a' (value 0x61).
        let n = 20usize;
        let mut char_data = [0u32; MAX_BUFF];
        for slot in char_data.iter_mut().take(n) {
            *slot = 0x61 | CHAR_CODE_MASK; // 'a' as a char-code cell
        }
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 0,
            new_char_count: n as u8,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        let chunks = only_unicode(&sink.ops);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 16);
        assert_eq!(chunks[1].len(), 4);
        assert!(chunks[0].iter().all(|&u| u == 0x61));
        assert!(chunks[1].iter().all(|&u| u == 0x61));
    }

    // -- _syncKey stack: VNI DoNothing normal/delete/break maintenance --

    #[test]
    fn sync_key_do_nothing_maintenance_vni() {
        let mut sk = Vec::new();
        // ext_code 3 (normal key) pushes 1 three times.
        for _ in 0..3 {
            let mut c = ctx(CodeTable::VniWindows, &mut sk);
            let snap = do_nothing(3);
            let mut sink = MockSink::default();
            dispatch(&mut sink, &mut c, &snap);
            assert!(sink.ops.is_empty());
        }
        assert_eq!(sk, vec![1, 1, 1]);

        // ext_code 2 (delete): pop last; last was 1 so no extra backspace.
        {
            let mut c = ctx(CodeTable::VniWindows, &mut sk);
            let mut sink = MockSink::default();
            dispatch(&mut sink, &mut c, &do_nothing(2));
            assert!(sink.ops.is_empty());
        }
        assert_eq!(sk, vec![1, 1]);

        // Manually make the top a 2-unit entry, then delete -> one backspace.
        sk.push(2);
        {
            let mut c = ctx(CodeTable::VniWindows, &mut sk);
            let mut sink = MockSink::default();
            dispatch(&mut sink, &mut c, &do_nothing(2));
            assert_eq!(sink.ops, vec![SinkOp::Backspace]);
        }
        assert_eq!(sk, vec![1, 1]);

        // ext_code 1 (break) clears the stack.
        {
            let mut c = ctx(CodeTable::VniWindows, &mut sk);
            let mut sink = MockSink::default();
            dispatch(&mut sink, &mut c, &do_nothing(1));
            assert!(sink.ops.is_empty());
        }
        assert!(sk.is_empty());
    }

    #[test]
    fn sync_key_untouched_for_unicode_table() {
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &do_nothing(3));
        assert!(sk.is_empty());
        assert!(sink.ops.is_empty());
    }

    // -- backspace with VNI 2-unit sync pops and doubles --

    #[test]
    fn send_backspace_doubles_for_two_unit_vni_cell() {
        let mut sk = vec![1u8, 2u8]; // top = 2 (a 2-unit char)
        let mut c = ctx(CodeTable::VniWindows, &mut sk);
        let mut sink = MockSink::default();
        send_backspace(&mut sink, &mut c);
        assert_eq!(sink.ops, vec![SinkOp::Backspace, SinkOp::Backspace]);
        assert_eq!(sk, vec![1]);
        // Next backspace: top = 1, single backspace.
        let mut c = ctx(CodeTable::VniWindows, &mut sk);
        send_backspace(&mut sink, &mut c);
        assert_eq!(
            sink.ops,
            vec![SinkOp::Backspace, SinkOp::Backspace, SinkOp::Backspace]
        );
        assert!(sk.is_empty());
    }

    #[test]
    fn send_backspace_single_for_unicode() {
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        send_backspace(&mut sink, &mut c);
        assert_eq!(sink.ops, vec![SinkOp::Backspace]);
        assert!(sk.is_empty());
    }

    // -- empty character quirk (nice space vs default) --

    #[test]
    fn empty_character_uses_nice_space_for_sublime() {
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.bundle_id = Some("com.sublimetext.3");
        let mut sink = MockSink::default();
        send_empty_character(&mut sink, &mut c);
        assert_eq!(sink.ops, vec![SinkOp::Unicode(vec![0x200C])]);
    }

    #[test]
    fn empty_character_default_char() {
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        send_empty_character(&mut sink, &mut c);
        assert_eq!(sink.ops, vec![SinkOp::Unicode(vec![0x202F])]);
    }

    // -- recommend-browser workaround: empty char + extra backspace --

    #[test]
    fn recommend_workaround_inserts_empty_char_and_extra_backspace() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = 0x61 | CHAR_CODE_MASK; // 'a'
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 1,
            new_char_count: 1,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.fix_recommend_browser = true; // ordinary app -> workaround active
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        // empty char (0x202F), then 1+1 = 2 backspaces, then the 'a'.
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Unicode(vec![0x202F]),
                SinkOp::Backspace,
                SinkOp::Backspace,
                SinkOp::Unicode(vec![0x61]),
            ]
        );
    }

    /// Chrome must rewrite by selection, never by Backspace: the omnibox's
    /// inline autocomplete swallows the first Backspace of a rewrite, so typing
    /// `dd` (-> `đ`) left `dđ` on screen. The empty character still goes first,
    /// to collapse any live autocomplete selection to a caret before Shift+Left
    /// starts extending from it.
    #[test]
    fn chrome_rewrites_by_selection_not_backspace() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = d_stroke(CodeTable::Unicode); // đ
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 1,
            new_char_count: 1,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.bundle_id = Some("com.google.Chrome");
        c.fix_recommend_browser = true;
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        // empty char, then 1+1 = 2 Shift+Lefts selecting "d" + the empty char,
        // then "đ" typed over the selection. No Backspace anywhere.
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Unicode(vec![0x202F]),
                SinkOp::ShiftLeft,
                SinkOp::ShiftLeft,
                SinkOp::Unicode(vec![0x0111]),
            ]
        );
    }

    /// The reported `gift` case: at the `f` the engine asks to replace `gi`
    /// with `gì` (backspace_count 2). Chrome used to render `ggì` because one
    /// deletion was lost.
    #[test]
    fn chrome_multi_char_rewrite_selects_every_char() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = 0x00EC | CHAR_CODE_MASK; // ì (rightmost)
        char_data[1] = 0x0067 | CHAR_CODE_MASK; // g
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 2,
            new_char_count: 2,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.bundle_id = Some("com.google.Chrome");
        c.fix_recommend_browser = true;
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Unicode(vec![0x202F]),
                SinkOp::ShiftLeft,
                SinkOp::ShiftLeft,
                SinkOp::ShiftLeft,
                SinkOp::Unicode("gì".encode_utf16().collect()),
            ]
        );
    }

    /// Safari is not Chromium and keeps the plain backspace path, so the
    /// browser quirk stays scoped to the apps that actually need it.
    #[test]
    fn safari_still_uses_backspaces() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = d_stroke(CodeTable::Unicode);
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 1,
            new_char_count: 1,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.bundle_id = Some("com.apple.Safari");
        c.fix_recommend_browser = true;
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Unicode(vec![0x202F]),
                SinkOp::Backspace,
                SinkOp::Backspace,
                SinkOp::Unicode(vec![0x0111]),
            ]
        );
    }

    #[test]
    fn recommend_workaround_skipped_when_ext_code_4() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = 0x61 | CHAR_CODE_MASK;
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 0,
            new_char_count: 1,
            ext_code: 4, // "should not send empty character"
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.fix_recommend_browser = true;
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        assert_eq!(sink.ops, vec![SinkOp::Unicode(vec![0x61])]);
    }

    // -- Restore re-emits the original key after the transformed chars --

    #[test]
    fn restore_reemits_letter_key() {
        // Simulate a Restore where the last key is 's' (maps to ASCII 's').
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = 0x61 | CHAR_CODE_MASK; // 'a'
        let snap = HookSnapshot {
            code: HookCode::Restore,
            backspace_count: 1,
            new_char_count: 1,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: keys::KEY_S,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        let res = dispatch(&mut sink, &mut c, &snap);
        assert!(res.swallow);
        // one backspace, then "a" + re-emitted 's' in one Unicode string.
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Backspace,
                SinkOp::Unicode(vec![0x61, b's' as u16]),
            ]
        );
    }

    #[test]
    fn restore_and_start_new_session_flag() {
        let snap = HookSnapshot {
            code: HookCode::RestoreAndStartNewSession,
            backspace_count: 0,
            new_char_count: 0,
            ext_code: 3,
            char_data: [0u32; MAX_BUFF],
            macro_data: Vec::new(),
            keycode: keys::KEY_A,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        let res = dispatch(&mut sink, &mut c, &snap);
        assert!(res.start_new_session);
    }

    // -- Restore with a control key (TAB) re-emits it as a keystroke --

    #[test]
    fn restore_control_key_reemitted_as_keystroke() {
        let snap = HookSnapshot {
            code: HookCode::Restore,
            backspace_count: 0,
            new_char_count: 0,
            ext_code: 3,
            char_data: [0u32; MAX_BUFF],
            macro_data: Vec::new(),
            keycode: keys::KEY_TAB, // no ASCII mapping -> control key
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        // Empty unicode post (no chars) then the TAB keystroke.
        assert_eq!(
            sink.ops,
            vec![
                SinkOp::Unicode(vec![]),
                SinkOp::KeyStroke {
                    keycode: keys::KEY_TAB,
                    shift: false
                },
            ]
        );
    }

    // -- step-by-step mode emits one event per cell --

    #[test]
    fn step_by_step_emits_per_cell() {
        let mut char_data = [0u32; MAX_BUFF];
        char_data[0] = 0x62 | CHAR_CODE_MASK; // 'b' (rightmost)
        char_data[1] = 0x61 | CHAR_CODE_MASK; // 'a' (leftmost)
        let snap = HookSnapshot {
            code: HookCode::WillProcess,
            backspace_count: 0,
            new_char_count: 2,
            ext_code: 3,
            char_data,
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        };
        let mut sk = Vec::new();
        let mut c = ctx(CodeTable::Unicode, &mut sk);
        c.send_step_by_step = true;
        let mut sink = MockSink::default();
        dispatch(&mut sink, &mut c, &snap);
        // leftmost first: 'a' then 'b'.
        assert_eq!(
            sink.ops,
            vec![SinkOp::Unicode(vec![0x61]), SinkOp::Unicode(vec![0x62])]
        );
    }

    fn do_nothing(ext_code: u8) -> HookSnapshot {
        HookSnapshot {
            code: HookCode::DoNothing,
            backspace_count: 0,
            new_char_count: 0,
            ext_code,
            char_data: [0u32; MAX_BUFF],
            macro_data: Vec::new(),
            keycode: 0,
            caps: false,
            shift: false,
        }
    }
}
