// "You asked your AI to fix one thing. It broke two more."
// 65s. The circle + fear of breaking. Settled vs in play.
// Recognition → Frustration → Dread → Reframe → Relief → Desire

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

const ChatLine: React.FC<{
  role: "user" | "ai";
  text: string;
  delay: number;
}> = ({ role, text, delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  return (
    <div style={{
      opacity: op, fontFamily: fontMono, fontSize: 17,
      color: role === "user" ? P.gold : P.dimWhite,
      lineHeight: 1.6, marginBottom: 2,
    }}>
      {role === "user" ? "> " : "  "}{text}
    </div>
  );
};

const LoopCounter: React.FC<{ delay: number }> = ({ delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const count = Math.floor(interpolate(
    frame, [delay, delay + 160], [1, 8],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  ));
  const color = count > 5 ? P.red : count > 3 ? P.dimGold : P.faintGold;

  return (
    <div style={{
      opacity: op, fontFamily: fontMono, fontSize: 13, color,
      position: "absolute", bottom: 12, right: 20,
    }}>
      {count} fix attempts
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneHook: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 44, color: P.white, textAlign: "center" }}>
      You asked your AI to fix one thing.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center", marginTop: 10 }}>
      It broke two more.
    </Fade>
  </AbsoluteFill>
);

const SceneWhackAMole: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%", position: "relative" }}>
        <Panel style={{ padding: "28px 36px", position: "relative" }}>
          <ChatLine role="user" text="Fix the login button" delay={0} />
          <ChatLine role="ai" text="Done! Fixed the click handler" delay={12} />
          <ChatLine role="user" text="Now the signup page is broken" delay={30} />
          <ChatLine role="ai" text="Fixed! Updated the form validation" delay={42} />
          <ChatLine role="user" text="The login button broke again" delay={62} />
          <ChatLine role="ai" text="Fixed! Restored the click handler" delay={74} />
          <ChatLine role="user" text="Now the profile page won't load" delay={94} />
          <ChatLine role="ai" text="Fixed! Adjusted the routing" delay={106} />
          <ChatLine role="user" text="The signup page is broken again" delay={126} />
          <ChatLine role="ai" text="Fixed! ..." delay={138} />
          <LoopCounter delay={10} />
        </Panel>
        <Fade delay={160} style={{
          fontFamily: fontMono, fontSize: 24, color: P.red,
          textAlign: "center", marginTop: 20,
        }}>
          Three hours. Same three bugs.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneDread: React.FC = () => {
  const frame = useCurrentFrame();

  const lines = [
    { text: "Every fix creates a new problem somewhere else.", delay: 10 },
    { text: "You stop making changes because you're afraid to break it.", delay: 65 },
    { text: "The thing that worked yesterday is the thing you can't trust today.", delay: 120 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        {lines.map((line, i) => {
          const op = interpolate(frame, [line.delay, line.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const nextDelay = lines[i + 1]?.delay ?? 999;
          const dimmed = frame > nextDelay + 5 && frame < 180;

          return (
            <div key={i} style={{
              display: "flex", alignItems: "flex-start", gap: 16,
              opacity: op * (dimmed ? 0.3 : 1),
            }}>
              <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 2 }}>{"\u25c7"}</span>
              <span style={{
                fontFamily: fontMono, fontSize: i === 2 ? 28 : 26,
                color: i === 2 ? P.gold : P.white, lineHeight: 1.5,
              }}>
                {line.text}
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
      Your AI isn't careless.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 34, color: P.gold, textAlign: "center", marginTop: 15 }}>
      It can't tell what's settled from what's still in play.
    </Fade>
  </AbsoluteFill>
);

const SceneBrandReveal: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const glyphScale = spring({ frame: frame - 5, fps, config: { damping: 25, stiffness: 60 } });
  const glyphOp = interpolate(frame, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: fadeOut }}>
      <div style={{ fontFamily: fontMono, fontSize: 100, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>{"\u25c7"}</div>
      <Fade delay={18} style={{ fontFamily: fontMono, fontSize: 56, color: P.white, letterSpacing: "0.15em", marginTop: 5 }}>werk</Fade>
      <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimGold, marginTop: 15 }}>
        It knows what's solid and what's still moving.
      </Fade>
    </AbsoluteFill>
  );
};

const SceneBeforeAfter: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const shrinkStart = 160;
  const shrinkEnd = 200;
  const chaosX = interpolate(frame, [shrinkStart, shrinkEnd], [0, -320], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const chaosScale = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.72], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const chaosDim = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.35], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const werkOp = interpolate(frame, [shrinkEnd, shrinkEnd + 20], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const werkX = interpolate(frame, [shrinkEnd, shrinkEnd + 25], [80, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const captionOp = interpolate(frame, [shrinkEnd + 140, shrinkEnd + 160], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const hereOp = frame > shrinkEnd + 60 ? 0.7 + Math.sin((frame - shrinkEnd - 60) * 0.1) * 0.3 : 0;

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 50px", opacity: fadeOut }}>
      <div style={{ display: "flex", gap: 30, maxWidth: 1350, width: "100%", alignItems: "flex-start" }}>
        <div style={{ flex: 1, transform: `translateX(${chaosX}px) scale(${chaosScale})`, opacity: chaosDim, transformOrigin: "center center" }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.faintGold, letterSpacing: "2px", marginBottom: 10 }}>WITHOUT</div>
          <Panel style={{ padding: "20px 28px" }}>
            {[
              { text: "- fixed login button (3 times)", color: P.faintGold },
              { text: "- signup form... maybe broken?", color: P.dimWhite },
              { text: "- profile page was loading before", color: P.dimWhite },
              { text: "- the routing thing", color: P.faintWhite },
              { text: "- don't touch the payment flow", color: P.red },
              { text: "- actually don't touch anything", color: P.red },
            ].map((item, i) => (
              <RevealLine key={i} text={item.text} color={item.color} delay={8 + i * 8} />
            ))}
          </Panel>
        </div>

        <div style={{ flex: 1, opacity: werkOp, transform: `translateX(${werkX}px)` }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>WITH WERK</div>
          <Panel highlight style={{ padding: "20px 28px" }}>
            <div style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, lineHeight: 1.5 }}>
              {"\u25c7"} People sign up and pay for my app
            </div>
            <div style={{ margin: "10px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
              {[
                { n: "5.", text: "payment flow processes cards", done: false, current: false, color: P.dimWhite },
                { n: "4.", text: "profile loads user data", done: false, current: false, color: P.dimWhite },
                { n: "3.", text: "signup creates working accounts", done: false, current: true, color: P.white },
                { n: "2.", text: "login authenticates users", done: true, current: false, color: P.green },
                { n: "1.", text: "landing page loads correctly", done: true, current: false, color: P.green },
              ].map((step, i) => {
                const stepOp = interpolate(frame, [shrinkEnd + 25 + i * 10, shrinkEnd + 35 + i * 10], [0, 1], {
                  extrapolateLeft: "clamp", extrapolateRight: "clamp",
                });
                return (
                  <div key={i} style={{ fontFamily: fontMono, fontSize: 16, color: step.color, lineHeight: 1.7, opacity: stepOp }}>
                    {step.done ? "\u2713" : " "} {step.n} {step.text}
                    {step.current && <span style={{ color: P.gold, opacity: hereOp }}> {"\u2190"} you are here</span>}
                  </div>
                );
              })}
            </div>
            <Fade delay={shrinkEnd + 80}>
              <div style={{ fontFamily: fontMono, fontSize: 17, color: P.white, lineHeight: 1.5, marginTop: 4 }}>
                {"\u25c6"} Landing and login work. Signup half-built.
              </div>
            </Fade>
          </Panel>
        </div>
      </div>

      <div style={{ fontFamily: fontMono, fontSize: 24, color: P.gold, textAlign: "center", marginTop: 24, opacity: captionOp }}>
        Same project. Now it knows what not to break.
      </div>
    </AbsoluteFill>
  );
};

const SceneAgentMoment: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%" }}>
        <Fade style={{ marginBottom: 20 }}>
          <Panel highlight style={{ padding: "16px 28px" }}>
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.gold }}>{"\u25c7"} People sign up and pay for my app</div>
            <div style={{ fontFamily: fontMono, fontSize: 15, color: P.white, paddingLeft: 16, marginTop: 4 }}>{"\u2192"} next: signup creates working accounts</div>
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, marginTop: 4 }}>{"\u25c6"} Landing and login work. Signup half-built.</div>
          </Panel>
        </Fade>

        <Fade delay={20}>
          <Panel style={{ padding: "20px 28px", borderColor: P.blue }}>
            <TypeLine text="Your next step is the signup flow." color={P.white} delay={35} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="Login and landing are settled — I won't touch those." color={P.white} delay={65} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="I'll build signup on the existing auth, not rewrite it." color={P.dimGold} delay={95} fontSize={18} speed={0.5} cursorColor={P.blue} />
          </Panel>
        </Fade>

        <Fade delay={140} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 18 }}>
          It fixes what's broken without breaking what's solid.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreeProps: React.FC = () => {
  const frame = useCurrentFrame();
  const props = [
    { text: "Settled work stays settled. No regressions.", delay: 8 },
    { text: "Your AI sees what's solid and what's in play.", delay: 50 },
    { text: "You change things without holding your breath.", delay: 92 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        {props.map((prop, i) => {
          const op = interpolate(frame, [prop.delay, prop.delay + 15], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
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
      <div style={{ fontFamily: fontMono, fontSize: 56, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>{"\u25c7"}</div>
      <div style={{ fontFamily: fontMono, fontSize: 48, color: P.white, letterSpacing: "0.15em", marginTop: 8, opacity: nameOp, transform: `translateY(${nameY}px)` }}>werk</div>
      <div style={{ position: "absolute", bottom: 180, opacity: tagOp, textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, lineHeight: 1.8 }}>
          Stop thrashing.
          <br />
          Start on solid ground.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER5_DURATION = s(65);

export const VibeCoder5: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/emotional-ancient-rite.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.14], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER5_DURATION - s(5), VIBECODER5_DURATION], [0.14, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      <Sequence from={0} durationInFrames={s(4)}><SceneHook /></Sequence>
      <Sequence from={s(4)} durationInFrames={s(10)}><SceneWhackAMole /></Sequence>
      <Sequence from={s(14)} durationInFrames={s(8)}><SceneDread /></Sequence>
      <Sequence from={s(22)} durationInFrames={s(5)}><ScenePivot /></Sequence>
      <Sequence from={s(27)} durationInFrames={s(4)}><SceneBrandReveal /></Sequence>
      <Sequence from={s(31)} durationInFrames={s(16)}><SceneBeforeAfter /></Sequence>
      <Sequence from={s(47)} durationInFrames={s(7)}><SceneAgentMoment /></Sequence>
      <Sequence from={s(54)} durationInFrames={s(5)}><SceneThreeProps /></Sequence>
      <Sequence from={s(59)} durationInFrames={s(6)}><SceneClosing /></Sequence>
    </AbsoluteFill>
  );
};
