// Video 3: "What Your Agent Doesn't Know" — 50s
// Clearer structure. What it needs mirrors actual werk output.
// Shorter outro.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { dark, fontMono, Fade, Panel, RevealLine, StandardClosing, s } from "./primitives";

const P = dark;

export const V03_DURATION = s(50);

export const V03_WhatYourAgentDoesntKnow: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/rhythmic-screen-saver.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.15], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V03_DURATION - s(3), V03_DURATION], [0.15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* Title */}
      <Sequence from={0} durationInFrames={s(5)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 52, color: P.white, textAlign: "center" }}>
            What your agent sees
            <br />
            <span style={{ color: P.red, fontSize: 60 }}>{"\u2260"}</span>
            <br />
            what your agent needs
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* What it sees */}
      <Sequence from={s(5)} durationInFrames={s(8)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 1000, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.red, letterSpacing: "2px", marginBottom: 14 }}>
              WHAT YOUR AGENT GETS TODAY
            </Fade>
            <Panel>
              <RevealLine text="342 source files" color={P.dimWhite} delay={6} />
              <RevealLine text="a CLAUDE.md last touched 12 days ago" color={P.red} delay={12} />
              <RevealLine text="git log: 200 commits of diffs" color={P.dimWhite} delay={18} />
              <RevealLine text="a README that describes v1" color={P.faintWhite} delay={24} />
            </Panel>
            <Fade delay={35} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 16 }}>
              Raw material. No intent. No priorities. No "where are we?"
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* What it needs — mirrors actual werk output */}
      <Sequence from={s(13)} durationInFrames={s(12)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 1000, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.gold, letterSpacing: "2px", marginBottom: 14 }}>
              WHAT YOUR AGENT GETS WITH WERK
            </Fade>
            <Panel highlight>
              <RevealLine text={"\u25c7 desired: product serves 100 paying users"} color={P.gold} delay={6} />
              <RevealLine text={"\u25c6 reality: API works, 8 free users, no billing"} color={P.white} delay={14} />
              <RevealLine text="" color={P.faintGold} delay={20} />
              <RevealLine text="  next step:  Stripe integration" color={P.white} delay={22} />
              <RevealLine text="  progress:   4 of 9 steps done" color={P.green} delay={28} />
              <RevealLine text="  overdue:    yes, by 8 days" color={P.red} delay={34} />
              <RevealLine text="  changed:    dropped docs step after learning users prefer FAQ" color={P.dimGold} delay={40} />
            </Panel>
            <Fade delay={52} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 16 }}>
              Structured intent. Current. Actionable.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Three lines */}
      <Sequence from={s(25)} durationInFrames={s(10)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            Files tell your agent what exists.
          </Fade>
          <Fade delay={20} style={{ fontFamily: fontMono, fontSize: 28, color: P.dimWhite, textAlign: "center", lineHeight: 1.8, marginTop: 5 }}>
            Git tells it what changed.
          </Fade>
          <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 32, color: P.gold, textAlign: "center", lineHeight: 1.8, marginTop: 5 }}>
            werk tells it what matters.
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
