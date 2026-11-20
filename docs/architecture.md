# Kiến trúc VNKey

Tài liệu đi sâu vào *cách VNKey hoạt động bên trong*. README nói về build và
tính năng; `CLAUDE.md` nói về quy ước và bẫy khi sửa code. File này giải thích
luồng dữ liệu, mô hình luồng (threading), và các giao thức nội bộ.

Tham chiếu ghi dạng `file.rs::hàm`. Số dòng chỉ mang tính chỉ đường vì sẽ trôi
theo thời gian.

## 1. Ba tầng

```
┌─────────────────────────────────────────────────────────────┐
│  crates/vnkey-app/src/ui/  (GPUI qua gpui-kit)              │
│  2 cửa sổ: Bảng điều khiển, Gõ tắt                          │
└───────────────▲─────────────────────────┬───────────────────┘
                │  observe_global          │  gọi hàm Rust
                │  (UiState)               │  commands::*
┌───────────────┴─────────────────────────▼───────────────────┐
│  crates/vnkey-app/src/  (Rust, tầng macOS)                  │
│                                                              │
│   platform/mac/   CGEventTap, inject, hotkey, TIS, quirks   │
│   state.rs        EngineCore (Mutex) + SharedConfig (atomic)│
│   commands.rs     hành động   tray.rs    menu V/E (muda)    │
│   settings/       JSON + blob app.rs     AppEvent, UiState  │
└───────────────────────────┬─────────────────────────────────┘
                            │  handle_event() -> &HookState
┌───────────────────────────▼─────────────────────────────────┐
│  crates/vnkey-engine/  (Rust thuần, không biết UI/macOS)    │
│  bảng mã, xử lý phím, chính tả, macro, smart-switch, convert│
└─────────────────────────────────────────────────────────────┘
```

**Ranh giới cứng:** `vnkey-engine` không phụ thuộc UI hay macOS. Nó nhận
`(sự kiện phím, keycode, caps)` và trả về một `HookState` mô tả *phải làm gì*,
chứ không tự làm. Nhờ vậy engine test được độc lập bằng golden test.

## 2. Khởi động

`app.rs::run` → callback khởi chạy của GPUI → `platform::mac::start`
(`platform/mac/mod.rs`):

1. `Settings::load()` - đọc `settings.json`, lần đầu thì ghi mặc định. **Phải
   xong trước khi dựng engine**, để phím đầu tiên đã thấy cấu hình thật.
2. `ffi::is_trusted()` - chưa có quyền Accessibility thì `prompt_for_trust()`.
   Vẫn spawn tap (nó hỏng mềm), người dùng cấp quyền rồi mở lại.
3. Dựng `Engine` + nạp `macros.bin`, `smart_switch.bin`.
4. Dựng `SharedConfig` từ options, `apply_to_shared_config`.
5. `config.set_frontmost_bundle_id(...)` - mồi cache bundle id **trên main
   thread**, để tap thread không bao giờ phải gọi NSWorkspace.
6. `workspace::register` - đăng ký observer NSWorkspace (main thread bắt buộc)
   và `tis::install_input_source_observer`.
7. `tap::spawn` - tạo thread riêng chạy CFRunLoop cho CGEventTap.

Sau đó `app.rs` dựng tray (`tray::build`), chạy task main-thread xử lý
`AppEvent`, và tự mở Bảng điều khiển nếu thiếu quyền hoặc người dùng bật
`show_ui_on_startup`. `show_icon_on_dock` tắt thì app khởi động ở chế độ
`Accessory` (`with_windowing(Headless)`) để không loé icon Dock.

## 3. Luồng một phím (phần lõi)

`tap::tap_callback` chạy trên **tap thread**, trong callback của CGEventTap.
Đây là hot path: mỗi phím người dùng gõ đều đi qua đây.

| Bước | Việc | Ghi chú |
|---|---|---|
| 0 | Bật lại tap nếu hệ thống tự tắt | `kCGEventTapDisabledBy*` |
| 1 | Bỏ qua sự kiện do chính VNKey sinh ra | tránh vòng lặp vô hạn |
| 1.5 | Đang ghi hotkey trong UI thì cho qua hết | `hotkey_recording` |
| 2 | Đổi keycode theo layout (Dvorak/Colemak) | chỉ KeyDown |
| 3 | Bắt hotkey chuyển Việt/Anh, chord tạm tắt | KeyDown + FlagsChanged |
| 4 | Chuột nhấn/kéo → reset phiên gõ | `request_new_session` |
| 5 | Chỉ KeyDown mới xuống engine | KeyUp/khác cho qua |
| 6 | Engine tắt (`language == 0`) → cho qua | **trước** bước 7 |
| 7 | Input source hệ thống không phải tiếng Anh → cho qua | đọc **cache** |
| 8 | **Chạy engine dưới khoá**, chụp quyết định, nhả khoá | xem dưới |
| 9 | Trả `sync_key`, chạy `start_new_session` bị hoãn | dưới khoá lần 2 |

Giữa bước 8 và 9 là **injection chạy không khoá**.

### Vì sao tách khoá như vậy

```rust
let (snap, mut sync_key, code_table) = {
    let mut core = ctx.engine.lock();
    let code_table = ctx.config.code_table();      // đọc CÙNG lần khoá
    let hook = core.engine.handle_event(...);      // engine encode ở đây
    let snap = HookSnapshot { ...hook };           // chụp lại
    let sk = std::mem::take(&mut core.sync_key);   // mượn ra ngoài
    (snap, sk, code_table)
};
// ... inject::dispatch(...) chạy LOCK-FREE ...
```

Ba lý do:

1. **Injection không được giữ khoá.** Post sự kiện bàn phím có thể khiến hệ
   thống gọi lại vào tap → nếu còn giữ khoá engine sẽ deadlock.
2. **`code_table` phải đọc trong cùng lần khoá** với lúc engine encode. Nếu đọc
   riêng, một lần đổi bảng mã (từ tray hoặc smart-switch) chen vào giữa sẽ làm
   decode và encode dùng hai bảng khác nhau → lệch `sync_key`.
3. **`sync_key` được mượn ra rồi trả lại** ở bước 9, vì injection cần cập nhật
   nó mà không giữ khoá.

## 4. Mô hình luồng

| Thread | Được chạm | KHÔNG được chạm |
|---|---|---|
| **Main** | AppKit, NSWorkspace, TIS/TSM, tray, GPUI (`App`, cửa sổ) | - |
| **Tap** | engine (dưới khoá), CGEvent post, atomics | **AppKit, TIS/TSM, NSWorkspace** |

Quy tắc: **mọi API AppKit/Carbon chỉ được gọi trên main thread.** Vi phạm gây
crash thật (macOS 15 bắt `dispatch_assert_queue` trong TSM).

Hai giá trị main-thread-only được **cache** để tap thread đọc lock-free:

- `SharedConfig::frontmost_bundle_id` - do observer `NSWorkspaceDidActivate`
  cập nhật (`workspace.rs`), dùng cho per-app quirks.
- `SharedConfig::input_is_english` - do `tis.rs::install_input_source_observer`
  cập nhật khi input source đổi.

Chiều ngược lại (tap thread hoặc observer cần đụng tray/UI) đi qua
`AppSender::send(AppEvent::..)` (`app.rs`): một task GPUI trên main thread
nhận và xử lý với `&mut App`. Ví dụ hotkey đổi ngôn ngữ →
`AppEvent::LiveChange` → `tray::sync_after_live_change` → cập nhật tray +
`UiState` + lưu đĩa. Gửi là fire-and-forget, tap thread không bao giờ chờ.

## 5. Hai loại state chia sẻ, vì sao

`state.rs` cố ý chia làm hai:

**`EngineCore` sau `Arc<Mutex<_>>`** - thứ cần nhất quán với nhau:
- `engine: Engine` - toàn bộ trạng thái gõ
- `sync_key: Vec<u8>` - số đơn vị UTF-16 của từng ký tự đã chèn, dùng để tính
  số backspace thật cho bảng mã 2 byte (VNI Windows, Unicode tổ hợp)
- `smart_switch: SmartSwitchTable` - nhớ Việt/Anh + bảng mã theo từng app

**`SharedConfig` gồm atomics** - cờ mà tap thread đọc **mỗi phím**, không đáng
để khoá: `enabled`, `other_language`, `input_is_english`, `code_table`,
`temp_off_spelling`, `hotkey_recording`, `fix_recommend_browser`, ...

Nguyên tắc: cờ đọc trên hot path → atomic; trạng thái phải nhất quán theo nhóm
→ dưới khoá.

## 6. Giao thức Engine ↔ Host

Engine không tự chèn chữ. Nó trả về `HookState` (`data.rs`):

```rust
pub struct HookState {
    pub code: HookCode,          // làm gì
    pub backspace_count: u8,     // xoá lùi bao nhiêu
    pub new_char_count: u8,      // chèn bao nhiêu ký tự
    pub ext_code: u8,            // 1 ngắt từ, 2 xoá, 3 phím thường, 4 không chèn ký tự rỗng
    pub char_data: [u32; 32],    // ĐẢO NGƯỢC: [0] là ký tự phải cùng
    pub macro_key: Vec<u32>,
    pub macro_data: Vec<u32>,
}
```

`HookCode`:

| Mã | Ý nghĩa |
|---|---|
| `DoNothing` | Cho phím đi qua nguyên vẹn |
| `WillProcess` | Xoá lùi rồi chèn chữ đã biến đổi (đường thường) |
| `Restore` | Khôi phục phím thô, **rồi phát lại** phím kích hoạt |
| `RestoreAndStartNewSession` | Như trên + bắt đầu phiên mới |
| `RestoreAndSwallow` | Khôi phục nhưng **nuốt** phím kích hoạt (dùng cho ESC) |
| `ReplaceMacro` | Thay bằng nội dung gõ tắt |

**Ba dạng ô trong `char_data`** (`inject.rs::encode_cell` giải mã):

1. Có `CHAR_CODE_MASK` → mã ký tự theo bảng mã đang dùng
2. Có `PURE_CHARACTER_MASK` → ký tự chuyển thẳng
3. Không cờ nào → **keycode thô**, ánh xạ ngược qua `key_code_to_character`
   (đây là dạng mà đường khôi phục dùng)

Host (`inject.rs::dispatch`) thực thi: workaround autocomplete (chèn ký tự rỗng
rồi xoá, để dẹp popup gợi ý) → backspace → `unicode_string` theo lô 16 đơn vị →
phát lại phím kích hoạt nếu `Restore*`.

## 7. Bên trong engine

### Buffer hai tầng

```rust
pub(crate) struct Buffer {
    pub word:  [u32; 32],   // từ ĐÃ biến đổi (có bit dấu/mũ/móc)
    pub index: i32,
    pub states:[u32; 32],   // phím THÔ như người dùng gõ
    pub state_index: i32,
    pub long_word_helper: Vec<u32>,  // tràn khi từ dài hơn 32
}
```

Mỗi ô là `keycode | CAPS_MASK | TONE_MASK | TONEW_MASK | MARKn_MASK`. `word` bị
`marks.rs` sửa tại chỗ khi thêm dấu; `states` là bản gốc để khôi phục.

Ngoài ra `english.rs::RawJournal` giữ một bản ghi phím thô thứ ba, **không bao
giờ bị rewind** - cần vì đường hoàn tác phím trùng có giảm `state_index` (gõ
`off` thì `states` chỉ còn `of`). ESC và các đường tự khôi phục tiếng Anh
(giữa từ và cuối từ, xem đầu `english.rs`) dùng journal này.

### `handle_event` chia 4 nhánh

| Nhánh | Kích hoạt | Việc chính |
|---|---|---|
| Ngắt từ | `BREAK_CODE` (`, . / ; ' \ - = ESC TAB ENTER` mũi tên), chuột | macro → gõ nhanh phụ âm → tự khôi phục → lưu từ vào lịch sử |
| SPACE | `KEY_SPACE` (**không** nằm trong `BREAK_CODE`) | như trên, nhánh riêng |
| DELETE | `KEY_DELETE` | lùi buffer, khôi phục từ lịch sử khi qua ranh giới từ |
| Phím thường | còn lại | `push_state` → biến đổi → `HookState` |

### Đường biến đổi một phím thường

`insert_key` → tuỳ phím mà gọi trong `marks.rs`:
`insert_mark` (dấu thanh) · `insert_d` (đ) · `insert_aoe` (â ê ô) ·
`insert_w` (ă ư ơ) · `remove_mark` (z) · `check_grammar` (đặt lại vị trí dấu).

`find_and_calculate_vowel` xác định vị trí nguyên âm (VSI/VEI/VWSM) mà dấu sẽ
rơi vào, có tính `use_modern_orthography` (hoà vs hòa).

### Cổng chính tả

`spelling.rs::check_spelling` khớp buffer với `CONSONANT_TABLE`,
`VOWEL_COMBINE`, `END_CONSONANT_TABLE` rồi kết luận:

```rust
self.temp_disable_key = !(self.spelling_ok && self.spelling_vowel_ok);
```

`temp_disable_key == true` chặn biến đổi tiếp theo → từ sai chính tả không bị
bỏ dấu. **Lưu ý:** cờ này *không thuần* là kết luận chính tả - `marks.rs` cũng
bật nó ở các ca hoàn tác dấu trùng.

Kiểm tra nguyên âm ghép chỉ chạy khi `force_check_vowel == true`, tức chỉ ở
cuối từ (SPACE / phím ngắt), không chạy từng phím - vì Telex đi qua các trạng
thái trung gian *không hợp lệ* trên đường tới từ hợp lệ (`uwowng` → `ương`).

## 8. Cấu hình và đồng bộ

Có **ba nguồn** đổi cấu hình, và chúng phải không giẫm lên nhau:

1. **UI** → `commands::apply_and_broadcast` (gọi hàm trực tiếp)
2. **Tray** → cùng `apply_and_broadcast`
3. **Hotkey / smart-switch** → đổi thẳng atomic trên tap thread, **không** ghi
   đĩa ngay → gửi `AppEvent::LiveChange` → `tray::sync_after_live_change`
   trên main thread

Vấn đề: (1) và (2) đọc `Settings::load()` từ đĩa, trong khi (3) chỉ đổi RAM.
Nếu ghép ngây thơ thì một lần đổi từ UI sẽ **ghi đè ngược** thay đổi do hotkey.

Cách giải: `tray::overlay_live_config` phủ hai trường mà đường (3) có thể đổi
(`language`, `code_table`) lên bản đọc từ đĩa, trước khi `mutate` chạy:

```
Settings::load()  →  overlay_live_config(config)  →  mutate  →  apply + save
```

`apply_settings_core` cập nhật engine option **và** `SharedConfig` trong cùng
một lần khoá, để tap thread không bao giờ thấy trạng thái nửa vời.

Lưu trữ: `~/Library/Application Support/VNKey/` gồm `settings.json` (serde),
`macros.bin`, `smart_switch.bin` (blob nhị phân riêng, không nhét vào JSON vì
lớn và ghi thêm liên tục).

## 9. UI (GPUI)

Một cửa sổ (`ui/control_panel.rs`): sidebar bên trái (icon, công tắc Tiếng
Việt/English, 5 mục: Bộ gõ, Gõ tắt, Chuyển mã, Hệ thống, Thông tin), view của
mục đang chọn bên phải. View Gõ tắt là entity riêng (`ui/macro_editor.rs`).
Mục "Gõ tắt..." trên menu V/E mở cửa sổ ở view đó (`ui::open_macro`). Cửa sổ
được **mở khi cần và huỷ khi đóng**; macOS giữ app chạy khi không còn cửa sổ
nào (`QuitMode` mặc định). Mở lại khi đang mở thì chỉ đưa lên trước và đổi
view. Bấm Dock khi không có cửa sổ nào (`on_reopen`) mở Bảng điều khiển.
Chưa có quyền Trợ năng lúc mở cửa sổ thì cả cửa sổ là màn 2 bước cấp quyền.

UI là **gương thụ động**: mọi hành động đổi cấu hình gọi
`commands::apply_and_broadcast`, hàm này thay global `UiState` (`app.rs`);
mỗi view `observe_global::<UiState>` rồi vẽ lại. Thay đổi từ tray, hotkey hay
smart-switch đi cùng đường đó (`app::broadcast`), nên UI tự cập nhật mà không
cần code đồng bộ riêng. Riêng các `Select` giữ state nội bộ nên
`ControlPanel::sync_selects` đặt lại index khi `UiState` đổi.

Ô ghi phím tắt (`ui/hotkey.rs`): GPUI không cho keycode thô, nên phím in được
quy về vị trí QWERTY bằng `layout::char_to_qwerty_keycode` (cùng bảng với
đường layout-compat). Khi ô đang focus và cửa sổ đang active,
`SharedConfig::hotkey_recording` bật để tap cho mọi phím đi qua.

## 10. Bất biến phải giữ

1. `vnkey-engine` không được phụ thuộc UI/macOS.
2. AppKit/Carbon/TIS chỉ gọi trên main thread; tap thread đọc cache.
3. Không giữ khoá engine khi post sự kiện bàn phím.
4. Đọc `code_table` cùng lần khoá với lúc engine encode.
5. Mọi đường đổi cấu hình đi qua `apply_and_broadcast` (trừ hotkey/smart-switch,
   vốn phải gọi `sync_after_live_change` sau đó).
6. `char_data` luôn đảo ngược; host duyệt từ cao xuống thấp.
7. Kiểm biên trước khi ép `usize` → `u8`: `char_data` rộng 32,
   `backspace_count`/`new_char_count` là `u8`.
