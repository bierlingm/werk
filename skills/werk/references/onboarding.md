# Onboarding Guide

When someone is new to werk, walk them through this sequence. Don't dump everything at once. One step at a time.

## Step 1: The First Tension

Ask: "What do you want that you don't have?"

Help them be specific. Not "be healthier" — "Run a marathon by July." Not "make more money" — "Have 48k in savings by end of year."

Then ask: "What's actually true right now?" Help them be honest. Concrete facts, not narratives.

```bash
werk init
werk add "Run a marathon by July" "Can run 5k. Haven't trained consistently in months."
```

## Step 2: See It

```bash
werk
```

Open the TUI. They see their tension with a ◇ glyph (germination — new, still forming). Press Space to gaze. They see desire and reality facing each other. The gap bar.

This is the moment: desire on one line, reality on the other. The gap computed. Most people have never seen this explicitly.

## Step 3: The Tree

Ask: "What would need to be true for this to happen?" Those are children.

```bash
werk add -p <id> "Training plan chosen and printed" "Haven't researched plans yet"
werk add -p <id> "Run 3x/week for 4 consecutive weeks" "Currently running 1x/week"
```

Now press `l` in the TUI to descend. The children appear. The parent is the header.

## Step 4: Update Reality

Something changes in the real world. Record it:

```bash
werk reality <id> "Chose Hal Higdon Novice 1 plan. Printed. On the fridge."
```

The TUI updates. The activity trail gets a ● dot. The dynamics start computing.

## Step 5: The Agent

When they're stuck or want perspective:

In the TUI: press `@` on a tension, type "What pattern do you see?"

Or from CLI:
```bash
werk run <id> "I keep skipping my long runs. What's going on?"
```

The agent receives all 13 dynamics and responds within the structure. Not generic advice — structural observation.

## Step 6: The Daimon

When they're ready for ambient monitoring:

```bash
werk watch --daemon
```

The daimon watches. When neglect sets in, when oscillation spikes, when a horizon passes — it notices and stores an observation. Next time they open werk, the insights are waiting.

## What NOT to Do

- Don't explain all 13 dynamics up front. Let them discover.
- Don't create 20 tensions on day one. Start with 1-3.
- Don't push horizons. Some tensions don't need deadlines.
- Don't resolve things for them. The human decides.
- Don't treat werk like a task manager. It's not about checking boxes. It's about holding the gap between desire and reality as a creative force.
