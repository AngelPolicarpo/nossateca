export type ReaderShortcutAction =
  | "nextPosition"
  | "previousPosition"
  | "openSearch"
  | "toggleReaderTheme"
  | "toggleFullscreen"
  | "createBookmark";

export type ReaderShortcutConfig = Record<ReaderShortcutAction, string>;

export type ReaderShortcutDefinition = {
  action: ReaderShortcutAction;
  label: string;
  description: string;
  placeholder: string;
};

export type ParsedShortcutBinding = {
  key: string;
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
  shift: boolean;
  mod: boolean;
};

export const READER_SHORTCUT_STORAGE_KEY = "reader.shortcuts";

export const DEFAULT_READER_SHORTCUTS: ReaderShortcutConfig = {
  nextPosition: "ArrowRight, PageDown",
  previousPosition: "ArrowLeft, PageUp",
  openSearch: "Ctrl+F, Cmd+F",
  toggleReaderTheme: "T",
  toggleFullscreen: "F",
  createBookmark: "B",
};

export const READER_SHORTCUT_DEFINITIONS: ReaderShortcutDefinition[] = [
  {
    action: "nextPosition",
    label: "Próxima página/capítulo",
    description: "Avança para o próximo capítulo no EPUB ou próxima página no PDF.",
    placeholder: "ArrowRight, PageDown",
  },
  {
    action: "previousPosition",
    label: "Página/capítulo anterior",
    description: "Volta para o capítulo anterior no EPUB ou página anterior no PDF.",
    placeholder: "ArrowLeft, PageUp",
  },
  {
    action: "openSearch",
    label: "Buscar no livro",
    description: "Abre e foca a busca textual no Reader EPUB.",
    placeholder: "Ctrl+F, Cmd+F",
  },
  {
    action: "toggleReaderTheme",
    label: "Alternar tema do Reader",
    description: "Alterna entre tema claro e escuro no modo de leitura.",
    placeholder: "T",
  },
  {
    action: "toggleFullscreen",
    label: "Alternar tela cheia",
    description: "Ativa ou desativa a leitura em tela cheia.",
    placeholder: "F",
  },
  {
    action: "createBookmark",
    label: "Salvar marcador",
    description: "Cria um marcador na posição atual (EPUB).",
    placeholder: "B",
  },
];

const MODIFIER_ALIASES: Record<string, keyof Omit<ParsedShortcutBinding, "key">> = {
  ctrl: "ctrl",
  control: "ctrl",
  cmd: "meta",
  command: "meta",
  meta: "meta",
  alt: "alt",
  option: "alt",
  shift: "shift",
  mod: "mod",
};

const KEY_ALIASES: Record<string, string> = {
  right: "arrowright",
  left: "arrowleft",
  up: "arrowup",
  down: "arrowdown",
  pgup: "pageup",
  pgdown: "pagedown",
  esc: "escape",
  return: "enter",
  " ": "space",
  spacebar: "space",
};

function normalizeKeyName(rawKey: string): string {
  if (rawKey === " ") {
    return "space";
  }

  const key = rawKey.trim().toLowerCase();
  if (key.length === 0) {
    return "";
  }

  return KEY_ALIASES[key] ?? key;
}

function parseShortcutBinding(value: string): ParsedShortcutBinding | null {
  const normalizedValue = value
    .trim()
    .replace(/ctrl\s*\/\s*cmd/gi, "mod")
    .replace(/cmd\s*\/\s*ctrl/gi, "mod");

  if (normalizedValue.length === 0) {
    return null;
  }

  const tokens = normalizedValue
    .split("+")
    .map((token) => token.trim())
    .filter((token) => token.length > 0);

  if (tokens.length === 0) {
    return null;
  }

  const key = normalizeKeyName(tokens[tokens.length - 1]);
  if (key.length === 0) {
    return null;
  }

  const shortcut: ParsedShortcutBinding = {
    key,
    ctrl: false,
    meta: false,
    alt: false,
    shift: false,
    mod: false,
  };

  for (const modifierToken of tokens.slice(0, -1)) {
    const normalizedModifier = modifierToken.toLowerCase();
    const mappedModifier = MODIFIER_ALIASES[normalizedModifier];

    if (!mappedModifier) {
      return null;
    }

    shortcut[mappedModifier] = true;
  }

  return shortcut;
}

export function parseShortcutBindings(value: string): ParsedShortcutBinding[] {
  const parts = value
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  if (parts.length === 0) {
    return [];
  }

  const parsedBindings: ParsedShortcutBinding[] = [];
  for (const part of parts) {
    const parsed = parseShortcutBinding(part);
    if (!parsed) {
      return [];
    }

    parsedBindings.push(parsed);
  }

  return parsedBindings;
}

function normalizeBindingPart(part: string): string {
  return part
    .trim()
    .replace(/\s*\+\s*/g, "+")
    .replace(/ctrl\s*\/\s*cmd/gi, "Ctrl/Cmd")
    .replace(/cmd\s*\/\s*ctrl/gi, "Ctrl/Cmd");
}

export function normalizeShortcutValue(value: string): string {
  return value
    .split(",")
    .map(normalizeBindingPart)
    .filter((part) => part.length > 0)
    .join(", ");
}

export function isShortcutEventMatch(
  event: KeyboardEvent,
  bindings: ParsedShortcutBinding[],
): boolean {
  const eventKey = normalizeKeyName(event.key);

  return bindings.some((binding) => {
    if (binding.key !== eventKey) {
      return false;
    }

    if (binding.mod) {
      if (!event.ctrlKey && !event.metaKey) {
        return false;
      }
    } else if (binding.ctrl !== event.ctrlKey || binding.meta !== event.metaKey) {
      return false;
    }

    return binding.alt === event.altKey && binding.shift === event.shiftKey;
  });
}

export function sanitizeReaderShortcutConfig(candidate: unknown): ReaderShortcutConfig {
  const sanitized: ReaderShortcutConfig = { ...DEFAULT_READER_SHORTCUTS };

  if (typeof candidate !== "object" || candidate === null) {
    return sanitized;
  }

  const rawRecord = candidate as Record<string, unknown>;
  for (const definition of READER_SHORTCUT_DEFINITIONS) {
    const rawValue = rawRecord[definition.action];
    if (typeof rawValue !== "string") {
      continue;
    }

    const normalizedValue = normalizeShortcutValue(rawValue);
    if (parseShortcutBindings(normalizedValue).length === 0) {
      continue;
    }

    sanitized[definition.action] = normalizedValue;
  }

  return sanitized;
}
