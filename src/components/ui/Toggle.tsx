/**
 * Pill switch that replaces raw settings checkboxes. Renders a bare
 * `<input role="switch">` + track so a surrounding `<label>` (see
 * `SettingRow`) owns the click target and text association; used standalone,
 * pass an `aria-label`.
 */
export function Toggle({
  checked,
  onChange,
  disabled,
  ariaLabel,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  ariaLabel?: string;
}) {
  return (
    <span className="ui-toggle" data-disabled={disabled ? "true" : "false"}>
      <input
        className="ui-toggle__input"
        type="checkbox"
        role="switch"
        aria-label={ariaLabel}
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="ui-toggle__track" aria-hidden="true">
        <span className="ui-toggle__knob" />
      </span>
    </span>
  );
}
