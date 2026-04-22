import type { InputHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type InputProps = InputHTMLAttributes<HTMLInputElement>;

export function Input({ className, type = "text", ...props }: InputProps) {
  return (
    <input
      type={type}
      className={cn(
        "w-full rounded-[var(--radius-4)] border border-[var(--color-input-border)] bg-[var(--color-surface-primary)] px-[6px] py-[6px] text-[14px] text-[var(--color-input-text)] transition-[border-color,box-shadow,background] duration-150",
        "placeholder:text-[var(--color-text-muted)] hover:border-[var(--color-border-strong)] focus-visible:border-[rgba(9,127,232,0.45)] focus-visible:bg-[var(--color-surface-primary)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)]",
        "disabled:cursor-not-allowed disabled:opacity-60",
        className,
      )}
      {...props}
    />
  );
}
