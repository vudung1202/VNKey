# CLAUDE.md - hướng dẫn cho agent làm việc trên VNKey

VNKey là bộ gõ tiếng Việt cho macOS: engine thuần Rust + tầng macOS
(CGEventTap/objc2) + UI Tauri v2 & React/TypeScript.

README mô tả tính năng và cách build cho người dùng. File này chỉ ghi những
thứ **không tự suy ra được từ code** - ranh giới kiến trúc, các bẫy đã trả giá
để biết, và quy ước của dự án. Đọc file này trước khi sửa.

## Lệnh hay dùng

```sh
export PATH="$HOME/.cargo/bin:$PATH"     # BẮT BUỘC, cargo không có sẵn trong PATH

cargo test --workspace                    # 231 test, phải xanh trước khi commit
cargo clippy --workspace --all-targets -- -D warnings
npm run build                             # tsc + vite (typecheck UI)
npx tauri build --debug                   # ra target/debug/bundle/macos/VNKey.app
./retest.sh                               # build + reset quyền TCC + mở app
```

## Ranh giới kiến trúc (quy tắc quan trọng nhất)

```
crates/vnkey-engine/   Thuần Rust, KHÔNG biết gì về Tauri/macOS.
                       Test độc lập bằng golden + unit test.
src-tauri/src/         Tầng macOS + Tauri. platform/mac/ là chỗ duy nhất
                       chạm AppKit/CoreGraphics.
src/                   UI React, route theo window label trong main.tsx.
```

Đừng để `vnkey-engine` phụ thuộc Tauri/macOS. Mọi thứ liên quan hệ điều hành
nằm trong `src-tauri`.

## Bẫy đã trả giá để biết

**1. TSM/TIS chỉ được gọi trên main thread.** Gọi
`TISCopyCurrentKeyboardInputSource` từ tap thread làm macOS 15 **crash**
(`dispatch_assert_queue`). Vì vậy `tis.rs` đọc TIS trên main thread rồi cache
vào `SharedConfig::input_is_english`; tap thread chỉ đọc atomic. Đừng gọi
thẳng TIS trong `tap.rs`.

**2. Nhiều đoạn trong engine trông "sai" nhưng là cố ý.** Thuật toán gõ tiếng
Việt có rất nhiều ca biên, và hành vi hiện tại đã được golden test ghim chặt.
Trước khi "sửa cho đẹp" hay đơn giản hoá, chạy `cargo test` - nếu test đỏ thì
đó là hành vi có chủ đích, không phải bug.

**3. `temp_disable_key` không thuần là "sai chính tả".** `spelling.rs` đặt nó
theo kết quả kiểm tra chính tả, nhưng `marks.rs` cũng đặt nó ở các ca hoàn tác
dấu trùng / "thoòng". Muốn thêm tín hiệu mới thì OR vào điều kiện, đừng bật
`temp_disable_key` để "báo hiệu" - sẽ vô tình chặn cả biến đổi giữa từ.

**4. `buf.states` bị rewind, `RawJournal` thì không.** Đường hoàn tác phím
trùng giảm `state_index`, phá lịch sử phím thô (gõ `off` chỉ còn `of`). Vì vậy
`english.rs` giữ `RawJournal` riêng cho ESC. Đường tự khôi phục vẫn dùng
`buf.states` vì số ký tự của nó đang bị test ghim - đừng gộp hai nguồn.

**5. `MAX_BUFF = 32`.** `push_state` ghim ghi ở `states[31]` mà không tăng
`state_index` khi vượt 32 phím. `backspace_count`/`new_char_count` là `u8` và
`char_data` rộng 32 - luôn kiểm tra biên trước khi ép kiểu.

**6. Ad-hoc signing làm macOS quên quyền mỗi lần build.** Quyền Accessibility
gắn với CDHash, mà CDHash đổi mỗi lần build. Sau khi build lại phải cấp quyền
lại. `./retest.sh` tự động phần reset. Đây là hành vi bình thường, không phải
lỗi cần sửa.

**7. Lỗi gõ ở thanh địa chỉ Chromium: kiểm `fix_recommend_browser` TRƯỚC.**
`gift`→`ggift`, `ddang`→`dđang`, `gg docs`→`gg dóoc` trông y như bug code,
nhưng cơ chế chữa nó đã có sẵn: ký tự rỗng U+202F, nằm sau công tắc **"Sửa lỗi
gợi ý (trình duyệt, Excel,...)"** (`fix_recommend_browser`, default `true`).
Tắt công tắc đó thì `should_use_recommend_workaround()` thoát sớm và **cả khối
autocomplete không chạy** - kể cả nhánh `fix_chromium_browser` lồng bên trong.
Bật lên là hết lỗi (kiểm tay 2026-08-17). Báo cáo `ggift` dẫn tới `f338e05` đến
từ một máy đang tắt công tắc này, nên cái được "sửa" thực ra chưa từng chạy.
Đọc `settings.json` để xác minh, đừng tin vào default trong code - khoá đã lưu
sẽ đè default.

**8. ĐỪNG cho Chromium dùng selection replacement.** Đã thử ở `f338e05` để sửa
lỗi omnibox, và nó **phá Google Docs**: Docs vẽ bằng canvas, tự quản lý
selection riêng, nên Shift+Left tổng hợp đua với lệnh chèn tổng hợp và những ký
tự lẽ ra bị xoá thì còn nguyên (`nguyễn thị phương thảo` ra `nguyễn uyê thị
phuoương thảo`). Là **race chứ không phải sai số đếm**: đo trong cùng một lần
gõ, `thao`+`r` và `phuo`+`w` yêu cầu engine sửa y hệt nhau (bpc=2) mà một cái
đúng một cái sai - nên không tinh chỉnh được cho ổn định. Bundle id không tách
được thanh địa chỉ khỏi nội dung trang, mà nội dung trang mới là chỗ hỏng tài
liệu, nên nội dung trang thắng. Có test ghim trong `quirks.rs`.

## Thêm một tuỳ chọn mới: phải đi hết 6 tầng

1. `crates/vnkey-engine/src/options.rs` - `EngineOptions` + default
2. `src-tauri/src/settings/mod.rs` - `Settings` + default + `to_engine_options`
3. `src-tauri/src/state.rs` - **chỉ khi** tap thread cần đọc trên hot path
4. `src/lib/types.ts` - mirror kiểu TS
5. `src/lib/strings.ts` - nhãn tiếng Việt
6. `src/components/ControlPanel/TypingTab.tsx` (hoặc tab tương ứng) - hàng UI

**Muốn một tuỳ chọn mặc định BẬT cho cả máy đã cài:** dùng
`#[serde(default = "default_true")]`. Lật default của field cũ **không có tác
dụng** vì khoá đó đã nằm sẵn trong `settings.json` với giá trị cũ. Chỉ field
mới (khoá vắng mặt) mới nhận default. Có test ghim điều này trong
`settings/mod.rs`.

## Quy ước

- **Mọi chuỗi tiếng Việt hiển thị nằm ở `src/lib/strings.ts`.** Không hardcode
  trong component.
- **Không dùng em-dash (U+2014) hay en-dash (U+2013)** ở bất cứ đâu, kể cả
  comment. Dùng `-`. Repo hiện sạch (grep ra 0), giữ nguyên như vậy.
- **Theme dùng `data-theme` trên `<html>`**, không dùng `@media
  (prefers-color-scheme)`. `lib/theme.ts` quyết định, script trong
  `index.html` chống nháy khi khởi động.
- **Menu bar chỉ hiện chữ V/E, không icon** - quyết định sản phẩm, cố ý
  (`tray.rs`).
- Comment giải thích **tại sao**, không mô tả lại code.

## Cách kiểm chứng hành vi engine

Đừng đoán engine làm gì - đo. Thêm test tạm vào
`crates/vnkey-engine/tests/golden_typing.rs` dùng `type_str(opts, "chuoiphim")`
rồi chạy `cargo test -p vnkey-engine <ten> -- --nocapture`, xong xoá test tạm.
Harness `type_str` mô phỏng đúng cách host áp `HookState` vào ô văn bản.

Ký tự sentinel trong `key_of`: `\u{8}` = DELETE, `\n` = ENTER, `\u{1b}` = ESC.

## Giới hạn đã biết (đừng coi là bug)

- **Tự khôi phục tiếng Anh không bắt được từ trùng âm tiết Việt hợp lệ**:
  `this`→thí, `mix`→mĩ, `test`→tét, `is`→í. Không luật nào phân biệt được nếu
  không có từ điển. **ESC** là lối thoát (khôi phục vô điều kiện). Đã ghi vào
  golden test để lần sau sửa là thay đổi có chủ ý.
- **`ass`→`as` là hành vi tiếng Việt cố ý** (có test ghim) và giống hệt
  `off`→`of` về cấu trúc, nên không luật tự động nào tách được hai ca này.
- Từ dài quá 32 phím mất lịch sử phím thô, không khôi phục trung thực được.

## Tài liệu khác

- `docs/architecture.md` - **đọc trước khi sửa sâu**: luồng một phím, mô hình
  luồng/khoá, giao thức `HookState`, cách đồng bộ cấu hình
- `README.md` - build, cấp quyền, tính năng (cho người dùng)
- `docs/PARITY-CHECKLIST.md` - checklist kiểm thử tay
- `docs/dev-signing.md` - tạo cert tự-ký để quyền không bị quên mỗi lần build

## Ghi công / giấy phép

Phần ghi công và `LICENSE` đã được chủ dự án chỉnh theo chủ đích. **Đừng tự ý
thêm, sửa hay xoá** - hỏi chủ dự án trước.
