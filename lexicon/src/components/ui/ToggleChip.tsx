import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type ToggleChipProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "type"> & {
  active?: boolean;
};

export function ToggleChip({
  active = false,
  className,
  "aria-pressed": ariaPressed,
  ...props
}: ToggleChipProps) {
  return (
    <button
      type="button"
      aria-pressed={ariaPressed ?? active}
      className={cn(
        "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)]",
        "disabled:pointer-events-none disabled:opacity-45",
        active && "active",
        className,
      )}
      {...props}
    />
  );
}
