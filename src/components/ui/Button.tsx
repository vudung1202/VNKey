import type { ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

/**
 * The one button in the app. Variant sets hierarchy: `primary` for the
 * confirming action, `secondary` for neutral, `ghost` for dismiss, `danger`
 * for destructive (Xoá / Kết thúc). All states (hover / active / focus /
 * disabled) live in `.ui-btn` in styles.css.
 */
export function Button({
  variant = "secondary",
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  return <button type={type} className={`ui-btn ui-btn--${variant}${className ? ` ${className}` : ""}`} {...props} />;
}
