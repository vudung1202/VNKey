#!/bin/bash
# Cài hoặc cập nhật VNKey bản mới nhất vào /Applications:
#   curl -fsSL https://github.com/vudung1202/VNKey/releases/latest/download/install.sh | bash
# Tải bằng curl nên app không bị gắn cờ "tải từ Internet", macOS không chặn bản chưa notarize.
# Mỗi lần phát hành phải tải kèm file này lên release, nếu không link trên trả về 404:
#   gh release upload v<phiên bản> install.sh
set -euo pipefail

REPO="vudung1202/VNKey"
APP="/Applications/VNKey.app"

say() { printf '%s\n' "$*"; }
fail() { printf 'Lỗi: %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == Darwin ]] || fail "VNKey chỉ chạy trên macOS."
# uname -m báo x86_64 khi Terminal chạy qua Rosetta, nên hỏi phần cứng.
[[ "$(sysctl -n hw.optional.arm64 2>/dev/null)" == 1 ]] || fail "bản cài đặt chỉ chạy trên Mac Apple Silicon."
[[ -w /Applications ]] || fail "không có quyền ghi vào /Applications, hãy dùng tài khoản admin."

# /releases/latest chuyển hướng sang /releases/tag/v<phiên bản>. Không gọi API nên không bị giới hạn lượt.
latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") \
  || fail "không kết nối được GitHub."
tag=${latest##*/}
[[ "$tag" == v* ]] || fail "không tìm thấy bản phát hành nào."
version=${tag#v}
url="https://github.com/$REPO/releases/download/$tag/VNKey_${version}_aarch64.dmg"

tmp=$(mktemp -d)
mnt="$tmp/mnt"
cleanup() {
  local rc=$?
  hdiutil detach "$mnt" -quiet 2>/dev/null || true
  rm -rf "$tmp"
  exit "$rc"
}
trap cleanup EXIT

say "Tải VNKey ${version}…"
curl -fL --progress-bar -o "$tmp/VNKey.dmg" "$url" || fail "không tải được $url"
mkdir "$mnt"
hdiutil attach -nobrowse -readonly -mountpoint "$mnt" "$tmp/VNKey.dmg" >/dev/null \
  || fail "không mở được file DMG."
codesign --verify --deep --strict "$mnt/VNKey.app" 2>/dev/null || fail "chữ ký của VNKey.app không hợp lệ."
# Chép ra trước rồi mới xoá bản cũ, để lỗi giữa chừng không làm mất app đang có.
ditto "$mnt/VNKey.app" "$tmp/VNKey.app"

pkill -f "$APP/Contents/MacOS/vnkey" 2>/dev/null && sleep 1 || true
rm -rf "$APP"
mv "$tmp/VNKey.app" "$APP"
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true
# Quyền Trợ năng gắn với chữ ký bản cũ: công tắc vẫn bật nhưng không còn tác dụng, phải xoá để cấp lại.
tccutil reset Accessibility com.vnkey.app >/dev/null 2>&1 || true
open "$APP"

say "Đã cài VNKey $version vào $APP. Còn 2 bước:"
say "  1. System Settings → Privacy & Security → Accessibility → bật VNKey"
say "  2. Trong cửa sổ VNKey, bấm \"Khởi động lại\""
