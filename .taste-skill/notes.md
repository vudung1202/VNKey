# VNKey app screens - notes

Style: achromatic-ledger (seed 1420560470), chosen over editorial-greeting: VNKey is a one-person
utility opened for a quick change, no greeting home; quiet ink chrome suits a settings tool.

## Inventory
- Objects: Settings (language V/E, input type x4, code table x5, switch chord + beep, 13 typing
  bools, 6 macro bools, 8 system bools + theme, convert options), macro table (key -> text,
  0 to a few hundred rows, UniKey import), Accessibility permission (granted / missing).
- Person: one Mac user, opens it rarely (first run, change method, edit shortcuts), quick look.
- Jobs: confirm/switch Vietnamese + method + table; grant permission (first run); add/edit
  shortcuts; flip a spelling rule; convert the clipboard.
- Own things: name + green VN keycap icon, the V/E letter in the menu bar, the keystroke
  transformation itself (vieejt -> việt), Vietnamese words.

## Questions (one per view)
- Bộ gõ: "How does VNKey turn my keys into Vietnamese right now?" Biggest block: the method panel
  led by the keystroke preview for the current input type. Finding line: mode + method + table, or
  how to turn Vietnamese back on with the real chord.
- Gõ tắt: "Which short keys expand into what?" Biggest block: the table. Primary: Thêm.
- Chuyển mã: "What will the clipboard become?" Preview of the case options on a real sentence.
  Primary: Chuyển mã clipboard.
- Hệ thống: "Is VNKey allowed to type, and how does it run on this Mac?" Permission row first,
  destructive reset last.

## Face
The brand green from the icon (#168060 light, 4.9:1; #28b888 dark) only on the live state: the dot
of "Tiếng Việt" in the sidebar switch and the output word of each preview. Product mark = the real
icon at the top of the sidebar. System font kept (user decision); mono only for keys and keycaps.

## Signature element
Keystroke preview: the keys as keycaps -> the word VNKey produces, examples verified on the engine
(golden probe 2026-11-20): Telex vieejt->việt, nguwowif->người, w->ư; VNI vie65t->việt,
d9a6y->đây; Simple Telex 1 w->w, uw->ư; Simple Telex 2 w->ư, ]->].

## Shell
Open plane, transparent titlebar. Sidebar 220 on the ground (mark, V/E switch, 5 nav rows on a
34px pitch, permission notice + Thoát at the foot). Canvas 540 at 760 wide: 24 + 492 + 24.
Panels white, 1px #e6e6e6, radius 8, 44px rows split by #f0f0f0 rules, headings 8px above.

## Critique (app screens §2.7), last round, light + dark at 760x560 and 760x1300
1. Question answered first: yes on every view (preview panel, table, case preview, permission row).
2. Edges: one left edge per view, panels end on the canvas edge; fixed in rounds 1-2: case
   segmented overflowing (shorter labels, full width), macro table blank band (shrinks to rows,
   6.5-row cap so a half row shows there is more).
3. Colour: brand green only on the Vietnamese dot and preview outputs; one near-black primary per
   view (Thêm, Chuyển mã clipboard, Khởi động lại); reset is outlined negative, last.
4. Numbers: "8 từ gõ tắt" = rows; previews measured on the engine; case preview unit-tested.
5. Empty: macro table keeps its frame with one sentence; permission missing is a notice + steps.
6. No legends or charts on these screens.
7. Phone: not applicable (desktop window, min 680x480).
Could not run: lint.mjs / inspect.mjs (GPUI renders natively, no HTML); contrast checked by hand
(text 5.0:1+ on ground, brand 4.9:1 light / 7.5:1 dark).
