import type { ReactNode } from "react";

export type SegmentedItem<T> = { value: T; label: ReactNode; accent?: boolean };

/**
 * Segmented control: a pill track with the active item raised. Used for the
 * VN/EN language switch (accent-tinted active) and the Control Panel tab bar.
 */
export function Segmented<T extends string | number>({
  value,
  onChange,
  items,
  full,
  ariaLabel,
}: {
  value: T;
  onChange: (value: T) => void;
  items: Array<SegmentedItem<T>>;
  full?: boolean;
  ariaLabel?: string;
}) {
  return (
    <div className={`ui-segmented${full ? " ui-segmented--full" : ""}`} role="tablist" aria-label={ariaLabel}>
      {items.map((item) => {
        const active = item.value === value;
        return (
          <button
            key={String(item.value)}
            type="button"
            role="tab"
            aria-selected={active}
            data-accent={item.accent ? "true" : undefined}
            className={`ui-segmented__item${active ? " is-active" : ""}`}
            onClick={() => onChange(item.value)}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
