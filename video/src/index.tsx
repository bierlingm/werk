import React from "react";
import { Composition, registerRoot } from "remotion";
import { BlindSpot, BLINDSPOT_DURATION } from "./BlindSpot";
import { IntentLayer, INTENTLAYER_DURATION } from "./IntentLayer";
import { V01_TheGap, V01_DURATION } from "./V01_TheGap";
import { V02_Stale, V02_DURATION } from "./V02_Stale";
import { V03_WhatYourAgentDoesntKnow, V03_DURATION } from "./V03_WhatYourAgentDoesntKnow";
import { V04_GhostGeometry, V04_DURATION } from "./V04_GhostGeometry";
import { V05_ClockworkDeity, V05_DURATION } from "./V05_ClockworkDeity";
import { V06_CoherenceIsScarce, V06_DURATION } from "./V06_CoherenceIsScarce";
import { V07_NotATodoList, V07_DURATION } from "./V07_NotATodoList";
import { V08_OneSession, V08_DURATION } from "./V08_OneSession";
import { V09_StructureDeterminesBehavior, V09_DURATION } from "./V09_StructureDeterminesBehavior";
import { V10_TheOperativeInstrument, V10_DURATION } from "./V10_TheOperativeInstrument";
import { VibeCoder, VIBECODER_DURATION } from "./VibeCoder";
import { VibeCoder2, VIBECODER2_DURATION } from "./VibeCoder2";
import { VibeCoder3, VIBECODER3_DURATION } from "./VibeCoder3";
import { VibeCoder4, VIBECODER4_DURATION } from "./VibeCoder4";
import { VibeCoder5, VIBECODER5_DURATION } from "./VibeCoder5";
import { VibeCoder6, VIBECODER6_DURATION } from "./VibeCoder6";
import { VibeCoder7, VIBECODER7_DURATION } from "./VibeCoder7";

const FPS = 30;

const Root: React.FC = () => {
  return (
    <>
      <Composition id="BlindSpot" component={BlindSpot} durationInFrames={BLINDSPOT_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="IntentLayer" component={IntentLayer} durationInFrames={INTENTLAYER_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v01-the-gap" component={V01_TheGap} durationInFrames={V01_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v02-stale" component={V02_Stale} durationInFrames={V02_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v03-agent-context" component={V03_WhatYourAgentDoesntKnow} durationInFrames={V03_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v04-ghost-geometry" component={V04_GhostGeometry} durationInFrames={V04_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v05-coherence" component={V05_ClockworkDeity} durationInFrames={V05_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v06-scarce" component={V06_CoherenceIsScarce} durationInFrames={V06_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v07-not-a-todo" component={V07_NotATodoList} durationInFrames={V07_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v08-one-session" component={V08_OneSession} durationInFrames={V08_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v09-structure" component={V09_StructureDeterminesBehavior} durationInFrames={V09_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="v10-instrument" component={V10_TheOperativeInstrument} durationInFrames={V10_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder" component={VibeCoder} durationInFrames={VIBECODER_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-2" component={VibeCoder2} durationInFrames={VIBECODER2_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-3" component={VibeCoder3} durationInFrames={VIBECODER3_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-4" component={VibeCoder4} durationInFrames={VIBECODER4_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-5" component={VibeCoder5} durationInFrames={VIBECODER5_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-6" component={VibeCoder6} durationInFrames={VIBECODER6_DURATION} fps={FPS} width={1920} height={1080} />
      <Composition id="vibe-coder-7" component={VibeCoder7} durationInFrames={VIBECODER7_DURATION} fps={FPS} width={1920} height={1080} />
    </>
  );
};

registerRoot(Root);
