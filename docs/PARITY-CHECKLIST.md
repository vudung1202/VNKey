# Checklist kiểm thử tay VNKey

Mục đích: giúp bạn tự kiểm tra tay rằng VNKey gõ/hoạt động đúng trên máy
thật. Cột **Kỳ vọng** ghi rõ kết quả đúng cho từng thao tác. Nếu muốn đối
chiếu với một bộ gõ tham chiếu, có thể cài thêm OpenKey và gõ song song,
nhưng không bắt buộc.

**Cách dùng:** tắt hết các bộ gõ khác (Unikey, EVKey, gõ tiếng Việt hệ
thống, OpenKey nếu có...), bật **một lúc một cái** (hai bộ gõ CGEventTap
chạy cùng lúc sẽ đụng nhau), gõ chuỗi phím vào Notes/TextEdit và so với
cột Kỳ vọng.

Mỗi dòng: **auto** = hành vi engine đã được test tự động (golden/unit test,
chạy bằng `cargo test --workspace`), bạn chỉ cần xác nhận UI xuất hiện đúng
và không treo; **manual** = cần bạn tự gõ và so bằng mắt, vì phụ thuộc hành
vi hệ thống thật (CGEventTap, focus app khác, sleep/wake...) không tự động
hoá được nếu không có quyền Accessibility.

## 1. Bốn kiểu gõ

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 1.1 | Telex: gõ `vieejt` | `việt` | auto |
| 1.2 | Telex: gõ `ddaay` | `đây` | auto |
| 1.3 | Telex: gõ `hoaf` (chính tả hiện đại) | `hoà` (dấu trên `a`) | auto |
| 1.4 | Telex: gõ `hoaf` (chính tả cũ, bật "orthography cũ") | `hòa` (dấu trên `o`) | auto |
| 1.5 | Telex: gõ `uwowng` | `ương` | auto |
| 1.6 | Telex: gõ `nghieeng` | `nghiêng` | auto |
| 1.7 | VNI: gõ `vie65t` | `việt` | auto |
| 1.8 | VNI: gõ `d9a6y` | `đây` | auto |
| 1.9 | VNI: các phím số 1-9,0 làm dấu/thanh (không chỉ chữ) | dấu/thanh đúng | auto |
| 1.10 | Simple Telex 1: `w` không tự thành ư đứng một mình | `w` giữ nguyên | auto |
| 1.11 | Simple Telex 2: các quy tắc rút gọn riêng | theo quy tắc Simple Telex 2 | auto |
| 1.12 | Đổi kiểu gõ qua tray (Kiểu gõ submenu) hoặc Bảng điều khiển, gõ lại thử | UI cập nhật, gõ theo kiểu mới ngay | **manual** |

## 2. Năm bảng mã

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 2.1 | Unicode dựng sẵn: gõ `vieejt` | ra đúng byte Unicode NFC (`việt`) | auto |
| 2.2 | TCVN3 (ABC): gõ trong app không hỗ trợ Unicode, dán ra app khác kiểm byte | ra đúng byte TCVN3 | auto |
| 2.3 | VNI Windows: gõ, kiểm byte | ra đúng byte VNI Windows | auto |
| 2.4 | Unicode tổ hợp: gõ, kiểm byte (dấu là ký tự tổ hợp riêng) | ra đúng byte tổ hợp | auto |
| 2.5 | Vietnamese Locale CP1258: gõ, kiểm byte | ra đúng byte CP1258 | auto |
| 2.6 | Đổi bảng mã qua tray ("Bảng mã khác" submenu) giữa phiên gõ | UI + engine đồng bộ ngay, không cần khởi động lại | **manual** (đổi bảng mã ở mức lệnh có unit test, nhưng đổi *giữa* một chuỗi phím đang gõ dở chưa test tự động) |
| 2.7 | Công cụ chuyển mã: dán đoạn Unicode, chuyển sang cả 4 bảng còn lại, dán ra kiểm | khớp từng ký tự | auto (ma trận chuyển mã 5×5 có golden test) |

## 3. Chính tả, orthography, gõ nhanh

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 3.1 | Gõ sai chính tả (vd phụ âm+nguyên âm không hợp lệ) rồi SPACE | tự khôi phục về đúng phím gốc đã gõ (không hiện chữ có dấu sai) | auto |
| 3.2 | Gõ lùi (backspace) giữa từ có dấu rồi gõ tiếp | dấu đặt lại đúng vị trí | auto |
| 3.3 | Gõ tắt phụ âm đầu: `fam` (quick start consonant bật) | `pham` → xử lý như phụ âm `ph` | auto |
| 3.4 | Gõ tắt phụ âm cuối: `hag` (quick end consonant bật) | `hang` → xử lý như `ng` | auto |
| 3.5 | Gõ nhanh: `cc` đầu từ (quick telex bật) | thành `ch` | auto |
| 3.6 | Phím `z` xoá dấu thanh vừa gõ | dấu thanh bị xoá | auto |
| 3.7 | Viết hoa đầu câu tự động (upper-case-first-char) sau dấu chấm + space | chữ đầu từ tiếp theo tự viết hoa | auto |
| 3.8 | Từ dài hơn buffer tối đa (32 ký tự) | không crash, xử lý qua "long word helper" | auto |
| 3.9 | Bật/tắt từng tùy chọn chính tả/orthography/quick-* qua Bảng điều khiển, gõ thử ngay sau khi đổi | hành vi đổi theo tuỳ chọn mới ngay lập tức, không cần khởi động lại app | **manual** |

## 4. Gõ tắt (macro) & smart-switch

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 4.1 | Thêm macro qua cửa sổ "Gõ tắt" (vd `ko` → `không`), gõ `ko ` trong app khác | tự thay bằng nội dung đầy đủ | auto (thuật toán) + **manual** để xác nhận UI Thêm→lưu→gõ thật hoạt động |
| 4.2 | Auto-caps macro: gõ `Ko ` (macro `ko`→`không`, bật tự động viết hoa) | ra `Không` | auto |
| 4.3 | Gõ `KO ` | ra `KHÔNG` (viết hoa toàn bộ) | auto |
| 4.4 | Macro hoạt động cả khi tắt gõ tiếng Việt (English mode), nếu bật tùy chọn tương ứng | thay thế vẫn chạy | auto |
| 4.5 | Nạp/Xuất file macro (.txt UniKey-style) | định dạng đọc/ghi round-trip đúng, macro cũ giữ nguyên khi import | auto + **manual** để xác nhận dialog chọn file trong UI |
| 4.6 | Smart-switch: bật Việt ở app A, chuyển English ở app B, quay lại app A | app A vẫn ở trạng thái Việt (nhớ theo app) | auto (thuật toán) + **manual** bắt buộc (cần đổi frontmost app thật) |
| 4.7 | Smart-switch nhớ luôn bảng mã riêng theo từng app (nếu bật) | app A dùng Unicode, app B dùng TCVN3 độc lập | **manual** |
| 4.8 | Khởi động lại VNKey, kiểm smart-switch/macro đã lưu còn nguyên | dữ liệu persist qua `macros.bin`/`smart_switch.bin` | auto (round-trip) + **manual** xác nhận trên máy thật |

## 5. Hotkey, beep, tạm tắt engine, other-language

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 5.1 | Nhấn ⌥Z (mặc định) | chuyển Việt⇄Anh, tray/UI cập nhật ngay | **manual** (cần CGEventTap thật) |
| 5.2 | Bật "beep" cho hotkey, nhấn ⌥Z | có tiếng beep hệ thống | **manual** |
| 5.3 | Đổi hotkey qua HotkeyRecorder trong Bảng điều khiển, thử tổ hợp mới | hotkey mới hoạt động ngay, không cần khởi động lại | **manual** |
| 5.4 | Giữ và nhả riêng phím Control (không kèm phím khác), khi "tạm tắt chính tả" (temp-off spelling) bật | engine tạm tắt kiểm tra chính tả cho từ tiếp theo | **manual** (state machine chord có unit test, nhưng cần tay xác nhận trên CGEventTap thật) |
| 5.5 | Giữ và nhả riêng phím Command, khi "tạm tắt VNKey" (temp-off engine) bật | engine tạm tắt hoàn toàn (gõ tiếng Anh thuần) cho từ tiếp theo | **manual** |
| 5.6 | Chuyển sang input source khác tiếng Anh (vd tiếng Nhật, tiếng Hàn) qua Input Source trong khi "other-language" bật | VNKey tự bỏ qua (passthrough), không can thiệp phím | **manual** (cần TIS thật) |
| 5.7 | Tổ hợp Cmd+Q khi hotkey đang ghi (HotkeyRecorder) | không bị nuốt nhầm làm hotkey, ESC huỷ ghi | **manual** |

## 6. Sửa lỗi riêng từng ứng dụng (quirks)

| # | App | Kỳ vọng | Trạng thái |
|---|-----|---------|------------|
| 6.1 | Google Chrome - thanh địa chỉ | workaround autocomplete dùng Shift+Left thay vì chèn ký tự rỗng (nếu bật "fix Chromium") | **manual** (`fix_chromium_browser` mặc định tắt, cần bật thủ công để thấy khác biệt) |
| 6.2 | Terminal.app | gõ dấu không vỡ dòng, không lặp ký tự | **manual** |
| 6.3 | Spotlight | gõ tiếng Việt hoạt động, không bị nuốt phím đầu | **manual** (Spotlight-visible detection còn là stub `false`; hành vi mặc định giống app thường) |
| 6.4 | Microsoft Excel | ô nhập liệu không bị lỗi ký tự thừa | **manual** |
| 6.5 | Sublime Text 2/3 | dùng ký tự trống "nice space" (`0x200C`); Sublime Text 4 KHÔNG nằm trong danh sách quirk | auto (unit test `quirks.rs`) + **manual** để xác nhận cảm giác gõ thật |
| 6.6 | Bất kỳ app Apple nào khác (Mail, Notes, Pages...) | dùng chế độ Unicode Compound app-quirk (prefix `com.apple.`) | auto (unit test quirks prefix match) + **manual** |

## 7. Sleep/wake, đổi Space, đa ứng dụng

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 7.1 | Đóng nắp máy / để máy sleep rồi wake lại, gõ tiếp | VNKey vẫn hoạt động bình thường, không cần khởi động lại | **manual** |
| 7.2 | Đổi Space (Mission Control) sang app khác, gõ | trạng thái Việt/Anh theo app đúng như smart-switch đã lưu | **manual** |
| 7.3 | Mở nhiều app cùng lúc, luân phiên gõ qua lại nhanh | không lẫn trạng thái giữa các app, không crash | **manual** |
| 7.4 | Tắt máy/mở lại (khởi động lại macOS), VNKey tự chạy nếu bật "Khởi động cùng macOS" | app tự khởi động, quyền Accessibility còn giữ (nếu đã ký bằng cert ổn định, xem docs/dev-signing.md) | **manual** |

## 8. UI chung

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 8.1 | Mở "Bảng điều khiển" qua tray | cửa sổ hiện, tiêu đề "VNKey - Bảng điều khiển"; nếu chưa cấp quyền Accessibility thì hiện màn hình "Cần cấp quyền Trợ năng" | **manual** |
| 8.2 | Mở "Gõ tắt" qua tray | cửa sổ "Thiết lập gõ tắt" hiện đủ bảng, form, nút Thêm/Xoá/Nạp/Xuất | **manual** |
| 8.3 | Mở "Công cụ chuyển mã" qua tray | cửa sổ hiện đủ 2 nhóm tùy chọn, select bảng mã, nút Chuyển mã/Đóng | **manual** |
| 8.4 | Mở "Giới thiệu" qua tray | cửa sổ hiện tên/mô tả/credit/link, checkbox kiểm tra bản mới | **manual** |
| 8.5 | Đóng cửa sổ bằng nút đỏ titlebar | ẩn cửa sổ (không thoát app); mở lại qua tray ra đúng cửa sổ cũ | **manual** |
| 8.6 | Thoát app qua tray "Thoát" | tiến trình kết thúc sạch | **manual** |
| 8.7 | Gõ tiếng Việt thật vào Chrome/Terminal | ra đúng chữ có dấu | **manual** |
| 8.8 | Đổi Giao diện (Tự động/Sáng/Tối) trong tab Hệ thống | mọi cửa sổ đổi theme ngay | **manual** |

## Ghi chú

- Các mục "auto" là hành vi engine (`vnkey-engine`) đã được golden/unit test
  bao phủ, chạy bằng `cargo test --workspace`.
- Các mục "manual" cần Accessibility thật hoặc cần thao tác hệ thống
  (sleep/wake, đổi Space, đổi Input Source) mà chỉ người dùng trên máy thật
  mới làm được.
- Nếu phát hiện lệch hành vi ở bất kỳ mục "manual" nào, ưu tiên nghi ngờ
  tầng macOS (`src-tauri/src/platform/mac/`) trước; engine (`vnkey-engine`)
  đã có test bao phủ khá kỹ.
