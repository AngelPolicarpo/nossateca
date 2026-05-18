import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function IconMinimize() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden="true">
      <line x1="2.5" y1="6" x2="9.5" y2="6" strokeLinecap="round" />
    </svg>
  );
}

function IconMaximize() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden="true">
      <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
    </svg>
  );
}

function IconRestore() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden="true">
      <rect x="3.5" y="3.5" width="6" height="6" rx="1" />
      <path d="M5 3.5V2.5h4.5V7H8.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function IconClose() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" aria-hidden="true">
      <path d="m3 3 6 6M9 3l-6 6" strokeLinecap="round" />
    </svg>
  );
}

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri) return;
    const win = getCurrentWindow();
    let cancelled = false;

    win.isMaximized().then((value) => {
      if (!cancelled) setIsMaximized(value);
    });

    const unlistenPromise = win.onResized(() => {
      win.isMaximized().then((value) => {
        if (!cancelled) setIsMaximized(value);
      });
    });

    return () => {
      cancelled = true;
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const win = isTauri ? getCurrentWindow() : null;

  return (
    <header className="lx-titlebar" data-tauri-drag-region>
      <span className="lx-titlebar-title" data-tauri-drag-region>
        Nossateca
      </span>
      <div className="lx-titlebar-controls">
        <button
          type="button"
          aria-label="Minimizar"
          onClick={() => win?.minimize()}
        >
          <IconMinimize />
        </button>
        <button
          type="button"
          aria-label={isMaximized ? "Restaurar" : "Maximizar"}
          onClick={() => win?.toggleMaximize()}
        >
          {isMaximized ? <IconRestore /> : <IconMaximize />}
        </button>
        <button
          type="button"
          className="close"
          aria-label="Fechar"
          onClick={() => win?.close()}
        >
          <IconClose />
        </button>
      </div>
    </header>
  );
}

export default TitleBar;
