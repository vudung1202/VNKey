// Every Vietnamese UI string the Control Panel uses. Labels are copied
// verbatim from OpenKey's own UI (`Base.lproj/Main.storyboard`,
// `ViewController.m`, `AppDelegate.m`) per the task brief's requirement that
// labels must match OpenKey exactly -- including OpenKey's own "Coleman"
// typo in the layout-compat checkbox.
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
    quickTelex: "Gõ nhanh (cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th)",
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
    performLayoutCompat: "Tương thích Telex trên các Layout khác (Dvorak, Coleman, ...)",
    checkUpdate: "Kiểm tra bản mới...",
    checking: "Đang kiểm tra...",
  },

  aboutTab: {
    appName: "VNKey",
    description: "Bộ gõ tiếng Việt mã nguồn mở, kế thừa OpenKey (GPL-3.0).",
    credit: "Dựa trên OpenKey của Mai Vũ Tuyên.",
    sourceCode: "Mã nguồn...",
    sourceCodeUrl: "https://github.com/tuyenvm/OpenKey",
  },

  footer: {
    ok: "OK",
    restoreDefault: "Mặc định",
    quit: "Kết thúc",
    restoreDefaultConfirm: "Bạn có chắc chắn muốn thiết lập lại cấu hình mặc định?",
    yes: "Có",
    no: "Không",
  },
};
