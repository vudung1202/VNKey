// Every Vietnamese UI string the Control Panel uses. Labels are copied
// verbatim from OpenKey's own UI (`Base.lproj/Main.storyboard`,
// `ViewController.m`, `AppDelegate.m`) per the task brief's requirement that
// labels must match OpenKey exactly.
export const strings = {
  topBar: {
    inputType: "Kiểu gõ:",
    codeTable: "Bảng mã:",
    mode: "Chế độ gõ:",
    vietnamese: "Tiếng Việt",
    english: "English",
    switchHotkey: "Phím chuyển:",
    beep: "Kêu beep",
    permissionMissing: "Bạn chưa cấp quyền cho ứng dụng hoạt động!",
    grantPermission: "Cấp quyền",
  },

  inputTypes: ["Telex", "VNI", "Simple Telex 1", "Simple Telex 2"],
  codeTables: ["Unicode dựng sẵn", "TCVN3 (ABC)", "VNI Windows", "Unicode tổ hợp", "Vietnamese Locale CP 1258"],

  tabs: {
    typing: "Bộ gõ",
    macro: "Gõ tắt",
    system: "Hệ thống",
    about: "Thông tin",
  },

  typingTab: {
    modernOrthography: "Đặt dấu oà, uý (thay vì òa, úy)",
    fixRecommendBrowser: "Sửa lỗi gợi ý (trình duyệt, Excel,...)",
    checkSpelling: "Kiểm tra chính tả",
    restoreIfWrongSpelling: "Tự khôi phục phím với từ sai",
    allowConsonantZfwj: 'Cho phép "z w j f" làm phụ âm',
    tempOffSpelling: "Tạm tắt chính tả bằng phím ⌃",
    useSmartSwitchKey: "Chuyển chế độ thông minh",
    upperCaseFirstChar: "Viết Hoa chữ cái đầu câu",
    rememberCode: "Tự ghi nhớ bảng mã theo ứng dụng",
    tempOffOpenKey: "Tạm tắt OpenKey bằng phím ⌘",
    otherLanguage: "Tắt tiếng Việt khi bộ gõ hệ thống khác tiếng Anh",
  },

  macroTab: {
    useMacro: "Cho phép gõ tắt",
    quickTelex: "Gõ nhanh (cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th, uu=ươ)",
    useMacroInEnglishMode: "Gõ tắt cả khi tắt gõ tiếng Việt",
    autoCapsMacro: "Tự động viết hoa theo phím tắt",
    quickStartConsonant: "Gõ tắt phụ âm đầu: f→ph, j→gi, w→qu",
    quickEndConsonant: "Gõ tắt phụ âm cuối: g→ng, h→nh, k→ch",
    openMacroTable: "Bảng gõ tắt...",
  },

  systemTab: {
    showIconOnDock: "Hiện biểu tượng trên thanh Dock",
    showUiOnStartup: "Bật bảng này khi khởi động",
    grayIcon: "Biểu tượng hiện đại trên thanh menu",
    runOnStartup: "Khởi động cùng macOS",
    sendKeyStepByStep: "Gửi từng phím (bật nếu bị lỗi)",
    checkUpdateOnStartup: "Kiểm tra bản mới lúc khởi động",
    fixChromiumBrowser: "Sửa lỗi trên Chromium (beta)",
    performLayoutCompat: "Tương thích Telex trên các Layout khác (Dvorak, Colemak, ...)",
    checkUpdate: "Kiểm tra bản mới...",
    checking: "Đang kiểm tra...",
  },

  aboutTab: {
    appName: "VNKey",
    description: "Bộ gõ tiếng Việt mã nguồn mở, kế thừa OpenKey (GPL-3.0).",
    credit: "Dựa trên OpenKey của Mai Vũ Tuyên.",
    sourceCode: "Mã nguồn...",
    // TODO: placeholder — replace with the real VNKey repo URL once it exists.
    // Used as both the "Trang chủ" (home page) and "Mã nguồn" (source code)
    // link in the About window/tab, since the project has no separate
    // homepage from its repo.
    sourceCodeUrl: "https://github.com/yourname/VNKey",
  },

  footer: {
    ok: "OK",
    restoreDefault: "Mặc định",
    quit: "Kết thúc",
    restoreDefaultConfirm: "Bạn có chắc chắn muốn thiết lập lại cấu hình mặc định?",
    yes: "Có",
    no: "Không",
  },

  // `MacroViewController.mm` + the macro scene in `Main.storyboard`.
  macroEditor: {
    keyColumn: "Từ gõ tắt",
    contentColumn: "Nội dung đầy đủ",
    add: "Thêm",
    edit: "Sửa",
    delete: "Xoá",
    importFile: "Nạp từ file...",
    exportFile: "Xuất ra file...",
    addNeedsBothFields: "Bạn hãy nhập từ cần gõ tắt!",
    deleteNeedsSelection: "Bạn hãy chọn từ cần xoá!",
    importDialogTitle: "Dữ liệu gõ tắt",
    importKeepDataQuestion: "Bạn có muốn giữ lại các dữ liệu hiện tại không?",
    alertTitle: "Gõ tắt",
  },

  // `ConvertToolViewController.mm` + the convert-tool scene in `Main.storyboard`.
  convertTool: {
    generalOptions: "Tùy chọn chung",
    toAllCaps: "Chuyển sang chữ HOA",
    toAllNonCaps: "Chuyển sang chữ thường",
    toCapsFirstLetter: "Đặt chữ Hoa đầu câu",
    toCapsEachWord: "Đặt chữ Hoa Sau Mỗi Từ",
    removeMark: "Loại bỏ dấu câu",
    notifyWhenDone: "Thông báo khi chuyển xong",
    options: "Lựa chọn",
    fromCode: "Bảng mã nguồn",
    toCode: "Bảng mã đích",
    reverse: "⇄",
    // Accessible name for the ⇄ button (not shown; the glyph alone has no name).
    reverseAria: "Đảo bảng mã nguồn và đích",
    quickHotkey: "Phím chuyển mã nhanh:",
    clipboardCheckbox: "Chuyển mã trong Clipboard",
    convert: "Chuyển mã",
    close: "Đóng",
    successTitle: "Chuyển mã thành công!",
    successSub: "Kết quả đã được lưu trong clipboard.",
    failTitle: "Không có dữ liệu trong clipboard!",
    failSub: "Hãy sao chép một đoạn text để chuyển đổi!",
  },

  // `AboutViewController.m` + the about scene in `Main.storyboard`; reuses
  // `aboutTab`'s appName/description/credit/sourceCodeUrl/sourceCode (same
  // content -- `sourceCode` deliberately not duplicated here, Task 14 review).
  about: {
    homePage: "Trang chủ",
    checkUpdateOnStartup: "Kiểm tra bản mới khi khởi động",
    close: "Đóng",
  },

  // Task 15: the Accessibility blocking view (`Permission/PermissionGate.tsx`).
  // Mirrors OpenKey's `AppDelegate.m: askPermission` alert content, reworded
  // as a full-window explainer instead of a modal `NSAlert`.
  permissionGate: {
    title: "Cần cấp quyền Trợ năng",
    explain:
      "VNKey cần quyền Trợ năng (Accessibility) để có thể gõ tiếng Việt vào các ứng dụng khác. Vui lòng cấp quyền trong System Settings, sau đó khởi động lại VNKey.",
    granted: "Đã cấp quyền! Vui lòng khởi động lại VNKey để áp dụng.",
    openSettings: "Mở cài đặt Trợ năng",
    restart: "Khởi động lại VNKey",
  },

  // Task 15: the first-run "import from OpenKey?" offer (a native confirm
  // dialog -- `footer.yes`/`footer.no` supply its Có/Không button labels).
  firstRun: {
    title: "Nhập dữ liệu OpenKey",
    offer: "Phát hiện dữ liệu OpenKey. Nhập cấu hình, gõ tắt và ghi nhớ ứng dụng từ OpenKey?",
  },
};
