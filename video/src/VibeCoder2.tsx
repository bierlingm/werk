// "Monday morning. New chat window. Where were we?"
// 65s. The context reset problem. Every session starts from zero.
// Recognition → Tedium → Pattern → Reframe → Relief → Desire

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

// ─── Custom Components ──────────────────────────────────────────────

const ChatLine: React.FC<{
  role: "user" | "ai";
  text: string;
  delay: number;
}> = ({ role, text, delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const isUser = role === "user";
  return (
    <div style={{
      opacity: op,
      fontFamily: fontMono,
      fontSize: 17,
      color: isUser ? P.gold : P.dimWhite,
      lineHeight: 1.6,
      marginBottom: 2,
    }}>
      {isUser ? "> " : "  "}{text}
    </div>
  );
};

const TokenCounter: React.FC<{ delay: number }> = ({ delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const count = Math.floor(interpolate(
    frame,
    [delay, delay + 200],
    [0, 8],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  ));
  const color = count > 5 ? P.red : count > 3 ? P.dimGold : P.faintGold;

  return (
    <div style={{
      opacity: op,
      fontFamily: fontMono,
      fontSize: 13,
      color,
      position: "absolute",
      bottom: 12,
      right: 20,
    }}>
      ~{count},000 tokens
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneHook: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 44, color: P.white, textAlign: "center" }}>
      Monday morning. New chat window.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center", marginTop: 12 }}>
      Where were we?
    </Fade>
  </AbsoluteFill>
);

const SceneReExplanation: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%", position: "relative" }}>
        <Panel style={{ padding: "28px 36px", position: "relative" }}>
          <ChatLine role="user" text="So I'm building a recipe app with AI..." delay={0} />
          <ChatLine role="ai" text="I'd love to help! What features are you thinking?" delay={18} />
          <ChatLine role="user" text="We already built the search and save flow last week" delay={38} />
          <ChatLine role="ai" text="Oh! Can you share what you have so far?" delay={56} />
          <ChatLine role="user" text="[pastes 40 lines of code]" delay={76} />
          <ChatLine role="ai" text="I see. And what's the database schema?" delay={94} />
          <ChatLine role="user" text="[pastes another 30 lines]" delay={114} />
          <ChatLine role="ai" text="OK, what were you working on when you stopped?" delay={132} />
          <ChatLine role="user" text="...I don't remember exactly" delay={160} />
          <TokenCounter delay={30} />
        </Panel>
        <Fade delay={190} style={{
          fontFamily: fontMono,
          fontSize: 24,
          color: P.red,
          textAlign: "center",
          marginTop: 20,
        }}>
          You haven't even started working yet.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneGroundhogDay: React.FC = () => {
  const frame = useCurrentFrame();

  const days = [
    { text: 'Tuesday. "Let me catch you up on my recipe app..."', delay: 10 },
    { text: 'Thursday. "OK so here\'s what we built so far..."', delay: 70 },
    { text: 'Next Monday. "I\'m building a recipe app with AI..."', delay: 130 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        {days.map((day, i) => {
          const op = interpolate(frame, [day.delay, day.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          // Dim previous when next appears
          const nextDelay = days[i + 1]?.delay ?? 999;
          const dimmed = frame > nextDelay + 5 && frame < 190;

          return (
            <div key={i} style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 16,
              opacity: op * (dimmed ? 0.3 : 1),
            }}>
              <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 2 }}>
                {"\u25c7"}
              </span>
              <span style={{
                fontFamily: fontMono,
                fontSize: 24,
                color: i === 2 ? P.red : P.white,
                lineHeight: 1.5,
              }}>
                {day.text}
              </span>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const ScenePivot: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.dimWhite, textAlign: "center" }}>
      Your AI doesn't forget because it's bad.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 34, color: P.gold, textAlign: "center", marginTop: 15 }}>
      It forgets because nothing remembers for it.
    </Fade>
  </AbsoluteFill>
);

const SceneBrandReveal: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  const glyphScale = spring({ frame: frame - 5, fps, config: { damping: 25, stiffness: 60 } });
  const glyphOp = interpolate(frame, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: fadeOut }}>
      <div style={{ fontFamily: fontMono, fontSize: 100, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>
        {"\u25c7"}
      </div>
      <Fade delay={18} style={{ fontFamily: fontMono, fontSize: 56, color: P.white, letterSpacing: "0.15em", marginTop: 5 }}>
        werk
      </Fade>
      <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimGold, marginTop: 15 }}>
        It remembers so you don't have to.
      </Fade>
    </AbsoluteFill>
  );
};

const SceneDemo: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // "you are here" pulse
  const hereOp = frame > 80
    ? 0.7 + Math.sin((frame - 80) * 0.1) * 0.3
    : 0;

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 950, width: "100%" }}>
        {/* Werk structure — what it already knows */}
        <Fade style={{ marginBottom: 20 }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>
            MONDAY MORNING — WERK ALREADY KNOWS:
          </div>
          <Panel highlight style={{ padding: "20px 28px" }}>
            <RevealLine text={"\u25c7 People discover and save recipes they love"} color={P.gold} delay={8} />
            <div style={{ margin: "10px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
              <RevealLine text={"  5. share collections with friends"} color={P.dimWhite} delay={40} />
              <RevealLine text={"  4. personalized recommendations"} color={P.dimWhite} delay={34} />
              <RevealLine text={"  3. saved recipes sync across devices     \u2190 next"} color={P.white} delay={28} />
              <RevealLine text={"  \u2713 2. search returns relevant results"} color={P.green} delay={22} />
              <RevealLine text={"  \u2713 1. core recipe database populated"} color={P.green} delay={16} />
            </div>
            <RevealLine text={"\u25c6 Search works. Save flow built. Sync not started."} color={P.dimWhite} delay={48} />
          </Panel>
        </Fade>

        {/* AI response — it already knows everything */}
        <Fade delay={80}>
          <Panel style={{ padding: "20px 28px", borderColor: P.blue }}>
            <TypeLine
              text="I can see you're working on recipe sync."
              color={P.white}
              delay={95}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="Search and save are done — I won't touch those."
              color={P.white}
              delay={125}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="I'll scaffold the sync architecture first."
              color={P.dimGold}
              delay={155}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="Ready to begin?"
              color={P.gold}
              delay={180}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
          </Panel>
        </Fade>

        {/* Caption */}
        <Fade delay={220} style={{
          fontFamily: fontMono,
          fontSize: 24,
          color: P.gold,
          textAlign: "center",
          marginTop: 20,
        }}>
          No pasting. No catching up. Just building.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreeProps: React.FC = () => {
  const frame = useCurrentFrame();

  const props = [
    { text: "Every session starts with full context.", delay: 8 },
    { text: "Your AI knows what's done, what's next, and why.", delay: 55 },
    { text: "You never re-explain your project again.", delay: 100 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        {props.map((prop, i) => {
          const op = interpolate(frame, [prop.delay, prop.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          return (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 16, opacity: op }}>
              <span style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, flexShrink: 0 }}>{"\u25c7"}</span>
              <span style={{ fontFamily: fontMono, fontSize: 24, color: P.white }}>{prop.text}</span>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const SceneContrast: React.FC = () => {
  const frame = useCurrentFrame();
  const line1Dim = interpolate(frame, [45, 60], [1, 0.3], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <Fade style={{
        fontFamily: fontMono,
        fontSize: 24,
        color: P.dimWhite,
        textAlign: "center",
        opacity: line1Dim,
      }}>
        Other tools: start every session from scratch.
      </Fade>
      <Fade delay={40} style={{
        fontFamily: fontMono,
        fontSize: 30,
        color: P.gold,
        textAlign: "center",
        marginTop: 15,
      }}>
        werk: start every session from where you left off.
      </Fade>
    </AbsoluteFill>
  );
};

// Closing — no fade out, freezes on last frame
const SceneClosing: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const glyphScale = spring({ frame: frame - 5, fps, config: { damping: 25, stiffness: 60 } });
  const glyphOp = interpolate(frame, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nameOp = interpolate(frame, [20, 38], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const nameY = interpolate(frame, [20, 38], [15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const tagOp = interpolate(frame, [55, 73], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
      <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>
        {"\u25c7"}
      </div>
      <div style={{
        fontFamily: fontMono,
        fontSize: 48,
        color: P.white,
        letterSpacing: "0.15em",
        marginTop: 8,
        opacity: nameOp,
        transform: `translateY(${nameY}px)`,
      }}>
        werk
      </div>
      <div style={{
        position: "absolute",
        bottom: 180,
        opacity: tagOp,
        textAlign: "center",
      }}>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, lineHeight: 1.8 }}>
          Stop re-explaining.
          <br />
          Start where you left off.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER2_DURATION = s(65);

export const VibeCoder2: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/minimal-outer-thoughts.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.14], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER2_DURATION - s(5), VIBECODER2_DURATION], [0.14, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* 1. Hook — 4s */}
      <Sequence from={0} durationInFrames={s(4)}>
        <SceneHook />
      </Sequence>

      {/* 2. The re-explanation — 11s */}
      <Sequence from={s(4)} durationInFrames={s(11)}>
        <SceneReExplanation />
      </Sequence>

      {/* 3. Groundhog Day — 8s */}
      <Sequence from={s(15)} durationInFrames={s(8)}>
        <SceneGroundhogDay />
      </Sequence>

      {/* 4. The reframe — 5s */}
      <Sequence from={s(23)} durationInFrames={s(5)}>
        <ScenePivot />
      </Sequence>

      {/* 5. Brand reveal — 4s */}
      <Sequence from={s(28)} durationInFrames={s(4)}>
        <SceneBrandReveal />
      </Sequence>

      {/* 6. Demo — what Monday looks like now — 15s */}
      <Sequence from={s(32)} durationInFrames={s(15)}>
        <SceneDemo />
      </Sequence>

      {/* 7. Three properties — 6s */}
      <Sequence from={s(47)} durationInFrames={s(6)}>
        <SceneThreeProps />
      </Sequence>

      {/* 8. Contrast beat — 5s */}
      <Sequence from={s(53)} durationInFrames={s(5)}>
        <SceneContrast />
      </Sequence>

      {/* 9. Closing — 7s, no fade out */}
      <Sequence from={s(58)} durationInFrames={s(7)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
