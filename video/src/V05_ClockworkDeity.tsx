// Video 5: "Coherence Amongst Complexity" — 50s
// Establish what a tension IS before using the term.
// Properly formatted bullets. Clear argument flow.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { dark, fontMono, Fade, Panel, StandardClosing, s } from "./primitives";

const P = dark;

export const V05_DURATION = s(50);

export const V05_ClockworkDeity: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/minimal-outer-thoughts.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.15], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V05_DURATION - s(4), V05_DURATION], [0.15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* The situation */}
      <Sequence from={0} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 30, color: P.white, textAlign: "center", lineHeight: 1.7 }}>
            5 projects. 3 agents. 200 open tasks.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 30, color: P.white, textAlign: "center", lineHeight: 1.7, marginTop: 5 }}>
            Everything is moving.
          </Fade>
          <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 26, color: P.gold, textAlign: "center", marginTop: 15 }}>
            How do you hold it all without losing the thread?
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Introduce the concept — plain language first */}
      <Sequence from={s(7)} durationInFrames={s(10)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 24, color: P.dimWhite, textAlign: "center", lineHeight: 1.8, marginBottom: 20 }}>
            What if each project had a simple, living structure:
          </Fade>
          <div style={{ maxWidth: 800, width: "100%" }}>
            {[
              { icon: "\u25c7", text: "What you want it to become", color: P.gold, delay: 20 },
              { icon: "\u25c6", text: "Where it actually stands right now", color: P.white, delay: 35 },
              { icon: "\u25c8", text: "Your ordered steps from here to there", color: P.white, delay: 50 },
              { icon: "\u25cb", text: "What's at the front of the line", color: P.dimGold, delay: 65 },
            ].map((item, i) => (
              <Fade key={i} delay={item.delay} style={{
                fontFamily: fontMono,
                fontSize: 22,
                color: item.color,
                display: "flex",
                alignItems: "center",
                gap: 16,
                marginBottom: 14,
              }}>
                <span style={{ fontSize: 20, flexShrink: 0 }}>{item.icon}</span>
                <span>{item.text}</span>
              </Fade>
            ))}
          </div>
          <Fade delay={85} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, textAlign: "center", marginTop: 20 }}>
            That's a tension. werk holds one for each thing you're advancing.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* What this enables */}
      <Sequence from={s(17)} durationInFrames={s(10)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 24, color: P.white, textAlign: "center", lineHeight: 1.8, marginBottom: 20 }}>
            Now multiply that across everything you're doing:
          </Fade>
          <div style={{ maxWidth: 800, width: "100%" }}>
            {[
              { text: "You see all 5 projects at a glance", delay: 20 },
              { text: "Your agent reads the same structure for context", delay: 35 },
              { text: "Overdue steps surface automatically", delay: 50 },
              { text: "When your aim changes, the structure adapts", delay: 65 },
            ].map((item, i) => (
              <Fade key={i} delay={item.delay} style={{
                fontFamily: fontMono,
                fontSize: 20,
                color: P.dimGold,
                marginBottom: 12,
                paddingLeft: 16,
              }}>
                {"\u2022"} {item.text}
              </Fade>
            ))}
          </div>
          <Fade delay={80} style={{ fontFamily: fontMono, fontSize: 20, color: P.white, textAlign: "center", marginTop: 20 }}>
            You reason better. Your agent acts with greater clarity.
            <br />
            Complexity stays complex. You stay coherent.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* The phrase */}
      <Sequence from={s(27)} durationInFrames={s(8)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 44, color: P.gold, textAlign: "center", lineHeight: 1.4 }}>
            Coherence amongst complexity.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(35)} durationInFrames={s(15)}>
        <StandardClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
