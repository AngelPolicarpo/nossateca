import type { HTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type StateMessageTone = "default" | "error";

type StateMessageProps = HTMLAttributes<HTMLParagraphElement> & {
  tone?: StateMessageTone;
};

const toneClasses: Record<StateMessageTone, string> = {
  default:
    "border-black/10 bg-[var(--color-surface-alt)] text-[var(--color-text-secondary)]",
  error:
    "border-[var(--color-danger-soft)] bg-[var(--color-danger-bg)] text-[var(--color-danger)]",
};

export function StateMessage({ className, tone = "default", ...props }: StateMessageProps) {
  return (
    <p
      className={cn(
        "m-0 rounded-[var(--radius-8)] border px-[var(--space-12)] py-[var(--space-11)] text-[14px] leading-[1.43]",
        toneClasses[tone],
        className,
      )}
      {...props}
    />
  );
}