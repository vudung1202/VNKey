import type { SelectHTMLAttributes } from "react";

/**
 * Native `<select>` with the platform chevron removed and a custom one drawn
 * by `.ui-select::after`, so it matches the rest of the controls in both
 * themes. Keeps the native popup + keyboard behavior.
 */
export function Select({ className, children, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <span className="ui-select">
      <select className={`ui-select__el${className ? ` ${className}` : ""}`} {...props}>
        {children}
      </select>
    </span>
  );
}
