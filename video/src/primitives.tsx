import React from "react";
import {
  AbsoluteFill,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  spring,
} from "remotion";

// ─── Color Palettes ─────────────────────────────────────────────────

export const dark = {
  bg: "#0a0a0a",
  gold: "#C4A035",
  dimGold: "#8B7355",
  faintGold: "#5a4d3a",
  white: "#e8e4de",
  dimWhite: "#9a958e",
  faintWhite: "#5a5650",
  red: "#a85454",
  green: "#5a9a5a",
  blue: "#5a7a9a",
  cyan: "#5a9a8a",
  panel: "#111110",
  panelBorder: "#5a4d3a",
};

export const warm = {
  bg: "#0d0b08",
  gold: "#d4a830",
  dimGold: "#9a7a4a",
  faintGold: "#6a5a3a",
  white: "#f0ece0",
  dimWhite: "#a89a88",
  faintWhite: "#6a6058",
  red: "#c06050",
  green: "#6aaa5a",
  blue: "#5a8ab0",
  cyan: "#60a898",
  panel: "#161210",
  panelBorder: "#6a5a3a",
};

export const cool = {
  bg: "#080a0d",
  gold: "#8a9ab0",
  dimGold: "#6a7a8a",
  faintGold: "#4a5a6a",
  white: "#dce0e8",
  dimWhite: "#8a9098",
  faintWhite: "#5a6068",
  red: "#b06068",
  green: "#60a080",
  blue: "#6090c0",
  cyan: "#60a0b0",
  panel: "#0e1014",
  panelBorder: "#4a5a6a",
};

export const mono = {
  bg: "#080808",
  gold: "#888888",
  dimGold: "#666666",
  faintGold: "#444444",
  white: "#d0d0d0",
  dimWhite: "#909090",
  faintWhite: "#606060",
  red: "#aa6666",
  green: "#66aa66",
  blue: "#6688aa",
  cyan: "#66aaaa",
  panel: "#111111",
  panelBorder: "#444444",
};

export type Palette = typeof dark;

// ─── Typography ─────────────────────────────────────────────────────

export const fontMono = `'Berkeley Mono', 'Menlo', monospace`;
export const fontSerif = fontMono; // Adhesion retired — mono everywhere

// ─── Components ─────────────────────────────────────────────────────

export const Fade: React.FC<{
  children: React.ReactNode;
  delay?: number;
  fadeIn?: number;
  style?: React.CSSProperties;
}> = ({ children, delay = 0, fadeIn = 15, style }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const opacity = interpolate(
    frame,
    [delay, delay + fadeIn, durationInFrames - 12, durationInFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  const y = interpolate(frame, [delay, delay + fadeIn + 5], [10, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <div style={{ opacity, transform: `translateY(${y}px)`, ...style }}>
      {children}
    </div>
  );
};

export const HardCut: React.FC<{
  children: React.ReactNode;
  delay?: number;
  style?: React.CSSProperties;
}> = ({ children, delay = 0, style }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const opacity = interpolate(
    frame,
    [delay, delay + 1, durationInFrames - 1, durationInFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  return <div style={{ opacity, ...style }}>{children}</div>;
};

export const TypeLine: React.FC<{
  text: string;
  color?: string;
  delay?: number;
  fontSize?: number;
  speed?: number;
  cursorColor?: string;
}> = ({ text, color = "#9a958e", delay = 0, fontSize = 22, speed = 0.8, cursorColor = "#C4A035" }) => {
  const frame = useCurrentFrame();

  const charsVisible = interpolate(
    frame,
    [delay, delay + text.length * speed],
    [0, text.length],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  const cursorOpacity =
    charsVisible < text.length
      ? frame % 16 < 8 ? 1 : 0.3
      : interpolate(frame, [delay + text.length * speed, delay + text.length * speed + 15], [1, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        });

  return (
    <div style={{ fontFamily: fontMono, fontSize, color, whiteSpace: "pre" }}>
      {text.slice(0, Math.floor(charsVisible))}
      <span style={{ opacity: cursorOpacity, color: cursorColor }}>_</span>
    </div>
  );
};

export const Panel: React.FC<{
  children: React.ReactNode;
  palette?: Palette;
  highlight?: boolean;
  style?: React.CSSProperties;
}> = ({ children, palette = dark, highlight = false, style }) => {
  return (
    <div
      style={{
        fontFamily: fontMono,
        fontSize: 22,
        lineHeight: 1.7,
        padding: "36px 50px",
        backgroundColor: highlight ? `${palette.gold}10` : palette.panel,
        borderRadius: 8,
        border: `1px solid ${highlight ? palette.gold : palette.panelBorder}`,
        maxWidth: 1100,
        width: "100%",
        ...style,
      }}
    >
      {children}
    </div>
  );
};

export const RevealLine: React.FC<{
  text: string;
  color?: string;
  delay?: number;
}> = ({ text, color = "#9a958e", delay = 0 }) => {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [delay, delay + 8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return (
    <div style={{ opacity, color, whiteSpace: "pre", fontFamily: fontMono, fontSize: 22 }}>
      {text}
    </div>
  );
};

export const Center: React.FC<{
  children: React.ReactNode;
  padding?: string;
}> = ({ children, padding = "0 120px" }) => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding }}>
    {children}
  </AbsoluteFill>
);

export const GlyphReveal: React.FC<{
  glyph?: string;
  color?: string;
  size?: number;
}> = ({ glyph = "\u25c7", color = "#C4A035", size = 120 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const scale = spring({ frame: frame - 5, fps, config: { damping: 25, stiffness: 60 } });
  const opacity = interpolate(frame, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <div style={{ fontFamily: fontMono, fontSize: size, color, opacity, transform: `scale(${scale})` }}>
      {glyph}
    </div>
  );
};

export const Divider: React.FC<{
  color?: string;
  width?: number;
  delay?: number;
}> = ({ color = "#5a4d3a", width = 60, delay = 0 }) => {
  const frame = useCurrentFrame();
  const w = interpolate(frame, [delay, delay + 20], [0, width], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return <div style={{ width: w, height: 1, backgroundColor: color, margin: "15px 0" }} />;
};

// ─── Standard Closing (V10 style) ───────────────────────────────────
// Push-up: optional message lines rise and fade, then glyph + "werk" + tagline.
// Use inside a Sequence with durationInFrames={s(15)} or more.

export const StandardClosing: React.FC<{
  /** Optional lines shown first, then pushed up */
  preLines?: string[];
  palette?: Palette;
}> = ({ preLines, palette = dark }) => {
  const frame = useCurrentFrame();
  const P = palette;

  const hasPreLines = preLines && preLines.length > 0;
  const pushStart = hasPreLines ? 70 : 0;
  const pushEnd = hasPreLines ? 100 : 0;

  // Pre-lines appear then push up
  const pushY = hasPreLines
    ? interpolate(frame, [pushStart, pushEnd], [0, -380], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })
    : -400;
  const preOp = hasPreLines
    ? interpolate(frame, [pushStart, pushEnd], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })
    : 0;

  // Glyph + name fade in
  const entryStart = hasPreLines ? pushEnd : 5;
  const closOp = interpolate(frame, [entryStart, entryStart + 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const closY = interpolate(frame, [entryStart, entryStart + 22], [25, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  // Tagline
  const tagOp = interpolate(frame, [entryStart + 35, entryStart + 50], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", overflow: "hidden" }}>
      {/* Pre-lines */}
      {hasPreLines && (
        <div style={{
          position: "absolute",
          transform: `translateY(${pushY}px)`,
          opacity: preOp,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 8,
        }}>
          {preLines!.map((line, i) => {
            const lineOp = interpolate(frame, [i * 14, i * 14 + 15], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
            });
            return (
              <div key={i} style={{
                fontFamily: fontMono,
                fontSize: 22,
                color: P.dimWhite,
                textAlign: "center",
                opacity: lineOp,
              }}>
                {line}
              </div>
            );
          })}
        </div>
      )}

      {/* Glyph + name */}
      <div style={{
        opacity: closOp,
        transform: `translateY(${closY}px)`,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold }}>{"\u25c7"}</div>
        <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em" }}>werk</div>
      </div>

      {/* Tagline */}
      <div style={{
        position: "absolute",
        bottom: 180,
        opacity: tagOp,
        textAlign: "center",
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, lineHeight: 1.8 }}>
          Structure determines behavior.
          <br />
          Build the structure that determines yours.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// Utility
export const s = (seconds: number) => seconds * 30;
