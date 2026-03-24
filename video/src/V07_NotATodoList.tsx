// Video 7: "Not a Todo List" — 55s
// Ascending step numbering. Lighter final scene.
// What's actually distinct: steps as hypotheses, desire-reality as driver.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { dark, fontMono, Fade, Panel, StandardClosing, s } from "./primitives";

const P = dark;

export const V07_DURATION = s(55);

export const V07_NotATodoList: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/rhythmic-screen-saver.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.14], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V07_DURATION - s(4), V07_DURATION], [0.14, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* Hook */}
      <Sequence from={0} durationInFrames={s(5)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.white, textAlign: "center", lineHeight: 1.8 }}>
            You know this feeling:
          </Fade>
          <Fade delay={18} style={{ fontFamily: fontMono, fontSize: 24, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 10 }}>
            47 tasks, all "in progress," no idea which one
            <br />
            actually advances what you care about.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Task shown first, alone */}
      <Sequence from={s(5)} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 550, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.faintGold, letterSpacing: "2px", marginBottom: 12 }}>
              A TASK
            </Fade>
            <Panel>
              <Fade delay={6} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimWhite, lineHeight: 1.8 }}>
                <div><span style={{ color: P.faintGold }}>title:</span> Build billing page</div>
              </Fade>
              <Fade delay={14} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimWhite, lineHeight: 1.8 }}>
                <div><span style={{ color: P.faintGold }}>status:</span> in-progress</div>
              </Fade>
              <Fade delay={22} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimWhite, lineHeight: 1.8 }}>
                <div><span style={{ color: P.faintGold }}>parent:</span> Epic: Monetization</div>
              </Fade>
              <Fade delay={30} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimWhite, lineHeight: 1.8 }}>
                <div><span style={{ color: P.faintGold }}>deps:</span> [auth, stripe-setup]</div>
              </Fade>
            </Panel>
            <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimGold, textAlign: "center", marginTop: 12 }}>
              Knows what to do and what it depends on.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Tension — ascending numbering, proper layout */}
      <Sequence from={s(12)} durationInFrames={s(14)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 600, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.gold, letterSpacing: "2px", marginBottom: 12 }}>
              A TENSION
            </Fade>
            <Panel highlight>
              <Fade delay={6} style={{ fontFamily: fontMono, fontSize: 20, color: P.gold }}>
                {"\u25c7"} users can pay for the product
              </Fade>

              <div style={{ margin: "12px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
                <Fade delay={34} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, lineHeight: 1.7 }}>
                  3. migration path tested
                </Fade>
                <Fade delay={28} style={{ fontFamily: fontMono, fontSize: 16, color: P.white, lineHeight: 1.7 }}>
                  2. Stripe handles subscriptions  {"\u2190"} next
                </Fade>
                <Fade delay={22} style={{ fontFamily: fontMono, fontSize: 16, color: P.green, lineHeight: 1.7 }}>
                  {"\u2713"} 1. usage metering works
                </Fade>
                <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 16, color: P.faintWhite, lineHeight: 1.7, fontStyle: "italic" }}>
                  held: pricing page design
                </Fade>
              </div>

              <Fade delay={46} style={{ fontFamily: fontMono, fontSize: 20, color: P.white }}>
                {"\u25c6"} 8 free users, API works, no billing yet
              </Fade>

              <Fade delay={54} style={{ fontFamily: fontMono, fontSize: 14, color: P.dimGold, marginTop: 10, fontStyle: "italic" }}>
                note: "Stripe webhooks need retry logic"
              </Fade>
            </Panel>
            <Fade delay={64} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimGold, textAlign: "center", marginTop: 12 }}>
              Knows what you want, where you are, and the plan to get there.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* The distinction — cleaner, focused */}
      <Sequence from={s(26)} durationInFrames={s(12)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 26, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            A task tracks what to do.
          </Fade>
          <Fade delay={18} style={{ fontFamily: fontMono, fontSize: 26, color: P.gold, textAlign: "center", lineHeight: 1.8, marginTop: 8 }}>
            A tension also tracks what to do —
            <br />
            plus <em>why</em>, <em>where you stand</em>, and <em>whether it still makes sense</em>.
          </Fade>
          <Fade delay={55} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 25 }}>
            The key difference: the steps aren't assignments.
            <br />
            They're hypotheses. They get replaced as you learn.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(38)} durationInFrames={s(17)}>
        <StandardClosing preLines={["The steps aren't assignments.", "They're hypotheses.", "They get replaced as you learn."]} />
      </Sequence>
    </AbsoluteFill>
  );
};
