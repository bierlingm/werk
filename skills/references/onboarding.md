# Onboarding Guide

When someone is new to werk, walk them through this sequence. One step at a time.

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
werk tree
```

They see their tension. Desire on one line, reality implied by the gap. Most people have never seen this explicitly — the structural tension between what they want and what's true, held in a form they can return to.

## Step 3: Theory of Closure

Ask: "What would need to be true for this to happen?" Those are children — the composed bridge from reality to desire.

```bash
werk add -p <id> "Training plan chosen and printed" "Haven't researched plans yet"
werk add -p <id> "Run 3x/week for 4 consecutive weeks" "Currently running 1x/week"
```

These are hypotheses, not commitments. They may be wrong. They exist to make the theory of closure explicit and revisable.

## Step 4: Update Reality (SITREP)

Something changes in the real world. Record it:

```bash
werk reality <id> "Chose Hal Higdon Novice 1 plan. Printed. On the fridge."
```

This is one of the most important gestures — grounding the instrument in what's actually happening. The quality of this compression (honesty, precision, completeness) affects every downstream interpretation.

## Step 5: The Frontier

As steps get resolved, the frontier of action advances. The operating envelope — the window around the frontier — shows what's action-relevant now: overdue steps, the next committed step, held steps awaiting commitment, recently resolved steps.

The envelope is where the user lands on opening. Everything else radiates outward from it.

## What NOT to Do

- Don't explain all dynamics up front. Let them discover through use.
- Don't create 20 tensions on day one. Start with 1-3.
- Don't push horizons. Some tensions don't need deadlines.
- Don't resolve things for them. The human decides.
- Don't treat werk like a task manager. It holds structural tension as creative force, not checkboxes.
