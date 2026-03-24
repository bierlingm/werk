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

const GOLD = "#C4A035";
const DIM_GOLD = "#8B7355";
const FAINT_GOLD = "#5a4d3a";
const BG = "#0a0a0a";
const WHITE = "#e8e4de";
const DIM_WHITE = "#9a958e";
const RED = "#a85454";
const GREEN = "#5a9a5a";

const fontMono = `'Berkeley Mono', 'Menlo', monospace`;

// ─── Reusable Components ────────────────────────────────────────────

const FadeText: React.FC<{
  children: React.ReactNode;
  delay?: number;
  style?: React.CSSProperties;
}> = ({ children, delay = 0, style }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const opacity = interpolate(
    frame,
    [delay, delay + 15, durationInFrames - 12, durationInFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  const y = interpolate(frame, [delay, delay + 20], [14, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <div style={{ opacity, transform: `translateY(${y}px)`, ...style }}>
      {children}
    </div>
  );
};

const TerminalBlock: React.FC<{
  lines: { text: string; color?: string; indent?: number; delay?: number }[];
  startDelay?: number;
}> = ({ lines, startDelay = 0 }) => {
  const frame = useCurrentFrame();

  return (
    <div
      style={{
        fontFamily: fontMono,
        fontSize: 22,
        lineHeight: 1.7,
        padding: "40px 60px",
        backgroundColor: "#111110",
        borderRadius: 8,
        border: `1px solid ${FAINT_GOLD}`,
        maxWidth: 1100,
        width: "100%",
      }}
    >
      {lines.map((line, i) => {
        const lineDelay = startDelay + (line.delay ?? i * 4);
        const opacity = interpolate(
          frame,
          [lineDelay, lineDelay + 8],
          [0, 1],
          { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
        );
        return (
          <div
            key={i}
            style={{
              opacity,
              color: line.color ?? DIM_WHITE,
              paddingLeft: (line.indent ?? 0) * 24,
              whiteSpace: "pre",
            }}
          >
            {line.text}
          </div>
        );
      })}
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

const SceneTitle: React.FC = () => {
  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 120px",
      }}
    >
      <FadeText
        style={{
          fontFamily: fontMono,
          fontSize: 52,
          color: WHITE,
          textAlign: "center",
          lineHeight: 1.5,
        }}
      >
        Every agentic coding setup
        <br />
        has the same blind spot.
      </FadeText>
    </AbsoluteFill>
  );
};

const SceneStalePlan: React.FC = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const fadeOut = interpolate(
    frame,
    [durationInFrames - 12, durationInFrames],
    [1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 100px",
        opacity: fadeOut,
      }}
    >
      <div style={{ width: "100%", maxWidth: 1100 }}>
        <FadeText
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            marginBottom: 20,
            letterSpacing: "0.5px",
          }}
        >
          $ cat PLAN.md | head -20
        </FadeText>
        <TerminalBlock
          lines={[
            { text: "# Project Plan (v3, updated March 1)", color: DIM_WHITE },
            { text: "" },
            { text: "## Goals", color: FAINT_GOLD },
            { text: "- Build the API layer", indent: 0 },
            { text: "- Integrate payment provider", indent: 0 },
            { text: "- Launch beta by March 15  \u2190 8 days ago", color: RED },
            { text: "" },
            { text: "## TODO", color: FAINT_GOLD },
            { text: "- [x] Set up project structure", color: FAINT_GOLD },
            { text: "- [ ] Design API endpoints", indent: 0 },
            { text: "- [ ] Write auth middleware", indent: 0 },
            { text: "- [ ] ... 47 more items", color: FAINT_GOLD },
          ]}
          startDelay={8}
        />
        <FadeText
          delay={60}
          style={{
            fontFamily: fontMono,
            fontSize: 24,
            color: RED,
            marginTop: 30,
            textAlign: "center",
          }}
        >
          Stale the moment an agent learns something the plan didn't anticipate.
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

const SceneProblem: React.FC = () => {
  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 120px",
      }}
    >
      <div style={{ maxWidth: 1000 }}>
        <FadeText
          style={{
            fontFamily: fontMono,
            fontSize: 28,
            color: WHITE,
            lineHeight: 1.8,
            textAlign: "center",
          }}
        >
          Agents know what task is next.
        </FadeText>
        <FadeText
          delay={25}
          style={{
            fontFamily: fontMono,
            fontSize: 28,
            color: WHITE,
            lineHeight: 1.8,
            textAlign: "center",
            marginTop: 8,
          }}
        >
          They don't know if the task still matters.
        </FadeText>
        <FadeText
          delay={55}
          style={{
            fontFamily: fontMono,
            fontSize: 22,
            color: DIM_GOLD,
            lineHeight: 1.8,
            textAlign: "center",
            marginTop: 40,
          }}
        >
          The bead graph routes mechanistically.
          <br />
          The structural question requires honest assessment
          <br />
          of desire vs. reality.
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

const SceneWerkTree: React.FC = () => {
  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 80px",
      }}
    >
      <div style={{ width: "100%", maxWidth: 1100 }}>
        <FadeText
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            marginBottom: 20,
            letterSpacing: "0.5px",
          }}
        >
          $ werk tree
        </FadeText>
        <TerminalBlock
          lines={[
            {
              text: "\u2514\u2500\u2500 #2 [2026-06] product is live and generating revenue",
              color: GOLD,
            },
            {
              text: "    \u251c\u2500\u2500 #3 API layer handles all payment flows       [3/5]",
              color: WHITE,
            },
            {
              text: "    \u2502   \u251c\u2500\u2500 #8 auth middleware passes security audit",
              color: GREEN,
              delay: 12,
            },
            {
              text: "    \u2502   \u251c\u2500\u2500 #9 Stripe integration handles subscriptions",
              color: WHITE,
              delay: 16,
            },
            {
              text: "    \u2502   \u2514\u2500\u2500 #10 rate limiting protects endpoints",
              color: WHITE,
              delay: 20,
            },
            {
              text: "    \u251c\u2500\u2500 #4 users can onboard without support        [1/3]",
              color: WHITE,
              delay: 24,
            },
            {
              text: "    \u2502   \u251c\u2500\u2500 #11 signup flow tested with 10 beta users",
              color: GREEN,
              delay: 28,
            },
            {
              text: "    \u2502   \u2514\u2500\u2500 #12 docs cover all common workflows",
              color: WHITE,
              delay: 32,
            },
            {
              text: "    \u2514\u2500\u2500 #5 beta feedback incorporated into v1        [0/4]",
              color: WHITE,
              delay: 36,
            },
            { text: "", delay: 36 },
            {
              text: "Total: 12  Active: 8  Resolved: 4",
              color: FAINT_GOLD,
              delay: 42,
            },
          ]}
          startDelay={6}
        />
      </div>
    </AbsoluteFill>
  );
};

const SceneDesireReality: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const lineHeight = spring({
    frame: frame - 20,
    fps,
    config: { damping: 20, stiffness: 40 },
    from: 0,
    to: 280,
  });

  const arrowOpacity = interpolate(frame, [50, 65], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 100px",
      }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 0,
        }}
      >
        {/* Desired */}
        <FadeText
          delay={5}
          style={{
            fontFamily: fontMono,
            fontSize: 30,
            color: GOLD,
            textAlign: "center",
          }}
        >
          \u25c7 desired: product is live and generating revenue
        </FadeText>

        {/* Vertical line with label */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            margin: "10px 0",
          }}
        >
          <div
            style={{
              width: 1,
              height: lineHeight,
              backgroundColor: FAINT_GOLD,
            }}
          />
          <div
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: FAINT_GOLD,
              opacity: arrowOpacity,
              position: "absolute",
              top: "50%",
              transform: "translateY(-50%)",
            }}
          >
            theory of closure
          </div>
        </div>

        {/* Reality */}
        <FadeText
          delay={30}
          style={{
            fontFamily: fontMono,
            fontSize: 30,
            color: DIM_WHITE,
            textAlign: "center",
          }}
        >
          \u25c6 reality: API 60% done, no users yet, launch date passed
        </FadeText>

        {/* Explanation */}
        <FadeText
          delay={70}
          style={{
            fontFamily: fontMono,
            fontSize: 22,
            color: DIM_GOLD,
            textAlign: "center",
            marginTop: 50,
            lineHeight: 1.8,
          }}
        >
          The gap is the tension. Children compose the bridge.
          <br />
          When reality shifts, the structure knows.
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

const SceneAgentContext: React.FC = () => {
  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 80px",
      }}
    >
      <div style={{ width: "100%", maxWidth: 1100 }}>
        <FadeText
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            marginBottom: 20,
            letterSpacing: "0.5px",
          }}
        >
          $ werk context --json | head -20
        </FadeText>
        <TerminalBlock
          lines={[
            { text: "{", color: FAINT_GOLD },
            {
              text: '  "tensions": [',
              color: DIM_WHITE,
            },
            { text: "    {", color: DIM_WHITE, delay: 8 },
            {
              text: '      "desired": "product is live and generating revenue",',
              color: GOLD,
              delay: 12,
            },
            {
              text: '      "reality": "API 60% done, no users yet",',
              color: DIM_WHITE,
              delay: 16,
            },
            {
              text: '      "closure": "3/12 steps resolved",',
              color: WHITE,
              delay: 20,
            },
            {
              text: '      "frontier": "Stripe integration",',
              color: WHITE,
              delay: 24,
            },
            {
              text: '      "urgency": 1.2,',
              color: RED,
              delay: 28,
            },
            {
              text: '      "horizon_drift": "+8 days"',
              color: RED,
              delay: 32,
            },
            { text: "    }", color: DIM_WHITE, delay: 36 },
            { text: "  ]", color: DIM_WHITE, delay: 36 },
            { text: "}", color: FAINT_GOLD, delay: 36 },
          ]}
          startDelay={6}
        />
        <FadeText
          delay={50}
          style={{
            fontFamily: fontMono,
            fontSize: 22,
            color: WHITE,
            marginTop: 30,
            textAlign: "center",
          }}
        >
          Better context than any CLAUDE.md you've ever written.
          <br />
          <span style={{ color: DIM_GOLD }}>
            Because it updates through use, not through maintenance.
          </span>
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

const SceneEpoch: React.FC = () => {
  const frame = useCurrentFrame();

  const beforeOpacity = interpolate(frame, [30, 55], [1, 0.2], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const afterOpacity = interpolate(frame, [55, 75], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const labelOpacity = interpolate(frame, [65, 80], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 80px",
      }}
    >
      <div
        style={{
          display: "flex",
          gap: 60,
          alignItems: "flex-start",
          maxWidth: 1200,
          width: "100%",
        }}
      >
        {/* Before */}
        <div style={{ flex: 1, opacity: beforeOpacity }}>
          <div
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: FAINT_GOLD,
              marginBottom: 16,
              letterSpacing: "1px",
            }}
          >
            EPOCH 1
          </div>
          <TerminalBlock
            lines={[
              { text: "\u25c7 launch beta by March 15", color: GOLD },
              { text: "  \u251c\u2500\u2500 build API layer", color: DIM_WHITE },
              { text: "  \u251c\u2500\u2500 integrate payments", color: DIM_WHITE },
              { text: "  \u2514\u2500\u2500 write docs", color: DIM_WHITE },
              { text: "" },
              { text: "\u25c6 API 60%, deadline passed", color: RED },
            ]}
            startDelay={0}
          />
        </div>

        {/* Arrow */}
        <div
          style={{
            fontFamily: fontMono,
            fontSize: 32,
            color: GOLD,
            opacity: labelOpacity,
            alignSelf: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span>\u2192</span>
          <span
            style={{ fontSize: 14, color: DIM_GOLD, letterSpacing: "1px" }}
          >
            PHASE
            <br />
            TRANSITION
          </span>
        </div>

        {/* After */}
        <div style={{ flex: 1, opacity: afterOpacity }}>
          <div
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: GOLD,
              marginBottom: 16,
              letterSpacing: "1px",
            }}
          >
            EPOCH 2
          </div>
          <TerminalBlock
            lines={[
              {
                text: "\u25c7 launch private beta by April 1",
                color: GOLD,
              },
              { text: "  \u251c\u2500\u2500 finish Stripe integration", color: WHITE },
              {
                text: "  \u251c\u2500\u2500 recruit 5 testers directly",
                color: WHITE,
              },
              { text: "  \u2514\u2500\u2500 skip docs, ship FAQ", color: WHITE },
              { text: "" },
              {
                text: "\u25c6 API works, payment needs 2 days",
                color: GREEN,
              },
            ]}
            startDelay={55}
          />
        </div>
      </div>

      <FadeText
        delay={85}
        style={{
          fontFamily: fontMono,
          fontSize: 22,
          color: DIM_GOLD,
          textAlign: "center",
          marginTop: 40,
        }}
      >
        Plans aren't contracts. They're hypotheses.
        <br />
        When you learn, the structure mutates.
      </FadeText>
    </AbsoluteFill>
  );
};

const SceneMRP: React.FC = () => {
  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
        padding: "0 120px",
      }}
    >
      <div style={{ maxWidth: 1000 }}>
        <FadeText
          style={{
            fontFamily: fontMono,
            fontSize: 44,
            color: WHITE,
            textAlign: "center",
            lineHeight: 1.4,
          }}
        >
          MRP for directed action.
        </FadeText>
        <FadeText
          delay={30}
          style={{
            fontFamily: fontMono,
            fontSize: 24,
            color: DIM_GOLD,
            textAlign: "center",
            lineHeight: 1.8,
            marginTop: 40,
          }}
        >
          It doesn't replace your execution tools.
          <br />
          It tells them when to stop executing
          <br />
          and start rethinking.
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

const SceneClosing: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const glyphScale = spring({
    frame: frame - 5,
    fps,
    config: { damping: 25, stiffness: 60 },
  });

  const glyphOpacity = interpolate(frame, [0, 15], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        justifyContent: "center",
        alignItems: "center",
      }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 30,
        }}
      >
        {/* Glyph */}
        <div
          style={{
            fontFamily: fontMono,
            fontSize: 120,
            color: GOLD,
            opacity: glyphOpacity,
            transform: `scale(${glyphScale})`,
          }}
        >
          \u25c7
        </div>

        {/* Title */}
        <FadeText
          delay={15}
          style={{
            fontFamily: fontMono,
            fontSize: 72,
            color: WHITE,
            letterSpacing: "0.15em",
          }}
        >
          werk
        </FadeText>

        {/* Subtitle */}
        <FadeText
          delay={30}
          style={{
            fontFamily: fontMono,
            fontSize: 22,
            color: DIM_GOLD,
            textAlign: "center",
            lineHeight: 1.8,
          }}
        >
          Open format. Terminal-native. Agent-ready.
        </FadeText>

        {/* Divider */}
        <FadeText delay={50} style={{}}>
          <div
            style={{
              width: 60,
              height: 1,
              backgroundColor: FAINT_GOLD,
              margin: "10px 0",
            }}
          />
        </FadeText>

        {/* Attribution */}
        <FadeText
          delay={60}
          style={{
            fontFamily: fontMono,
            fontSize: 18,
            color: FAINT_GOLD,
            textAlign: "center",
            lineHeight: 1.8,
          }}
        >
          The intent layer your execution tools are missing.
          <br />
          Built with gratitude for the open source AI community.
        </FadeText>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const BLINDSPOT_DURATION = 30 * 80; // 80 seconds at 30fps

export const BlindSpot: React.FC = () => {
  const s = (seconds: number) => seconds * 30;

  // Music: fade in at start, fade out at end
  // Replace hildegard.mp3 with your chosen track
  const musicVolume = (f: number) => {
    const fadeIn = interpolate(f, [0, s(3)], [0, 0.25], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    });
    const fadeOut = interpolate(
      f,
      [BLINDSPOT_DURATION - s(4), BLINDSPOT_DURATION],
      [0.25, 0],
      { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );
    return Math.min(fadeIn, fadeOut);
  };

  return (
    <AbsoluteFill style={{ backgroundColor: BG }}>
      {/* Background music — swap the file as needed */}
      <Audio src={staticFile("hildegard.mp3")} volume={musicVolume} startFrom={30 * 30} />

      {/* Scene 1: Title — 5s */}
      <Sequence from={0} durationInFrames={s(5)}>
        <SceneTitle />
      </Sequence>

      {/* Scene 2: Stale plan — 9s */}
      <Sequence from={s(5)} durationInFrames={s(9)}>
        <SceneStalePlan />
      </Sequence>

      {/* Scene 3: The problem — 8s */}
      <Sequence from={s(14)} durationInFrames={s(8)}>
        <SceneProblem />
      </Sequence>

      {/* Scene 4: werk tree — 9s */}
      <Sequence from={s(22)} durationInFrames={s(9)}>
        <SceneWerkTree />
      </Sequence>

      {/* Scene 5: Desire / Reality — 10s */}
      <Sequence from={s(31)} durationInFrames={s(10)}>
        <SceneDesireReality />
      </Sequence>

      {/* Scene 6: Agent context — 10s */}
      <Sequence from={s(41)} durationInFrames={s(10)}>
        <SceneAgentContext />
      </Sequence>

      {/* Scene 7: Epoch transition — 12s */}
      <Sequence from={s(51)} durationInFrames={s(12)}>
        <SceneEpoch />
      </Sequence>

      {/* Scene 8: MRP — 7s */}
      <Sequence from={s(63)} durationInFrames={s(7)}>
        <SceneMRP />
      </Sequence>

      {/* Scene 9: Closing — 10s */}
      <Sequence from={s(70)} durationInFrames={s(10)}>
        <SceneClosing />
      </Sequence>
    </AbsoluteFill>
  );
};
