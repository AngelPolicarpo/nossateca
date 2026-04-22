import type { ElementType, ReactNode } from "react";
import { cn } from "../../lib/cn";

type EmptyStateProps = {
  title: string;
  description: string;
  titleAs?: ElementType;
  compact?: boolean;
  className?: string;
  action?: ReactNode;
};

export function EmptyState({
  title,
  description,
  titleAs,
  compact = false,
  className,
  action,
}: EmptyStateProps) {
  const TitleTag = titleAs ?? "h2";

  return (
    <section
      className={cn(
        "rounded-[var(--radius-16)] px-[var(--space-24)] text-center",
        compact ? "py-[var(--space-24)]" : "py-[var(--space-48)]",
        className,
      )}
    >
      <TitleTag className="m-0 text-[var(--type-sub-size)] font-[var(--type-sub-weight)] leading-[var(--type-sub-line)] tracking-[var(--type-sub-track)] text-[var(--color-text-primary)]">
        {title}
      </TitleTag>
      <p className="mx-auto mb-[var(--space-16)] mt-[var(--space-12)] max-w-[56ch] text-[14px] leading-[1.43] text-[var(--color-text-secondary)]">
        {description}
      </p>
      {action}
    </section>
  );
}