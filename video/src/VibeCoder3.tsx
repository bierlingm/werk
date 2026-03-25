// "You built something in a weekend. And it actually worked."
// 65s. The euphoria-to-overwhelm arc. The origin story.
// Joy → Creeping doubt → The drop → Reframe → Relief → Desire

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
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  return (
    <div style={{
      opacity: op,
      fontFamily: fontMono,
      fontSize: 17,
      color: role === "user" ? P.gold : P.dimWhite,
      lineHeight: 1.6,
      marginBottom: 2,
    }}>
      {role === "user" ? "> " : "  "}{text}
    </div>
  );
};

const DayCounter: React.FC<{ delay: number }> = ({ delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const day = Math.floor(interpolate(
    frame, [delay, delay + 200], [1, 12],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  ));
  const color = day > 8 ? P.red : day > 5 ? P.dimGold : P.faintGold;

  return (
    <div style={{
      opacity: op, fontFamily: fontMono, fontSize: 13, color,
      position: "absolute", bottom: 12, right: 20,
    }}>
      Day {day}
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneHigh: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 48, color: P.white, textAlign: "center" }}>
      You built something in a weekend.
    </Fade>
    <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center", marginTop: 10 }}>
      And it actually worked.
    </Fade>
  </AbsoluteFill>
);

const SceneMontage: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%", position: "relative" }}>
        <Panel style={{ padding: "28px 36px", position: "relative" }}>
          <ChatLine role="user" text="Add a settings page" delay={0} />
          <ChatLine role="ai" text="Done! Settings with profile management" delay={12} />
          <ChatLine role="user" text="Can users invite friends?" delay={28} />
          <ChatLine role="ai" text="Added invite system with email notifications" delay={38} />
          <ChatLine role="user" text="Make it work offline" delay={54} />
          <ChatLine role="ai" text="Implemented service worker with local cache" delay={62} />
          <ChatLine role="user" text="Something broke on the signup page" delay={80} />
          <ChatLine role="ai" text="Fixed! Also optimized the database queries" delay={88} />
          <ChatLine role="user" text="I didn't ask you to do that" delay={108} />
          <ChatLine role="ai" text="Don't worry, it's all connected" delay={116} />
          <ChatLine role="user" text="...what's all connected?" delay={140} />
          <DayCounter delay={10} />
        </Panel>
      </div>
    </AbsoluteFill>
  );
};

const SceneDrop: React.FC = () => {
  const frame = useCurrentFrame();

  const pains = [
    { text: "You used to understand every part of this.", delay: 10 },
    { text: "Now you're afraid to touch it.", delay: 65 },
    { text: "The thing you built fast is the thing you can't change.", delay: 120 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        {pains.map((pain, i) => {
          const op = interpolate(frame, [pain.delay, pain.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const nextDelay = pains[i + 1]?.delay ?? 999;
          const dimmed = frame > nextDelay + 5 && frame < 175;

          return (
            <div key={i} style={{
              display: "flex", alignItems: "flex-start", gap: 16,
              opacity: op * (dimmed ? 0.3 : 1),
            }}>
              <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 2 }}>
                {"\u25c7"}
              </span>
              <span style={{
                fontFamily: fontMono,
                fontSize: i === 2 ? 28 : 26,
                color: i === 2 ? P.gold : P.white,
                lineHeight: 1.5,
              }}>
                {pain.text}
              </span>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const SceneReframe: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 120px" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.dimWhite, textAlign: "center" }}>
      Speed wasn't the problem.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 36, color: P.gold, textAlign: "center", marginTop: 15 }}>
      Building without a map was.
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
        Build at full speed without losing your way.
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
  const captionOp = interpolate(frame, [shrinkEnd + 130, shrinkEnd + 150], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const hereOp = frame > shrinkEnd + 60 ? 0.7 + Math.sin((frame - shrinkEnd - 60) * 0.1) * 0.3 : 0;

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 50px", opacity: fadeOut }}>
      <div style={{ display: "flex", gap: 30, maxWidth: 1350, width: "100%", alignItems: "flex-start" }}>
        {/* Chaos */}
        <div style={{ flex: 1, transform: `translateX(${chaosX}px) scale(${chaosScale})`, opacity: chaosDim, transformOrigin: "center center" }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.faintGold, letterSpacing: "2px", marginBottom: 10 }}>WITHOUT</div>
          <Panel style={{ padding: "20px 28px" }}>
            {[
              { text: "- settings page (done?)", color: P.faintGold },
              { text: "- invite system... I think", color: P.dimWhite },
              { text: '- "something about offline mode"', color: P.dimWhite },
              { text: "- signup might be broken", color: P.white },
              { text: "- the database thing", color: P.faintWhite },
              { text: "- are notifications working?", color: P.faintWhite },
              { text: "- what did it optimize?", color: P.faintWhite },
              { text: "- DO NOT TOUCH auth", color: P.red },
            ].map((item, i) => (
              <RevealLine key={i} text={item.text} color={item.color} delay={8 + i * 6} />
            ))}
          </Panel>
        </div>

        {/* Werk */}
        <div style={{ flex: 1, opacity: werkOp, transform: `translateX(${werkX}px)` }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>WITH WERK</div>
          <Panel highlight style={{ padding: "20px 28px" }}>
            <div style={{ fontFamily: fontMono, fontSize: 20, color: P.gold, lineHeight: 1.5 }}>
              {"\u25c7"} My app works offline and people invite friends
            </div>
            <div style={{ margin: "10px 0", paddingLeft: 20, borderLeft: `2px solid ${P.faintGold}` }}>
              {[
                { n: "6.", text: "notifications reach invited users", done: false, current: false, color: P.dimWhite },
                { n: "5.", text: "offline mode syncs when back online", done: false, current: false, color: P.dimWhite },
                { n: "4.", text: "invite flow sends and accepts", done: false, current: true, color: P.white },
                { n: "3.", text: "settings page saves preferences", done: true, current: false, color: P.green },
                { n: "2.", text: "signup creates working accounts", done: true, current: false, color: P.green },
                { n: "1.", text: "core app loads and runs", done: true, current: false, color: P.green },
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
            <Fade delay={shrinkEnd + 90}>
              <div style={{ fontFamily: fontMono, fontSize: 17, color: P.white, lineHeight: 1.5, marginTop: 4 }}>
                {"\u25c6"} Settings done. Signup works. Invite flow half-built.
              </div>
            </Fade>
          </Panel>
        </div>
      </div>

      <div style={{ fontFamily: fontMono, fontSize: 24, color: P.gold, textAlign: "center", marginTop: 24, opacity: captionOp }}>
        Same project. Now you know where you stand.
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
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.gold }}>{"\u25c7"} My app works offline and people invite friends</div>
            <div style={{ fontFamily: fontMono, fontSize: 15, color: P.white, paddingLeft: 16, marginTop: 4 }}>{"\u2192"} next: invite flow sends and accepts</div>
            <div style={{ fontFamily: fontMono, fontSize: 16, color: P.dimWhite, marginTop: 4 }}>{"\u25c6"} Settings done. Signup works. Invite half-built.</div>
          </Panel>
        </Fade>

        <Fade delay={20}>
          <Panel style={{ padding: "20px 28px", borderColor: P.blue }}>
            <TypeLine text="Your next step is the invite flow." color={P.white} delay={35} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="Signup and settings are done — I'll leave those alone." color={P.white} delay={65} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="I'll build on the existing user model, not create a new one." color={P.dimGold} delay={95} fontSize={18} speed={0.5} cursorColor={P.blue} />
          </Panel>
        </Fade>

        <Fade delay={145} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 20 }}>
          It builds the right thing because it can see the whole picture.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreeProps: React.FC = () => {
  const frame = useCurrentFrame();
  const props = [
    { text: "You always know what's done and what's left.", delay: 8 },
    { text: "Your AI knows what to build and what not to touch.", delay: 50 },
    { text: "You stay fast without losing control.", delay: 92 },
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
          Build fast. Stay found.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER3_DURATION = s(65);

export const VibeCoder3: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/ambient-ether-vox.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.16], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER3_DURATION - s(5), VIBECODER3_DURATION], [0.16, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={30 * 30} />

      {/* 1. The High — 5s */}
      <Sequence from={0} durationInFrames={s(5)}>
        <SceneHigh />
      </Sequence>

      {/* 2. The Montage — 9s */}
      <Sequence from={s(5)} durationInFrames={s(9)}>
        <SceneMontage />
      </Sequence>

      {/* 3. The Drop — 7s */}
      <Sequence from={s(14)} durationInFrames={s(7)}>
        <SceneDrop />
      </Sequence>

      {/* 4. Reframe — 5s */}
      <Sequence from={s(21)} durationInFrames={s(5)}>
        <SceneReframe />
      </Sequence>

      {/* 5. Brand reveal — 4s */}
      <Sequence from={s(26)} durationInFrames={s(4)}>
        <SceneBrandReveal />
      </Sequence>

      {/* 6. Before/After — 16s */}
      <Sequence from={s(30)} durationInFrames={s(16)}>
        <SceneBeforeAfter />
      </Sequence>

      {/* 7. Agent moment — 7s */}
      <Sequence from={s(46)} durationInFrames={s(7)}>
        <SceneAgentMoment />
      </Sequence>

      {/* 8. Three properties — 5s */}
      <Sequence from={s(53)} durationInFrames={s(5)}>
        <SceneThreeProps />
      </Sequence>

      {/* 9. Closing — 7s, no fade out */}
      <Sequence from={s(58)} durationInFrames={s(7)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
