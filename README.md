# VNKey

Bộ gõ tiếng Việt cho macOS, hỗ trợ Telex, VNI, gõ tắt và chuyển mã văn bản.
VNKey chạy trên thanh menu: **V** khi bật tiếng Việt, **E** khi tắt tiếng Việt.

## Tính năng

- **Kiểu gõ và bảng mã:** Telex, VNI, Simple Telex 1, Simple Telex 2; Unicode
  dựng sẵn, TCVN3 (ABC), VNI Windows, Unicode tổ hợp và Vietnamese Locale CP 1258.
- **Chính tả và khôi phục:** chọn cách đặt dấu `òa`/`oà`, `úy`/`uý`; kiểm tra
  chính tả, tự khôi phục từ tiếng Anh và nhấn `Esc` để trả lại phím gốc của từ đang gõ.
- **Gõ nhanh và gõ tắt:** mở rộng phụ âm, tạo bảng gõ tắt, nhập/xuất tệp `.txt`,
  tự viết hoa theo phím tắt và tùy chọn dùng gõ tắt cả ở chế độ tiếng Anh.
- **Thiết lập theo ứng dụng:** ghi nhớ chế độ Việt/Anh và bảng mã riêng;
  hỗ trợ xử lý lỗi gợi ý trong trình duyệt và một số ứng dụng.
- **Tiện ích:** chuyển mã nội dung Clipboard, chuyển hoa/thường, bỏ dấu,
  phím tắt tùy chỉnh, khởi động cùng macOS và giao diện sáng/tối/theo hệ thống.

## Cài đặt từ mã nguồn

### Yêu cầu

- Máy Mac dùng Apple Silicon hoặc Intel.
- Rust stable mới nhất và Cargo, cài qua [rustup](https://rustup.rs).
- Node.js 22.12+ và npm; cũng hỗ trợ nhánh Node.js 20 từ phiên bản 20.19.
- Xcode Command Line Tools. Nếu chưa có, chạy `xcode-select --install`.

### Build và mở ứng dụng

1. Tải mã nguồn và cài các gói giao diện:

   ```sh
   git clone https://github.com/vudung1202/VNKey.git
   cd VNKey
   npm ci
   ```

2. Build bản release và mở VNKey:

   ```sh
   ./retest.sh
   ```

   Script build ứng dụng, thoát bản VNKey đang chạy, đặt lại quyền Accessibility
   của VNKey rồi mở bản mới. Ứng dụng nằm ở
   `target/release/bundle/macos/VNKey.app`; script không cài vào `/Applications`.

3. Vào **System Settings → Privacy & Security → Accessibility**, bật **VNKey**.
   Có thể mở trang này bằng nút **Mở cài đặt Trợ năng** trong ứng dụng.

4. Nhấn **Khởi động lại VNKey** trên màn hình cấp quyền. VNKey cần quyền này
   để nhận và chèn phím vào các ứng dụng khác.

**Sau mỗi lần build lại:** chữ ký ad-hoc thay đổi nên macOS có thể không còn
chấp nhận quyền đã cấp, kể cả khi công tắc VNKey vẫn bật. Dùng `./retest.sh`
để xử lý quyền cũ, sau đó cấp quyền và khởi động lại như trên.

## Sử dụng

### Bắt đầu gõ

1. Chọn nguồn nhập **ABC** hoặc **U.S.** của macOS; tắt các bộ gõ tiếng Việt
   khác để tránh xử lý phím hai lần.
2. Nhấn chữ **V/E** trên thanh menu, mở **Bảng điều khiển...**.
3. Chọn **Telex**, **Unicode dựng sẵn** và chế độ **Tiếng Việt**.
4. Mở một ô nhập văn bản, gõ `vieejt` rồi nhấn dấu cách. Kết quả: `việt `.

Phím chuyển Việt/Anh mặc định là **⌥Z (Option+Z)**. Đổi phím này tại
**Phím chuyển** trong Bảng điều khiển. Có thể đổi kiểu gõ và bảng mã ngay từ
menu **V/E**.

### Gõ xen kẽ tiếng Anh

Trong tab **Bộ gõ**, hai tùy chọn sau mặc định được bật:

- **Tự động khôi phục từ tiếng Anh:** trả lại phím gốc khi nhận diện được
  từ tiếng Anh, trong lúc gõ hoặc khi kết thúc từ.
- **Nhấn ESC để khôi phục từ đang gõ:** chủ động hoàn tác việc đặt dấu của
  từ hiện tại. Ví dụ, gõ `this` rồi nhấn `Esc` để nhận `this` thay vì `thí`.

Nhấn `Esc` trước dấu cách hoặc dấu câu. Các từ trùng âm tiết tiếng Việt như
`this`, `mix`, `test`, `been` không được tự động khôi phục chỉ vì cũng là từ
tiếng Anh. Khi gõ một đoạn tiếng Anh dài, dùng **⌥Z** để chuyển chế độ.

### Gõ tắt và gõ nhanh

1. Mở tab **Gõ tắt**, bật **Cho phép gõ tắt**.
2. Chọn **Bảng gõ tắt...** để thêm từ viết tắt và nội dung thay thế.
3. Gõ từ viết tắt trong ứng dụng đích rồi nhấn dấu cách để mở rộng.

Bảng gõ tắt có nút nhập/xuất tệp `.txt`. Khi nhập, ứng dụng hỏi có giữ lại
các mục hiện tại hay thay thế chúng.

Các tùy chọn gõ nhanh nằm trong cùng tab và mặc định tắt:

- **Gõ nhanh:** `cc→ch`, `gg→gi`, `kk→kh`, `nn→ng`, `qq→qu`, `pp→ph`,
  `tt→th`, `uu→ươ`.
- **Phụ âm đầu:** `f→ph`, `j→gi`, `w→qu`.
- **Phụ âm cuối:** `g→ng`, `h→nh`, `k→ch`.

### Chuyển mã văn bản

1. Sao chép đoạn văn bản cần chuyển mã.
2. Mở **Bảng điều khiển → Chuyển mã**, chọn bảng mã nguồn và bảng mã đích.
3. Chọn thêm chuyển hoa/thường hoặc bỏ dấu nếu cần, rồi nhấn **Chuyển mã**.
4. Dán kết quả từ Clipboard vào tài liệu.

Thao tác này thay nội dung văn bản trong Clipboard. Có thể đặt **Phím chuyển
mã nhanh** trong cùng tab để dùng lại cấu hình chuyển mã mà không mở bảng.

## Xử lý sự cố và giới hạn

| Hiện tượng | Cách kiểm tra hoặc xử lý |
| --- | --- |
| Không gõ được tiếng Việt | Kiểm tra chữ **V** trên thanh menu, quyền Accessibility và nguồn nhập **ABC/U.S.**. Tùy chọn tắt tiếng Việt khi nguồn nhập hệ thống khác tiếng Anh mặc định bật. |
| Đã bật quyền nhưng bản build mới vẫn yêu cầu cấp quyền | Chạy lại qua `./retest.sh`, cấp lại quyền rồi khởi động lại VNKey. |
| Lặp ký tự hoặc lỗi dấu ở thanh địa chỉ trình duyệt | Kiểm tra **Bộ gõ → Sửa lỗi gợi ý (trình duyệt, Excel,...)** trước. Tùy chọn **Sửa lỗi trên Chromium (beta)** không thay thế công tắc này. |
| Từ tiếng Anh vẫn bị đặt dấu | Nhấn `Esc` khi còn đang gõ từ hoặc chuyển sang chế độ **E**. Dấu có thể xuất hiện tạm thời trước khi từ được tự khôi phục. |
| Không thấy ứng dụng trên Dock | VNKey mặc định chạy ở thanh menu. Mở Bảng điều khiển từ chữ **V/E**, rồi bật **Hệ thống → Hiện biểu tượng trên thanh Dock** nếu cần. |

Khôi phục phím gốc có giới hạn lịch sử 32 phím cho một từ; chuỗi dài hơn
không được bảo đảm khôi phục đầy đủ. `Esc` cũng không bảo đảm khôi phục từ
đã bị chỉnh sửa bằng phím xóa.

Chức năng kiểm tra bản mới chưa kết nối nguồn phát hành. Hiện cần cập nhật
mã nguồn và build lại; không dựa vào nút kiểm tra cập nhật trong giao diện.

## Cấu hình và dữ liệu

VNKey lưu dữ liệu tại `~/Library/Application Support/VNKey/`:

| Tệp | Nội dung |
| --- | --- |
| `settings.json` | Thiết lập bộ gõ, giao diện, phím tắt và chuyển mã |
| `macros.bin` | Bảng gõ tắt |
| `smart_switch.bin` | Chế độ Việt/Anh và bảng mã được ghi nhớ theo ứng dụng |

Thoát VNKey trước khi sao lưu hoặc chỉnh sửa trực tiếp các tệp này. Các giá
trị đã lưu được ưu tiên hơn mặc định của phiên bản mới.

## Phát triển

### Chạy và đóng gói

Đảm bảo Cargo có trong `PATH` trước khi dùng các lệnh build hoặc test:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Chạy ứng dụng ở chế độ phát triển, tự tải lại khi sửa giao diện:

```sh
npm run tauri dev
```

Chỉ chạy `npm run dev` sẽ mở máy chủ Vite, không khởi động bộ gõ macOS.

Khi cần build lại và kiểm thử bộ gõ thực tế, dùng script xử lý quyền:

```sh
./retest.sh --debug
```

Bản debug nằm ở `target/debug/bundle/macos/VNKey.app`.

Nếu chỉ cần đóng gói, không mở ứng dụng:

```sh
npm run tauri build
```

Bản release nằm ở `target/release/bundle/macos/VNKey.app`; tệp `.dmg` nằm
trong `target/release/bundle/dmg/` nếu bước đóng gói DMG thành công. Build
thông thường theo kiến trúc máy đang chạy, không mặc định tạo bản universal.

Cấu hình mặc định không đặt `signingIdentity`. Hướng dẫn dùng chứng chỉ tự-ký
khi phát triển nằm trong [docs/dev-signing.md](docs/dev-signing.md).

### Kiểm thử

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run build
```

Các lệnh lần lượt chạy test Rust, kiểm tra lint Rust, rồi typecheck và build
giao diện. Có thể kiểm thử riêng engine mà không cần chạy ứng dụng:

```sh
cargo test -p vnkey-engine
```

Test tự động kiểm tra thuật toán gõ, bảng mã, gõ tắt, khôi phục tiếng Anh và
các phần logic của tầng macOS. Việc bắt/chèn phím, phím tắt hệ thống và đổi
ứng dụng vẫn cần kiểm thử thực tế theo
[docs/PARITY-CHECKLIST.md](docs/PARITY-CHECKLIST.md).

### Cấu trúc dự án

```text
crates/vnkey-engine/   Engine thuần Rust, không phụ thuộc Tauri hay macOS
src-tauri/src/        Tầng macOS, lệnh Tauri, cấu hình và menu V/E
src/                  Giao diện React và TypeScript trên Tauri v2
docs/                 Kiến trúc, kiểm thử thủ công và hướng dẫn ký ứng dụng
```

Phần tích hợp AppKit/CoreGraphics và `CGEventTap` nằm trong
`src-tauri/src/platform/mac/`. Luồng xử lý phím và ranh giới engine/ứng dụng
được mô tả trong [docs/architecture.md](docs/architecture.md).
