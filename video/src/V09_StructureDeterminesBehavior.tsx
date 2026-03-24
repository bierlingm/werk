// Video 9: "Structure Determines Behavior" — 60s
// Visual compound divergence. Two trajectory lines diverging over time.
// Ascending numbering. Push-up outro.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { dark, fontMono, Fade, Panel, StandardClosing, s } from "./primitives";

const P = dark;

// Animated diverging trajectories
const TrajectoryDivergence: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const progress = interpolate(frame, [0, durationInFrames - 30], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const fadeIn = interpolate(frame, [0, 20], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const w = 1000;
  const h = 400;
  const margin = 60;

  // Scattered trajectory: oscillates, barely advances
  const scatteredPoints: string[] = [];
  // Directed trajectory: steady compound growth
  const directedPoints: string[] = [];

  const steps = 80;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    if (t > progress) break;
    const x = margin + t * (w - 2 * margin);

    // Scattered: noisy oscillation around a slowly rising line
    const scatteredY = h - margin - (t * 40) - Math.sin(t * 18) * 30 - Math.cos(t * 7) * 20;
    scatteredPoints.push(`${x},${scatteredY}`);

    // Directed: compound curve (exponential-ish)
    const directedY = h - margin - (Math.pow(t, 1.5) * (h - 2 * margin) * 0.9);
    directedPoints.push(`${x},${directedY}`);
  }

  // Time labels
  const timeLabels = [
    { x: margin, label: "Day 1" },
    { x: margin + (w - 2 * margin) * 0.25, label: "1 month" },
    { x: margin + (w - 2 * margin) * 0.5, label: "3 months" },
    { x: margin + (w - 2 * margin) * 0.75, label: "1 year" },
    { x: w - margin, label: "3 years" },
  ];

  const labelsOp = interpolate(frame, [10, 30], [0, 0.6], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: Math.min(fadeIn, fadeOut) }}>
      <svg width={w} height={h + 60} style={{ overflow: "visible" }}>
        {/* Axis */}
        <line x1={margin} y1={h - margin} x2={w - margin} y2={h - margin} stroke={P.faintGold} strokeWidth={1} opacity={0.3} />

        {/* Time labels */}
        {timeLabels.map((tl, i) => (
          <text key={i} x={tl.x} y={h - margin + 25} fill={P.faintGold} fontSize={12} fontFamily="Berkeley Mono, Menlo, monospace" textAnchor="middle" opacity={labelsOp}>
            {tl.label}
          </text>
        ))}

        {/* Scattered line */}
        {scatteredPoints.length > 1 && (
          <polyline
            points={scatteredPoints.join(" ")}
            fill="none"
            stroke={P.red}
            strokeWidth={2.5}
            opacity={0.8}
          />
        )}

        {/* Directed line */}
        {directedPoints.length > 1 && (
          <polyline
            points={directedPoints.join(" ")}
            fill="none"
            stroke={P.gold}
            strokeWidth={2.5}
            opacity={0.9}
          />
        )}

        {/* Labels at end of lines */}
        {progress > 0.85 && (
          <>
            <text x={w - margin + 10} y={Number(scatteredPoints[scatteredPoints.length - 1]?.split(",")[1]) || 0} fill={P.red} fontSize={14} fontFamily="Berkeley Mono, Menlo, monospace" dominantBaseline="middle" opacity={interpolate(progress, [0.85, 0.95], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })}>
              scattered
            </text>
            <text x={w - margin + 10} y={Number(directedPoints[directedPoints.length - 1]?.split(",")[1]) || 0} fill={P.gold} fontSize={14} fontFamily="Berkeley Mono, Menlo, monospace" dominantBaseline="middle" opacity={interpolate(progress, [0.85, 0.95], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" })}>
              directed
            </text>
          </>
        )}
      </svg>
    </AbsoluteFill>
  );
};

// Push-up closing
const Closing: React.FC = () => {
  const frame = useCurrentFrame();
  const pushStart = 50;
  const pushEnd = 80;
  const pushY = interpolate(frame, [pushStart, pushEnd], [0, -350], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const msgOp = interpolate(frame, [pushStart, pushEnd], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const closOp = interpolate(frame, [pushEnd, pushEnd + 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const closY = interpolate(frame, [pushEnd, pushEnd + 22], [25, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const tagOp = interpolate(frame, [pushEnd + 35, pushEnd + 50], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", overflow: "hidden" }}>
      <div style={{ position: "absolute", transform: `translateY(${pushY}px)`, opacity: msgOp, textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 26, color: P.gold, lineHeight: 1.6 }}>
          The structure changed.
          <br />
          And the behavior followed.
        </div>
      </div>
      <div style={{ opacity: closOp, transform: `translateY(${closY}px)`, display: "flex", flexDirection: "column", alignItems: "center", gap: 8 }}>
        <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold }}>{"\u25c7"}</div>
        <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em" }}>werk</div>
      </div>
      <div style={{ position: "absolute", bottom: 180, opacity: tagOp, textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, lineHeight: 1.8 }}>
          Structure determines behavior.
          <br />
          Build the structure that determines yours.
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const V09_DURATION = s(60);

export const V09_StructureDeterminesBehavior: React.FC = () => {
  const musicVolume = (f: number) => {
    const fi = interpolate(f, [0, s(3)], [0, 0.18], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
    const fo = interpolate(f, [V09_DURATION - s(4), V09_DURATION], [0.18, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
    return Math.min(fi, fo);
  };

  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/cinematic-night-vigil.mp3")} volume={musicVolume} startFrom={30 * 30} />

      {/* Setup */}
      <Sequence from={0} durationInFrames={s(5)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            Same person. Same work. Same 24 hours.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 28, color: P.gold, textAlign: "center", marginTop: 10 }}>
            Why do some days feel scattered and others feel directed?
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Todo list alone */}
      <Sequence from={s(5)} durationInFrames={s(8)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 700, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.faintGold, letterSpacing: "2px", marginBottom: 14 }}>MONDAY: THE TODO LIST</Fade>
            <Panel>
              {["Fix billing bug", "Write blog post", "Update API docs", "Call investor", "Review PR #47", "Ship onboarding flow", "... 24 more"].map((item, i) => (
                <Fade key={i} delay={4 + i * 4} style={{ fontFamily: fontMono, fontSize: 18, color: i < 6 ? P.dimWhite : P.faintWhite, lineHeight: 1.7 }}>
                  {"- [ ] "}{item}
                </Fade>
              ))}
            </Panel>
            <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 18, color: P.red, textAlign: "center", marginTop: 14 }}>
              Everything equally loud. No direction.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Tension alone — ascending */}
      <Sequence from={s(13)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 700, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.gold, letterSpacing: "2px", marginBottom: 14 }}>SAME MONDAY: THE TENSION FIELD</Fade>
            <Panel highlight>
              <Fade delay={6} style={{ fontFamily: fontMono, fontSize: 20, color: P.gold }}>{"\u25c7"} product serves paying users</Fade>
              <div style={{ margin: "10px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
                <Fade delay={22} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, lineHeight: 1.7 }}>4. first 10 users paying</Fade>
                <Fade delay={19} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, lineHeight: 1.7 }}>3. onboarding converts</Fade>
                <Fade delay={16} style={{ fontFamily: fontMono, fontSize: 16, color: P.white, lineHeight: 1.7 }}>2. billing handles subscriptions  {"\u2190"} here</Fade>
                <Fade delay={13} style={{ fontFamily: fontMono, fontSize: 16, color: P.green, lineHeight: 1.7 }}>{"\u2713"} 1. usage metering works</Fade>
              </div>
              <Fade delay={28} style={{ fontFamily: fontMono, fontSize: 20, color: P.white }}>{"\u25c6"} 8 free users, API stable, no revenue</Fade>
            </Panel>
            <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 18, color: P.green, textAlign: "center", marginTop: 14 }}>
              One aim. Ordered steps. You know exactly what's next.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Text setup */}
      <Sequence from={s(22)} durationInFrames={s(5)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 24, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            One directed day doesn't look different from a scattered one.
          </Fade>
          <Fade delay={20} style={{ fontFamily: fontMono, fontSize: 26, color: P.gold, textAlign: "center", marginTop: 10 }}>
            But the difference compounds.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Chart alone, full screen */}
      <Sequence from={s(27)} durationInFrames={s(9)}>
        <TrajectoryDivergence />
      </Sequence>

      {/* Close */}
      <Sequence from={s(36)} durationInFrames={s(24)}>
        <StandardClosing preLines={[
          "Over years, one person shipped what they set out to ship.",
          "The other is still managing tasks.",
        ]} />
      </Sequence>
    </AbsoluteFill>
  );
};
