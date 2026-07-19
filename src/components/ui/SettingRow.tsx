import { Toggle } from "./Toggle";

/**
 * One settings line: label (+ optional hint) on the left, a `Toggle` on the
 * right, the whole row a `<label>` so clicking anywhere flips the switch.
 * Drop several inside a `.ui-card` to get hairline dividers between them.
 * Prop shape is the plain {label, checked, onChange, disabled} a settings
 * checkbox needs, so tabs map straight onto it.
 */
export function SettingRow({
  label,
  checked,
  onChange,
  disabled,
  hint,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  hint?: string;
}) {
  return (
    <label className="setting-row" data-disabled={disabled ? "true" : "false"}>
      <span className="setting-row__text">
        <span className="setting-row__label">{label}</span>
        {hint && <span className="setting-row__hint">{hint}</span>}
      </span>
      <Toggle checked={checked} onChange={onChange} disabled={disabled} />
    </label>
  );
}
