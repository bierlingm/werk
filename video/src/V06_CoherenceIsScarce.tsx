// Video 6: "Coherence is Scarce" — 45s
// Three statements. Scarce explanation as question. Cleaner layout.
// No Adhesion.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { mono, fontMono, Fade, StandardClosing, s } from "./primitives";

const P = { ...mono, gold: "#C4A035", dimGold: "#8B7355", faintGold: "#5a4d3a" };

export const V06_DURATION = s(45);

export const V06_CoherenceIsScarce: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/emotional-ancient-rite.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.12], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V06_DURATION - s(4), V06_DURATION], [0.12, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* Statement 1 */}
      <Sequence from={0} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 48, color: P.white, textAlign: "center" }}>
            Execution is cheap.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimWhite, textAlign: "center", marginTop: 16 }}>
            AI agents write code, draft docs, run campaigns.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Statement 2 */}
      <Sequence from={s(7)} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 48, color: P.white, textAlign: "center" }}>
            Context is expensive.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimWhite, textAlign: "center", marginTop: 16 }}>
            Every interaction starts with: what are we working on?
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Statement 3 — as question */}
      <Sequence from={s(14)} durationInFrames={s(8)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center" }}>
            Coherence is scarce.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimWhite, textAlign: "center", marginTop: 20, lineHeight: 1.8 }}>
            Can you say, right now, across everything you're working on:
            <br />
            what you want, where you are, and whether the plan still makes sense?
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* What werk offers — with transition from the question */}
      <Sequence from={s(22)} durationInFrames={s(10)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 22, color: P.dimWhite, textAlign: "center", marginBottom: 24 }}>
            werk makes coherence possible by giving you:
          </Fade>
          <div style={{ maxWidth: 800, width: "100%" }}>
            {[
              { text: "A living model of what you want and where you stand", delay: 12 },
              { text: "Steps that update as you work, not as a separate chore", delay: 26 },
              { text: "Context your agent can read at the start of every session", delay: 40 },
              { text: "The structural shape of your progress over time", delay: 54 },
            ].map((item, i) => (
              <Fade key={i} delay={item.delay} style={{
                fontFamily: fontMono,
                fontSize: 22,
                color: P.white,
                marginBottom: 16,
                display: "flex",
                gap: 14,
              }}>
                <span style={{ color: P.gold, flexShrink: 0 }}>{"\u25c7"}</span>
                <span>{item.text}</span>
              </Fade>
            ))}
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(32)} durationInFrames={s(13)}>
        <StandardClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
