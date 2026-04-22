import type { HTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type BadgeTone = "default" | "muted" | "accent";

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: BadgeTone;
};

const toneClasses: Record<BadgeTone, string> = {
  default: "bg-[var(--color-surface-alt)] text-[var(--color-text-secondary)]",
  muted: "bg-[var(--color-surface-primary)] text-[var(--color-text-muted)]",
  accent: "bg-[var(--color-badge-bg)] text-[var(--color-badge-text)]",
};

export function Badge({ className, tone = "default", ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-[var(--radius-pill)] border border-black/10 px-[var(--space-8)] py-[var(--space-4)] text-[12px] font-semibold leading-[1.33] tracking-[0.125px]",
        toneClasses[tone],
        className,
      )}
      {...props}
    />
  );
}
