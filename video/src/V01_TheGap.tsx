// Video 1: "The Gap" — 50s

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate, spring } from "remotion";
import { dark, fontMono, Fade, StandardClosing, s } from "./primitives";

const P = dark;

const VerticalGap: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  const gapSize = spring({ frame: frame - 10, fps, config: { damping: 15, stiffness: 25 }, from: 40, to: 320 });
  const desireIn = interpolate(frame, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const realityIn = interpolate(frame, [18, 36], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const lineIn = interpolate(frame, [30, 50], [0, 0.6], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const children = [
    { pos: 0.85, label: "1. pricing tested against real demand", delay: 65 },
    { pos: 0.62, label: "2. 10 beta users validate the product", delay: 77 },
    { pos: 0.39, label: "3. onboarding converts without support", delay: 89 },
    { pos: 0.16, label: "4. payment system handles subscriptions", delay: 101 },
  ];

  const fadeOut = interpolate(frame, [durationInFrames - 18, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: fadeOut }}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center" }}>
        <div style={{ opacity: desireIn, display: "flex", alignItems: "center", gap: 14 }}>
          <span style={{ fontFamily: fontMono, fontSize: 32, color: P.gold }}>{"\u25c7"}</span>
          <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold }}>100 paying users</span>
        </div>
        <div style={{ position: "relative", width: 2, height: gapSize, backgroundColor: P.dimGold, opacity: lineIn, margin: "6px 0" }}>
          {children.map((child, i) => {
            const op = interpolate(frame, [child.delay, child.delay + 12], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
            return (
              <div key={i} style={{ position: "absolute", top: `${child.pos * 100}%`, left: 24, transform: "translateY(-50%)", display: "flex", alignItems: "center", gap: 10, opacity: op, whiteSpace: "nowrap" }}>
                <span style={{ fontFamily: fontMono, fontSize: 14, color: P.dimGold }}>{"\u25c6"}</span>
                <span style={{ fontFamily: fontMono, fontSize: 16, color: P.white }}>{child.label}</span>
              </div>
            );
          })}
        </div>
        <div style={{ opacity: realityIn, display: "flex", alignItems: "center", gap: 14 }}>
          <span style={{ fontFamily: fontMono, fontSize: 32, color: P.white }}>{"\u25c6"}</span>
          <span style={{ fontFamily: fontMono, fontSize: 22, color: P.white }}>prototype works, no users, no revenue</span>
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const V01_DURATION = s(50);

export const V01_TheGap: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/ambient-ether-vox.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.2], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V01_DURATION - s(4), V01_DURATION], [0.2, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* Relatable opening */}
      <Sequence from={0} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 26, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            You know where you want to be.
            <br />
            You know where you are.
          </Fade>
          <Fade delay={35} style={{ fontFamily: fontMono, fontSize: 28, color: P.gold, textAlign: "center", marginTop: 15 }}>
            The space between those two things is the most
            <br />
            powerful force in your creative process.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* The visual */}
      <Sequence from={s(7)} durationInFrames={s(15)}>
        <VerticalGap />
      </Sequence>

      {/* Name the concept */}
      <Sequence from={s(22)} durationInFrames={s(12)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center" }}>
            Structural tension.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 15 }}>
            The distance between the two is what drives action.
            <br />
            The steps in between are your current best guess.
            <br />
            They get replaced as you learn.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Standard closing */}
      <Sequence from={s(34)} durationInFrames={s(16)}>
        <StandardClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
