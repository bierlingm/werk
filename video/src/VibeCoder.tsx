// "You're building something with AI. It's actually working."
// 65s. For people who vibe code. Zero jargon. Pure emotional arc.
// Recognition → Pain → Reframe → Relief → Desire

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

const FileCounter: React.FC<{ delay: number }> = ({ delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const count = Math.floor(interpolate(
    frame,
    [delay, delay + 150],
    [12, 247],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  ));
  const color = count > 150 ? P.red : count > 80 ? P.dimGold : P.faintGold;

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
      {count} files changed
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneHook: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 48, color: P.white, textAlign: "center" }}>
      You're building something with AI.
    </Fade>
    <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center", marginTop: 10 }}>
      It's actually working.
    </Fade>
  </AbsoluteFill>
);

const SceneSpiral: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%", position: "relative" }}>
        <Panel style={{ padding: "28px 36px", position: "relative" }}>
          <ChatLine role="user" text="Build me a landing page" delay={0} />
          <ChatLine role="ai" text="Done! Created index.html with hero section" delay={12} />
          <ChatLine role="user" text="Add a payment form" delay={28} />
          <ChatLine role="ai" text="Added Stripe checkout flow" delay={40} />
          <ChatLine role="user" text="Actually, make it subscriptions" delay={56} />
          <ChatLine role="ai" text="Refactored to recurring billing" delay={68} />
          <ChatLine role="user" text="The signup form broke" delay={84} />
          <ChatLine role="ai" text="Fixed! Also updated the database schema" delay={96} />
          <ChatLine role="user" text="Wait, what database?" delay={116} />
          <ChatLine role="ai" text="I added PostgreSQL for user management" delay={128} />
          <ChatLine role="user" text="I just wanted a landing page" delay={148} />
          <FileCounter delay={40} />
        </Panel>
        <Fade delay={170} style={{
          fontFamily: fontMono,
          fontSize: 24,
          color: P.red,
          textAlign: "center",
          marginTop: 20,
        }}>
          Sound familiar?
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreePains: React.FC = () => {
  const frame = useCurrentFrame();

  const pains = [
    { text: "Your project grew faster than your understanding of it.", delay: 10 },
    { text: "Your AI doesn't remember what matters.", delay: 65 },
    { text: "You can't tell what's done, what's left, or what changed.", delay: 120 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        {pains.map((pain, i) => {
          const op = interpolate(frame, [pain.delay, pain.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          // Dim previous items when new one appears
          const nextDelay = pains[i + 1]?.delay ?? 999;
          const brighten = interpolate(frame, [175, 190], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const dimmed = frame > nextDelay + 5 && frame < 175;
          const finalOp = op * (dimmed ? 0.35 : 1);

          return (
            <div key={i} style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 16,
              opacity: finalOp,
            }}>
              <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 2 }}>
                {"\u25c7"}
              </span>
              <span style={{ fontFamily: fontMono, fontSize: 26, color: P.white, lineHeight: 1.5 }}>
                {pain.text}
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
    <Fade style={{ fontFamily: fontMono, fontSize: 30, color: P.dimWhite, textAlign: "center" }}>
      The problem isn't the AI.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center", marginTop: 15 }}>
      Nobody is holding the big picture.
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
        A living map of what you're building and why.
      </Fade>
    </AbsoluteFill>
  );
};

const SceneBeforeAfter: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  // Phase A: chaos panel (0-180), then shrinks left
  const shrinkStart = 170;
  const shrinkEnd = 210;
  const chaosX = interpolate(frame, [shrinkStart, shrinkEnd], [0, -320], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const chaosScale = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.72], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const chaosDim = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.35], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Phase B: werk panel slides in from right
  const werkOp = interpolate(frame, [shrinkEnd, shrinkEnd + 20], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const werkX = interpolate(frame, [shrinkEnd, shrinkEnd + 25], [80, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Caption
  const captionOp = interpolate(frame, [shrinkEnd + 120, shrinkEnd + 140], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // Fade out
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  // "you are here" pulse
  const hereOp = frame > shrinkEnd + 60
    ? 0.7 + Math.sin((frame - shrinkEnd - 60) * 0.1) * 0.3
    : 0;

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 50px", opacity: fadeOut }}>
      <div style={{ display: "flex", gap: 30, maxWidth: 1350, width: "100%", alignItems: "flex-start" }}>
        {/* Chaos panel */}
        <div style={{
          flex: 1,
          transform: `translateX(${chaosX}px) scale(${chaosScale})`,
          opacity: chaosDim,
          transformOrigin: "center center",
        }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.faintGold, letterSpacing: "2px", marginBottom: 10 }}>
            WITHOUT
          </div>
          <Panel style={{ padding: "20px 28px" }}>
            {[
              { text: "- landing page (done?)", color: P.faintGold },
              { text: "- fix signup form", color: P.white },
              { text: "- Stripe integration", color: P.white },
              { text: '- "something about a database"', color: P.dimWhite },
              { text: "- write tests maybe?", color: P.faintWhite },
              { text: "- subscription model", color: P.white },
              { text: "- the onboarding thing", color: P.faintWhite },
              { text: "- wait is the API done?", color: P.faintWhite },
              { text: "- launch???", color: P.red },
            ].map((item, i) => (
              <RevealLine key={i} text={item.text} color={item.color} delay={8 + i * 6} />
            ))}
          </Panel>
        </div>

        {/* Werk panel */}
        <div style={{
          flex: 1,
          opacity: werkOp,
          transform: `translateX(${werkX}px)`,
        }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>
            WITH WERK
          </div>
          <Panel highlight style={{ padding: "20px 28px" }}>
            <div style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, lineHeight: 1.5 }}>
              {"\u25c7"} People pay monthly to use my product
            </div>
            <div style={{ margin: "12px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
              {[
                { n: "5.", text: "launch to first 10 users", done: false, current: false, color: P.dimWhite },
                { n: "4.", text: "users can sign up without help", done: false, current: false, color: P.dimWhite },
                { n: "3.", text: "subscription billing works", done: false, current: true, color: P.white },
                { n: "2.", text: "payment form processes cards", done: true, current: false, color: P.green },
                { n: "1.", text: "landing page converts visitors", done: true, current: false, color: P.green },
              ].map((step, i) => {
                const stepOp = interpolate(frame, [shrinkEnd + 25 + i * 10, shrinkEnd + 35 + i * 10], [0, 1], {
                  extrapolateLeft: "clamp", extrapolateRight: "clamp",
                });
                return (
                  <div key={i} style={{
                    fontFamily: fontMono,
                    fontSize: 16,
                    color: step.color,
                    lineHeight: 1.7,
                    opacity: stepOp,
                  }}>
                    {step.done ? "\u2713" : " "} {step.n} {step.text}
                    {step.current && <span style={{ color: P.gold, opacity: hereOp }}> {"\u2190"} you are here</span>}
                  </div>
                );
              })}
            </div>
            <Fade delay={shrinkEnd + 85}>
              <div style={{ fontFamily: fontMono, fontSize: 18, color: P.white, lineHeight: 1.5, marginTop: 4 }}>
                {"\u25c6"} Landing page live, payments half-built, no users yet
              </div>
            </Fade>
          </Panel>
        </div>
      </div>

      {/* Caption */}
      <div style={{
        fontFamily: fontMono,
        fontSize: 24,
        color: P.gold,
        textAlign: "center",
        marginTop: 24,
        opacity: captionOp,
      }}>
        Same project. Now you can see it.
      </div>
    </AbsoluteFill>
  );
};

const SceneAgentInsight: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%" }}>
        {/* Compact werk context */}
        <Fade style={{ marginBottom: 20 }}>
          <Panel highlight style={{ padding: "16px 28px" }}>
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.gold }}>{"\u25c7"} People pay monthly to use my product</div>
            <div style={{ fontFamily: fontMono, fontSize: 15, color: P.white, paddingLeft: 16, marginTop: 4 }}>
              {"\u2192"} next: subscription billing works
            </div>
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, marginTop: 4 }}>{"\u25c6"} Payments half-built, no users yet</div>
          </Panel>
        </Fade>

        {/* AI response */}
        <Fade delay={20}>
          <Panel style={{ padding: "20px 28px", borderColor: P.blue }}>
            <TypeLine
              text="I see your next step is subscription billing."
              color={P.white}
              delay={35}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="Payments are half-built, so I'll extend that"
              color={P.white}
              delay={70}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="code rather than starting over."
              color={P.white}
              delay={95}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
            <TypeLine
              text="I won't touch the landing page — that's done."
              color={P.dimGold}
              delay={120}
              fontSize={18}
              speed={0.5}
              cursorColor={P.blue}
            />
          </Panel>
        </Fade>

        <Fade delay={155} style={{
          fontFamily: fontMono,
          fontSize: 20,
          color: P.dimGold,
          textAlign: "center",
          marginTop: 20,
        }}>
          Your AI builds the right thing because it knows what matters.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreeProperties: React.FC = () => {
  const frame = useCurrentFrame();

  const props = [
    { text: "It updates as you work. Not when you remember to.", delay: 8 },
    { text: "Your AI reads it at the start of every session.", delay: 50 },
    { text: "You always know where you stand.", delay: 92 },
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

// Closing — no fade out, freeze on last frame for X autoplay
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
          Know what you're building.
          <br />
          Know what's next.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER_DURATION = s(65);

export const VibeCoder: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/tense-stay-the-course.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.14], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER_DURATION - s(5), VIBECODER_DURATION], [0.14, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={20 * 30} />

      {/* 1. Hook — 5s */}
      <Sequence from={0} durationInFrames={s(5)}>
        <SceneHook />
      </Sequence>

      {/* 2. The Spiral — 9s */}
      <Sequence from={s(5)} durationInFrames={s(9)}>
        <SceneSpiral />
      </Sequence>

      {/* 3. Three Pains — 8s */}
      <Sequence from={s(14)} durationInFrames={s(8)}>
        <SceneThreePains />
      </Sequence>

      {/* 4. The Pivot — 5s */}
      <Sequence from={s(22)} durationInFrames={s(5)}>
        <ScenePivot />
      </Sequence>

      {/* 5. Brand Reveal — 5s */}
      <Sequence from={s(27)} durationInFrames={s(5)}>
        <SceneBrandReveal />
      </Sequence>

      {/* 6. Before/After Demo — 16s */}
      <Sequence from={s(32)} durationInFrames={s(16)}>
        <SceneBeforeAfter />
      </Sequence>

      {/* 7. Agent Insight — 8s */}
      <Sequence from={s(48)} durationInFrames={s(8)}>
        <SceneAgentInsight />
      </Sequence>

      {/* 8. Three Properties — 5s */}
      <Sequence from={s(56)} durationInFrames={s(5)}>
        <SceneThreeProperties />
      </Sequence>

      {/* 9. Closing — 4s, no fade out */}
      <Sequence from={s(61)} durationInFrames={s(4)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
