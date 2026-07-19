import { HotkeyRecorder } from "./HotkeyRecorder";

type ModKey = "ctrl" | "opt" | "cmd" | "shift";

const MODIFIERS: Array<{ key: ModKey; glyph: string; name: string }> = [
  { key: "ctrl", glyph: "⌃", name: "Control" },
  { key: "opt", glyph: "⌥", name: "Option" },
  { key: "cmd", glyph: "⌘", name: "Command" },
  { key: "shift", glyph: "⇧", name: "Shift" },
];

/**
 * The modifier chip row shared by the switch-key (TopBar) and quick-convert
 * (ConvertTool) hotkeys: four toggle chips (glyph + aria-pressed) followed by
 * the keycap-styled recorder for the non-modifier key.
 */
export function HotkeyChips({
  mods,
  char,
  onToggle,
  onRecord,
}: {
  mods: Record<ModKey, boolean>;
  char: number;
  onToggle: (key: ModKey) => void;
  onRecord: (keycode: number, char: number) => void;
}) {
  return (
    <div className="cp-chips">
      {MODIFIERS.map((m) => (
        <button
          key={m.key}
          type="button"
          className={`ui-chip${mods[m.key] ? " is-on" : ""}`}
          aria-pressed={mods[m.key]}
          aria-label={m.name}
          onClick={() => onToggle(m.key)}
        >
          {m.glyph}
        </button>
      ))}
      <HotkeyRecorder char={char} onChange={onRecord} />
    </div>
  );
}
