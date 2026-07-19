# VNKey

<!-- TODO: replace https://github.com/yourname/VNKey with the real repo URL everywhere it appears (this file, docs/, src/lib/strings.ts) once the project has a home. -->

VNKey là bộ gõ tiếng Việt cho macOS, viết lại từ
[OpenKey](https://github.com/tuyenvm/OpenKey) (tác giả Mai Vũ Tuyên) bằng
Rust + [Tauri](https://tauri.app). VNKey giữ nguyên logic gõ và các bảng mã
của OpenKey (đã kiểm chứng bit-exact qua differential testing so với bản
C++ gốc) nhưng thay toàn bộ phần nền tảng: engine thuần Rust (không FFI, không
C++), tầng macOS dùng `CGEventTap` viết bằng Rust/objc2, và giao diện
Tauri + React/TypeScript thay cho AppKit/Storyboard.

VNKey là **derivative work** của OpenKey (GPL-3.0) nên cũng phân phối theo
GPL-3.0. Xem [LICENSE](./LICENSE) và [NOTICE.md](./NOTICE.md) để biết chi
tiết ghi công.

> Repo gốc dự án (placeholder, sẽ thay khi có repo thật):
> **https://github.com/yourname/VNKey** <!-- TODO -->

## English summary

VNKey is a Rust + Tauri rewrite of the OpenKey Vietnamese IME for macOS.
The typing engine is pure Rust and has been differential-tested against the
original C++ engine across hundreds of thousands of keystroke sequences (0
divergences). The macOS integration (key interception/injection, hotkeys,
smart per-app switching, sleep/wake handling) is written directly against
AppKit/CoreGraphics via `objc2`, and the UI (Control Panel, macro editor,
convert tool, About) is a Tauri v2 + React/TypeScript app. Licensed
GPL-3.0 as a derivative of OpenKey. See the sections below (in Vietnamese)
for build instructions, the Accessibility permission grant, and the test
suite.

## Kiến trúc

```
crates/vnkey-engine/   engine thuần Rust: bảng mã, xử lý phím, chính tả,
                       gõ tắt (macro), smart-switch, chuyển mã.
                       Không phụ thuộc Tauri/macOS -- test được độc lập.
crates/openkey-oracle/ FFI wrapper quanh mã nguồn C++ gốc của OpenKey, chỉ
                       dùng làm oracle cho differential testing (không
                       build vào app).
src-tauri/src/         tầng macOS + Tauri:
  platform/mac/          CGEventTap bắt/inject phím, hotkey, TIS
                          (other-language), quan sát vòng đời app
                          (sleep/wake, đổi app/Space), smart-switch cache.
  settings/               settings.json + macros.bin + smart_switch.bin,
                          import từ OpenKey (NSUserDefaults).
  tray.rs, commands.rs,   tray 15 mục, IPC (~23 lệnh), pasteboard, events.
  pasteboard.rs
src/                   UI React/TypeScript (Bảng điều khiển, Gõ tắt,
                       Công cụ chuyển mã, Giới thiệu), route theo cửa sổ
                       (window label) trong main.tsx.
```

Engine (`vnkey-engine`) không biết gì về Tauri hay macOS; toàn bộ phần nền
tảng nằm trong `src-tauri`. Đây là ranh giới quan trọng nhất của kiến trúc:
engine có thể test độc lập bằng differential testing so với oracle C++,
tầng macOS chỉ test được bằng unit test (mock) + kiểm tay.

## Build từ nguồn

**Yêu cầu:**
- macOS (Apple Silicon hoặc Intel).
- Rust (qua [rustup](https://rustup.rs)) -- bản `edition = "2024"`, dùng
  `rustc`/`cargo` mới.
- Node.js 20+ và npm.
- Xcode Command Line Tools (`xcode-select --install`) -- cần cho
  `objc2`/CoreGraphics FFI và để build bundle `.app`.

**Cài đặt:**

```sh
git clone <repo-url> VNKey   # TODO: thay <repo-url> bằng URL thật
cd VNKey
npm install
```

**Build & chạy khi phát triển** (hot reload UI):

```sh
npx tauri dev
```

**Build bản debug** (nhanh, không tối ưu, dùng để kiểm tra nhanh):

```sh
npx tauri build --debug
```

Artifact: `target/debug/bundle/macos/VNKey.app` (và
`target/debug/bundle/dmg/VNKey_<version>_aarch64.dmg` nếu bước đóng DMG
thành công).

**Build bản release** (tối ưu, dùng để phân phối):

```sh
npx tauri build
```

Artifact: `target/release/bundle/macos/VNKey.app` và
`target/release/bundle/dmg/VNKey_<version>_aarch64.dmg`.

Ứng dụng được build **ad-hoc sign** (không có Developer ID) -- xem phần
"Ký code" bên dưới về hệ quả với quyền Accessibility.

## Cấp quyền Trợ năng (Accessibility)

VNKey cần quyền **Accessibility** để bắt và chèn phím vào các ứng dụng
khác (giống mọi bộ gõ tiếng Việt trên macOS: Unikey, OpenKey, EVKey...).
Không có quyền này, VNKey mở lên sẽ chỉ hiện màn hình yêu cầu cấp quyền,
không gõ được.

1. Mở VNKey lần đầu -- nếu chưa có quyền, cửa sổ Bảng điều khiển sẽ tự hiện
   màn hình "Cần cấp quyền Trợ năng" với nút **Mở cài đặt Trợ năng**.
2. Hoặc vào thủ công: **System Settings → Privacy & Security →
   Accessibility**, bật công tắc cho **VNKey**.
3. Khởi động lại VNKey (nút **Khởi động lại VNKey** trên chính màn hình đó,
   hoặc thoát/mở lại qua tray).

### Lưu ý về ad-hoc signing

VNKey build ad-hoc (không có certificate ổn định). macOS gắn quyền TCC
(Accessibility) với **chữ ký code** của app, mà chữ ký ad-hoc **đổi mỗi lần
build lại** -- nghĩa là mỗi lần bạn `cargo build`/`tauri build` lại từ
nguồn, macOS sẽ quên quyền đã cấp và bạn phải cấp lại trong System
Settings. Đây là hành vi bình thường khi build từ nguồn, không phải lỗi.

Nếu muốn quyền được nhớ qua các lần build lại (khi phát triển), xem
[docs/dev-signing.md](./docs/dev-signing.md) để tạo certificate tự-ký
"VNKey Dev" ổn định và cấu hình `signingIdentity` trong
`tauri.conf.json`. **Bản build mặc định trong repo này KHÔNG bật
`signingIdentity`** (dùng ad-hoc) -- đây là lựa chọn có chủ đích cho giai
đoạn hiện tại, chưa cấu hình sẵn trong `tauri.conf.json`.

## Import cấu hình từ OpenKey

Nếu máy bạn từng cài OpenKey, VNKey có thể đọc cấu hình cũ
(`NSUserDefaults` domain `com.tuyenmai.openkey`) và chuyển sang VNKey:

- **Tự động:** lần đầu mở VNKey (sau khi đã cấp quyền Accessibility), nếu
  phát hiện có dữ liệu OpenKey và chưa từng hỏi trước đó, VNKey sẽ đề nghị
  import ngay trên Bảng điều khiển.
- **Thủ công:** có thể trigger lại qua nút import trong Bảng điều khiển
  bất cứ lúc nào (lệnh IPC `import_openkey_settings` / `has_openkey_data`).

Import chuyển: kiểu gõ, bảng mã, các tùy chọn (chính tả, gõ tắt,
smart-switch, hotkey...), và dữ liệu macro/smart-switch nếu đọc được từ
blob cũ.

## Tính năng

- **4 kiểu gõ:** Telex, VNI, Simple Telex 1, Simple Telex 2.
- **5 bảng mã:** Unicode dựng sẵn, TCVN3 (ABC), VNI Windows, Unicode tổ
  hợp, Vietnamese Locale CP1258.
- **Chính tả & tự khôi phục:** phát hiện gõ sai chính tả, tự trả lại phím
  gốc; chọn kiểu chính tả hiện đại/cũ (oà/òa, uý/úy...).
- **Gõ nhanh (quick telex/consonant):** cc→ch, gg→gi, kk→kh, nn→ng,
  qq→qu, pp→ph, tt→th, uu→ươ; phụ âm đầu f/j/w; phụ âm cuối g/h/k.
  z/w/j/f như phím phụ âm gõ tắt kiểu Telex.
- **Gõ tắt (macro):** bảng gõ tắt tự định nghĩa, tự động viết hoa theo
  ngữ cảnh (ko/Ko/KO), dùng được cả khi tắt gõ tiếng Việt.
- **Smart-switch:** nhớ trạng thái Việt/Anh và bảng mã riêng cho từng ứng
  dụng.
- **Hotkey:** phím tắt chuyển Việt/Anh (mặc định ⌥Z) có tiếng beep tùy
  chọn; tổ hợp Ctrl/Cmd tạm tắt engine; bỏ qua khi gõ ở layout bàn phím
  khác tiếng Anh (other-language).
- **Công cụ chuyển mã:** chuyển đổi giữa 5 bảng mã, kèm chuyển hoa/thường
  và bỏ dấu; có phím tắt chuyển mã nhanh nội dung Clipboard.
- **Sửa lỗi riêng cho từng ứng dụng:** Chrome/Chromium, Terminal,
  Spotlight, Excel/Sublime (theo danh sách quirk kế thừa từ OpenKey).
- **Tray:** menu 15 mục (bật/tắt, kiểu gõ, bảng mã, công cụ chuyển mã, mở
  Bảng điều khiển/Gõ tắt/Giới thiệu, thoát...).

## Testing

```sh
cargo test --workspace
```

Chạy toàn bộ test: unit test engine + macOS platform layer, và quan trọng
nhất là bộ **differential test** so sánh `vnkey-engine` (Rust) với
`openkey-oracle` (FFI wrapper quanh C++ gốc của OpenKey) trên hàng trăm
nghìn chuỗi phím ngẫu nhiên (sinh bằng `proptest`) cùng một tập golden
case cố định. Mỗi chuỗi phím được gõ song song vào cả hai engine và so
từng ký tự/trạng thái kết quả -- lệch một ký tự cũng fail. Đây là cách xác
nhận engine Rust *bit-exact* với hành vi gốc mà không cần đọc lại tay toàn
bộ `Engine.cpp`.

Chạy một đợt differential lớn hơn mặc định (vd trước khi release):

```sh
PROPTEST_CASES=50000 cargo test -p vnkey-engine --release
```

Kiểm tra thêm:

```sh
cargo clippy --workspace --all-targets -- -D warnings   # lint Rust
npm run build                                            # typecheck + build UI
```

Phần tầng macOS (CGEventTap, injection, hotkey thật, smart-switch qua đổi
app thật...) không thể differential-test tự động vì phụ thuộc vào
`AppKit`/quyền hệ thống -- xem
[docs/PARITY-CHECKLIST.md](./docs/PARITY-CHECKLIST.md) để tự kiểm tay so
với OpenKey.

## License

GPL-3.0. VNKey là derivative work của OpenKey (Tuyen Mai và các
contributor) -- xem [LICENSE](./LICENSE) và [NOTICE.md](./NOTICE.md).
