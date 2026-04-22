import { cn } from "../../lib/cn";

type BookCoverProps = {
  title: string;
  author?: string | null;
  format?: string;
  className?: string;
  size?: "sm" | "md" | "lg";
};

const PALETTES: Array<{ bg: string; text: string; accent: string }> = [
  { bg: "linear-gradient(150deg, #1a2235 0%, #0d1320 100%)", text: "#c8d4e8", accent: "#5b8dd9" },
  { bg: "linear-gradient(150deg, #1d2b1d 0%, #0f1f0f 100%)", text: "#b8cdb8", accent: "#6bbf6b" },
  { bg: "linear-gradient(150deg, #2b1e14 0%, #180f08 100%)", text: "#d4c0a8", accent: "#d48a4a" },
  { bg: "linear-gradient(150deg, #1e1b2e 0%, #110f1c 100%)", text: "#c4b8d8", accent: "#9b78d4" },
  { bg: "linear-gradient(150deg, #2a1a1a 0%, #180d0d 100%)", text: "#d4b8b8", accent: "#d45a5a" },
  { bg: "linear-gradient(150deg, #0f2424 0%, #081616 100%)", text: "#a8cccc", accent: "#44b8b8" },
  { bg: "linear-gradient(150deg, #24200f 0%, #161308 100%)", text: "#d0c898", accent: "#c8b040" },
  { bg: "linear-gradient(150deg, #1e2828 0%, #111818 100%)", text: "#b0c8c0", accent: "#50a898" },
];

function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash * 31 + str.charCodeAt(i)) & 0xffffffff;
  }
  return Math.abs(hash);
}

export function BookCover({ title, author, format, className, size = "md" }: BookCoverProps) {
  const palette = PALETTES[hashString(title) % PALETTES.length];

  const sizeClasses: Record<string, string> = {
    sm: "h-[124px] w-[84px] min-w-[84px] text-[10px]",
    md: "h-[clamp(200px,22vw,280px)] w-full",
    lg: "h-[clamp(260px,28vw,340px)] w-full",
  };

  return (
    <div
      className={cn("relative overflow-hidden", sizeClasses[size], className)}
      style={{ background: palette.bg }}
      aria-hidden="true"
    >
      {/* Grain overlay */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.06,
          backgroundImage:
            "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='200' height='200'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='200' height='200' filter='url(%23n)'/%3E%3C/svg%3E\")",
          backgroundSize: "200px 200px",
          pointerEvents: "none",
        }}
      />

      {/* Format badge */}
      {format && (
        <div
          style={{
            position: "absolute",
            top: size === "sm" ? 6 : 10,
            right: size === "sm" ? 6 : 10,
            fontSize: size === "sm" ? 8 : 9,
            fontWeight: 700,
            letterSpacing: "0.08em",
            color: palette.accent,
            background: "rgba(0,0,0,0.35)",
            borderRadius: 3,
            padding: size === "sm" ? "1px 4px" : "2px 6px",
            textTransform: "uppercase",
            fontFamily: "var(--sx-font-mono)",
          }}
        >
          {format}
        </div>
      )}

      {/* Title + author */}
      <div
        style={{
          position: "absolute",
          bottom: 0,
          left: 0,
          right: 0,
          padding: size === "sm" ? "8px 8px 8px" : "12px 14px 14px",
          background: "linear-gradient(to top, rgba(0,0,0,0.65) 0%, transparent 100%)",
        }}
      >
        <div
          style={{
            fontSize: size === "sm" ? 9 : 11,
            fontWeight: 700,
            color: palette.text,
            lineHeight: 1.3,
            display: "-webkit-box",
            WebkitBoxOrient: "vertical",
            WebkitLineClamp: size === "sm" ? 2 : 3,
            overflow: "hidden",
            fontFamily: "var(--sx-font-reading)",
            letterSpacing: "-0.01em",
          }}
        >
          {title}
        </div>
        {author && size !== "sm" && (
          <div
            style={{
              fontSize: 9,
              color: palette.text,
              opacity: 0.7,
              marginTop: 3,
              fontFamily: "var(--sx-font-ui)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {author}
          </div>
        )}
      </div>
    </div>
  );
}
