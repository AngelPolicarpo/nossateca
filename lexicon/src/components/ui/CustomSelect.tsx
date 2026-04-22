import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { cn } from "../../lib/cn";

export type SelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

type CustomSelectProps = {
  id?: string;
  ariaLabel?: string;
  value: string;
  options: ReadonlyArray<SelectOption>;
  onValueChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  triggerClassName?: string;
  menuClassName?: string;
  optionClassName?: string;
};

function findFirstEnabledIndex(options: ReadonlyArray<SelectOption>): number {
  return options.findIndex((option) => !option.disabled);
}

function findLastEnabledIndex(options: ReadonlyArray<SelectOption>): number {
  for (let index = options.length - 1; index >= 0; index -= 1) {
    if (!options[index]?.disabled) {
      return index;
    }
  }

  return -1;
}

function findNextEnabledIndex(
  options: ReadonlyArray<SelectOption>,
  startIndex: number,
  direction: 1 | -1,
): number {
  if (options.length === 0) {
    return -1;
  }

  let cursor = startIndex;
  for (let step = 0; step < options.length; step += 1) {
    cursor += direction;

    if (cursor < 0) {
      cursor = options.length - 1;
    } else if (cursor >= options.length) {
      cursor = 0;
    }

    if (!options[cursor]?.disabled) {
      return cursor;
    }
  }

  return -1;
}

export function CustomSelect({
  id,
  ariaLabel,
  value,
  options,
  onValueChange,
  placeholder,
  disabled = false,
  className,
  triggerClassName,
  menuClassName,
  optionClassName,
}: CustomSelectProps) {
  const [open, setOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const listboxId = useId();

  const selectedIndex = useMemo(
    () => options.findIndex((option) => option.value === value),
    [options, value],
  );

  const selectedLabel = selectedIndex >= 0 ? options[selectedIndex]?.label ?? "" : (placeholder ?? "");

  const closeMenu = () => {
    setOpen(false);
    setHighlightedIndex(-1);
  };

  const selectIndex = (index: number) => {
    const option = options[index];
    if (!option || option.disabled) {
      return;
    }

    onValueChange(option.value);
    closeMenu();
    triggerRef.current?.focus();
  };

  const openMenu = (direction?: 1 | -1) => {
    if (disabled) {
      return;
    }

    let nextIndex = -1;

    if (direction === 1) {
      if (selectedIndex >= 0 && !options[selectedIndex]?.disabled) {
        nextIndex = selectedIndex;
      } else {
        nextIndex = findFirstEnabledIndex(options);
      }
    } else if (direction === -1) {
      if (selectedIndex >= 0 && !options[selectedIndex]?.disabled) {
        nextIndex = selectedIndex;
      } else {
        nextIndex = findLastEnabledIndex(options);
      }
    } else if (selectedIndex >= 0 && !options[selectedIndex]?.disabled) {
      nextIndex = selectedIndex;
    } else {
      nextIndex = findFirstEnabledIndex(options);
    }

    setHighlightedIndex(nextIndex);
    setOpen(true);
  };

  useEffect(() => {
    if (!open) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        closeMenu();
      }
    };

    const handleWindowBlur = () => {
      closeMenu();
    };

    document.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("blur", handleWindowBlur);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("blur", handleWindowBlur);
    };
  }, [open]);

  useEffect(() => {
    if (!open || highlightedIndex < 0) {
      return;
    }

    optionRefs.current[highlightedIndex]?.scrollIntoView({ block: "nearest" });
  }, [open, highlightedIndex]);

  const handleTriggerKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (disabled) {
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) {
        openMenu(1);
        return;
      }

      setHighlightedIndex((previous) => {
        const from = previous >= 0 ? previous : (selectedIndex >= 0 ? selectedIndex : -1);
        return findNextEnabledIndex(options, from, 1);
      });
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) {
        openMenu(-1);
        return;
      }

      setHighlightedIndex((previous) => {
        const from = previous >= 0 ? previous : (selectedIndex >= 0 ? selectedIndex : 0);
        return findNextEnabledIndex(options, from, -1);
      });
      return;
    }

    if (event.key === "Home" && open) {
      event.preventDefault();
      setHighlightedIndex(findFirstEnabledIndex(options));
      return;
    }

    if (event.key === "End" && open) {
      event.preventDefault();
      setHighlightedIndex(findLastEnabledIndex(options));
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (!open) {
        openMenu();
        return;
      }

      if (highlightedIndex >= 0) {
        selectIndex(highlightedIndex);
      }
      return;
    }

    if (event.key === "Escape" && open) {
      event.preventDefault();
      closeMenu();
      return;
    }

    if (event.key === "Tab" && open) {
      closeMenu();
    }
  };

  return (
    <div ref={rootRef} className={cn("relative min-w-0", className)}>
      <button
        ref={triggerRef}
        id={id}
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listboxId : undefined}
        className={cn(
          "inline-flex min-h-[var(--control-height)] w-full items-center justify-between gap-[var(--space-8)] rounded-[var(--radius-8)] border border-[var(--color-border-soft)] bg-[var(--color-surface-primary)] px-[var(--space-12)] text-left text-[14px] font-medium text-[var(--color-text-primary)] outline-none transition-[border-color,box-shadow,background,color] duration-150 hover:border-[var(--color-border-strong)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus)] disabled:cursor-not-allowed disabled:opacity-60",
          triggerClassName,
        )}
        disabled={disabled}
        onClick={() => {
          if (open) {
            closeMenu();
          } else {
            openMenu();
          }
        }}
        onKeyDown={handleTriggerKeyDown}
      >
        <span className="truncate">{selectedLabel}</span>

        <svg
          viewBox="0 0 20 20"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.7"
          className={cn(
            "h-[15px] w-[15px] shrink-0 text-[var(--color-text-muted)] transition-transform duration-150",
            open && "rotate-180",
          )}
          aria-hidden="true"
          focusable="false"
        >
          <path d="m5.75 7.5 4.25 5 4.25-5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      {open && (
        <ul
          id={listboxId}
          role="listbox"
          aria-labelledby={id}
          className={cn(
            "absolute left-0 top-[calc(100%+6px)] z-[var(--z-overlay-panel)] grid max-h-64 w-full gap-[2px] overflow-auto rounded-[var(--radius-10)] border border-[var(--color-border-soft)] bg-[var(--color-surface-primary)] p-[var(--space-4)] shadow-[var(--shadow-soft)]",
            menuClassName,
          )}
        >
          {options.map((option, index) => {
            const selected = option.value === value;
            const highlighted = index === highlightedIndex;

            return (
              <li key={`${option.value}-${index}`}>
                <button
                  ref={(node) => {
                    optionRefs.current[index] = node;
                  }}
                  type="button"
                  role="option"
                  aria-selected={selected}
                  className={cn(
                    "flex w-full items-center justify-between gap-[var(--space-8)] rounded-[var(--radius-8)] px-[var(--space-10)] py-[var(--space-8)] text-left text-[13px] leading-[1.3] text-[var(--color-text-secondary)] outline-none transition-[background,color] duration-100",
                    highlighted && "bg-[var(--color-control-secondary-bg)] text-[var(--color-text-primary)]",
                    selected && "font-semibold text-[var(--color-text-primary)]",
                    option.disabled && "cursor-not-allowed opacity-45",
                    optionClassName,
                  )}
                  disabled={option.disabled}
                  onMouseEnter={() => setHighlightedIndex(index)}
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => selectIndex(index)}
                >
                  <span className="truncate">{option.label}</span>
                  {selected && (
                    <svg
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      className="h-[12px] w-[12px] shrink-0"
                      aria-hidden="true"
                      focusable="false"
                    >
                      <path d="m3.25 8.25 2.75 2.75 6-6" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
