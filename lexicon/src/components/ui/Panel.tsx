import type { ComponentPropsWithoutRef, ElementType } from "react";
import { cn } from "../../lib/cn";

type PanelProps<T extends ElementType> = {
  as?: T;
  className?: string;
} & Omit<ComponentPropsWithoutRef<T>, "as" | "className">;

export function Panel<T extends ElementType = "section">({
  as,
  className,
  ...props
}: PanelProps<T>) {
  const Component = as ?? "section";

  return (
    <Component
      className={cn(
        "rounded-[var(--radius-12)]  p-[var(--space-24)]",
        "shadow-[var(--shadow-card)]",
        className,
      )}
      {...props}
    />
  );
}
