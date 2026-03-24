// Video 10: "The Operative Instrument" — 65s
// Four concepts, clear for uninitiated. No Adhesion.
// Outro: four summary lines push up and out, then closing screen.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { warm, fontMono, Fade, Divider, s } from "./primitives";

const P = warm;

const Concept: React.FC<{
  term: string;
  plain: string;
  example: string;
}> = ({ term, plain, example }) => {
  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ maxWidth: 900 }}>
        <Fade style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center" }}>
          {term}
        </Fade>
        <Fade delay={15} style={{ fontFamily: fontMono, fontSize: 24, color: P.white, textAlign: "center", lineHeight: 1.8, marginTop: 20 }}>
          {plain}
        </Fade>
        <Fade delay={45} style={{ display: "flex", justifyContent: "center" }}>
          <Divider color={P.faintGold} delay={45} />
        </Fade>
        <Fade delay={55} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, textAlign: "center", lineHeight: 1.8 }}>
          {example}
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// Closing scene with push-up transition
const ClosingScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  // Four summary lines visible at start, then push upward
  const summaryLines = [
    "Tension generates energy.",
    "Your theory channels it.",
    "Gestures enact it.",
    "Silence protects your attention.",
  ];

  // Lines push upward starting at frame 90
  const pushStart = 90;
  const pushEnd = 130;
  const pushY = interpolate(frame, [pushStart, pushEnd], [0, -400], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const summaryOpacity = interpolate(frame, [pushStart, pushEnd], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Closing elements fade in after push
  const closingOpacity = interpolate(frame, [pushEnd, pushEnd + 20], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const closingY = interpolate(frame, [pushEnd, pushEnd + 25], [30, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const taglineOpacity = interpolate(frame, [pushEnd + 40, pushEnd + 55], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", overflow: "hidden" }}>
      {/* Summary lines — push upward */}
      <div style={{
        position: "absolute",
        transform: `translateY(${pushY}px)`,
        opacity: summaryOpacity,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
      }}>
        {summaryLines.map((line, i) => {
          const lineOp = interpolate(frame, [i * 12, i * 12 + 15], [0, 1], {
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

      {/* Closing: glyph, name, tagline — fade in after push */}
      <div style={{
        opacity: closingOpacity,
        transform: `translateY(${closingY}px)`,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold }}>{"\u25c7"}</div>
        <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em" }}>werk</div>
      </div>

      <div style={{
        position: "absolute",
        bottom: 180,
        opacity: taglineOpacity,
        textAlign: "center",
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, lineHeight: 1.6 }}>
          Structure determines behavior.
          <br />
          Build the structure that determines yours.
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const V10_DURATION = s(65);

export const V10_TheOperativeInstrument: React.FC = () => {
  const musicVolume = (f: number) => {
    const fi = interpolate(f, [0, s(5)], [0, 0.22], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
    const fo = interpolate(f, [V10_DURATION - s(5), V10_DURATION], [0.22, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
    return Math.min(fi, fo);
  };

  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/emotional-ancient-rite.mp3")} volume={musicVolume} />

      {/* Opening */}
      <Sequence from={0} durationInFrames={s(7)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 26, color: P.dimWhite, textAlign: "center", lineHeight: 1.8 }}>
            Most tools show you information and wait for instructions.
          </Fade>
          <Fade delay={30} style={{ fontFamily: fontMono, fontSize: 30, color: P.gold, textAlign: "center", marginTop: 15, lineHeight: 1.5 }}>
            werk is different: the structure of the tool
            <br />
            shapes how you see your work and what you do next.
          </Fade>
          <Fade delay={65} style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, textAlign: "center", marginTop: 15 }}>
            Four ideas make this possible.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      <Sequence from={s(7)} durationInFrames={s(10)}>
        <Concept
          term="Structural Tension"
          plain={"The gap between what you want and where you are.\nThat gap is what drives action \u2014 not motivation, not discipline."}
          example="Every tension in werk has a desired outcome and a current reality. The distance between them is the engine."
        />
      </Sequence>

      <Sequence from={s(17)} durationInFrames={s(10)}>
        <Concept
          term="Theory of Closure"
          plain={"Your steps aren't a task list. They're your best guess\nat how to get from here to there. They might be wrong."}
          example="When you learn something new, you restructure. That restructuring is a first-class event, not a cleanup chore."
        />
      </Sequence>

      <Sequence from={s(27)} durationInFrames={s(10)}>
        <Concept
          term="Gesture"
          plain={"Every meaningful action is a gesture:\nresolving a step, updating reality, evolving your aim."}
          example="The instrument records what you meant, not what you typed. Meaning, not mechanics."
        />
      </Sequence>

      <Sequence from={s(37)} durationInFrames={s(10)}>
        <Concept
          term="Signal by Exception"
          plain={"When everything is on track, the instrument is silent.\nSignals appear only when something needs your attention."}
          example="No dashboards. No notification badges. No noise. Only what matters, where it matters."
        />
      </Sequence>

      {/* Closing with push-up transition */}
      <Sequence from={s(47)} durationInFrames={s(18)}>
        <ClosingScene />
      </Sequence>
    </AbsoluteFill>
  );
};
