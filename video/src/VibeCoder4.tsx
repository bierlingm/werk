// "You have so many ideas. You've started all of them."
// 65s. Multiple half-built projects. The graveyard.
// Recognition → Guilt → Pattern → Reframe → Relief → Desire

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

const ProjectGhost: React.FC<{
  name: string;
  lastMsg: string;
  when: string;
  delay: number;
  x: number;
  y: number;
  rotate?: number;
}> = ({ name, lastMsg, when, delay, x, y, rotate = 0 }) => {
  const frame = useCurrentFrame();

  // Appears bright, then dims
  const enterOp = interpolate(frame, [delay, delay + 12], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const dimOp = interpolate(frame, [delay + 40, delay + 70], [1, 0.5], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const borderColor = interpolate(frame, [delay, delay + 15, delay + 50], [0, 1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <div style={{
      position: "absolute",
      left: x, top: y,
      opacity: enterOp * dimOp,
      transform: `rotate(${rotate}deg)`,
      width: 380,
    }}>
      <div style={{
        fontFamily: fontMono, fontSize: 16, lineHeight: 1.6,
        padding: "14px 18px",
        backgroundColor: P.panel,
        borderRadius: 6,
        border: `1px solid ${borderColor > 0.5 ? P.gold : P.faintGold}`,
      }}>
        <div style={{ color: P.gold, fontSize: 15, marginBottom: 4 }}>{name}</div>
        <div style={{ color: P.dimWhite, fontSize: 14, fontStyle: "italic" }}>"{lastMsg}"</div>
        <div style={{ color: P.faintGold, fontSize: 12, marginTop: 4 }}>{when}</div>
      </div>
    </div>
  );
};

const ProjectCounter: React.FC<{ delay: number }> = ({ delay }) => {
  const frame = useCurrentFrame();
  const op = interpolate(frame, [delay, delay + 10], [0, 1], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });
  const count = Math.floor(interpolate(
    frame, [delay, delay + 130], [1, 5],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  ));
  const color = count > 3 ? P.red : count > 2 ? P.dimGold : P.faintGold;

  return (
    <div style={{
      opacity: op, fontFamily: fontMono, fontSize: 14, color,
      position: "absolute", bottom: 40, right: 80,
    }}>
      {count} projects started
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneHook: React.FC = () => (
  <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
    <Fade style={{ fontFamily: fontMono, fontSize: 44, color: P.white, textAlign: "center" }}>
      You have so many ideas.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 48, color: P.gold, textAlign: "center", marginTop: 10 }}>
      You've started all of them.
    </Fade>
  </AbsoluteFill>
);

const SceneGraveyard: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ opacity: fadeOut }}>
      <ProjectGhost name="Recipe app" lastMsg="Almost done, just need auth..." when="3 weeks ago" delay={0} x={180} y={200} rotate={-1.5} />
      <ProjectGhost name="Portfolio site" lastMsg="Broke the layout, leaving it..." when="2 months ago" delay={30} x={1100} y={180} rotate={1} />
      <ProjectGhost name="Habit tracker" lastMsg="Need to figure out the database..." when="6 weeks ago" delay={55} x={620} y={380} rotate={0.5} />
      <ProjectGhost name="Budget tool" lastMsg="Works locally, need to deploy..." when="last month" delay={80} x={250} y={560} rotate={-0.8} />
      <ProjectGhost name="AI chatbot" lastMsg="It's doing something weird..." when="yesterday" delay={105} x={1020} y={520} rotate={1.2} />

      <ProjectCounter delay={10} />

      <div style={{
        position: "absolute",
        bottom: 80,
        left: 0, right: 0,
        textAlign: "center",
      }}>
        <Fade delay={170} style={{ fontFamily: fontMono, fontSize: 24, color: P.red }}>
          Which one are you finishing?
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const ScenePattern: React.FC = () => {
  const frame = useCurrentFrame();

  const lines = [
    { text: "You start something new because starting feels like progress.", delay: 10 },
    { text: "You leave something old because you lost track of where it was.", delay: 70 },
    { text: "Five beginnings. No endings.", delay: 130 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        {lines.map((line, i) => {
          const op = interpolate(frame, [line.delay, line.delay + 15], [0, 1], {
            extrapolateLeft: "clamp", extrapolateRight: "clamp",
          });
          const nextDelay = lines[i + 1]?.delay ?? 999;
          const dimmed = frame > nextDelay + 5 && frame < 185;

          return (
            <div key={i} style={{
              display: "flex", alignItems: "flex-start", gap: 16,
              opacity: op * (dimmed ? 0.3 : 1),
            }}>
              <span style={{ fontFamily: fontMono, fontSize: 22, color: P.gold, flexShrink: 0, marginTop: 2 }}>{"\u25c7"}</span>
              <span style={{
                fontFamily: fontMono,
                fontSize: i === 2 ? 28 : 26,
                color: i === 2 ? P.gold : P.white,
                lineHeight: 1.5,
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
      You don't have a finishing problem.
    </Fade>
    <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 34, color: P.gold, textAlign: "center", marginTop: 15 }}>
      You have a visibility problem.
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
      <div style={{ fontFamily: fontMono, fontSize: 100, color: P.gold, opacity: glyphOp, transform: `scale(${glyphScale})` }}>{"\u25c7"}</div>
      <Fade delay={18} style={{ fontFamily: fontMono, fontSize: 56, color: P.white, letterSpacing: "0.15em", marginTop: 5 }}>werk</Fade>
      <Fade delay={45} style={{ fontFamily: fontMono, fontSize: 22, color: P.dimGold, marginTop: 15 }}>
        See everything. Finish what matters.
      </Fade>
    </AbsoluteFill>
  );
};

const SceneSurvey: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const shrinkStart = 170;
  const shrinkEnd = 210;
  const chaosX = interpolate(frame, [shrinkStart, shrinkEnd], [0, -320], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const chaosScale = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.72], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const chaosDim = interpolate(frame, [shrinkStart, shrinkEnd], [1, 0.35], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const werkOp = interpolate(frame, [shrinkEnd, shrinkEnd + 20], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const werkX = interpolate(frame, [shrinkEnd, shrinkEnd + 25], [80, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const captionOp = interpolate(frame, [shrinkEnd + 160, shrinkEnd + 180], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const projects = [
    { name: "Recipe app", desire: "find and save recipes they love", next: "auth lets users log in", reality: "Search works. Save works. No auth.", progress: "4/5", delay: 0 },
    { name: "Portfolio site", desire: "clients see my work and hire me", next: "fix responsive layout", reality: "Content done. Layout broken.", progress: "2/4", delay: 18 },
    { name: "Habit tracker", desire: "I track habits and see streaks", next: "set up database", reality: "UI sketched. No backend.", progress: "0/3", delay: 36 },
    { name: "Budget tool", desire: "see where my money goes", next: "deploy so I can use it", reality: "Works locally. Not deployed.", progress: "3/4", delay: 54 },
    { name: "AI chatbot", desire: "people get smart answers", next: "fix hallucination issue", reality: "Basic flow works. Answers unreliable.", progress: "1/5", delay: 72 },
  ];

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 40px", opacity: fadeOut }}>
      <div style={{ display: "flex", gap: 24, maxWidth: 1400, width: "100%", alignItems: "flex-start" }}>
        {/* Chaos */}
        <div style={{ flex: 1, transform: `translateX(${chaosX}px) scale(${chaosScale})`, opacity: chaosDim, transformOrigin: "center center" }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.faintGold, letterSpacing: "2px", marginBottom: 10 }}>WITHOUT</div>
          <Panel style={{ padding: "20px 24px" }}>
            {[
              { text: "- recipe app (how close am I?)", color: P.faintGold },
              { text: "- portfolio... is it broken?", color: P.dimWhite },
              { text: "- that habit thing", color: P.faintWhite },
              { text: "- budget tool somewhere", color: P.faintWhite },
              { text: "- chatbot was yesterday", color: P.white },
              { text: "- maybe start something new?", color: P.red },
            ].map((item, i) => (
              <RevealLine key={i} text={item.text} color={item.color} delay={8 + i * 8} />
            ))}
          </Panel>
        </div>

        {/* Werk tree view */}
        <div style={{ flex: 1.3, opacity: werkOp, transform: `translateX(${werkX}px)` }}>
          <div style={{ fontFamily: fontMono, fontSize: 13, color: P.gold, letterSpacing: "2px", marginBottom: 10 }}>WITH WERK</div>
          <Panel highlight style={{ padding: "16px 22px" }}>
            {projects.map((proj, i) => {
              const pOp = interpolate(frame, [shrinkEnd + 20 + proj.delay, shrinkEnd + 35 + proj.delay], [0, 1], {
                extrapolateLeft: "clamp", extrapolateRight: "clamp",
              });
              const isAlmost = proj.name === "Recipe app" || proj.name === "Budget tool";
              return (
                <div key={i} style={{ opacity: pOp, marginBottom: i < projects.length - 1 ? 10 : 0 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                    <span style={{ fontFamily: fontMono, fontSize: 15, color: P.gold }}>{proj.name}</span>
                    <span style={{ fontFamily: fontMono, fontSize: 12, color: isAlmost ? P.green : P.faintGold }}>[{proj.progress}]</span>
                  </div>
                  <div style={{ fontFamily: fontMono, fontSize: 13, color: P.dimGold, paddingLeft: 12 }}>
                    {"\u25c7"} {proj.desire}
                  </div>
                  <div style={{ fontFamily: fontMono, fontSize: 13, color: P.white, paddingLeft: 12 }}>
                    {"\u2192"} {proj.next}
                  </div>
                  <div style={{ fontFamily: fontMono, fontSize: 13, color: P.dimWhite, paddingLeft: 12 }}>
                    {"\u25c6"} {proj.reality}
                  </div>
                  {i < projects.length - 1 && (
                    <div style={{ borderBottom: `1px solid ${P.faintGold}`, margin: "8px 0", opacity: 0.3 }} />
                  )}
                </div>
              );
            })}
          </Panel>
        </div>
      </div>

      <div style={{ fontFamily: fontMono, fontSize: 24, color: P.gold, textAlign: "center", marginTop: 20, opacity: captionOp }}>
        Five projects. One clear picture.
      </div>
    </AbsoluteFill>
  );
};

const ScenePrioritize: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const fadeOut = interpolate(frame, [durationInFrames - 15, durationInFrames], [1, 0], {
    extrapolateLeft: "clamp", extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px", opacity: fadeOut }}>
      <div style={{ maxWidth: 900, width: "100%" }}>
        {/* Scoreboard */}
        <Fade style={{ marginBottom: 20 }}>
          <Panel highlight style={{ padding: "16px 24px" }}>
            {[
              { name: "Recipe app", next: "auth", prog: "4/5", close: true },
              { name: "Budget tool", next: "deploy", prog: "3/4", close: true },
              { name: "Portfolio site", next: "fix layout", prog: "2/4", close: false },
              { name: "AI chatbot", next: "fix hallucinations", prog: "1/5", close: false },
              { name: "Habit tracker", next: "set up database", prog: "0/3", close: false },
            ].map((p, i) => {
              const lOp = interpolate(frame, [8 + i * 8, 16 + i * 8], [0, 1], {
                extrapolateLeft: "clamp", extrapolateRight: "clamp",
              });
              return (
                <div key={i} style={{
                  fontFamily: fontMono, fontSize: 15, lineHeight: 1.8,
                  display: "flex", justifyContent: "space-between",
                  opacity: lOp,
                  color: p.close ? P.green : P.dimWhite,
                }}>
                  <span>{"\u25c7"} {p.name}</span>
                  <span>{p.next}</span>
                  <span>[{p.prog}]</span>
                </div>
              );
            })}
          </Panel>
        </Fade>

        {/* AI insight */}
        <Fade delay={60}>
          <Panel style={{ padding: "18px 24px", borderColor: P.blue }}>
            <TypeLine text="Recipe app is one step from done." color={P.white} delay={75} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="Budget tool just needs deployment." color={P.white} delay={100} fontSize={18} speed={0.5} cursorColor={P.blue} />
            <TypeLine text="Finish those two. Then decide what's next." color={P.gold} delay={130} fontSize={18} speed={0.5} cursorColor={P.blue} />
          </Panel>
        </Fade>

        <Fade delay={170} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 18 }}>
          You don't need more time. You need to see what's closest.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

const SceneThreeProps: React.FC = () => {
  const frame = useCurrentFrame();
  const props = [
    { text: "See all your projects in one place.", delay: 8 },
    { text: "Know which ones are closest to done.", delay: 50 },
    { text: "Finish what matters instead of starting what's new.", delay: 92 },
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
          See everything you're building.
          <br />
          Finish what matters.
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const VIBECODER4_DURATION = s(65);

export const VibeCoder4: React.FC = () => {
  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/cinematic-night-vigil.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(4)], [0, 0.15], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [VIBECODER4_DURATION - s(5), VIBECODER4_DURATION], [0.15, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} startFrom={60 * 30} />

      {/* 1. Hook — 4s */}
      <Sequence from={0} durationInFrames={s(4)}>
        <SceneHook />
      </Sequence>

      {/* 2. The Graveyard — 10s */}
      <Sequence from={s(4)} durationInFrames={s(10)}>
        <SceneGraveyard />
      </Sequence>

      {/* 3. The Pattern — 8s */}
      <Sequence from={s(14)} durationInFrames={s(8)}>
        <ScenePattern />
      </Sequence>

      {/* 4. Pivot — 5s */}
      <Sequence from={s(22)} durationInFrames={s(5)}>
        <ScenePivot />
      </Sequence>

      {/* 5. Brand reveal — 4s */}
      <Sequence from={s(27)} durationInFrames={s(4)}>
        <SceneBrandReveal />
      </Sequence>

      {/* 6. Survey — before/after — 16s */}
      <Sequence from={s(31)} durationInFrames={s(16)}>
        <SceneSurvey />
      </Sequence>

      {/* 7. Prioritization — 7s */}
      <Sequence from={s(47)} durationInFrames={s(7)}>
        <ScenePrioritize />
      </Sequence>

      {/* 8. Three properties — 5s */}
      <Sequence from={s(54)} durationInFrames={s(5)}>
        <SceneThreeProps />
      </Sequence>

      {/* 9. Closing — 6s, no fade out */}
      <Sequence from={s(59)} durationInFrames={s(6)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
