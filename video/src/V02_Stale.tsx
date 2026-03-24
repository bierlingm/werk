// Video 2: "Stale" — 40s
// Documents appearing and aging. No overlap between items.
// Stronger case for why werk stays current.

import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile, useCurrentFrame, useVideoConfig, interpolate } from "remotion";
import { dark, fontMono, Fade, Panel, StandardClosing, s } from "./primitives";

const P = dark;

const StaleDoc: React.FC<{ title: string; age: string }> = ({ title, age }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const appear = interpolate(frame, [0, 10], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const yellow = interpolate(frame, [20, durationInFrames - 5], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const ageAppear = interpolate(frame, [25, 40], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const exit = interpolate(frame, [durationInFrames - 8, durationInFrames], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });

  const color = yellow > 0.5 ? P.faintGold : P.white;

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: Math.min(appear, exit) }}>
      <div style={{ textAlign: "center" }}>
        <div style={{ fontFamily: fontMono, fontSize: 36, color, filter: `brightness(${1 - yellow * 0.4})` }}>
          {title}
        </div>
        <div style={{ fontFamily: fontMono, fontSize: 18, color: P.red, marginTop: 16, opacity: ageAppear }}>
          {age}
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const V02_DURATION = s(40);

export const V02_Stale: React.FC = () => {
  // Each doc gets its own non-overlapping window
  const docs = [
    { title: "CLAUDE.md", age: "last updated: 12 days ago", dur: 3 },
    { title: "PROJECT_PLAN.md", age: "v3 — but reality moved to v5", dur: 3 },
    { title: "TODO.md", age: "47 unchecked items, 12 irrelevant", dur: 3 },
    { title: "CONTEXT.md", age: "references files that no longer exist", dur: 3 },
    { title: "notes/strategy-FINAL-v2.md", age: "which version is this?", dur: 3 },
  ];

  let offset = 0;

  return (
    <AbsoluteFill style={{ backgroundColor: P.bg }}>
      <Audio src={staticFile("music/tense-stay-the-course.mp3")} volume={(f: number) => {
        const fi = interpolate(f, [0, s(2)], [0, 0.18], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fo = interpolate(f, [V02_DURATION - s(3), V02_DURATION], [0.18, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        return Math.min(fi, fo);
      }} />

      {/* Stale docs — strictly sequential, no overlap */}
      {docs.map((doc, i) => {
        const from = s(offset);
        offset += doc.dur;
        return (
          <Sequence key={i} from={from} durationInFrames={s(doc.dur)}>
            <StaleDoc title={doc.title} age={doc.age} />
          </Sequence>
        );
      })}

      {/* Why */}
      <Sequence from={s(15)} durationInFrames={s(5)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}>
          <Fade style={{ fontFamily: fontMono, fontSize: 28, color: P.white, textAlign: "center", lineHeight: 1.8 }}>
            These aren't bad documents.
            <br />
            They're <span style={{ color: P.red }}>disconnected from the work</span>.
          </Fade>
          <Fade delay={30} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", lineHeight: 1.8, marginTop: 15 }}>
            Updating them is a separate chore. So it never happens.
          </Fade>
        </AbsoluteFill>
      </Sequence>

      {/* werk tree */}
      <Sequence from={s(20)} durationInFrames={s(10)}>
        <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}>
          <div style={{ maxWidth: 1000, width: "100%" }}>
            <Fade style={{ fontFamily: fontMono, fontSize: 18, color: P.dimGold, marginBottom: 16 }}>
              $ werk tree
            </Fade>
            <Panel>
              <Fade delay={8} style={{ color: P.gold, whiteSpace: "pre", fontSize: 20 }}>{"\u2514\u2500\u2500 #1 ship v2 with billing          [4/9]"}</Fade>
              <Fade delay={14} style={{ color: P.white, whiteSpace: "pre", fontSize: 20 }}>{"    \u251c\u2500\u2500 #3 Stripe handles renewals"}</Fade>
              <Fade delay={20} style={{ color: P.green, whiteSpace: "pre", fontSize: 20 }}>{"    \u251c\u2500\u2500 #4 \u2713 usage metering works"}</Fade>
              <Fade delay={26} style={{ color: P.white, whiteSpace: "pre", fontSize: 20 }}>{"    \u2514\u2500\u2500 #5 migration path tested"}</Fade>
            </Panel>
            <Fade delay={40} style={{ fontFamily: fontMono, fontSize: 20, color: P.dimGold, textAlign: "center", marginTop: 24 }}>
              Updated 4 minutes ago.
            </Fade>
            <Fade delay={55} style={{ fontFamily: fontMono, fontSize: 20, color: P.white, textAlign: "center", marginTop: 8 }}>
              Not because someone maintained it.
              <br />
              Because resolving a step and updating reality <em>is</em> the work.
            </Fade>
          </div>
        </AbsoluteFill>
      </Sequence>

      {/* Close */}
      <Sequence from={s(30)} durationInFrames={s(10)}>
        <StandardClosing preLines={["The structural model that can't go stale.", "Because using it is the practice."]} />
      </Sequence>
    </AbsoluteFill>
  );
};
