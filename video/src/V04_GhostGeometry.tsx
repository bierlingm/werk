// Video 4: "Ghost Geometry" — 60s
// Ascending step numbering. Same tension evolving.
// Desire also evolves between epochs.
// Visual geometric representation of the ghost geometry concept.
// Larger, brighter epoch cards.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate, spring } from "remotion";
import { warm, fontMono, Fade, Panel, StandardClosing, s } from "./primitives";

const P = warm;

// Triangle/pyramid representing a delta — the geometric ghost
const DeltaPyramid: React.FC<{
  desire: string;
  reality: string;
  completion: number; // 0-1
  dimmed?: boolean;
  delay?: number;
}> = ({ desire, reality, completion, dimmed = false, delay = 0 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const op = interpolate(frame, [delay, delay + 20], [0, dimmed ? 0.4 : 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const h = spring({ frame: frame - delay - 5, fps, config: { damping: 18, stiffness: 30 }, from: 0, to: 180 });
  const w = 160;
  const fillH = h * completion;

  return (
    <div style={{ opacity: op, display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
      <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, textAlign: "center", maxWidth: 180 }}>
        {desire}
      </div>
      {/* Pyramid with fill */}
      <svg width={w} height={h + 4} style={{ overflow: "visible" }}>
        {/* Filled portion from base */}
        <polygon
          points={`${w * 0.5},${h - fillH} ${w * (0.5 - 0.4 * (fillH / h))},${h} ${w * (0.5 + 0.4 * (fillH / h))},${h}`}
          fill={dimmed ? P.faintGold : P.dimGold}
          opacity={0.4}
        />
        {/* Outline */}
        <polygon
          points={`${w * 0.5},0 ${w * 0.1},${h} ${w * 0.9},${h}`}
          fill="none"
          stroke={dimmed ? P.faintGold : P.gold}
          strokeWidth={1.5}
          opacity={dimmed ? 0.5 : 0.8}
        />
      </svg>
      <div style={{ fontFamily: fontMono, fontSize: 12, color: P.dimWhite, textAlign: "center", maxWidth: 180 }}>
        {reality}
      </div>
    </div>
  );
};

export const V04_DURATION = s(60);

export const V04_GhostGeometry: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/cinematic-night-vigil.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.2], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V04_DURATION - s(4), V04_DURATION], [0.2, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* The problem */}
      <Sequence from={0} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.white, textAlign: "center", lineHeight: 1.8 }}>
            Your context files do one of two things:
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 24, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 12 }}>
            Accumulate outdated information that pollutes context,
            <br />
            or get deleted and lost for good.
          </Fade>
          <Fade delay={55} style={{ fontFamily: fontMono, fontSize: 24, color: P.gold, textAlign: "center", lineHeight: 1.8, marginTop: 12 }}>
            What if there was a third option?
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Three epochs — same tension, desire evolving, larger text */}
      <Sequence from={s(7)} durationInFrames={s(18)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 50px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 15, color: P.faintGold, letterSpacing: "1.5px", marginBottom: 14, textAlign: "center" }}>
            THE SAME PROJECT, THREE MONTHS APART
          </Fade>
          <div style={{ display: "flex", gap: 28, maxWidth: 1350, width: "100%" }}>
            {/* January */}
            <Fade delay={0} style={{ flex: 1 }}>
              <div style={{ fontFamily: fontMono, fontSize: 14, color: P.dimGold, letterSpacing: "2px", marginBottom: 10 }}>JANUARY</div>
              <Panel palette={warm} style={{ padding: "18px 22px" }}>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.gold }}>{"\u25c7"} ship a working MVP</div>
                <div style={{ margin: "10px 0", paddingLeft: 18, borderLeft: `2px solid ${P.faintGold}` }}>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.dimWhite }}>1. validate the idea</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.dimWhite }}>2. build core features</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.dimWhite }}>3. deploy somewhere</div>
                </div>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.dimWhite }}>{"\u25c6"} idea validated, nothing built</div>
              </Panel>
            </Fade>
            {/* February */}
            <Fade delay={30} style={{ flex: 1 }}>
              <div style={{ fontFamily: fontMono, fontSize: 14, color: P.dimGold, letterSpacing: "2px", marginBottom: 10 }}>FEBRUARY</div>
              <Panel palette={warm} style={{ padding: "18px 22px" }}>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.gold }}>{"\u25c7"} get first 10 real users</div>
                <div style={{ margin: "10px 0", paddingLeft: 18, borderLeft: `2px solid ${P.faintGold}` }}>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.green }}>1. {"\u2713"} deploy MVP</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.white }}>2. fix billing (broken)</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.dimWhite }}>3. onboard beta testers</div>
                </div>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.white }}>{"\u25c6"} MVP live, 0 signups, billing broken</div>
              </Panel>
            </Fade>
            {/* Now */}
            <Fade delay={60} style={{ flex: 1 }}>
              <div style={{ fontFamily: fontMono, fontSize: 14, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>NOW</div>
              <Panel palette={warm} highlight style={{ padding: "18px 22px" }}>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.gold }}>{"\u25c7"} product serves paying users</div>
                <div style={{ margin: "10px 0", paddingLeft: 18, borderLeft: `2px solid ${P.gold}` }}>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.green }}>1. {"\u2713"} Stripe handles subs</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.white }}>2. convert 5 testers to paid</div>
                  <div style={{ fontFamily: fontMono, fontSize: 15, color: P.dimWhite }}>3. FAQ replaces docs</div>
                </div>
                <div style={{ fontFamily: fontMono, fontSize: 18, color: P.white }}>{"\u25c6"} 8 users, billing works, 0 revenue</div>
              </Panel>
            </Fade>
          </div>
          <Fade delay={90} style={{ fontFamily: fontMono, fontSize: 16, color: P.dimGold, marginTop: 14, textAlign: "center" }}>
            Same project. The desire sharpened. The strategy changed. Each version learned from the last.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Ghost geometry — the visual */}
      <Sequence from={s(25)} durationInFrames={s(14)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center", marginBottom: 25 }}>
            Ghost geometry.
          </Fade>
          {/* Three pyramids — each representing a delta */}
          <div style={{ display: "flex", gap: 50, alignItems: "flex-end" }}>
            <DeltaPyramid
              desire="ship MVP"
              reality="idea"
              completion={1.0}
              dimmed
              delay={15}
            />
            <DeltaPyramid
              desire="get 10 users"
              reality="MVP live"
              completion={0.6}
              dimmed
              delay={30}
            />
            <DeltaPyramid
              desire="paying users"
              reality="8 free users"
              completion={0.35}
              delay={45}
            />
          </div>
          <Fade delay={70} style={{ fontFamily: fontMono, fontSize: 20, color: P.white, textAlign: "center", lineHeight: 1.8, marginTop: 25 }}>
            Every previous version of your plan is preserved.
            <br />
            Not cluttering your context. Not deleted.
            <br />
            Available when you need to trace how you got here.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Practical payoff */}
      <Sequence from={s(39)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 24, color: P.white, textAlign: "center", lineHeight: 1.8 }}>
            werk preserves the full history and shows it to you directly.
            <br />
            Or diff two git commits of the exported state
            <br />
            and see how your strategy evolved, not just your code.
          </Fade>
          <Fade delay={35} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 20 }}>
            The shape of your progress over time
            <br />
            tells you something no snapshot can.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(48)} durationInFrames={s(12)}>
        <StandardClosing palette={warm} />
      </Sequence>
    </AbsoluteFill>
  );
};
