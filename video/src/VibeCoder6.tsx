// "What do you want this to do?"  "..."
// 65s. The root cause beneath every symptom.
// Not a pain point video. A revelation.
// Confusion → Recognition → "oh..." → Clarity → Desire

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
import { dark, fontMono, Fade, Panel, RevealLine, TypeLine, Divider, s } from "./primitives";

const P = dark;

// ─── Scenes ─────────────────────────────────────────────────────────

// Scene 1: The AI asks. You can't answer.
const SceneQuestion: React.FC = () => {
  const frame = useCurrentFrame();

  const dotsOp = interpolate(frame, [85, 100], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <div style={{ maxWidth: 800 }}>
        <TypeLine
          text="What do you want this to do?"
          color={P.blue}
          delay={15}
          fontSize={36}
          speed={1.2}
          cursorColor={P.blue}
        />
        <div style={{
          fontFamily: fontMono, fontSize: 36, color: P.faintWhite,
          marginTop: 20, opacity: dotsOp,
        }}>
          ...
        </div>
      </div>
    </AbsoluteFill>
  );
};

// Scene 2: Inner monologue. Not a chat. A mind struggling to articulate.
const SceneMonologue: React.FC = () => {
  const frame = useCurrentFrame();

  const lines = [
    { text: "I want it to... work.", delay: 15, color: P.white, size: 28 },
    { text: "Like, users should be able to...", delay: 75, color: P.dimWhite, size: 26 },
    { text: "It's a thing that lets people...", delay: 135, color: P.dimWhite, size: 26 },
    { text: "OK honestly I'm not sure yet.", delay: 195, color: P.gold, size: 30 },
    { text: "I just started building.", delay: 250, color: P.faintWhite, size: 24 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
      {lines.map((line, i) => {
        const op = interpolate(frame, [line.delay, line.delay + 18], [0, 1], {
          extrapolateLeft: "clamp", extrapolateRight: "clamp",
        });
        // Each line fully disappears when the next appears
        const nextDelay = lines[i + 1]?.delay ?? 999;
        const dim = interpolate(frame, [nextDelay - 5, nextDelay + 5], [1, 0], {
          extrapolateLeft: "clamp", extrapolateRight: "clamp",
        });

        return (
          <div key={i} style={{
            position: "absolute",
            fontFamily: fontMono,
            fontSize: line.size,
            color: line.color,
            textAlign: "center",
            opacity: op * dim,
          }}>
            {line.text}
          </div>
        );
      })}
    </AbsoluteFill>
  );
};

// Scene 3: This is normal. Two lines. Slow.
const SceneNormal: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
    <div style={{ display: "flex", flexDirection: "column", gap: 32, alignItems: "center" }}>
      <Fade style={{
        display: "flex", alignItems: "flex-start", gap: 16,
      }}>
        <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 3 }}>{"\u25c7"}</span>
        <span style={{ fontFamily: fontMono, fontSize: 28, color: P.white, lineHeight: 1.5 }}>
          Nobody writes down what "done" looks like.
        </span>
      </Fade>
      <Fade delay={90} style={{
        display: "flex", alignItems: "flex-start", gap: 16,
      }}>
        <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 3 }}>{"\u25c7"}</span>
        <span style={{ fontFamily: fontMono, fontSize: 32, color: P.gold, lineHeight: 1.5 }}>
          We just start building and hope it converges.
        </span>
      </Fade>
    </div>
  </AbsoluteFill>
);

// Scene 4: The root. One line. Lots of space.
const SceneRoot: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade delay={30} fadeIn={22} style={{
      fontFamily: fontMono, fontSize: 42, color: P.gold,
      textAlign: "center", letterSpacing: "0.02em",
    }}>
      You don't know what "done" looks like.
    </Fade>
  </AbsoluteFill>
);

// Scene 5: The cascade — every symptom traces here.
const SceneCascade: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const consequences = [
    { text: "So your project has no target to converge on.", delay: 35 },
    { text: "So there's nothing stable to orient to between sessions.", delay: 80 },
    { text: "So speed just creates more confusion.", delay: 125 },
    { text: "So you start something new instead of finishing.", delay: 170 },
    { text: "So your AI optimizes for activity, not outcome.", delay: 215 },
  ];

  // Vertical line grows
  const lastDelay = consequences[consequences.length - 1].delay;
  const lineH = interpolate(frame, [20, lastDelay + 30], [0, 260], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // After all shown, root re-brightens
  const rootBright = interpolate(frame, [lastDelay + 40, lastDelay + 60], [0.4, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "flex-start", padding: "120px 160px", opacity: fadeOut }}>
      {/* Root line */}
      <div style={{
        fontFamily: fontMono, fontSize: 20, color: P.gold,
        opacity: rootBright, marginBottom: 16,
      }}>
        You don't know what "done" looks like.
      </div>

      {/* Vertical connector line */}
      <div style={{ display: "flex", gap: 20 }}>
        <div style={{
          width: 2, height: lineH, backgroundColor: P.faintGold, opacity: 0.5,
          flexShrink: 0, marginLeft: 8, marginTop: 4,
        }} />

        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {consequences.map((c, i) => {
            const op = interpolate(frame, [c.delay, c.delay + 15], [0, 1], {
              extrapolateLeft: "clamp", extrapolateRight: "clamp",
            });
            const isLast = i === consequences.length - 1;
            return (
              <div key={i} style={{
                fontFamily: fontMono,
                fontSize: 22,
                color: isLast ? P.red : P.white,
                opacity: op,
                lineHeight: 1.5,
              }}>
                {"\u2192"} {c.text}
              </div>
            );
          })}
        </div>
      </div>
    </AbsoluteFill>
  );
};

// Scene 6: The flip. A single panel builds from empty.
const SceneFlip: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const hereOp = frame > 155 ? 0.7 + Math.sin((frame - 155) * 0.1) * 0.3 : 0;
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%" }}>
        <Panel highlight style={{ padding: "24px 32px" }}>
          {/* Desire types in */}
          <TypeLine
            text={"\u25c7 People find recipes, save them, and cook from my app"}
            color={P.gold}
            delay={8}
            fontSize={20}
            speed={0.6}
            cursorColor={P.gold}
          />

          {/* Steps materialize */}
          <div style={{ margin: "14px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
            <RevealLine text={"  5. share recipes with friends"} color={P.dimWhite} delay={95} />
            <RevealLine text={"  4. cooking mode with timers"} color={P.dimWhite} delay={103} />
            <RevealLine text={"  3. user accounts save across devices"} color={P.white} delay={111} />
            <RevealLine text={"  \u2713 2. save flow remembers favorites"} color={P.green} delay={119} />
            <RevealLine text={"  \u2713 1. search returns good results"} color={P.green} delay={127} />
          </div>

          {/* "you are here" */}
          {frame > 135 && (
            <div style={{
              fontFamily: fontMono, fontSize: 14, color: P.gold,
              paddingLeft: 36, marginTop: -8, marginBottom: 8, opacity: hereOp,
            }}>
              {"\u2190"} you are here (step 3)
            </div>
          )}

          {/* Reality types in */}
          <TypeLine
            text={"\u25c6 Search works. Save works. Nothing else yet."}
            color={P.dimWhite}
            delay={60}
            fontSize={20}
            speed={0.6}
            cursorColor={P.dimGold}
          />
        </Panel>

        <Fade delay={155} style={{
          fontFamily: fontMono, fontSize: 22, color: P.dimGold,
          textAlign: "center", marginTop: 20, lineHeight: 1.8,
        }}>
          The steps were always there.
          <br />
          You just hadn't named the ends.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// Scene 7: What changes — three before/after pairs.
const SceneWhatChanges: React.FC = () => {
  const frame = useCurrentFrame();

  const pairs = [
    {
      without: '"Build me a recipe app"',
      with_: '"Next step is user accounts. Search and save are done."',
      delay: 8,
    },
    {
      without: "A new chat window, starting from zero.",
      with_: "A map your AI reads before writing a single line.",
      delay: 78,
    },
    {
      without: "Five projects, all half-built.",
      with_: "Five projects, each with a clear next step.",
      delay: 148,
    },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 30, maxWidth: 900 }}>
        {pairs.map((pair, i) => {
          const op = interpolate(frame, [pair.delay, pair.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const nextDelay = pairs[i + 1]?.delay ?? 999;
          const dimmed = frame > nextDelay + 5 && frame < 195;

          return (
            <div key={i} style={{ opacity: op * (dimmed ? 0.35 : 1) }}>
              <div style={{ fontFamily: fontMono, fontSize: 17, color: P.faintWhite, lineHeight: 1.6 }}>
                Without:  {pair.without}
              </div>
              <div style={{ fontFamily: fontMono, fontSize: 20, color: P.white, lineHeight: 1.6, marginTop: 4 }}>
                With:  {pair.with_}
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

// Scene 8: The principle — text that pushes up into the closing.
const ScenePrincipleAndClose: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // Principle appears, holds, then pushes up
  const principleOp = interpolate(frame, [10, 28], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const pushStart = 130;
  const pushEnd = 165;
  const principleY = interpolate(frame, [pushStart, pushEnd], [0, -350], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const principleFade = interpolate(frame, [pushStart, pushEnd], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Glyph + name appear after push
  const glyphScale = spring({ frame: frame - pushEnd - 5, fps, config: { damping: 25, stiffness: 60 } });
  const glyphOp = interpolate(frame, [pushEnd + 5, pushEnd + 23], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const nameOp = interpolate(frame, [pushEnd + 25, pushEnd + 43], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const nameY = interpolate(frame, [pushEnd + 25, pushEnd + 43], [15, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const tagOp = interpolate(frame, [pushEnd + 60, pushEnd + 78], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", overflow: "hidden" }}>
      {/* Principle text — pushes up */}
      <div style={{
        position: "absolute",
        transform: `translateY(${principleY}px)`,
        opacity: principleOp * principleFade,
        textAlign: "center",
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 28, color: P.white, lineHeight: 1.8 }}>
          Name what you want. Name where you are.
          <br />
          The path appears.
        </div>
      </div>

      {/* Glyph + name — after push */}
      <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>{"\u25c7"}</div>
      <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em", marginTop: 8, opacity: nameOp, transform: `translateY(${nameY}px)` }}>werk</div>

      {/* Tagline */}
      <div style={{ position: "absolute", bottom: 180, opacity: tagOp, textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, lineHeight: 1.8 }}>
          Know what done looks like.
          <br />
          Everything else follows.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER6_DURATION = s(65);

export const VibeCoder6: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/cinematic-night-vigil.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(6)], [0, 0.12], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER6_DURATION - s(6), VIBECODER6_DURATION], [0.12, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={90 * 30} />

      <Sequence from={s(0)} durationInFrames={s(5)}><SceneQuestion /></Sequence>
      <Sequence from={s(5)} durationInFrames={s(10)}><SceneMonologue /></Sequence>
      <Sequence from={s(15)} durationInFrames={s(8)}><SceneNormal /></Sequence>
      <Sequence from={s(23)} durationInFrames={s(6)}><SceneRoot /></Sequence>
      <Sequence from={s(29)} durationInFrames={s(10)}><SceneCascade /></Sequence>
      <Sequence from={s(39)} durationInFrames={s(7)}><SceneFlip /></Sequence>
      <Sequence from={s(46)} durationInFrames={s(7)}><SceneWhatChanges /></Sequence>
      <Sequence from={s(53)} durationInFrames={s(12)}><ScenePrincipleAndClose /></Sequence>
    </AbsoluteFill>
  );
};
