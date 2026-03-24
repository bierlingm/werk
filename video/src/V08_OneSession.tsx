// Video 8: "One Session" — 65s
// More evocative. Visual progress indicator. The rhythm as felt experience.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { cool, fontMono, Fade, TypeLine, Panel, StandardClosing, s } from "./primitives";

const P = cool;

const PhaseHeader: React.FC<{
  phase: string;
  time: string;
  step: number;
  total: number;
}> = ({ phase, time, step, total }) => {
  const frame = useCurrentFrame();
  const progress = interpolate(frame, [0, 20], [0, step / total], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <div style={{ maxWidth: 1000, width: "100%", marginBottom: 24 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <Fade style={{ fontFamily: fontMono, fontSize: 14, color: P.gold, letterSpacing: "2px" }}>{phase}</Fade>
        <Fade delay={5} style={{ fontFamily: fontMono, fontSize: 14, color: P.faintGold }}>{time}</Fade>
      </div>
      {/* Progress bar */}
      <div style={{ width: "100%", height: 2, backgroundColor: P.faintGold, marginTop: 10, borderRadius: 1 }}>
        <div style={{ width: `${progress * 100}%`, height: "100%", backgroundColor: P.gold, borderRadius: 1 }} />
      </div>
    </div>
  );
};

export const V08_DURATION = s(65);

export const V08_OneSession: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/tense-stay-the-course.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(3)], [0, 0.15], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V08_DURATION - s(4), V08_DURATION], [0.15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={30 * 30} />

      {/* Title */}
      <Sequence from={0} durationInFrames={s(6)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 44, color: P.gold, textAlign: "center" }}>
            One session with werk.
          </Fade>
          <Fade delay={25} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, marginTop: 12 }}>
            Two hours. Five gestures. A structure that evolved.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* OPEN */}
      <Sequence from={s(6)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <PhaseHeader phase="OPEN" time="0:00" step={0} total={5} />
          <Panel palette={cool} style={{ maxWidth: 1000 }}>
            <Fade delay={8}>
              <TypeLine text='$ werk context --json | claude "continue where we left off"' color={P.dimGold} delay={12} fontSize={18} speed={0.5} cursorColor={P.gold} />
            </Fade>
            <Fade delay={50} style={{ color: P.white, fontSize: 18, marginTop: 16, lineHeight: 1.7 }}>
              The agent sees: 3 root tensions. 9 active steps.{"\n"}
              Frontier: Stripe integration. Urgency: 1.2.{"\n"}
              It knows exactly where to start.
            </Fade>
          </Panel>
        </AbsoluteFill>
      </Sequence>

      {/* WORK */}
      <Sequence from={s(15)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <PhaseHeader phase="WORK" time="0:05 \u2013 1:30" step={2} total={5} />
          <Panel palette={cool} style={{ maxWidth: 1000 }}>
            <Fade delay={8}>
              <TypeLine text="$ werk resolve 9" color={P.green} delay={12} fontSize={18} speed={0.6} cursorColor={P.gold} />
            </Fade>
            <Fade delay={35} style={{ color: P.dimGold, fontSize: 16, marginTop: 8 }}>
              {"\u2713"} Stripe integration handles subscriptions
            </Fade>
            <Fade delay={50}>
              <TypeLine text='$ werk note 3 "webhooks need retry logic — edge case in test"' color={P.dimGold} delay={55} fontSize={18} speed={0.4} cursorColor={P.gold} />
            </Fade>
            <Fade delay={85} style={{ color: P.dimWhite, fontSize: 18, marginTop: 16, lineHeight: 1.7 }}>
              Steps resolve. Notes capture learnings.{"\n"}
              The structure mutates through the work itself.
            </Fade>
          </Panel>
        </AbsoluteFill>
      </Sequence>

      {/* LEARN */}
      <Sequence from={s(24)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <PhaseHeader phase="LEARN" time="1:30" step={3} total={5} />
          <Panel palette={cool} style={{ maxWidth: 1000 }}>
            <Fade delay={8}>
              <TypeLine text='$ werk reality 3 "API works. Stripe needs webhook retries. No docs yet."' color={P.dimGold} delay={12} fontSize={18} speed={0.4} cursorColor={P.gold} />
            </Fade>
            <Fade delay={60} style={{ color: P.white, fontSize: 18, marginTop: 16, lineHeight: 1.7 }}>
              The reality update. The most important gesture.{"\n"}
              Compressing what you've learned into the structure.{"\n"}
              <span style={{ color: P.gold }}>Honest. Precise. This is where the instrument earns trust.</span>
            </Fade>
          </Panel>
        </AbsoluteFill>
      </Sequence>

      {/* RESTRUCTURE */}
      <Sequence from={s(33)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <PhaseHeader phase="RESTRUCTURE" time="1:40" step={4} total={5} />
          <Panel palette={cool} style={{ maxWidth: 1000 }}>
            <Fade delay={8}>
              <TypeLine text="$ werk add -p 3 'webhook retry logic handles all failure modes'" color={P.dimGold} delay={12} fontSize={18} speed={0.4} cursorColor={P.gold} />
            </Fade>
            <Fade delay={55} style={{ color: P.white, fontSize: 18, marginTop: 16, lineHeight: 1.7 }}>
              New step added. The theory of closure evolves.{"\n"}
              Not because the plan failed — because you learned.{"\n"}
              The structure adapts to what you now know.
            </Fade>
          </Panel>
        </AbsoluteFill>
      </Sequence>

      {/* CLOSE */}
      <Sequence from={s(42)} durationInFrames={s(9)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <PhaseHeader phase="CLOSE" time="1:55" step={5} total={5} />
          <Panel palette={cool} style={{ maxWidth: 1000 }}>
            <Fade delay={8}>
              <TypeLine text="$ werk tree && git add .werk/ && git commit" color={P.dimGold} delay={12} fontSize={18} speed={0.5} cursorColor={P.gold} />
            </Fade>
            <Fade delay={50} style={{ color: P.white, fontSize: 18, marginTop: 16, lineHeight: 1.7 }}>
              The structural snapshot commits alongside the code.{"\n"}
              Tomorrow's diff shows how your strategy evolved.{"\n"}
              Not just what files changed — what you <em>learned</em>.
            </Fade>
          </Panel>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(51)} durationInFrames={s(14)}>
        <StandardClosing palette={cool} preLines={[
          "Open. Work. Learn. Restructure. Close.",
          "Using it is the practice.",
        ]} />
      </Sequence>
    </AbsoluteFill>
  );
};
