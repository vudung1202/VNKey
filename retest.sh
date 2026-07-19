#!/bin/bash
# retest.sh — build lại VNKey (ad-hoc) rồi mở để test, xử lý luôn việc TCC quên quyền.
#
# Ad-hoc sign: mỗi lần build đổi CDHash → macOS quên quyền Accessibility đã cấp.
# Script này: build → thoát app cũ → reset entry TCC stale → mở app mới.
# Sau khi chạy, bạn chỉ cần bật lại toggle VNKey trong
# System Settings → Privacy & Security → Accessibility, rồi bấm "Khởi động lại VNKey".
#
# Dùng: ./retest.sh            (build release, khuyến nghị)
#       ./retest.sh --debug    (build debug, nhanh hơn khi lặp nhiều)

set -e
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")"

MODE="release"
APP="target/release/bundle/macos/VNKey.app"
if [ "$1" = "--debug" ]; then
  MODE="debug"
  APP="target/debug/bundle/macos/VNKey.app"
fi

echo "▸ Build VNKey ($MODE)…"
if [ "$MODE" = "debug" ]; then
  npx tauri build --debug
else
  npx tauri build
fi

echo "▸ Thoát VNKey đang chạy (nếu có)…"
pkill -f "VNKey.app/Contents/MacOS/vnkey" 2>/dev/null && sleep 1 || true

echo "▸ Reset quyền Accessibility stale (grant cũ khác CDHash)…"
tccutil reset Accessibility com.vnkey.app || true

echo "▸ Mở bản mới…"
open "$APP"

cat <<'EOF'

────────────────────────────────────────────────────────────
  Còn 2 bước TAY (macOS bắt buộc, không script được):
  1. System Settings → Privacy & Security → Accessibility → bật VNKey
  2. Trong cửa sổ VNKey → bấm "Khởi động lại VNKey"
  Sau đó gõ:  vieejt<space>  → việt
────────────────────────────────────────────────────────────
EOF
