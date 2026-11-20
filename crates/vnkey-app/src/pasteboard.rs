//! Clipboard (NSPasteboard) access for the convert-tool.
//!
//! Pure logic ([`quick_convert`]) is separated from the NSPasteboard-backed
//! I/O ([`SystemClipboard`]) via the [`Clipboard`] trait, so `quick_convert`
//! is unit-testable with a mock. The convert operation reads the pasteboard's
//! string + HTML representations, runs `convert_util` over whichever are
//! present, and writes the results back.

use vnkey_engine::{convert_util, ConvertOptions};

/// What's currently on the pasteboard, as the two representations
/// `quick_convert` reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipboardContents {
    pub string: Option<String>,
    pub html: Option<String>,
}

/// Clipboard read/write, abstracted so [`quick_convert`] doesn't need a live
/// NSPasteboard to test.
pub trait Clipboard {
    fn read(&self) -> ClipboardContents;
    fn write(&self, contents: &ClipboardContents);
}

/// Convert the string and HTML
/// clipboard representations independently (each only if present) and write
/// the results back. Returns `false` (no-op) if the clipboard had neither
/// representation.
pub fn quick_convert(clipboard: &impl Clipboard, options: &ConvertOptions) -> bool {
    let contents = clipboard.read();
    if contents.string.is_none() && contents.html.is_none() {
        return false;
    }
    let converted = ClipboardContents {
        string: contents.string.as_deref().map(|s| convert_util(s, options)),
        html: contents.html.as_deref().map(|s| convert_util(s, options)),
    };
    clipboard.write(&converted);
    true
}

/// The real, NSPasteboard-backed [`Clipboard`].
#[cfg(target_os = "macos")]
pub struct SystemClipboard;

#[cfg(target_os = "macos")]
impl Clipboard for SystemClipboard {
    fn read(&self) -> ClipboardContents {
        mac::read()
    }

    fn write(&self, contents: &ClipboardContents) {
        mac::write(contents)
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
    use objc2_foundation::NSString;

    use super::ClipboardContents;

    pub fn read() -> ClipboardContents {
        let pb = NSPasteboard::generalPasteboard();
        // SAFETY: `NSPasteboardTypeString`/`NSPasteboardTypeHTML` are
        // framework-provided extern statics, valid for the process lifetime.
        let (string_type, html_type) = unsafe { (NSPasteboardTypeString, NSPasteboardTypeHTML) };
        ClipboardContents {
            string: pb.stringForType(string_type).map(|s| s.to_string()),
            html: pb.stringForType(html_type).map(|s| s.to_string()),
        }
    }

    pub fn write(contents: &ClipboardContents) {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        // SAFETY: see `read`.
        let (string_type, html_type) = unsafe { (NSPasteboardTypeString, NSPasteboardTypeHTML) };
        if let Some(s) = &contents.string {
            pb.setString_forType(&NSString::from_str(s), string_type);
        }
        if let Some(h) = &contents.html {
            pb.setString_forType(&NSString::from_str(h), html_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use vnkey_engine::CodeTable;

    use super::*;

    /// An in-memory mock clipboard for testing `quick_convert` without a
    /// live NSPasteboard.
    #[derive(Default)]
    struct MockClipboard {
        contents: RefCell<ClipboardContents>,
    }

    impl MockClipboard {
        fn seeded(contents: ClipboardContents) -> Self {
            Self { contents: RefCell::new(contents) }
        }
    }

    impl Clipboard for MockClipboard {
        fn read(&self) -> ClipboardContents {
            self.contents.borrow().clone()
        }

        fn write(&self, contents: &ClipboardContents) {
            *self.contents.borrow_mut() = contents.clone();
        }
    }

    fn tcvn3_to_unicode() -> ConvertOptions {
        ConvertOptions {
            from_code: CodeTable::Tcvn3,
            to_code: CodeTable::Unicode,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn quick_convert_converts_string_only() {
        // Plain ASCII round-trips unchanged through any code-table pair, so
        // this exercises the "string present, html absent" branch without
        // hand-encoding TCVN3 bytes.
        let clipboard = MockClipboard::seeded(ClipboardContents {
            string: Some("abc".to_string()),
            html: None,
        });
        let converted = quick_convert(&clipboard, &tcvn3_to_unicode());
        assert!(converted);
        let after = clipboard.read();
        assert_eq!(after.string.as_deref(), Some("abc"));
        assert_eq!(after.html, None);
    }

    #[test]
    fn quick_convert_converts_string_and_html_independently() {
        let clipboard = MockClipboard::seeded(ClipboardContents {
            string: Some("plain".to_string()),
            html: Some("<b>bold</b>".to_string()),
        });
        let converted = quick_convert(&clipboard, &ConvertOptions::default());
        assert!(converted);
        let after = clipboard.read();
        assert_eq!(after.string.as_deref(), Some("plain"));
        assert_eq!(after.html.as_deref(), Some("<b>bold</b>"));
    }

    #[test]
    fn quick_convert_empty_clipboard_is_a_no_op() {
        let clipboard = MockClipboard::default();
        let converted = quick_convert(&clipboard, &ConvertOptions::default());
        assert!(!converted);
        assert_eq!(clipboard.read(), ClipboardContents::default());
    }

    #[test]
    fn quick_convert_applies_case_transform_from_options() {
        let clipboard = MockClipboard::seeded(ClipboardContents {
            string: Some("hello world".to_string()),
            html: None,
        });
        let opts = ConvertOptions { to_all_caps: true, ..ConvertOptions::default() };
        quick_convert(&clipboard, &opts);
        assert_eq!(clipboard.read().string.as_deref(), Some("HELLO WORLD"));
    }
}
