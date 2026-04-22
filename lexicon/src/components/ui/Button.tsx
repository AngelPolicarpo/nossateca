import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "md" | "sm";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  fullWidth?: boolean;
};

const variantClasses: Record<ButtonVariant, string> = {
  primary:
    "border border-transparent bg-[var(--color-primary)] text-white hover:bg-[var(--color-primary-active)] active:scale-[0.9]",
  secondary:
    "border border-[var(--color-border-soft)] bg-[var(--color-control-secondary-bg)] text-[var(--color-text-primary)] hover:border-[var(--color-border-strong)] hover:text-[var(--color-text-primary)] active:scale-[0.9]",
  ghost:
    "border border-transparent bg-transparent text-[var(--color-text-primary)] hover:underline",
  danger:
    "border border-[var(--color-danger-soft)] bg-[var(--color-danger-bg)] text-[var(--color-danger)] hover:bg-[var(--color-danger-bg-active)] active:scale-[0.9]",
};

const sizeClasses: Record<ButtonSize, string> = {
  md: "min-h-[var(--control-height)] px-4 py-2 text-[15px] leading-[1.33]",
  sm: "min-h-[var(--control-height-compact)] px-3 py-1 text-[14px] leading-[1.33]",
};

export function Button({
  className,
  variant = "secondary",
  size = "md",
  fullWidth = false,
  type = "button",
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center rounded-full font-semibold transition-[transform,border-color,background,color,box-shadow] duration-150 ease-out",
        "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)] focus-visible:shadow-[var(--shadow-card)]",
        "disabled:pointer-events-none disabled:opacity-45",
        variantClasses[variant],
        sizeClasses[size],
        fullWidth && "w-full",
        className,
      )}
      {...props}
    />
  );
}
