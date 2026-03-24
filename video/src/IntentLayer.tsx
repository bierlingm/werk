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
const BLUE = "#5a7a9a";

const fontMono = `'Berkeley Mono', 'Menlo', monospace`;

// ─── Primitives ─────────────────────────────────────────────────────

const Fade: React.FC<{
  children: React.ReactNode;
  delay?: number;
  style?: React.CSSProperties;
}> = ({ children, delay = 0, style }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const opacity = interpolate(
    frame,
    [delay, delay + 18, durationInFrames - 15, durationInFrames],
    [0, 1, 1, 0],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  const y = interpolate(frame, [delay, delay + 22], [10, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <div style={{ opacity, transform: `translateY(${y}px)`, ...style }}>
      {children}
    </div>
  );
};

const TypeLine: React.FC<{
  text: string;
  color?: string;
  delay?: number;
  fontSize?: number;
}> = ({ text, color = DIM_WHITE, delay = 0, fontSize = 22 }) => {
  const frame = useCurrentFrame();

  const charsVisible = interpolate(
    frame,
    [delay, delay + text.length * 0.8],
    [0, text.length],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  const cursorOpacity =
    charsVisible < text.length
      ? frame % 16 < 8
        ? 1
        : 0.3
      : interpolate(frame, [delay + text.length * 0.8, delay + text.length * 0.8 + 15], [1, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        });

  return (
    <div
      style={{
        fontFamily: fontMono,
        fontSize,
        color,
        whiteSpace: "pre",
      }}
    >
      {text.slice(0, Math.floor(charsVisible))}
      <span style={{ opacity: cursorOpacity, color: GOLD }}>_</span>
    </div>
  );
};

// ─── Scenes ─────────────────────────────────────────────────────────

// Scene 1: The question nobody asks
const SceneQuestion: React.FC = () => {
  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 140px" }}
    >
      <Fade
        style={{
          fontFamily: fontMono,
          fontSize: 26,
          color: DIM_WHITE,
          textAlign: "center",
          lineHeight: 2.0,
        }}
      >
        Your agent can write 10,000 lines of code in an hour.
      </Fade>
      <Fade
        delay={40}
        style={{
          fontFamily: fontMono,
          fontSize: 26,
          color: WHITE,
          textAlign: "center",
          lineHeight: 2.0,
          marginTop: 10,
        }}
      >
        But can it tell you whether any of it matters?
      </Fade>
    </AbsoluteFill>
  );
};

// Scene 2: What memory gets wrong
const SceneMemory: React.FC = () => {
  const frame = useCurrentFrame();

  const strikethrough = interpolate(frame, [70, 90], [0, 100], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const replacementOpacity = interpolate(frame, [95, 115], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}
    >
      <div style={{ maxWidth: 900, textAlign: "center" }}>
        <Fade
          delay={5}
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            letterSpacing: "2px",
            marginBottom: 40,
          }}
        >
          THE AI ECOSYSTEM SAYS:
        </Fade>
        <Fade
          delay={20}
          style={{
            fontFamily: fontMono,
            fontSize: 38,
            color: WHITE,
            lineHeight: 1.6,
            position: "relative",
          }}
        >
          <span>
            "Agents need better{" "}
            <span style={{ position: "relative", display: "inline-block" }}>
              memory
              <div
                style={{
                  position: "absolute",
                  top: "50%",
                  left: 0,
                  width: `${strikethrough}%`,
                  height: 2,
                  backgroundColor: RED,
                }}
              />
            </span>
            "
          </span>
        </Fade>
        <div style={{ opacity: replacementOpacity, marginTop: 40 }}>
          <div
            style={{
              fontFamily: fontMono,
              fontSize: 38,
              color: GOLD,
              lineHeight: 1.6,
            }}
          >
            Agents need structure.
          </div>
          <div
            style={{
              fontFamily: fontMono,
              fontSize: 20,
              color: DIM_GOLD,
              marginTop: 30,
              lineHeight: 1.8,
            }}
          >
            Memory accumulates what happened.
            <br />
            Structure maintains the relationship between
            <br />
            where you are and where you're going.
          </div>
        </div>
      </div>
    </AbsoluteFill>
  );
};

// Scene 3: The biological truth
const SceneBiology: React.FC = () => {
  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}
    >
      <div style={{ maxWidth: 1000 }}>
        <Fade
          delay={5}
          style={{
            fontFamily: fontMono,
            fontSize: 18,
            color: FAINT_GOLD,
            letterSpacing: "1.5px",
            marginBottom: 30,
          }}
        >
          NEUROSCIENCE AGREES
        </Fade>
        {[
          { text: "The hippocampus runs a generative model, not a filing cabinet.", source: "Greve et al., 2020", delay: 15 },
          { text: "Memories are active agents in sense-making, not stored records.", source: "Levin, 2024", delay: 45 },
          { text: "Each cortical column maintains structural relationships, not flat data.", source: "Hawkins, 2021", delay: 75 },
        ].map((item, i) => (
          <Fade
            key={i}
            delay={item.delay}
            style={{ marginBottom: 28 }}
          >
            <div
              style={{
                fontFamily: fontMono,
                fontSize: 24,
                color: WHITE,
                lineHeight: 1.6,
              }}
            >
              {item.text}
            </div>
            <div
              style={{
                fontFamily: fontMono,
                fontSize: 16,
                color: DIM_GOLD,
                marginTop: 4,
              }}
            >
              — {item.source}
            </div>
          </Fade>
        ))}
      </div>
    </AbsoluteFill>
  );
};

// Scene 4: What a tension is — typed out live
const SceneTension: React.FC = () => {
  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}
    >
      <div style={{ maxWidth: 1100, width: "100%" }}>
        <TypeLine
          text="$ werk add"
          color={DIM_GOLD}
          delay={5}
          fontSize={20}
        />
        <div style={{ height: 20 }} />
        <div
          style={{
            fontFamily: fontMono,
            fontSize: 22,
            lineHeight: 1.7,
            padding: "40px 60px",
            backgroundColor: "#111110",
            borderRadius: 8,
            border: `1px solid ${FAINT_GOLD}`,
          }}
        >
          <TypeLine
            text='  Desired: "the product serves 100 paying users"'
            color={GOLD}
            delay={30}
          />
          <div style={{ height: 8 }} />
          <TypeLine
            text='  Reality: "prototype works locally, no users, no payments"'
            color={DIM_WHITE}
            delay={70}
          />
          <div style={{ height: 20 }} />
          <TypeLine
            text="✓ Created tension #1"
            color={GREEN}
            delay={115}
          />
        </div>
        <Fade
          delay={130}
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            textAlign: "center",
            marginTop: 30,
            lineHeight: 1.8,
          }}
        >
          Not a task. Not a goal. A structural tension.
          <br />
          The gap generates the energy. The children compose the bridge.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// Scene 5: Two users, same instrument
const SceneTwoUsers: React.FC = () => {
  const frame = useCurrentFrame();

  const dividerWidth = interpolate(frame, [0, 30], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 80px" }}
    >
      <div
        style={{
          display: "flex",
          gap: 80,
          maxWidth: 1200,
          width: "100%",
          alignItems: "flex-start",
        }}
      >
        {/* Human side */}
        <div style={{ flex: 1 }}>
          <Fade
            delay={5}
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: GOLD,
              letterSpacing: "1.5px",
              marginBottom: 20,
            }}
          >
            THE PRACTITIONER
          </Fade>
          <Fade
            delay={20}
            style={{
              fontFamily: fontMono,
              fontSize: 20,
              color: WHITE,
              lineHeight: 1.8,
            }}
          >
            Opens the TUI.
            <br />
            Navigates the tension structure.
            <br />
            Updates reality. Evolves desire.
            <br />
            Sees patterns in their own process.
          </Fade>
          <Fade
            delay={50}
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: DIM_GOLD,
              marginTop: 20,
              lineHeight: 1.8,
            }}
          >
            The instrument shapes cognition
            <br />
            toward the user's aims.
          </Fade>
        </div>

        {/* Divider */}
        <div
          style={{
            width: 1,
            height: 300,
            backgroundColor: FAINT_GOLD,
            opacity: dividerWidth,
            alignSelf: "center",
          }}
        />

        {/* Agent side */}
        <div style={{ flex: 1 }}>
          <Fade
            delay={15}
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: BLUE,
              letterSpacing: "1.5px",
              marginBottom: 20,
            }}
          >
            THE AGENT
          </Fade>
          <Fade
            delay={30}
            style={{
              fontFamily: fontMono,
              fontSize: 20,
              color: WHITE,
              lineHeight: 1.8,
            }}
          >
            Reads `werk context --json`.
            <br />
            Knows what matters right now.
            <br />
            Resolves steps. Notes learnings.
            <br />
            Maintains structure autonomously.
          </Fade>
          <Fade
            delay={60}
            style={{
              fontFamily: fontMono,
              fontSize: 16,
              color: DIM_GOLD,
              marginTop: 20,
              lineHeight: 1.8,
            }}
          >
            The user never touches the CLI.
            <br />
            The agent holds coherence.
          </Fade>
        </div>
      </div>

      <Fade
        delay={80}
        style={{
          fontFamily: fontMono,
          fontSize: 22,
          color: GOLD,
          textAlign: "center",
          marginTop: 50,
        }}
      >
        Same data. Same structure. Two ways in.
      </Fade>
    </AbsoluteFill>
  );
};

// Scene 6: The factory metaphor
const SceneFactory: React.FC = () => {
  const frame = useCurrentFrame();

  const layers = [
    { label: "INTENT", desc: "What needs to be true?", color: GOLD, tool: "werk" },
    { label: "EXECUTION", desc: "What should agents do next?", color: WHITE, tool: "beads, taskmaster, ..." },
    { label: "CODE", desc: "What changed?", color: DIM_WHITE, tool: "git" },
  ];

  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}
    >
      <div style={{ maxWidth: 900, width: "100%" }}>
        <Fade
          delay={0}
          style={{
            fontFamily: fontMono,
            fontSize: 18,
            color: FAINT_GOLD,
            letterSpacing: "1.5px",
            marginBottom: 30,
            textAlign: "center",
          }}
        >
          MRP TRANSFORMED MANUFACTURING. THIS IS MRP FOR KNOWLEDGE WORK.
        </Fade>

        {layers.map((layer, i) => {
          const layerDelay = 20 + i * 35;
          const opacity = interpolate(
            frame,
            [layerDelay, layerDelay + 15],
            [0, 1],
            { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
          );

          const highlight = i === 0;

          return (
            <div
              key={i}
              style={{
                opacity,
                display: "flex",
                alignItems: "center",
                padding: "20px 30px",
                marginBottom: 12,
                backgroundColor: highlight ? "#1a1608" : "#111110",
                borderRadius: 8,
                border: `1px solid ${highlight ? GOLD : FAINT_GOLD}`,
              }}
            >
              <div
                style={{
                  fontFamily: fontMono,
                  fontSize: 14,
                  color: layer.color,
                  letterSpacing: "2px",
                  width: 140,
                  flexShrink: 0,
                }}
              >
                {layer.label}
              </div>
              <div
                style={{
                  fontFamily: fontMono,
                  fontSize: 20,
                  color: WHITE,
                  flex: 1,
                }}
              >
                {layer.desc}
              </div>
              <div
                style={{
                  fontFamily: fontMono,
                  fontSize: 16,
                  color: DIM_GOLD,
                  flexShrink: 0,
                }}
              >
                {layer.tool}
              </div>
            </div>
          );
        })}

        <Fade
          delay={130}
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            textAlign: "center",
            marginTop: 30,
            lineHeight: 1.8,
          }}
        >
          Without the intent layer, execution is efficient but directionless.
          <br />
          With it, every tool knows what it's for.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// Scene 7: The living model
const SceneLiving: React.FC = () => {
  const frame = useCurrentFrame();

  const items = [
    { icon: "\u25c7", text: "Updates through use, not maintenance", delay: 10 },
    { icon: "\u25c6", text: "Exports to JSON. Commits to git. Diffs show structural evolution.", delay: 40 },
    { icon: "\u25c8", text: "Your agent reads it. Your coach reads it. You read it.", delay: 70 },
    { icon: "\u25c9", text: "Silence is the default. Signal by exception.", delay: 100 },
  ];

  return (
    <AbsoluteFill
      style={{ justifyContent: "center", alignItems: "center", padding: "0 100px" }}
    >
      <div style={{ maxWidth: 900 }}>
        <Fade
          delay={0}
          style={{
            fontFamily: fontMono,
            fontSize: 18,
            color: FAINT_GOLD,
            letterSpacing: "1.5px",
            marginBottom: 35,
          }}
        >
          A LIVING STRUCTURAL MODEL
        </Fade>
        {items.map((item, i) => {
          const opacity = interpolate(
            frame,
            [item.delay, item.delay + 15],
            [0, 1],
            { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
          );
          return (
            <div
              key={i}
              style={{
                opacity,
                display: "flex",
                alignItems: "flex-start",
                gap: 20,
                marginBottom: 24,
              }}
            >
              <div
                style={{
                  fontFamily: fontMono,
                  fontSize: 24,
                  color: GOLD,
                  flexShrink: 0,
                  marginTop: 2,
                }}
              >
                {item.icon}
              </div>
              <div
                style={{
                  fontFamily: fontMono,
                  fontSize: 22,
                  color: WHITE,
                  lineHeight: 1.6,
                }}
              >
                {item.text}
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

// Scene 8: Closing — different tone
const SceneEnd: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const glyphScale = spring({
    frame: frame - 5,
    fps,
    config: { damping: 30, stiffness: 50 },
  });

  const glyphOpacity = interpolate(frame, [0, 20], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 25,
        }}
      >
        <div
          style={{
            fontFamily: fontMono,
            fontSize: 100,
            color: GOLD,
            opacity: glyphOpacity,
            transform: `scale(${glyphScale})`,
          }}
        >
          {"\u25c7"}
        </div>

        <Fade
          delay={20}
          style={{
            fontFamily: fontMono,
            fontSize: 64,
            color: WHITE,
            letterSpacing: "0.15em",
          }}
        >
          werk
        </Fade>

        <Fade
          delay={40}
          style={{
            fontFamily: fontMono,
            fontSize: 20,
            color: DIM_GOLD,
            textAlign: "center",
            lineHeight: 2.0,
          }}
        >
          Structure determines behavior.
          <br />
          Build the structure that determines yours.
        </Fade>

        <Fade delay={70} style={{}}>
          <div
            style={{
              width: 60,
              height: 1,
              backgroundColor: FAINT_GOLD,
              margin: "15px 0",
            }}
          />
        </Fade>

        <Fade
          delay={80}
          style={{
            fontFamily: fontMono,
            fontSize: 16,
            color: FAINT_GOLD,
            textAlign: "center",
            lineHeight: 1.8,
          }}
        >
          Open format. Terminal-native. Agent-ready.
          <br />
          From the structural dynamics tradition.
        </Fade>
      </div>
    </AbsoluteFill>
  );
};

// ─── Main Composition ───────────────────────────────────────────────

export const INTENTLAYER_DURATION = 30 * 90; // 90 seconds

export const IntentLayer: React.FC = () => {
  const s = (seconds: number) => seconds * 30;

  const musicVolume = (f: number) => {
    const fadeIn = interpolate(f, [0, s(4)], [0, 0.2], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    });
    const fadeOut = interpolate(
      f,
      [INTENTLAYER_DURATION - s(5), INTENTLAYER_DURATION],
      [0.2, 0],
      { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
    );
    return Math.min(fadeIn, fadeOut);
  };

  return (
    <AbsoluteFill style={{ backgroundColor: BG }}>
      <Audio src={staticFile("hildegard.mp3")} volume={musicVolume} startFrom={45 * 30} />

      {/* Scene 1: The question — 6s */}
      <Sequence from={0} durationInFrames={s(6)}>
        <SceneQuestion />
      </Sequence>

      {/* Scene 2: Memory vs structure — 11s */}
      <Sequence from={s(6)} durationInFrames={s(11)}>
        <SceneMemory />
      </Sequence>

      {/* Scene 3: Biology — 10s */}
      <Sequence from={s(17)} durationInFrames={s(10)}>
        <SceneBiology />
      </Sequence>

      {/* Scene 4: What a tension is — 12s */}
      <Sequence from={s(27)} durationInFrames={s(12)}>
        <SceneTension />
      </Sequence>

      {/* Scene 5: Two users — 11s */}
      <Sequence from={s(39)} durationInFrames={s(11)}>
        <SceneTwoUsers />
      </Sequence>

      {/* Scene 6: The factory / MRP — 12s */}
      <Sequence from={s(50)} durationInFrames={s(12)}>
        <SceneFactory />
      </Sequence>

      {/* Scene 7: Living model — 10s */}
      <Sequence from={s(62)} durationInFrames={s(10)}>
        <SceneLiving />
      </Sequence>

      {/* Scene 8: Closing — 18s */}
      <Sequence from={s(72)} durationInFrames={s(18)}>
        <SceneEnd />
      </Sequence>
    </AbsoluteFill>
  );
};
