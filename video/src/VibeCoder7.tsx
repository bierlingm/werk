// "Tuesday." — The Limitless Version
// 65s. The triumph video. Everything clicks.
//
// Visual concept: The world goes from grey/dim/chaotic to sharp/gold/clear.
// The "pill moment" is when the werk structure appears.
// After that, everything is faster, sharper, more confident.
//
// Three acts:
//   I.  The fog (0-15s): grey, slow, confused, dim
//   II. The click (15-20s): the structure appears, color floods in
//   III. The ride (20-55s): fast, sharp, gold, everything working
//   IV. Close (55-65s): the swagger

import React from "react";
import {
  AbsoluteFill,
  Audio,
  Sequence,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  spring,
} from "remotion";
import { dark, fontMono, Fade, Panel, RevealLine, TypeLine, s } from "./primitives";

const P = dark;

// Desaturated/fog colors for Act I
const FOG = {
  text: "#707070",
  dim: "#484848",
  faint: "#333333",
  bg: "#060606",
};

// ─── ACT I: THE FOG ────────────────────────────────────────────────

// Confused prompting. Slow. Grey. Lost.
const SceneFog1: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
    <Fade delay={15} fadeIn={25} style={{ fontFamily: fontMono, fontSize: 32, color: FOG.text, textAlign: "center", lineHeight: 1.8 }}>
      You've been building for two weeks.
    </Fade>
    <Fade delay={55} fadeIn={25} style={{ fontFamily: fontMono, fontSize: 28, color: FOG.dim, textAlign: "center", marginTop: 12 }}>
      You're not sure what you have.
    </Fade>
  </AbsoluteFill>
);

const SceneFog2: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 10, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Slow, grey chat. Everything feels heavy.
  const lines = [
    { r: "user" as const, t: "what did we build yesterday", d: 8 },
    { r: "ai" as const, t: "I don't have context from previous sessions", d: 28 },
    { r: "user" as const, t: "right... ok let me explain the whole thing", d: 55 },
    { r: "ai" as const, t: "I'm ready to help! What are we building?", d: 75 },
    { r: "user" as const, t: "...", d: 100 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px", opacity: fadeOut }}>
      <div style={{ maxWidth: 750, width: "100%" }}>
        <Panel style={{ padding: "24px 32px", borderColor: FOG.faint, backgroundColor: "#0a0a0a" }}>
          {lines.map((line, i) => {
            const op = interpolate(frame, [line.d, line.d + 12], [0, 1], {
              extrapolateLeft: "clamp", extrapolateRight: "clamp",
            });
            return (
              <div key={i} style={{
                opacity: op, fontFamily: fontMono, fontSize: 18,
                color: line.r === "user" ? FOG.text : FOG.dim,
                lineHeight: 1.7, marginBottom: 2,
              }}>
                {line.r === "user" ? "> " : "  "}{line.t}
              </div>
            );
          })}
        </Panel>
      </div>
    </AbsoluteFill>
  );
};

// ─── ACT II: THE CLICK ─────────────────────────────────────────────

// The structure appears. Color floods in. The Limitless moment.
const SceneClick: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Start grey, transition to gold
  const colorShift = interpolate(frame, [0, 60], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Panel border goes from grey to gold
  const borderColor = colorShift > 0.5 ? P.gold : FOG.faint;
  const bgTint = interpolate(colorShift, [0, 1], [0, 0.06], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Text colors shift
  const desireColor = colorShift > 0.3 ? P.gold : FOG.text;
  const stepColor = colorShift > 0.5 ? P.white : FOG.dim;
  const doneColor = colorShift > 0.6 ? P.green : FOG.dim;
  const realityColor = colorShift > 0.5 ? P.dimWhite : FOG.dim;

  const hereOp = colorShift > 0.7 ? 0.6 + Math.sin((frame - 50) * 0.1) * 0.4 : 0;

  // Glow effect around the panel as color arrives
  const glowOp = interpolate(frame, [30, 50, 70, 90], [0, 0.15, 0.15, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ maxWidth: 800, width: "100%", position: "relative" }}>
        {/* Glow behind panel */}
        <div style={{
          position: "absolute", inset: -20, borderRadius: 16,
          background: `radial-gradient(ellipse, ${P.gold}${Math.round(glowOp * 255).toString(16).padStart(2, "0")} 0%, transparent 70%)`,
        }} />

        <Panel style={{
          padding: "22px 30px",
          borderColor,
          backgroundColor: `rgba(17, 17, 16, ${1 - bgTint})`,
          position: "relative",
        }}>
          <RevealLine text={"\u25c7 Customers book appointments through my app"} color={desireColor} delay={5} />
          <div style={{ margin: "10px 0", paddingLeft: 20, borderLeft: `2px solid ${colorShift > 0.5 ? P.faintGold : FOG.faint}` }}>
            <RevealLine text={"  5. email confirmations go out"} color={colorShift > 0.5 ? P.dimWhite : FOG.faint} delay={18} />
            <RevealLine text={"  4. calendar syncs with Google"} color={colorShift > 0.5 ? P.dimWhite : FOG.faint} delay={24} />
            <div style={{ display: "flex", alignItems: "center" }}>
              <RevealLine text={"  3. payment collected at booking"} color={stepColor} delay={30} />
              {hereOp > 0 && <span style={{ fontFamily: fontMono, fontSize: 18, color: P.gold, opacity: hereOp, marginLeft: 12 }}>{"\u2190"}</span>}
            </div>
            <RevealLine text={"  \u2713 2. customers pick time slots"} color={doneColor} delay={36} />
            <RevealLine text={"  \u2713 1. business sets their schedule"} color={doneColor} delay={42} />
          </div>
          <RevealLine text={"\u25c6 Scheduling works. Payment not started."} color={realityColor} delay={50} />
        </Panel>
      </div>
    </AbsoluteFill>
  );
};

// ─── ACT III: THE RIDE ──────────────────────────────────────────────

// Fast. Sharp. Confident. Gold everything. The swagger montage.

const SceneRide1: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
    <div style={{ maxWidth: 850, width: "100%" }}>
      <Panel style={{ padding: "18px 26px", borderColor: P.blue }}>
        <TypeLine text="Payment step. Stripe. Existing booking flow." color={P.white} delay={5} fontSize={19} speed={0.35} cursorColor={P.blue} />
        <TypeLine text="Scheduling is settled. Not touching it." color={P.dimGold} delay={30} fontSize={19} speed={0.35} cursorColor={P.blue} />
        <TypeLine text="Done." color={P.green} delay={52} fontSize={19} speed={0.3} cursorColor={P.blue} />
      </Panel>
    </div>
  </AbsoluteFill>
);

const SceneRide2: React.FC = () => {
  const frame = useCurrentFrame();
  const checkOp = interpolate(frame, [15, 30], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nextOp = interpolate(frame, [35, 50], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
        <div style={{ fontFamily: fontMono, fontSize: 24, color: P.green, opacity: checkOp }}>
          {"\u2713"} payment collected at booking
        </div>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, opacity: nextOp }}>
          {"\u2192"} calendar syncs with Google
        </div>
      </div>
    </AbsoluteFill>
  );
};

const SceneRide3: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
    <div style={{ maxWidth: 850, width: "100%" }}>
      <Panel style={{ padding: "18px 26px", borderColor: P.blue }}>
        <TypeLine text="Calendar integration. Google API." color={P.white} delay={5} fontSize={19} speed={0.35} cursorColor={P.blue} />
        <TypeLine text="Booking and payment are solid. Building on top." color={P.dimGold} delay={28} fontSize={19} speed={0.35} cursorColor={P.blue} />
        <TypeLine text="Done." color={P.green} delay={50} fontSize={19} speed={0.3} cursorColor={P.blue} />
      </Panel>
    </div>
  </AbsoluteFill>
);

const SceneRide4: React.FC = () => {
  const frame = useCurrentFrame();
  // Rapid-fire completions
  const checks = [
    { text: "\u2713 3. payment collected at booking", delay: 5 },
    { text: "\u2713 4. calendar syncs with Google", delay: 25 },
    { text: "\u2192 5. email confirmations go out", delay: 50 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        {checks.map((c, i) => {
          const op = interpolate(frame, [c.delay, c.delay + 10], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const isNext = i === 2;
          return (
            <div key={i} style={{
              fontFamily: fontMono, fontSize: 22,
              color: isNext ? P.gold : P.green,
              opacity: op,
            }}>
              {c.text}
            </div>
          );
        })}
        <Fade delay={75} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, marginTop: 12 }}>
          One step left.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// The feeling. Not stated. Shown through pacing.
const SceneFeeling: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade delay={10} style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center", lineHeight: 1.5 }}>
      You know exactly where you are.
    </Fade>
    <Fade delay={50} style={{ fontFamily: fontMono, fontSize: 36, color: P.white, textAlign: "center", marginTop: 8 }}>
      You know exactly what's next.
    </Fade>
    <Fade delay={85} style={{ fontFamily: fontMono, fontSize: 24, color: P.dimGold, textAlign: "center", marginTop: 20 }}>
      And so does your AI.
    </Fade>
  </AbsoluteFill>
);

// ─── ACT IV: THE SWAGGER ────────────────────────────────────────────

const SceneClosing: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const pushStart = 80;
  const pushEnd = 110;
  const preY = interpolate(frame, [pushStart, pushEnd], [0, -350], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const preOp = interpolate(frame, [pushStart, pushEnd], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const glyphScale = spring({ frame: frame - pushEnd - 5, fps, config: { damping: 25, stiffness: 60 } });
  const glyphOp = interpolate(frame, [pushEnd + 5, pushEnd + 23], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nameOp = interpolate(frame, [pushEnd + 25, pushEnd + 43], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nameY = interpolate(frame, [pushEnd + 25, pushEnd + 43], [15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const tagOp = interpolate(frame, [pushEnd + 60, pushEnd + 78], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", overflow: "hidden" }}>
      <div style={{ position: "absolute", transform: `translateY(${preY}px)`, opacity: preOp, textAlign: "center" }}>
        {["Same you. Same AI.", "Different structure."].map((line, i) => {
          const lOp = interpolate(frame, [i * 18, i * 18 + 15], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
          return <div key={i} style={{ fontFamily: fontMono, fontSize: 24, color: P.dimWhite, opacity: lOp }}>{line}</div>;
        })}
      </div>

      <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>{"\u25c7"}</div>
      <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em", marginTop: 8, opacity: nameOp, transform: `translateY(${nameY}px)` }}>werk</div>
      <div style={{ position: "absolute", bottom: 180, opacity: tagOp, textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, lineHeight: 1.8 }}>
          Build things that compound.
          <br />
          Not things that collapse.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER7_DURATION = s(65);

export const VibeCoder7: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      {/* Nessun Dorma: silent during fog, enters at the click, builds through the ride.
          Start from ~95s into the track (the orchestral build before the final climax).
          The "Vincerò!" hits around 160-170s, which maps to ~our 33-39s mark. */}
      <Audio src={staticFile("music/nessun-dorma.mp3")} volume={(f: number) => {
        // Silent for first 10s (the fog). Enters softly during the click. Builds to full.
        const fi = interpolate(f, [s(10), s(20)], [0, 0.35], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER7_DURATION - s(5), VIBECODER7_DURATION], [0.35, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={95 * 30} />

      {/* ACT I: THE FOG — grey, slow, lost */}
      <Sequence from={s(0)} durationInFrames={s(5)}>
        <SceneFog1 />
      </Sequence>
      <Sequence from={s(5)} durationInFrames={s(7)}>
        <SceneFog2 />
      </Sequence>

      {/* ACT II: THE CLICK — color floods in */}
      <Sequence from={s(12)} durationInFrames={s(6)}>
        <SceneClick />
      </Sequence>

      {/* ACT III: THE RIDE — fast, sharp, confident */}
      <Sequence from={s(18)} durationInFrames={s(4)}>
        <SceneRide1 />
      </Sequence>
      <Sequence from={s(22)} durationInFrames={s(3)}>
        <SceneRide2 />
      </Sequence>
      <Sequence from={s(25)} durationInFrames={s(4)}>
        <SceneRide3 />
      </Sequence>
      <Sequence from={s(29)} durationInFrames={s(4)}>
        <SceneRide4 />
      </Sequence>
      <Sequence from={s(33)} durationInFrames={s(6)}>
        <SceneFeeling />
      </Sequence>

      {/* ACT IV: CLOSE */}
      <Sequence from={s(39)} durationInFrames={s(26)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
