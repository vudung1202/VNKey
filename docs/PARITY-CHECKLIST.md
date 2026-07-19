# Parity checklist: VNKey vs OpenKey

Mục đích: giúp bạn tự kiểm tra tay rằng VNKey gõ/hoạt động **giống hệt**
OpenKey trên máy thật, sau khi 15 task trước đã hoàn thành và bộ
differential test (engine) đã xác nhận bit-exact ở mức thuật toán.

**Cách dùng:** cài cả VNKey và OpenKey, tắt hết các bộ gõ khác (Unikey,
EVKey, gõ tiếng Việt hệ thống...), bật từng app **một lúc một cái** (tắt
cái kia đi khi test cái này, vì hai bộ gõ CGEventTap chạy cùng lúc sẽ
đụng nhau), gõ cùng một chuỗi phím vào cùng notes/TextEdit, so kết quả.

Mỗi dòng: **auto** = đã được differential test tự động xác nhận bit-exact
với oracle C++ (không cần bạn so tay phần thuật toán, chỉ cần xác nhận UI
xuất hiện đúng và không bị treo); **manual** = cần bạn tự gõ và so bằng
mắt, vì phụ thuộc hành vi hệ thống thật (CGEventTap, focus app khác,
sleep/wake...) mà agent không thể tự động hoá (không có quyền
Accessibility).

## 1. Bốn kiểu gõ

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 1.1 | Telex: gõ `vieejt` | `việt` | auto (differential + golden `telex_vieejt_gives_viet`) |
| 1.2 | Telex: gõ `ddaay` | `đây` | auto (golden `telex_ddaay_gives_day`) |
| 1.3 | Telex: gõ `hoaf` (chính tả hiện đại) | `hoà` (dấu trên `a`) | auto (golden `telex_hoaf_modern_orthography_is_hoa_grave_on_a`) |
| 1.4 | Telex: gõ `hoaf` (chính tả cũ, bật "orthography cũ") | `hòa` (dấu trên `o`) | auto (golden `telex_hoaf_old_orthography_is_hoa_grave_on_o`) |
| 1.5 | Telex: gõ `uwowng` | `ương` | auto (golden `telex_uwowng_gives_uong_horns`) |
| 1.6 | Telex: gõ `nghieeng` | `nghiêng` | auto (differential corpus phủ mẫu tương tự) |
| 1.7 | VNI: gõ `vie65t` | `việt` | auto (golden `vni_vie65t_gives_viet`) |
| 1.8 | VNI: gõ `d9a6y` | `đây` | auto (golden `vni_d9a6y_gives_day`) |
| 1.9 | VNI: các phím số 1-9,0 làm dấu/thanh (không chỉ chữ) | khớp OpenKey | auto (`differential_vni_digits`, thêm ở Task 5 vì thiếu ban đầu) |
| 1.10 | Simple Telex 1: `w` không tự thành ư đứng một mình | khớp OpenKey | auto (golden `simple_telex1_w_is_not_standalone_u_horn`) |
| 1.11 | Simple Telex 2: các quy tắc rút gọn riêng | khớp OpenKey | auto (`differential_general` phủ cả 4 input type) |
| 1.12 | Đổi kiểu gõ qua tray (Kiểu gõ submenu) hoặc Bảng điều khiển, gõ lại thử | UI cập nhật, gõ theo kiểu mới ngay | **manual** |

## 2. Năm bảng mã

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 2.1 | Unicode dựng sẵn: gõ `vieejt` | ra đúng byte Unicode NFC (`việt`) | auto (`differential_unicode` + `convert_golden_matrix_all_tables_all_case_options`) |
| 2.2 | TCVN3 (ABC): gõ trong app không hỗ trợ Unicode, dán ra app khác kiểm byte | khớp byte OpenKey xuất ra | auto (`differential_other_code_tables`, `convert` differential) |
| 2.3 | VNI Windows: gõ, kiểm byte | khớp OpenKey | auto (`differential_other_code_tables`) |
| 2.4 | Unicode tổ hợp: gõ, kiểm byte (dấu là ký tự tổ hợp riêng) | khớp OpenKey | auto (`differential_other_code_tables`) |
| 2.5 | Vietnamese Locale CP1258: gõ, kiểm byte | khớp OpenKey | auto (`differential_other_code_tables`) |
| 2.6 | Đổi bảng mã qua tray ("Bảng mã khác" submenu) giữa phiên gõ | UI + engine đồng bộ ngay, không cần khởi động lại | **manual** (có unit test `on_table_code_change_matches_oracle` ở mức lệnh, nhưng chưa differential-test đổi bảng mã *giữa* một chuỗi phím đang gõ dở) |
| 2.7 | Công cụ chuyển mã: dán đoạn Unicode, chuyển sang cả 4 bảng còn lại, dán ra so với OpenKey Convert Tool | khớp từng ký tự | auto (`convert_golden_matrix_all_tables_all_case_options` phủ 5×5, `convert_differential_random_text`) |

## 3. Chính tả, orthography, gõ nhanh

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 3.1 | Gõ sai chính tả (vd phụ âm+nguyên âm không hợp lệ) rồi SPACE | tự khôi phục về đúng phím gốc đã gõ (không hiện chữ có dấu sai) | auto (golden `restore_if_wrong_spelling_restores_raw_keys_on_space`) |
| 3.2 | Gõ lùi (backspace) giữa từ có dấu rồi gõ tiếp | dấu đặt lại đúng vị trí (check_grammar re-run) | auto (golden `backspace_then_retype_repositions_mark_via_check_grammar`) |
| 3.3 | Gõ tắt phụ âm đầu: `fam` (quick start consonant bật) | `pham` → xử lý như phụ âm `ph` | auto (golden `quick_start_consonant_fanh_space_gives_phanh`) |
| 3.4 | Gõ tắt phụ âm cuối: `hag` (quick end consonant bật) | `hang` → xử lý như `ng` | auto (golden `quick_end_consonant_hag_space_gives_hang`) |
| 3.5 | Gõ nhanh: `cc` đầu từ (quick telex bật) | thành `ch` | auto (golden `quick_telex_cc_starts_word_with_ch`) |
| 3.6 | Phím `z` xoá dấu thanh vừa gõ | khớp OpenKey | auto (golden `telex_z_key_removes_mark`) |
| 3.7 | Viết hoa đầu câu tự động (upper-case-first-char) sau dấu chấm + space | chữ đầu từ tiếp theo tự viết hoa | auto (golden `upper_case_first_char_after_dot_space_capitalizes_next_word`) |
| 3.8 | Từ dài hơn buffer tối đa (32 ký tự) | không crash, xử lý qua "long word helper" giống OpenKey | auto (golden `word_longer_than_max_buff_exercises_long_word_helper`) |
| 3.9 | Bật/tắt từng tùy chọn chính tả/orthography/quick-* qua Bảng điều khiển, gõ thử ngay sau khi đổi | hành vi đổi theo tuỳ chọn mới ngay lập tức, không cần khởi động lại app | **manual** |

## 4. Gõ tắt (macro) & smart-switch

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 4.1 | Thêm macro qua cửa sổ "Gõ tắt" (vd `ko` → `không`), gõ `ko ` trong app khác | tự thay bằng nội dung đầy đủ | auto thuật toán (`macro_typing_differential`, `find_macro_matches_oracle`) + **manual** để xác nhận UI Thêm→lưu→gõ thật hoạt động |
| 4.2 | Auto-caps macro: gõ `Ko ` (macro `ko`→`không`, bật tự động viết hoa) | ra `Không` | auto (`macro_typing_hand_cases`, `english_mode_macro_differential`) |
| 4.3 | Gõ `KO ` | ra `KHÔNG` (viết hoa toàn bộ) | auto (cùng bộ test 4.2) |
| 4.4 | Macro hoạt động cả khi tắt gõ tiếng Việt (English mode), nếu bật tùy chọn tương ứng | thay thế vẫn chạy | auto (`english_mode_macro_differential`) |
| 4.5 | Nạp/Xuất file macro (.txt UniKey-style) | định dạng đọc/ghi khớp OpenKey, macro cũ giữ nguyên khi import | auto (`unikey_txt_export_import_round_trip`, `unikey_txt_handles_colon_in_macro_name`) + **manual** để xác nhận dialog chọn file trong UI |
| 4.6 | Smart-switch: bật Việt ở app A, chuyển English ở app B, quay lại app A | app A vẫn ở trạng thái Việt (nhớ theo app) | auto thuật toán (`smart_switch_get_set_sequence_matches_oracle`) + **manual** bắt buộc (cần đổi frontmost app thật, không mô phỏng được mà không có Accessibility) |
| 4.7 | Smart-switch nhớ luôn bảng mã riêng theo từng app (nếu bật) | app A dùng Unicode, app B dùng TCVN3 độc lập | **manual** |
| 4.8 | Khởi động lại VNKey, kiểm smart-switch/macro đã lưu còn nguyên | dữ liệu persist qua `macros.bin`/`smart_switch.bin` | auto (round-trip test) + **manual** xác nhận trên máy thật |

## 5. Hotkey, beep, tạm tắt engine, other-language

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 5.1 | Nhấn ⌥Z (mặc định) | chuyển Việt⇄Anh, tray/UI cập nhật ngay | **manual** (cần CGEventTap thật) |
| 5.2 | Bật "beep" cho hotkey, nhấn ⌥Z | có tiếng beep hệ thống | **manual** |
| 5.3 | Đổi hotkey qua HotkeyRecorder trong Bảng điều khiển, thử tổ hợp mới | hotkey mới hoạt động ngay, không cần khởi động lại | **manual** |
| 5.4 | Giữ và nhả riêng phím Control (không kèm phím khác), khi "tạm tắt chính tả" (temp-off spelling) bật | engine tạm tắt kiểm tra chính tả cho từ tiếp theo | **manual** (chord edge-release logic — có unit test `hotkey.rs` cho state machine, nhưng cần tay xác nhận trên CGEventTap thật) |
| 5.5 | Giữ và nhả riêng phím Command, khi "tạm tắt OpenKey" (temp-off engine) bật | engine tạm tắt hoàn toàn (gõ tiếng Anh thuần) cho từ tiếp theo | **manual** |
| 5.6 | Chuyển sang layout bàn phím khác tiếng Anh (vd tiếng Nhật, tiếng Hàn) qua Input Source trong khi "other-language" bật | VNKey tự bỏ qua (passthrough), không can thiệp phím | **manual** (cần TIS thật) |
| 5.7 | Tổ hợp Cmd+Q khi hotkey đang ghi (HotkeyRecorder) | không bị nuốt nhầm làm hotkey, ESC huỷ ghi | **manual** |

## 6. Sửa lỗi riêng từng ứng dụng (quirks)

| # | App | Kỳ vọng | Trạng thái |
|---|-----|---------|------------|
| 6.1 | Google Chrome — thanh địa chỉ | gợi ý workaround autocomplete dùng Shift+Left thay vì chèn ký tự rỗng (nếu bật "fix Chromium") | **manual** (`fix_chromium_browser` mặc định tắt, nhánh code có nhưng cần bật thủ công để thấy khác biệt — xem `docs`/ghi chú Task 8-10) |
| 6.2 | Terminal.app | gõ dấu không vỡ dòng, không lặp ký tự | **manual** |
| 6.3 | Spotlight | gõ tiếng Việt hoạt động, không bị nuốt phím đầu | **manual** (Spotlight-visible detection còn là stub `false` — xem Concerns; hành vi mặc định giống app thường) |
| 6.4 | Microsoft Excel | ô nhập liệu không bị lỗi ký tự thừa | **manual** |
| 6.5 | Sublime Text 2/3 | dùng ký tự trống "nice space" (`0x200C`) đúng như OpenKey — Sublime Text 4 KHÔNG nằm trong danh sách (khớp OpenKey) | auto (unit test `quirks.rs`: `resolve_quirks(Some("com.sublimetext.3")).nice_space`, `!resolve_quirks(Some("com.sublimetext.4")).nice_space`) + **manual** để xác nhận cảm giác gõ thật |
| 6.6 | Bất kỳ app Apple nào khác (Mail, Notes, Pages...) | dùng chế độ Unicode Compound app-quirk (prefix `com.apple.`) giống OpenKey | auto (unit test quirks prefix match) + **manual** |

## 7. Sleep/wake, đổi Space, đa ứng dụng

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 7.1 | Đóng nắp máy / để máy sleep rồi wake lại, gõ tiếp | VNKey vẫn hoạt động bình thường, không cần khởi động lại | **manual** |
| 7.2 | Đổi Space (Mission Control) sang app khác, gõ | trạng thái Việt/Anh theo app đúng như smart-switch đã lưu | **manual** |
| 7.3 | Mở nhiều app cùng lúc, luân phiên gõ qua lại nhanh | không lẫn trạng thái giữa các app, không crash | **manual** |
| 7.4 | Tắt máy/mở lại (khởi động lại macOS), VNKey tự chạy nếu bật "Khởi động cùng macOS" | app tự khởi động, quyền Accessibility còn giữ (nếu đã ký bằng cert ổn định — xem docs/dev-signing.md) | **manual** |

## 8. UI chung (đã agent tự kiểm qua osascript — Task 16)

| # | Thao tác | Kỳ vọng | Trạng thái |
|---|----------|---------|------------|
| 8.1 | Mở "Bảng điều khiển" qua tray | cửa sổ hiện, có tiêu đề "VNKey – Bảng điều khiển"; nếu chưa cấp quyền Accessibility thì hiện màn hình "Cần cấp quyền Trợ năng" | **đã tự kiểm** (Task 16 osascript drive — permission gate hiện đúng vì agent không có quyền) |
| 8.2 | Mở "Gõ tắt" qua tray | cửa sổ "Thiết lập gõ tắt" hiện đủ bảng, form, nút Thêm/Xoá/Nạp/Xuất | **đã tự kiểm** |
| 8.3 | Mở "Công cụ chuyển mã" qua tray | cửa sổ hiện đủ 2 nhóm tùy chọn, select bảng mã, nút Chuyển mã/Đóng | **đã tự kiểm** |
| 8.4 | Mở "Giới thiệu" qua tray | cửa sổ hiện tên/mô tả/credit/link, checkbox kiểm tra bản mới | **đã tự kiểm** |
| 8.5 | Đóng cửa sổ bằng nút đỏ titlebar | ẩn cửa sổ (không thoát app); mở lại qua tray ra đúng cửa sổ cũ (không mất trạng thái) | **đã tự kiểm** |
| 8.6 | Thoát app qua tray "Thoát" | tiến trình kết thúc sạch | **đã tự kiểm** |
| 8.7 | Gõ tiếng Việt thật vào Chrome/Terminal (không qua UI) | ra đúng chữ có dấu | **đã user xác nhận trước đó** (progress ledger: "USER XÁC NHẬN VNKey GÕ ĐƯỢC TIẾNG VIỆT THẬT trên Chrome + Terminal") ở Task 8; vẫn nên gõ lại lần cuối sau khi build release để chắc chắn không có regression từ Task 9-15 |

## Ghi chú

- Các mục "auto" đã được xác nhận bit-exact bằng differential testing (so
  với `openkey-oracle`, FFI wrapper quanh C++ gốc) trên hàng trăm nghìn
  chuỗi phím proptest + tập golden case cố định — xem README phần
  Testing.
- Các mục "manual" cần Accessibility thật (agent không thể tự cấp quyền
  này) hoặc cần thao tác hệ thống (sleep/wake, đổi Space, đổi Input
  Source) mà chỉ người dùng trên máy thật mới làm được.
- Nếu phát hiện lệch hành vi ở bất kỳ mục "manual" nào, ưu tiên nghi ngờ
  tầng macOS (`src-tauri/src/platform/mac/`) trước — engine (`vnkey-engine`)
  đã được differential test rất kỹ và gần như chắc chắn đúng.
