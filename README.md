# VNKey

<!-- TODO: replace https://github.com/yourname/VNKey with the real repo URL everywhere it appears (this file, docs/, src/lib/strings.ts) once the project has a home. -->

VNKey là bộ gõ tiếng Việt cho macOS, viết bằng Rust +
[Tauri](https://tauri.app): engine thuần Rust (không FFI, không C++), tầng
macOS dùng `CGEventTap` viết bằng Rust/objc2, giao diện Tauri +
React/TypeScript.

> Repo gốc dự án (placeholder, sẽ thay khi có repo thật):
> **https://github.com/yourname/VNKey** <!-- TODO -->

## English summary

VNKey is a Vietnamese input method for macOS built with Rust + Tauri. The
typing engine is pure Rust (no FFI, no C++) covered by a golden test suite.
The macOS integration (key interception/injection, hotkeys, smart per-app
switching, sleep/wake handling) is written directly against
AppKit/CoreGraphics via `objc2`, and the UI (Control Panel, macro editor,
convert tool, About) is a Tauri v2 + React/TypeScript app.
See the sections below (in Vietnamese) for build instructions, the
Accessibility permission grant, and the test suite.

## Kiến trúc

```
crates/vnkey-engine/   engine thuần Rust: bảng mã, xử lý phím, chính tả,
                       gõ tắt (macro), smart-switch, chuyển mã.
                       Không phụ thuộc Tauri/macOS -- test được độc lập.
src-tauri/src/         tầng macOS + Tauri:
  platform/mac/          CGEventTap bắt/inject phím, hotkey, TIS
                          (other-language), quan sát vòng đời app
                          (sleep/wake, đổi app/Space), smart-switch cache.
  settings/               settings.json + macros.bin + smart_switch.bin.
  tray.rs, commands.rs,   tray 15 mục, IPC, pasteboard, events.
  pasteboard.rs
src/                   UI React/TypeScript (Bảng điều khiển, Gõ tắt,
                       Công cụ chuyển mã, Giới thiệu), route theo cửa sổ
                       (window label) trong main.tsx.
```

Engine (`vnkey-engine`) không biết gì về Tauri hay macOS; toàn bộ phần nền
tảng nằm trong `src-tauri`. Đây là ranh giới quan trọng nhất của kiến trúc:
engine test được độc lập (golden + unit test), tầng macOS chỉ test được bằng
unit test (mock) + kiểm tay.

Chi tiết bên trong (luồng một phím, mô hình luồng, giao thức engine/host):
xem [docs/architecture.md](./docs/architecture.md).

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
khác (giống mọi bộ gõ tiếng Việt trên macOS: Unikey, EVKey...).
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
- **Giao diện:** sáng/tối (theo hệ thống hoặc tự chọn trong tab Hệ thống).
- **Sửa lỗi riêng cho từng ứng dụng:** Chrome/Chromium, Terminal,
  Spotlight, Excel/Sublime (theo danh sách quirk per-app).
- **Tray:** menu 15 mục (bật/tắt, kiểu gõ, bảng mã, công cụ chuyển mã, mở
  Bảng điều khiển/Gõ tắt/Giới thiệu, thoát...).

## Testing

```sh
cargo test --workspace
```

Chạy toàn bộ test: golden test cho engine (các chuỗi phím mẫu ra đúng kết
quả kỳ vọng cho cả 4 kiểu gõ + 5 bảng mã + macro/smart-switch/chuyển mã) và
unit test cho tầng macOS.

Kiểm tra thêm:

```sh
cargo clippy --workspace --all-targets -- -D warnings   # lint Rust
npm run build                                            # typecheck + build UI
```

Phần tầng macOS (CGEventTap, injection, hotkey thật, smart-switch qua đổi
app thật...) không thể test tự động vì phụ thuộc vào `AppKit`/quyền hệ
thống -- xem [docs/PARITY-CHECKLIST.md](./docs/PARITY-CHECKLIST.md) để tự
kiểm tay.

Thuật toán gõ tiếng Việt (bảng mã, quy tắc đặt dấu, kiểm tra chính tả) tham
khảo từ [OpenKey](https://github.com/tuyenvm/OpenKey).
