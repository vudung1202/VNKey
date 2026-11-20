//! Every Vietnamese string the windows show, in one place so labels stay
//! consistent and are easy to review or translate. (The status-bar menu keeps
//! its own labels next to its item ids in `tray.rs`.)

pub const INPUT_TYPES: [&str; 4] = ["Telex", "VNI", "Simple Telex 1", "Simple Telex 2"];
pub const CODE_TABLES: [&str; 5] =
    ["Unicode dựng sẵn", "TCVN3 (ABC)", "VNI Windows", "Unicode tổ hợp", "Vietnamese Locale CP 1258"];

pub const SETTINGS_WINDOW_TITLE: &str = "VNKey - Bảng điều khiển";

pub mod sidebar {
    pub const SWITCH_HINT_SUFFIX: &str = "để chuyển";
    pub const NO_SWITCH_HOTKEY: &str = "Chưa đặt phím chuyển";
    pub const PERMISSION_MISSING: &str = "Chưa có quyền Trợ năng";
    pub const QUIT: &str = "Thoát VNKey";
}

pub mod top_bar {
    pub const INPUT_TYPE: &str = "Kiểu gõ";
    pub const CODE_TABLE: &str = "Bảng mã";
    pub const VIETNAMESE: &str = "Tiếng Việt";
    pub const ENGLISH: &str = "English";
    pub const SWITCH_HOTKEY: &str = "Phím chuyển";
    pub const GRANT_PERMISSION: &str = "Cấp quyền";
}

pub const TABS: [&str; 5] = ["Bộ gõ", "Gõ tắt", "Chuyển mã", "Hệ thống", "Thông tin"];

pub mod typing_view {
    pub const ON_PREFIX: &str = "Đang gõ tiếng Việt:";
    pub const OFF: &str = "Đang gõ tiếng Anh.";
    pub const OFF_PRESS: &str = "Nhấn";
    pub const OFF_TO_TURN_ON: &str = "để bật tiếng Việt.";
    pub const METHOD: &str = "Cách gõ";
    pub const SWITCH_BEEP: &str = "Kêu beep khi chuyển";
    pub const MARKS: &str = "Dấu và chính tả";
    pub const ENGLISH: &str = "Từ tiếng Anh";
    pub const PER_APP: &str = "Theo ứng dụng";
    pub const TEMP_OFF: &str = "Tạm tắt bằng phím";
}

pub mod typing_tab {
    pub const MODERN_ORTHOGRAPHY: &str = "Đặt dấu oà, uý (thay vì òa, úy)";
    pub const FIX_RECOMMEND_BROWSER: &str = "Sửa lỗi gợi ý (trình duyệt, Excel,...)";
    pub const CHECK_SPELLING: &str = "Kiểm tra chính tả";
    pub const RESTORE_IF_WRONG_SPELLING: &str = "Tự khôi phục phím với từ sai";
    pub const ALLOW_CONSONANT_ZFWJ: &str = "Cho phép \"z w j f\" làm phụ âm";
    pub const TEMP_OFF_SPELLING: &str = "Tạm tắt chính tả bằng phím ⌃";
    pub const USE_SMART_SWITCH_KEY: &str = "Chuyển chế độ thông minh";
    pub const UPPER_CASE_FIRST_CHAR: &str = "Viết Hoa chữ cái đầu câu";
    pub const REMEMBER_CODE: &str = "Tự ghi nhớ bảng mã theo ứng dụng";
    pub const TEMP_OFF_ENGINE: &str = "Tạm tắt VNKey bằng phím ⌘";
    pub const AUTO_RESTORE_ENGLISH: &str = "Tự động khôi phục từ tiếng Anh";
    pub const ESC_RESTORE_ENGLISH: &str = "Nhấn ESC để khôi phục từ đang gõ";
    pub const OTHER_LANGUAGE: &str = "Tắt tiếng Việt khi bộ gõ hệ thống khác tiếng Anh";
}

pub mod macro_view {
    pub const COUNT_SUFFIX: &str = "từ gõ tắt.";
    pub const NONE_YET: &str = "Chưa có từ gõ tắt nào.";
    pub const DISABLED: &str = "Gõ tắt đang tắt.";
    pub const TABLE: &str = "Bảng gõ tắt";
    pub const EMPTY: &str = "Chưa có từ gõ tắt. Thêm ở dưới, hoặc nạp file gõ tắt của UniKey.";
    pub const OPTIONS: &str = "Tuỳ chọn";
    pub const QUICK_TYPING: &str = "Gõ nhanh";
}

pub mod macro_tab {
    pub const USE_MACRO: &str = "Cho phép gõ tắt";
    pub const QUICK_TELEX: &str = "Gõ nhanh (cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th, uu=ươ)";
    pub const USE_MACRO_IN_ENGLISH_MODE: &str = "Gõ tắt cả khi tắt gõ tiếng Việt";
    pub const AUTO_CAPS_MACRO: &str = "Tự động viết hoa theo phím tắt";
    pub const QUICK_START_CONSONANT: &str = "Gõ tắt phụ âm đầu: f→ph, j→gi, w→qu";
    pub const QUICK_END_CONSONANT: &str = "Gõ tắt phụ âm cuối: g→ng, h→nh, k→ch";
}

pub mod system_view {
    pub const GRANTED: &str = "đã có quyền Trợ năng.";
    pub const MISSING: &str = "VNKey chưa có quyền Trợ năng nên chưa gõ được vào ứng dụng khác.";
    pub const PERMISSION: &str = "Quyền Trợ năng";
    pub const PERMISSION_ROW: &str = "Trợ năng (Accessibility)";
    pub const PILL_GRANTED: &str = "Đã cấp";
    pub const PILL_MISSING: &str = "Chưa cấp";
    pub const APPEARANCE: &str = "Hiển thị";
    pub const STARTUP: &str = "Khởi động và cập nhật";
    pub const COMPAT: &str = "Tương thích";
    pub const RESET: &str = "Khôi phục mặc định";
    pub const RESET_EXPLAIN: &str = "Đưa mọi cài đặt về mặc định. Bảng gõ tắt giữ nguyên.";
    pub const RESET_BUTTON: &str = "Khôi phục mặc định…";
    pub const RESET_CONFIRM_TITLE: &str = "Khôi phục cài đặt mặc định?";
    pub const RESET_CONFIRM_OK: &str = "Khôi phục";
    pub const RESET_CONFIRM_CANCEL: &str = "Huỷ";
}

pub mod system_tab {
    pub const SHOW_ICON_ON_DOCK: &str = "Hiện biểu tượng trên thanh Dock";
    pub const SHOW_UI_ON_STARTUP: &str = "Bật bảng này khi khởi động";
    pub const GRAY_ICON: &str = "Biểu tượng hiện đại trên thanh menu";
    pub const RUN_ON_STARTUP: &str = "Khởi động cùng macOS";
    pub const SEND_KEY_STEP_BY_STEP: &str = "Gửi từng phím (bật nếu bị lỗi)";
    pub const CHECK_UPDATE_ON_STARTUP: &str = "Kiểm tra bản mới lúc khởi động";
    pub const FIX_CHROMIUM_BROWSER: &str = "Sửa lỗi trên Chromium (beta)";
    pub const PERFORM_LAYOUT_COMPAT: &str = "Tương thích Telex trên các Layout khác (Dvorak, Colemak, ...)";
    pub const CHECK_UPDATE: &str = "Kiểm tra bản mới...";
    pub const UP_TO_DATE: &str = "Bạn đang dùng bản mới nhất.";
    pub const THEME: &str = "Giao diện";
    pub const THEMES: [&str; 3] = ["Tự động", "Sáng", "Tối"];
}

pub mod about_tab {
    pub const APP_NAME: &str = "VNKey";
    pub const VERSION_PREFIX: &str = "Phiên bản";
    pub const DESCRIPTION: &str = "Bộ gõ tiếng Việt cho macOS.";
    pub const CREDIT: &str = "Thuật toán gõ tham khảo từ OpenKey của Mai Vũ Tuyên.";
    pub const SOURCE_CODE: &str = "Mã nguồn...";
    pub const SOURCE_CODE_URL: &str = "https://github.com/vudung1202/VNKey";
}

pub mod footer {
    pub const YES: &str = "Có";
    pub const NO: &str = "Không";
}

pub mod macro_editor {
    pub const KEY_COLUMN: &str = "Từ gõ tắt";
    pub const CONTENT_COLUMN: &str = "Nội dung đầy đủ";
    pub const ADD: &str = "Thêm";
    pub const EDIT: &str = "Sửa";
    pub const DELETE: &str = "Xoá";
    pub const IMPORT_FILE: &str = "Nạp từ file...";
    pub const EXPORT_FILE: &str = "Xuất ra file...";
    pub const ADD_NEEDS_BOTH_FIELDS: &str = "Bạn hãy nhập từ cần gõ tắt!";
    pub const DELETE_NEEDS_SELECTION: &str = "Bạn hãy chọn từ cần xoá!";
    pub const IMPORT_DIALOG_TITLE: &str = "Dữ liệu gõ tắt";
    pub const IMPORT_KEEP_DATA_QUESTION: &str = "Bạn có muốn giữ lại các dữ liệu hiện tại không?";
    pub const ALERT_TITLE: &str = "Gõ tắt";
    pub const EXPORT_DEFAULT_NAME: &str = "VNKeyMacro.txt";
}

pub mod convert_view {
    pub const SUBLINE_PREFIX: &str = "Đổi văn bản trong clipboard từ";
    pub const SUBLINE_TO: &str = "sang";
    pub const CODES: &str = "Bảng mã";
    pub const CASE: &str = "Chữ hoa, chữ thường";
    // Each label is set in the case it produces, so the short forms still read.
    pub const CASES: [&str; 5] = ["Giữ nguyên", "HOA", "thường", "Hoa đầu câu", "Hoa Mỗi Từ"];
    pub const HOTKEY: &str = "Phím tắt";
    pub const CONVERT_CLIPBOARD: &str = "Chuyển mã clipboard";
    /// The sentence the case options are previewed on.
    pub const SAMPLE: &str = "xin chào. hôm nay trời đẹp";
}

pub mod convert_tab {
    pub const REMOVE_MARK: &str = "Loại bỏ dấu tiếng Việt";
    pub const NOTIFY_WHEN_DONE: &str = "Thông báo khi chuyển xong";
    pub const FROM_CODE: &str = "Bảng mã nguồn";
    pub const TO_CODE: &str = "Bảng mã đích";
    pub const REVERSE: &str = "⇄";
    pub const QUICK_HOTKEY: &str = "Phím chuyển mã nhanh";
    pub const SUCCESS_TITLE: &str = "Chuyển mã thành công!";
    pub const SUCCESS_SUB: &str = "Kết quả đã được lưu trong clipboard.";
    pub const FAIL_TITLE: &str = "Không có dữ liệu trong clipboard!";
    pub const FAIL_SUB: &str = "Hãy sao chép một đoạn text để chuyển đổi!";
}

/// The full-window Accessibility explainer shown instead of the tabs.
pub mod permission_gate {
    pub const TITLE: &str = "Cần cấp quyền Trợ năng";
    pub const EXPLAIN: &str = "VNKey cần quyền Trợ năng (Accessibility) để có thể gõ tiếng Việt vào các ứng dụng khác. Vui lòng cấp quyền trong System Settings, sau đó khởi động lại VNKey.";
    pub const GRANTED: &str = "Đã cấp quyền! Vui lòng khởi động lại VNKey để áp dụng.";
    pub const OPEN_SETTINGS: &str = "Mở cài đặt Trợ năng";
    pub const STEP_GRANT: &str = "Bật VNKey trong Trợ năng";
    pub const STEP_RESTART: &str = "Khởi động lại để bắt đầu gõ";
    pub const DONE: &str = "Đã bật";
    pub const RESTART_SHORT: &str = "Khởi động lại";
}
