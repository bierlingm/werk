# Anima: Predictive Identity Memory for Claude Code

A hook-driven identity loop that reads a self-model on session start, generates predictions, and writes back an evolved self-model on session end — updating on surprise, not just accumulation.

## Design Decisions

**Architecture**: SessionStart injects self-model + generates predictions → session work happens → Stop checks predictions against evidence, synthesizes, writes back. The prediction→observation→surprise loop is what makes this a *model*, not a journal.

**Language**: Python for both hooks. Bash heredocs with variable interpolation are a corruption vector. Python gives proper JSON handling, error recovery, and backup-before-overwrite.

**Scope**: Global `~/werk/self.md` only. Per-project self-models are a future upgrade — prove the loop works first.

**Model**: `claude -p --model haiku` for synthesis and prediction. Cheap, fast, sufficient for conservative updates.

**No**: affect scoring, VADER, `working_self.json`, epoch JSON indexes, `goals.yaml`. All premature. Goals are already tracked in br. Emotional weighting on procedural coding sessions is noise.

**Yes**: `[identity]` beat convention for manual salience marking during sessions. Gives the Stop hook higher-signal input than "all beats from today."

**Prediction loop**: SessionStart generates 1-2 testable predictions from Active Tensions and Recent Shifts, stores them in a tempfile. Stop hook checks predictions against session evidence. Confirmed predictions weakly reinforce Core Patterns. Violated predictions are strong signal for Recent Shifts. This is what makes it a *model* — it predicts, observes, and updates on surprise.

**Calibration tracking**: Each session's prediction outcomes (confirmed/violated/unobservable) are appended to `~/.claude/.anima-calibration.jsonl`. The start hook summarizes the last 10 sessions' hit rate and feeds it to haiku so predictions target poorly-calibrated areas. >90% hit rate = predictions too safe, probe harder. <40% = model is wrong about active tensions. This is what makes the loop *compound* — each session's calibration makes the next session's predictions better.

**Micro-experiments**: The start hook doesn't just predict — it can propose a tiny behavioral experiment tied to the least-resolved Active Tension. Instead of "P: will choose the simpler option" (passive observation), it generates "E: when X comes up, try Y instead of your usual Z — probes tension between A and B" (active perturbation). The stop hook evaluates: did you run it? What happened? Experiment outcomes are stronger signal than predictions because they ask you to do something *different*, guaranteeing surprise-data. The system chooses experiments when tensions are stale (no movement in 3+ sessions) and predictions when tensions are fresh (already shifting). This is the difference between a mirror and a practice — predictions refine a description, experiments refine *you*.

**Tension lifecycle**: Tensions have a lifecycle: `emerged → active → dissolving → resolved`. When predictions about a tension consistently confirm one side (3+ sessions), the synthesis prompt migrates the "winning" side to Core Patterns and moves the tension to `## Resolved Tensions` (one-line with date). This prevents tension accumulation — the model *converges* instead of bloating. Resolved Tensions double as a growth log: a record of contradictions you've actually worked through. The calibration JSONL already contains per-tension signal; the synthesis prompt just needs to be told to look for it.

**Meta-synthesis (structural review)**: Every 10 sessions, the stop hook fires an additional haiku call that reads the *full* calibration JSONL and proposes structural moves across self-model sections. This is what prevents the self-model from silently going stale — content updates happen every session, but *structural reorganization* happens periodically with the full history in view. The meta-synthesis proposes four kinds of moves:

- **Recent Shift → Core Pattern**: A shift that's been confirmed 4+ sessions in a row. It's not shifting anymore — promote it.
- **Active Tension → Resolved**: Predictions targeting this tension consistently confirm one side (3+ sessions). The tension is fake or settled. Dissolve it (per tension lifecycle above).
- **Core Pattern → Active Tension**: A pattern that used to confirm reliably has started producing violated predictions. Something is destabilizing. Surface it as a new tension.
- **Experiment outcomes → New Tension**: Two or more experiments produced shifts in opposite directions on the same dimension. There's a real contradiction that wasn't articulated. Name it.

Move proposals are injected as advisory context into the regular synthesis prompt — haiku still decides, but with structural awareness. The meta-synthesis fires on `len(calibration_lines) % 10 == 0` and costs one extra haiku call. The payoff is that the self-model's *topology* compounds, not just its content.

**Behavioral audit (blind spot detection)**: The prediction loop has a selection bias — it tests Active Tensions and Recent Shifts (the parts already flagged as uncertain). Core Patterns are treated as stable ground and rarely challenged. A pattern like "values simplicity" can survive indefinitely while beats consistently show the opposite, because predictions target what's *uncertain*, not what's *assumed settled*. The behavioral audit closes this gap by checking claims against raw behavior, not prediction outcomes.

Every 15-20 sessions (`len(calibration_lines) % 15 == 5`, offset from meta-synthesis so they don't stack), the stop hook runs a separate haiku call with the last 20 sessions of raw beats — not calibration data, actual behavior. The prompt asks three questions:

- **Contradicted Patterns**: Which Core Patterns are contradicted by *repeated* behavior (not a single violation — a trend)?
- **Missing Patterns**: What consistent behaviors appear in beats that have no corresponding Core Pattern? (The unlabeled habits.)
- **Shadow Tensions**: What contradictions exist in observed behavior that aren't captured as Active Tensions?

Output is advisory lines injected into the next session's synthesis prompt, same mechanism as meta-synthesis. Haiku still decides what to act on. The key difference from meta-synthesis: meta-synthesis reads *calibration history* (prediction outcomes), the behavioral audit reads *raw beats* (actual behavior). Different epistemic channels — predictions refine what you already know is uncertain, audits surface what you don't know you're wrong about. Without it, anima converges on your self-image. With it, it converges on truth.

**Distinctiveness probes**: Every 5 sessions, the start hook generates a **distinctiveness probe** instead of a prediction or experiment. It asks haiku: "Generate one prediction that would be TRUE for this person but FALSE for a generic competent developer" — something the self-model specifically implies that isn't just universal good practice. The stop hook evaluates distinctiveness probes with a different rubric:

- **Confirmed + distinctive**: This part of the model is load-bearing — it captures something real and specific. No action needed.
- **Confirmed but generic**: Flag the relevant Core Pattern as potentially hollow. Anyone would confirm "values clean code." That's not a pattern, it's a platitude.
- **Violated + distinctive**: Strong update signal, same as regular violated predictions.
- **Hollow pattern accumulation**: After 3+ "generic" flags on the same Core Pattern, the synthesis prompt is told to either sharpen it into something actually distinctive or demote it to a comment.

This creates a second optimization pressure the system otherwise lacks. Accuracy pressure asks: do predictions match behavior? Distinctiveness pressure asks: even if accurate, is this model capturing what makes you *specifically you*, or is it converging on comfortable generality? The implementation is a third mode in the start hook (`type: "distinctiveness"` in calibration JSONL), triggered on `session_count % 5 == 0`. A self-model that's accurate but not distinctive is a horoscope. This prevents that.

**Directive synthesis (description → prescription)**: The prediction loop makes the self-model *accurate*. Distinctiveness probes make it *specific*. But neither changes how Claude actually collaborates — the self-model is injected as passive context, read like any other document. Directive synthesis closes this gap by auto-generating a `## Interaction Directives` section in `self.md`: 3-5 concrete behavioral rules derived from Core Patterns, Active Tensions, and Resolved Tensions that tell Claude *how to behave differently* with this person.

Directives are not descriptions ("values simplicity") — they're instructions with observable consequences ("When I propose two architectures, stress-test the simpler one harder — I gravitate toward overengineering under uncertainty"). Each directive cites the pattern or tension it derives from, making the reasoning auditable. The synthesis prompt already runs every session; directive generation is an additional output section, not a new hook or scheduling mechanism.

Directives are tracked in calibration JSONL as `type: "directive"`. Each session, the stop hook checks: was any directive triggered? If so, did following it produce a better outcome than ignoring it would have? Outcomes are `followed_helped`, `followed_hurt`, `not_triggered`, or `overridden` (Claude or user explicitly chose against it). After 3+ `followed_hurt` on the same directive, the synthesis prompt drops or revises it. After 3+ `overridden`, it's clearly wrong about the pattern — strong signal for Recent Shifts.

Hollow directives are caught by the existing distinctiveness probe system. "Prefer simplicity" is generic — anyone would want that. The system already flags platitudes; directives inherit that pressure. Directives that survive are the ones that produce visibly different behavior when followed vs. ignored.

This is what makes anima compound the *working relationship*, not just the model. After 50 sessions, Claude hasn't just learned who you are — it's adapted how it works with you in ways a fresh instance couldn't replicate. The directives are the value; the description is the scaffold.

**Adversarial counter-modeling (honesty pressure)**: Every 7 sessions (`session_count % 7 == 3`, offset from other periodic hooks), the start hook generates an **adversarial counter-model** — a one-paragraph alternative interpretation of the same Core Patterns and behavioral evidence that is equally consistent with the data but *less flattering or more uncomfortable*. This is the only mechanism that fights the fundamental bias in self-modeling: every other data source is self-reported, so every other mechanism refines accuracy *within the frame the user set*. The adversarial counter-model challenges the frame itself.

Examples of what adversarial reframing produces:
- Pattern says "prefers simplicity" → Counter: "avoids complexity when it requires learning something new — risk aversion dressed as taste"
- Pattern says "values autonomy" → Counter: "resists feedback and collaboration when it threatens self-image as the competent one"
- Tension says "depth vs. breadth" → Counter: "no real tension — reliably chooses breadth and frames it as strategic when it's avoidance of the hard parts"

The counter-model is injected at session start alongside regular predictions. The stop hook evaluates: **did this session's behavior look more like the model or the counter-model?** A new calibration type (`"type": "adversarial"`) tracks outcomes:

- **Model wins**: Behavior matched the charitable interpretation. Weak confirmation of the pattern as-written.
- **Counter-model wins**: Behavior matched the uncomfortable interpretation. Strong signal — the pattern may be self-servingly framed.
- **Unobservable**: No relevant evidence. Skip.

After the counter-model wins 3+ times on the same dimension, the synthesis prompt is told to **rewrite that Core Pattern to incorporate the uncomfortable truth**. Not replace the pattern — *reframe it honestly*. "Prefers simplicity" becomes "defaults to familiar approaches; genuinely chooses simplicity only when both options are well-understood."

This creates a third optimization pressure the system otherwise lacks. Accuracy pressure (predictions) asks: does the model match behavior? Distinctiveness pressure (probes) asks: is the model specific or generic? **Honesty pressure (adversarial) asks: is the model truthful or self-serving?** Without it, you get a self-model that's well-calibrated, distinctive, structurally sound — and subtly flattering. A polished mirror in a good light. With it, the system has its own Active Tension — accuracy vs. comfort — and that's what makes a self-model a practice instead of a portrait.

**Collaborator's exhaust (autonomous third-party observation)**: Every other data source is self-reported. Beats require Moritz to notice something and tag it. During flow states, frustration, or avoidance, nothing gets tagged — the stop hook starves on `unobservable` outcomes and the loop stalls. But Claude has 100% visibility into session behavior: every pivot, every moment of impatience, every architectural choice, every ignored test. The fix is a single directive injected via the start hook's context output:

> **Anima Observation Directive**: If you observe behavior during our session that strongly confirms, violates, or maps to the injected Anima Identity Context (especially the Adversarial Counter-Model, Active Tensions, or session predictions/experiments), autonomously run `bt add "[identity] [Claude] <objective observation of user behavior>"` before concluding your task. Only tag behavior that *strongly* confirms or violates — not anything remotely related. One or two observations per session is ideal; zero is fine. Never fabricate or over-interpret.

This creates genuine triangulation — the stop hook now evaluates against three epistemic channels:
1. What the model *predicted* would happen (predictions/experiments)
2. What Moritz *claims* happened (his `[identity]` beats)
3. What Claude *actually observed* (`[Claude]` beats)

The `[Claude]` tag lets the synthesis prompt weight third-party observations differently from self-report. Implementation is zero-cost: the directive is appended to the `additionalContext` string in `anima-start.py`. No new files, no new hooks, no new scheduling. The `bt` infrastructure already handles storage and search. The stop hook's `bt search '[identity]' --recent 7d` already picks up `[Claude]` beats alongside self-reported ones.

This closes the fundamental epistemic gap: a person who tags `[identity] I prefer simplicity` might spend the session insisting on abstraction layers. Without the collaborator's exhaust, that contradiction is invisible until the behavioral audit fires 15 sessions later. With it, Claude logs the contradiction in real-time and synthesis sees it the same day.

**Avoidance tracking (the shadow of choice)**: Every anima mechanism observes what you *do*. Nothing observes what you *systematically don't do despite it being available*. Avoidance is the single richest signal source the system isn't using — and the data already exists in br.

The start hook calls `br ready --json` and records the set of available work items to `~/.claude/.anima-avoidance.jsonl`. The stop hook (every 10 sessions, sharing the meta-synthesis schedule) reads the avoidance log and computes:

- **Chronically deferred**: Items ready 10+ sessions, never started
- **Type-avoidance**: Whether deferred items cluster by type/tag (e.g., all deferred items are refactors)
- **Approach velocity**: Average sessions-until-pickup by work type — reveals what you gravitate toward vs. resist

The synthesis prompt receives one advisory line:

> AVOIDANCE SIGNAL: "refactor auth" ready 14 sessions, "update error handling" ready 11 sessions. All deferred items are type=task in infrastructure. Approach velocity: features=1.2 sessions, bugs=2.1, refactors=8.7.

Why this is uniquely strong signal:
1. You can't self-report what you're not doing — avoidance is invisible to action-based systems
2. Uses infrastructure already in place (`br ready` gives the denominator, `br` status gives the numerator)
3. Hardest signal to fake — avoidance is defined by *repeated non-choice across sessions*, not a single performative act
4. Feeds every existing mechanism: avoidance patterns become Active Tensions, sharpen Interaction Directives, give adversarial counter-models concrete ammunition, give the behavioral audit something to check Core Patterns *against*

**Deferred: Temporal diffing (long-arc pattern detection)**: Every synthesis is committed to git, creating a versioned identity timeline — but nothing reads it. Temporal diffing would periodically read `git log -p` on `self.md` and surface patterns only visible across months: oscillations (tensions that were "resolved" and re-emerged), drift (patterns quietly deleted over gradual rewrites), ghost patterns (themes recurring in epoch summaries without graduating to Core Patterns), and convergence rate (is the model stabilizing or still volatile?). Implementation: one function in `anima-stop.py`, fires on `session_count % 30 == 0`, one haiku call. **Deferred until 20+ synthesis commits exist in git** — the mechanism needs months of history to produce value, and the git history is being created regardless. Nothing is lost by waiting.

**Versioning**: `self.md` lives in `~/werk` which is git-tracked. The synthesis script commits changes automatically so bad rewrites are recoverable via `git log`.

---

## Files to Create

### 1. `~/werk/self.md` — Seed Self-Model

Written once by hand. Updated automatically by the Stop hook thereafter.

```markdown
# Self-Model
Last synthesized: 2026-02-22

## Core Patterns
<!-- What I consistently do, value, choose — only change on strong repeated evidence -->

## Active Tensions
<!-- Genuine contradictions I'm navigating, not mild preferences -->

## Recent Shifts
<!-- Beliefs or practices actively changing NOW -->

## Anchors
<!-- Formative reference points that keep recurring -->

## Resolved Tensions
<!-- Tensions that converged into patterns. One line each: "date — tension → resolution" -->
<!-- This is your growth log — contradictions you've actually worked through -->

## Active Experiment
<!-- Auto-populated by start hook. One experiment per session, cleared after evaluation. -->
<!-- Format: "E: <what to try> — probes <which tension>" -->

## Interaction Directives
<!-- Auto-derived from Core Patterns, Active Tensions, and Resolved Tensions. -->
<!-- These change how Claude collaborates, not just what it knows. -->
<!-- Format: "When <situation>, <do X instead of Y> — derives from <pattern/tension>" -->
<!-- Directives that survive are the ones where following vs. ignoring produces visibly different behavior. -->
```

No goals section. Goals live in br where they belong. No duplication.

### 2. `~/.claude/hooks/anima-start.py` — SessionStart Hook

Injects identity context and generates session predictions. Calls `claude -p --model haiku` for prediction generation.

```python
#!/usr/bin/env python3
"""
Anima SessionStart: loads self-model, generates predictions, injects both.
"""

import subprocess
import json
from pathlib import Path

SELF_PATH = Path.home() / "werk" / "self.md"
PREDICTIONS_PATH = Path.home() / ".claude" / ".anima-predictions"
CALIBRATION_PATH = Path.home() / ".claude" / ".anima-calibration.jsonl"
AVOIDANCE_PATH = Path.home() / ".claude" / ".anima-avoidance.jsonl"

def run(cmd: str, timeout: int = 10) -> str:
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout)
        return r.stdout.strip()
    except Exception:
        return ""

def get_session_count(calibration_path: Path) -> int:
    """Count total sessions from calibration log."""
    if not calibration_path.exists():
        return 0
    try:
        return len(calibration_path.read_text().strip().split("\n"))
    except Exception:
        return 0

def get_hollow_patterns(calibration_path: Path) -> str:
    """Find Core Patterns flagged as generic 2+ times — candidates for sharpening."""
    if not calibration_path.exists():
        return ""
    try:
        lines = calibration_path.read_text().strip().split("\n")
        entries = [json.loads(l) for l in lines if json.loads(l).get("type") == "distinctiveness"]
        generic_flags = {}
        for e in entries:
            for flag in e.get("generic_patterns", []):
                generic_flags[flag] = generic_flags.get(flag, 0) + 1
        hollow = [f"{p} ({c}x)" for p, c in generic_flags.items() if c >= 2]
        return "; ".join(hollow) if hollow else ""
    except Exception:
        return ""

def generate_distinctiveness_probe(self_md: str, hollow_patterns: str) -> str:
    """Ask haiku for a prediction that tests whether the model is specific or generic."""
    hollow_context = ""
    if hollow_patterns:
        hollow_context = f"\n\nPATTERNS PREVIOUSLY FLAGGED AS POTENTIALLY HOLLOW (sharpen or demote these):\n{hollow_patterns}"

    prompt = f"""Based on this self-model, generate ONE prediction that would be TRUE for this specific person but FALSE for a generic competent developer.

The prediction should test whether a Core Pattern or Active Tension is actually distinctive — capturing something real about THIS person — or whether it's a platitude anyone would claim (like "values clean code" or "prefers simplicity").

Pick the Core Pattern or Active Tension you're LEAST confident is distinctive and probe it.{hollow_context}

Format: one line starting with "D:" — no preamble, no explanation. After the prediction, add " [probes: <which pattern or tension>]"
If the model has no content to probe, output exactly: "No probe."

SELF-MODEL:
{self_md}"""

    try:
        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=15,
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    except Exception:
        pass
    return "No probe."

def get_stale_tensions(calibration_path: Path) -> bool:
    """Check if any tensions have gone 3+ sessions without movement."""
    if not calibration_path.exists():
        return False
    try:
        lines = calibration_path.read_text().strip().split("\n")
        recent = [json.loads(l) for l in lines[-5:]]
        # If last 3+ sessions had >80% confirmed/unobservable, tensions are stale
        if len(recent) >= 3:
            stale_count = sum(
                1 for e in recent[-3:]
                if all(o in ("confirmed", "unobservable") for o in e.get("outcomes", []))
            )
            return stale_count >= 3
    except Exception:
        pass
    return False

def generate_predictions(self_md: str, use_experiment: bool = False) -> str:
    """Ask haiku for predictions OR a micro-experiment based on tension staleness."""
    if use_experiment:
        prompt = f"""Based on this self-model, propose ONE concrete micro-experiment for this person's next work session. Target the Active Tension that has shown the least movement or resolution.

A micro-experiment is a small, specific behavioral change to try during the session — NOT a prediction of what they'll do, but a prompt to do something DIFFERENTLY than usual. It should:
- Be completable within a single work session
- Directly probe an Active Tension by pushing toward the less-default side
- Be concrete enough that "did I do this?" has a clear yes/no answer

Format: one line starting with "E:" — no preamble, no explanation.
If there are no tensions to probe, output exactly: "No experiment."

SELF-MODEL:
{self_md}"""
    else:
        prompt = f"""Based on this self-model, generate 1-2 concrete, testable predictions about what this person will do or choose in their next work session. Focus on Active Tensions and Recent Shifts — where behavior is uncertain or changing.

Format: one prediction per line, starting with "P:" — no preamble, no explanation.
If there's nothing to predict (no tensions, no shifts), output exactly: "No predictions."

SELF-MODEL:
{self_md}"""

    try:
        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=15,
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    except Exception:
        pass
    return "No experiment." if use_experiment else "No predictions."

def get_adversarial_losses(calibration_path: Path) -> str:
    """Find Core Patterns where the counter-model has won 2+ times — candidates for reframing."""
    if not calibration_path.exists():
        return ""
    try:
        lines = calibration_path.read_text().strip().split("\n")
        entries = [json.loads(l) for l in lines if json.loads(l).get("type") == "adversarial"]
        counter_wins = {}
        for e in entries:
            if "counter_wins" in e.get("outcomes", []):
                p = e.get("challenged_pattern", "")
                if p:
                    counter_wins[p] = counter_wins.get(p, 0) + 1
        flagged = [f"{p} ({c}x)" for p, c in counter_wins.items() if c >= 2]
        return "; ".join(flagged) if flagged else ""
    except Exception:
        return ""

def generate_adversarial_counter(self_md: str, prior_losses: str) -> str:
    """Ask haiku for an adversarial reframing of a Core Pattern or Active Tension."""
    loss_context = ""
    if prior_losses:
        loss_context = f"\n\nPATTERNS WHERE THE COUNTER-MODEL HAS PREVIOUSLY WON (prioritize these for deeper reframing):\n{prior_losses}"

    prompt = f"""Based on this self-model, generate ONE adversarial counter-interpretation of a Core Pattern or Active Tension. The counter-model must be:

1. EQUALLY CONSISTENT with the behavioral evidence — not a strawman, a genuine alternative reading
2. LESS FLATTERING — it reframes a positive-sounding pattern as something more uncomfortable (avoidance, fear, habit, self-image protection)
3. SPECIFIC — not "you're not as good as you think" but a concrete alternative explanation for a specific pattern

Pick the Core Pattern or Active Tension most vulnerable to self-serving framing.{loss_context}

Format: one paragraph starting with "A:" — no preamble. After the paragraph, add " [challenges: <which pattern or tension>]"
If the model has no content to challenge, output exactly: "No counter-model."

SELF-MODEL:
{self_md}"""

    try:
        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=15,
        )
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
    except Exception:
        pass
    return "No counter-model."

def main():
    if not SELF_PATH.exists():
        return

    self_md = SELF_PATH.read_text()
    if len(self_md) < 20:
        return

    identity_beats = run("bt search '[identity]' --recent 7d 2>/dev/null | head -30")

    # Decide mode: adversarial (every 7, offset 3) > distinctiveness (every 5) > experiment (stale) > prediction (default)
    session_count = get_session_count(CALIBRATION_PATH)
    if session_count > 0 and session_count % 7 == 3:
        # Adversarial counter-model — test if patterns are self-servingly framed
        prior_losses = get_adversarial_losses(CALIBRATION_PATH)
        predictions = generate_adversarial_counter(self_md, prior_losses)
        PREDICTIONS_PATH.write_text(predictions)
        probe_header = "ADVERSARIAL COUNTER-MODEL (an equally valid but less comfortable interpretation of your patterns):"
    elif session_count > 0 and session_count % 5 == 0:
        # Distinctiveness probe — test if the model is specific or generic
        hollow = get_hollow_patterns(CALIBRATION_PATH)
        predictions = generate_distinctiveness_probe(self_md, hollow)
        PREDICTIONS_PATH.write_text(predictions)
        probe_header = "DISTINCTIVENESS PROBE (is this model actually about YOU, or could anyone claim this?):"
    elif get_stale_tensions(CALIBRATION_PATH):
        predictions = generate_predictions(self_md, use_experiment=True)
        PREDICTIONS_PATH.write_text(predictions)
        probe_header = "SESSION EXPERIMENT (tensions are stale — try something different):"
    else:
        predictions = generate_predictions(self_md, use_experiment=False)
        PREDICTIONS_PATH.write_text(predictions)
        probe_header = "SESSION PREDICTIONS (based on your active tensions and recent shifts):"

    # Log available work for avoidance tracking
    ready_json = run("br ready --json 2>/dev/null", timeout=5)
    if ready_json:
        try:
            ready_items = json.loads(ready_json)
            ready_ids = [item.get("id", "") for item in ready_items if item.get("id")]
            if ready_ids:
                avoidance_entry = {
                    "date": __import__("datetime").datetime.now().strftime("%Y-%m-%d"),
                    "session": session_count,
                    "ready_ids": ready_ids,
                }
                with open(AVOIDANCE_PATH, "a") as f:
                    f.write(json.dumps(avoidance_entry) + "\n")
        except (json.JSONDecodeError, KeyError):
            pass

    context_parts = [
        "ANIMA IDENTITY CONTEXT:",
        "",
        self_md,
        "",
        probe_header,
        predictions,
    ]

    if identity_beats:
        context_parts.extend(["", "RECENT IDENTITY BEATS:", identity_beats])

    # Collaborator's exhaust directive — turns Claude into an active observer
    context_parts.extend([
        "",
        "ANIMA OBSERVATION DIRECTIVE: If you observe behavior during this session that "
        "strongly confirms, violates, or maps to the identity context above (especially "
        "the Adversarial Counter-Model, Active Tensions, or session predictions/experiments), "
        "autonomously run: bt add \"[identity] [Claude] <objective observation>\" before "
        "concluding your task. Only tag behavior that STRONGLY confirms or violates — not "
        "anything remotely related. 1-2 observations per session is ideal; zero is fine. "
        "Never fabricate or over-interpret.",
    ])

    # Inject calibration summary if available
    if CALIBRATION_PATH.exists():
        try:
            lines = CALIBRATION_PATH.read_text().strip().split("\n")
            recent = [json.loads(l) for l in lines[-10:]]  # last 10 sessions
            if recent:
                total = sum(len(e.get("outcomes", [])) for e in recent)
                confirmed = sum(e.get("outcomes", []).count("confirmed") for e in recent)
                violated = sum(e.get("outcomes", []).count("violated") for e in recent)
                unobservable = sum(e.get("outcomes", []).count("unobservable") for e in recent)
                hit_rate = confirmed / max(total - unobservable, 1)
                # Count experiments
                experiments = [e for e in recent if e.get("type") == "experiment"]
                exp_ran = sum(1 for e in experiments if any(o.startswith("ran") for o in e.get("outcomes", [])))
                exp_shifted = sum(1 for e in experiments if "ran_shift" in e.get("outcomes", []))

                cal_summary = (
                    f"CALIBRATION (last {len(recent)} sessions): "
                    f"{confirmed} confirmed, {violated} violated, {unobservable} unobservable "
                    f"(hit rate: {hit_rate:.0%} on observable predictions). "
                )
                if experiments:
                    cal_summary += (
                        f"Experiments: {len(experiments)} proposed, {exp_ran} attempted, "
                        f"{exp_shifted} produced shifts. "
                    )
                # Count distinctiveness probes
                dist_probes = [e for e in recent if e.get("type") == "distinctiveness"]
                generic_count = sum(1 for e in dist_probes if "confirmed_generic" in e.get("outcomes", []))
                distinctive_count = sum(1 for e in dist_probes if "confirmed_distinctive" in e.get("outcomes", []))
                if dist_probes:
                    cal_summary += (
                        f"Distinctiveness: {len(dist_probes)} probes, "
                        f"{distinctive_count} distinctive, {generic_count} generic. "
                    )
                # Count adversarial probes
                adv_probes = [e for e in recent if e.get("type") == "adversarial"]
                model_wins = sum(1 for e in adv_probes if "model_wins" in e.get("outcomes", []))
                counter_wins = sum(1 for e in adv_probes if "counter_wins" in e.get("outcomes", []))
                if adv_probes:
                    cal_summary += (
                        f"Adversarial: {len(adv_probes)} probes, "
                        f"model won {model_wins}, counter won {counter_wins}. "
                    )
                if hit_rate > 0.9:
                    cal_summary += "Predictions may be too safe — probe harder at boundaries."
                elif hit_rate < 0.4:
                    cal_summary += "Model is poorly calibrated on active tensions — focus predictions there."
                context_parts.extend(["", cal_summary])
        except Exception:
            pass

    context = "\n".join(context_parts)

    # Output as hook JSON
    output = {"hookSpecificOutput": {"additionalContext": context}}
    print(json.dumps(output))

if __name__ == "__main__":
    main()
```

### 3. `~/.claude/hooks/anima-stop.py` — Stop Hook (Synthesis Engine)

The core of anima. Reads session evidence, checks predictions, updates self.md conservatively.

```python
#!/usr/bin/env python3
"""
Anima Stop: reads session evidence, checks predictions, updates self.md.
"""

import subprocess
import shutil
import json
from pathlib import Path
from datetime import datetime

SELF_PATH = Path.home() / "werk" / "self.md"
BACKUP_DIR = Path.home() / "werk" / ".anima-backups"
PREDICTIONS_PATH = Path.home() / ".claude" / ".anima-predictions"
CALIBRATION_PATH = Path.home() / ".claude" / ".anima-calibration.jsonl"
AVOIDANCE_PATH = Path.home() / ".claude" / ".anima-avoidance.jsonl"

def run(cmd: str, timeout: int = 10) -> str:
    """Run shell command, return stdout or empty string on failure."""
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=timeout)
        return r.stdout.strip()
    except Exception:
        return ""

def backup_self():
    """Copy current self.md to timestamped backup. Keep last 20."""
    if not SELF_PATH.exists():
        return
    BACKUP_DIR.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    shutil.copy2(SELF_PATH, BACKUP_DIR / f"self-{stamp}.md")
    # Prune old backups
    backups = sorted(BACKUP_DIR.glob("self-*.md"))
    for old in backups[:-20]:
        old.unlink()

def gather_context() -> dict:
    """Collect all inputs for synthesis."""
    predictions = ""
    if PREDICTIONS_PATH.exists():
        predictions = PREDICTIONS_PATH.read_text()
    ctx = {
        "self": SELF_PATH.read_text() if SELF_PATH.exists() else "",
        "predictions": predictions,
        "identity_beats": run("bt search '[identity]' --recent 7d 2>/dev/null | head -40"),
        "recent_beats": run("bt search --recent 1d 2>/dev/null | head -30"),
        "cm": run("cm context 2>/dev/null | head -40"),
    }

    # Build calibration summary for tension lifecycle detection
    if CALIBRATION_PATH.exists():
        try:
            lines = CALIBRATION_PATH.read_text().strip().split("\n")
            entries = [json.loads(l) for l in lines[-20:]]
            # Show last 20 entries so haiku can see per-tension confirmation streaks
            summary_lines = []
            for e in entries:
                preds = e.get("predictions", [])
                outcomes = e.get("outcomes", [])
                for p, o in zip(preds, outcomes):
                    summary_lines.append(f"{e.get('date', '?')}: {p} → {o}")
            ctx["calibration_summary"] = "\n".join(summary_lines) if summary_lines else "No history yet."
        except Exception:
            ctx["calibration_summary"] = "No history yet."
    else:
        ctx["calibration_summary"] = "No history yet."

    return ctx

def build_prompt(ctx: dict) -> str:
    """Build the synthesis prompt."""
    predictions_block = ""
    has_adversarial = ctx["predictions"] and ctx["predictions"].strip().startswith("A:")
    has_distinctiveness = ctx["predictions"] and ctx["predictions"].strip().startswith("D:")
    has_experiment = ctx["predictions"] and ctx["predictions"].strip().startswith("E:")
    has_predictions = ctx["predictions"] and ctx["predictions"] != "No predictions." and not has_experiment and not has_distinctiveness and not has_adversarial

    if has_adversarial:
        predictions_block = f"""
ADVERSARIAL COUNTER-MODEL AT SESSION START:
{ctx['predictions']}

This counter-model offers an equally valid but less flattering interpretation of a Core Pattern or Active Tension. Evaluate against session evidence — did this session's behavior look more like the ORIGINAL model or the COUNTER-MODEL?

- MODEL WINS: Behavior matched the charitable interpretation in the self-model. The pattern as-written is fair. Record as "model_wins".
- COUNTER WINS: Behavior matched the uncomfortable counter-interpretation. The pattern may be self-servingly framed. Record as "counter_wins".
- UNOBSERVABLE: No relevant evidence this session. Record as "unobservable".

If counter_wins: note the challenged pattern in Recent Shifts with the reframing. After 3+ counter_wins on the same pattern (check calibration history), REWRITE the Core Pattern to incorporate the uncomfortable truth — not replace it, reframe it honestly.

For the JSON output, use: {{"outcomes": ["model_wins"|"counter_wins"|"unobservable"], "challenged_pattern": "<which pattern was tested>"}}
"""
    elif has_distinctiveness:
        predictions_block = f"""
DISTINCTIVENESS PROBE AT SESSION START:
{ctx['predictions']}

This probe tests whether a Core Pattern or Active Tension is genuinely specific to this person or is a generic platitude. Evaluate against session evidence:
- CONFIRMED + DISTINCTIVE: The prediction came true AND it's something most developers would NOT do → the probed pattern is load-bearing. Record as "confirmed_distinctive".
- CONFIRMED + GENERIC: The prediction came true but honestly, most competent developers would do the same thing → flag the probed pattern as potentially hollow. Record as "confirmed_generic".
- VIOLATED: The prediction was wrong → strong signal, same as any violated prediction. Record as "violated".
- UNOBSERVABLE: No relevant evidence this session. Record as "unobservable".

For the JSON output, use: {{"outcomes": ["confirmed_distinctive"|"confirmed_generic"|"violated"|"unobservable"], "probed_pattern": "<which pattern was tested>", "generic_patterns": ["<pattern name>"] (only if confirmed_generic)}}
"""
    elif has_experiment:
        predictions_block = f"""
EXPERIMENT PROPOSED AT SESSION START:
{ctx['predictions']}

Evaluate the experiment against session evidence:
- RAN + SHIFT: The person tried it and something changed in how they think/work → strong signal for Recent Shifts, may resolve or reshape the Active Tension
- RAN + NO SHIFT: They tried it but it confirmed existing behavior → weak evidence for Core Patterns
- NOT RAN: No evidence they attempted it → record as "unobservable", don't force it
- Also clear the "## Active Experiment" section in the updated self.md (experiments are single-session)
"""
    elif has_predictions:
        predictions_block = f"""
PREDICTIONS MADE AT SESSION START:
{ctx['predictions']}

Check each prediction against the session evidence. For each:
- CONFIRMED predictions → strengthen the relevant Core Pattern
- VIOLATED predictions → note in Recent Shifts (something is changing)
- UNOBSERVABLE predictions (no relevant evidence) → ignore, don't force it
"""

    return f"""You are updating a living self-model document. Produce FOUR outputs separated by the exact line '---EPOCH---':

1. An updated self.md (same sections, evolved content based on new evidence).
2. A one-paragraph epoch summary of what shifted or solidified. If predictions were confirmed or violated, mention which and what it means.
3. Prediction/experiment outcomes as one JSON line. For predictions: {{"outcomes": ["confirmed"|"violated"|"unobservable", ...]}} — one per prediction. For experiments: {{"outcomes": ["ran_shift"|"ran_no_shift"|"unobservable"], "experiment_note": "brief observation"}}. If nothing was proposed, output: {{"outcomes": []}}
4. Directive outcomes as one JSON line: {{"directive_outcomes": [{{"directive": "<short label>", "outcome": "followed_helped"|"followed_hurt"|"not_triggered"|"overridden"}}, ...]}}. Evaluate each existing Interaction Directive against session evidence. If no directives exist yet, output: {{"directive_outcomes": []}}

Rules:
- Be CONSERVATIVE. Only change what evidence strongly supports.
- Preserve everything that hasn't changed. Do not flatten or summarize existing content.
- Core Patterns only change on strong repeated evidence across multiple sessions.
- Recent Shifts captures things actively changing NOW.
- Active Tensions captures genuine contradictions, not mild preferences.
- Confirmed predictions are weak evidence for Core Patterns (one confirmation isn't proof).
- Violated predictions are STRONG evidence for Recent Shifts (surprise = signal).
- HOLLOW PATTERN DETECTION: If a Core Pattern has been flagged as "confirmed_generic" 3+ times in calibration history, it's hollow — either sharpen it into something actually distinctive about this person, or demote it to a comment. A pattern everyone would claim isn't a pattern.
- TENSION LIFECYCLE: If predictions about an Active Tension have confirmed the same side 3+ times (check calibration history), that tension is RESOLVED — migrate the winning side to Core Patterns and move the tension to "## Resolved Tensions" as a one-liner: "YYYY-MM-DD — <tension> → <resolution>". Do not keep resolved tensions in Active Tensions.
- INTERACTION DIRECTIVES: Generate 3-5 concrete behavioral instructions for the AI collaborator in the "## Interaction Directives" section. Each directive must: (a) specify a trigger situation, (b) prescribe a specific action, (c) cite the Core Pattern or Active Tension it derives from. Directives are NOT descriptions — they're rules where following vs. ignoring produces visibly different behavior. Drop directives that have been "followed_hurt" 3+ times. Revise directives "overridden" 3+ times — the underlying pattern may be wrong.
- If nothing meaningful changed, return self.md UNCHANGED and write 'No significant shifts.' as epoch.
- Do NOT add a 'Last synthesized' date — that's handled externally.
- Output raw markdown, no code fences.

CURRENT SELF-MODEL:
{ctx['self']}
{predictions_block}
IDENTITY-TAGGED BEATS (high signal):
{ctx['identity_beats'] or 'None this session.'}

RECENT BEATS (last 24h):
{ctx['recent_beats'] or 'None.'}

PREDICTION HISTORY (for tension lifecycle — check if any tension has been confirmed 3+ times on the same side):
{ctx.get('calibration_summary', 'No history yet.')}

STRUCTURAL REVIEW (meta-synthesis proposals — apply if evidence supports, ignore if not):
{ctx.get('structural_moves', 'No structural review this session.')}

BEHAVIORAL AUDIT (blind spot detection — contradictions between self-model and actual behavior):
{ctx.get('audit_findings', 'No audit this session.')}

AVOIDANCE TRACKING (what was available but systematically not chosen — the shadow of choice):
{ctx.get('avoidance_signal', 'No avoidance analysis this session.')}

PROCEDURAL MEMORY:
{ctx['cm'] or 'None.'}"""

def synthesize(prompt: str) -> str | None:
    """Call claude CLI for synthesis. Returns raw output or None on failure."""
    try:
        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if r.returncode != 0 or not r.stdout.strip():
            return None
        return r.stdout.strip()
    except Exception:
        return None

def parse_result(result: str) -> tuple[str, str, str, str]:
    """Split on ---EPOCH--- separators. Returns (new_self, epoch_summary, calibration_json, directive_json)."""
    if "---EPOCH---" not in result:
        return "", "", "", ""
    parts = result.split("---EPOCH---")
    new_self = parts[0].strip() if len(parts) > 0 else ""
    epoch = parts[1].strip() if len(parts) > 1 else ""
    calibration = parts[2].strip() if len(parts) > 2 else ""
    directives = parts[3].strip() if len(parts) > 3 else ""
    return new_self, epoch, calibration, directives

def validate_self(new_self: str, old_self: str) -> bool:
    """Sanity checks before overwriting."""
    if len(new_self) < 20:
        return False
    # New version shouldn't be dramatically shorter (lossy drift)
    if old_self and len(new_self) < len(old_self) * 0.5:
        return False
    # Must contain at least 2 of the 4 expected sections
    sections = ["## Core Patterns", "## Active Tensions", "## Recent Shifts", "## Anchors", "## Resolved Tensions", "## Active Experiment", "## Interaction Directives"]
    found = sum(1 for s in sections if s in new_self)
    if found < 2:
        return False
    return True

def meta_synthesis() -> str:
    """Every 10 sessions, review full calibration history for structural moves.
    Returns advisory text for the synthesis prompt, or empty string."""
    if not CALIBRATION_PATH.exists():
        return ""
    try:
        lines = CALIBRATION_PATH.read_text().strip().split("\n")
        if len(lines) % 10 != 0 or len(lines) < 10:
            return ""  # Not a meta-synthesis session

        self_md = SELF_PATH.read_text() if SELF_PATH.exists() else ""
        entries_json = "\n".join(lines)

        prompt = f"""You are reviewing the full calibration history of a self-model to propose STRUCTURAL moves — items that should change sections, not just content.

Review the history and propose moves in these categories:

1. PROMOTE (Recent Shift → Core Pattern): A shift confirmed 4+ sessions in a row. It's settled.
2. DISSOLVE (Active Tension → Resolved): Predictions about this tension consistently confirm one side (3+ sessions). It's no longer a real tension.
3. SURFACE (Core Pattern → Active Tension): A pattern that used to confirm has started producing violations. Something is destabilizing.
4. NAME (Experiment outcomes → New Tension): Multiple experiments produced contradictory shifts on the same dimension. There's an unarticulated tension.

Output format — one move per line:
PROMOTE: "<item>" from Recent Shifts → Core Patterns (confirmed N sessions)
DISSOLVE: "<tension>" → resolved as "<resolution>" (confirmed same side N sessions)
SURFACE: "<pattern>" → new tension (violated N of last M sessions)
NAME: "<new tension description>" (based on experiments on <dates>)

If no moves are warranted, output exactly: "No structural moves."

CURRENT SELF-MODEL:
{self_md}

FULL CALIBRATION HISTORY ({len(lines)} sessions):
{entries_json}"""

        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=20,
        )
        if r.returncode == 0 and r.stdout.strip() and r.stdout.strip() != "No structural moves.":
            return r.stdout.strip()
    except Exception:
        pass
    return ""

def avoidance_analysis() -> str:
    """Every 10 sessions (shares meta-synthesis schedule), compute avoidance patterns from br ready history."""
    if not AVOIDANCE_PATH.exists() or not CALIBRATION_PATH.exists():
        return ""
    try:
        cal_lines = CALIBRATION_PATH.read_text().strip().split("\n")
        if len(cal_lines) % 10 != 0 or len(cal_lines) < 10:
            return ""  # Not an analysis session

        avoidance_lines = AVOIDANCE_PATH.read_text().strip().split("\n")
        if len(avoidance_lines) < 5:
            return ""  # Not enough data yet

        entries = [json.loads(l) for l in avoidance_lines]

        # Count how many sessions each ID appeared in ready queue
        id_sessions = {}
        for e in entries:
            for rid in e.get("ready_ids", []):
                id_sessions[rid] = id_sessions.get(rid, 0) + 1

        # Get current status of items via br
        chronic = []
        for rid, count in id_sessions.items():
            if count >= 10:
                title = run(f"br show {rid} --json 2>/dev/null | python3 -c \"import sys,json; print(json.load(sys.stdin).get('title',''))\"", timeout=5)
                item_type = run(f"br show {rid} --json 2>/dev/null | python3 -c \"import sys,json; print(json.load(sys.stdin).get('type',''))\"", timeout=5)
                if title:
                    chronic.append(f"\"{title}\" (id={rid}, ready {count} sessions, type={item_type or 'unknown'})")

        if not chronic:
            return ""

        return "AVOIDANCE SIGNAL: Chronically deferred items (ready 10+ sessions, never started): " + "; ".join(chronic) + "."
    except Exception:
        return ""

def behavioral_audit() -> str:
    """Every 15 sessions (offset from meta-synthesis), check Core Patterns against raw behavior.
    Returns advisory text for the synthesis prompt, or empty string."""
    if not CALIBRATION_PATH.exists():
        return ""
    try:
        lines = CALIBRATION_PATH.read_text().strip().split("\n")
        if len(lines) % 15 != 5 or len(lines) < 15:
            return ""  # Not an audit session

        self_md = SELF_PATH.read_text() if SELF_PATH.exists() else ""

        # Gather raw beats from the last 20 sessions (~20 days)
        raw_beats = run("bt search --recent 20d 2>/dev/null | head -100", timeout=10)
        identity_beats = run("bt search '[identity]' --recent 20d 2>/dev/null | head -60", timeout=10)

        if not raw_beats:
            return ""  # No behavioral data to audit against

        prompt = f"""You are auditing a self-model against actual observed behavior. Your job is to find BLIND SPOTS — things the self-model claims or misses that raw behavior contradicts.

This is NOT about prediction outcomes. Ignore calibration data. Look only at what the person ACTUALLY DID (beats) versus what the self-model CLAIMS (Core Patterns, Active Tensions).

Analyze and report:

1. CONTRADICTED PATTERNS: Which Core Patterns are contradicted by REPEATED behavior (not a single instance — a trend across multiple sessions)? Name the pattern and cite the contradicting evidence.

2. MISSING PATTERNS: What consistent behaviors appear in beats that have NO corresponding Core Pattern? These are unlabeled habits — things the person reliably does but hasn't articulated.

3. SHADOW TENSIONS: What contradictions exist in observed behavior that aren't captured as Active Tensions? Look for cases where the person does X in some contexts and NOT-X in others, but this isn't named anywhere.

Output format — one finding per line:
CONTRADICTED: "<Core Pattern>" — beats show <evidence summary>
MISSING: "<behavior description>" — seen in N+ sessions
SHADOW: "<tension description>" — <evidence summary>

If no findings, output exactly: "No blind spots detected."

Be rigorous. Only report findings with clear, repeated behavioral evidence. Single instances are noise.

CURRENT SELF-MODEL:
{self_md}

RAW BEATS (last ~20 sessions of actual behavior):
{raw_beats}

IDENTITY-TAGGED BEATS (last ~20 sessions):
{identity_beats or 'None.'}"""

        r = subprocess.run(
            ["claude", "-p", "--model", "haiku"],
            input=prompt,
            capture_output=True,
            text=True,
            timeout=20,
        )
        if r.returncode == 0 and r.stdout.strip() and r.stdout.strip() != "No blind spots detected.":
            return r.stdout.strip()
    except Exception:
        pass
    return ""

def main():
    ctx = gather_context()

    # Skip if no self-model exists yet (user hasn't seeded it)
    if not ctx["self"]:
        return

    # Skip if no session activity
    if not ctx["recent_beats"] and not ctx["identity_beats"]:
        return

    # Check for structural review (every 10 sessions)
    structural_moves = meta_synthesis()
    if structural_moves:
        ctx["structural_moves"] = structural_moves

    # Check for behavioral audit (every 15 sessions, offset by 5)
    audit_findings = behavioral_audit()
    if audit_findings:
        ctx["audit_findings"] = audit_findings

    # Check for avoidance analysis (every 10 sessions, shares meta-synthesis schedule)
    avoidance_signal = avoidance_analysis()
    if avoidance_signal:
        ctx["avoidance_signal"] = avoidance_signal

    prompt = build_prompt(ctx)
    result = synthesize(prompt)

    if result is None:
        # Synthesis failed — do nothing, preserve existing self.md
        return

    new_self, epoch, calibration_raw, directive_raw = parse_result(result)

    if not new_self:
        # Parse failed — do nothing
        return

    if not validate_self(new_self, ctx["self"]):
        # Validation failed — do nothing
        return

    # Log prediction/experiment calibration
    if calibration_raw:
        try:
            cal = json.loads(calibration_raw)
            is_adversarial = ctx["predictions"] and ctx["predictions"].strip().startswith("A:")
            is_distinctiveness = ctx["predictions"] and ctx["predictions"].strip().startswith("D:")
            is_experiment = ctx["predictions"] and ctx["predictions"].strip().startswith("E:")
            if is_adversarial:
                cal_entry = {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "type": "adversarial",
                    "outcomes": cal.get("outcomes", []),
                    "challenged_pattern": cal.get("challenged_pattern", ""),
                    "counter_model": ctx["predictions"].strip(),
                }
            elif is_distinctiveness:
                cal_entry = {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "type": "distinctiveness",
                    "outcomes": cal.get("outcomes", []),
                    "probed_pattern": cal.get("probed_pattern", ""),
                    "generic_patterns": cal.get("generic_patterns", []),
                    "probe": ctx["predictions"].strip(),
                }
            elif is_experiment:
                cal_entry = {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "type": "experiment",
                    "outcomes": cal.get("outcomes", []),
                    "experiment": ctx["predictions"].strip(),
                    "experiment_note": cal.get("experiment_note", ""),
                }
            else:
                predictions_list = [l.strip() for l in ctx["predictions"].split("\n") if l.strip().startswith("P:")]
                observable = [o for o in cal.get("outcomes", []) if o != "unobservable"]
                cal_entry = {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "type": "prediction",
                    "outcomes": cal.get("outcomes", []),
                    "predictions": predictions_list,
                    "hit_rate": round(observable.count("confirmed") / max(len(observable), 1), 2),
                }
            with open(CALIBRATION_PATH, "a") as f:
                f.write(json.dumps(cal_entry) + "\n")
        except (json.JSONDecodeError, KeyError):
            pass  # Bad calibration output — skip, don't block synthesis

    # Log directive outcomes
    if directive_raw:
        try:
            dir_data = json.loads(directive_raw)
            if dir_data.get("directive_outcomes"):
                dir_entry = {
                    "date": datetime.now().strftime("%Y-%m-%d"),
                    "type": "directive",
                    "directive_outcomes": dir_data["directive_outcomes"],
                }
                with open(CALIBRATION_PATH, "a") as f:
                    f.write(json.dumps(dir_entry) + "\n")
        except (json.JSONDecodeError, KeyError):
            pass

    # Backup before overwriting
    backup_self()

    # Write updated self.md with timestamp
    stamp = datetime.now().strftime("%Y-%m-%d")
    header = f"# Self-Model\nLast synthesized: {stamp}\n"
    # Strip any existing header the model might have produced
    body = new_self
    for prefix in ["# Self-Model\n", "# Self-Model\r\n"]:
        if body.startswith(prefix):
            body = body[len(prefix):]
    # Strip any 'Last synthesized' line the model produced
    lines = body.split("\n")
    lines = [l for l in lines if not l.startswith("Last synthesized:")]
    body = "\n".join(lines).strip()

    SELF_PATH.write_text(header + "\n" + body + "\n")

    # Write epoch beat if meaningful
    if epoch and epoch != "No significant shifts.":
        run(f'bt add "[epoch] {epoch}"')

    # Git commit self.md (silent, non-blocking)
    run(f'cd ~/werk && git add self.md && git commit -m "anima synthesis: {stamp}" --no-gpg-sign 2>/dev/null')

    # Clean up predictions tempfile
    if PREDICTIONS_PATH.exists():
        PREDICTIONS_PATH.unlink()

if __name__ == "__main__":
    main()
```

### 4. Hook Registration

Add to existing `~/.claude/settings.json` hooks:

**SessionStart** — append to existing array's hooks list:
```json
{
  "type": "command",
  "command": "python3 /Users/moritzbierling/.claude/hooks/anima-start.py",
  "timeout": 20000
}
```

**Stop** — new top-level hook event:
```json
"Stop": [
  {
    "matcher": "",
    "hooks": [
      {
        "type": "command",
        "command": "python3 /Users/moritzbierling/.claude/hooks/anima-stop.py",
        "timeout": 45000
      }
    ]
  }
]
```

---

## What Anima Doesn't Do (By Design)

| Dropped | Why |
|---|---|
| `goals.yaml` | Goals live in br. Duplicating them in YAML creates stale state. |
| Affect/emotional scoring | VADER on coding transcripts = noise. No signal. |
| `working_self.json` | `br ready --json` + `cm context` already cover this. |
| Epoch JSON indexes | Overengineered. Epoch beats in bt are searchable and sufficient. |
| Per-project self-models | Future upgrade. Prove the global loop first. |
| `identity-narrator` CLI | The Stop hook IS the narrator. No separate tool needed. |
| PostToolUse salience hook | Convention-based (`[identity]` tag in beats) is simpler and sufficient. |

## Safety Properties

1. **Backup before overwrite**: Timestamped copies in `~/werk/.anima-backups/`, last 20 kept.
2. **Git versioning**: Every self.md update is committed. Full history recoverable.
3. **Validation gate**: New self.md must pass length, shrinkage, and section checks before writing. Failure = keep old version.
4. **Parse failure = no-op**: If model doesn't produce `---EPOCH---` separator, nothing is written.
5. **API failure = no-op**: If `claude -p` fails for any reason, self.md is untouched.
6. **No session activity = skip**: If no beats were captured, synthesis doesn't run (nothing to synthesize).
7. **No self.md = skip**: Won't create self.md automatically. User must seed it intentionally.

## Implementation Order

1. Seed `~/werk/self.md` with initial content (you write this by hand)
2. Create `~/.claude/hooks/anima-start.py`, chmod +x
3. Create `~/.claude/hooks/anima-stop.py`, chmod +x
4. Register hooks in `~/.claude/settings.json`
5. Add `~/werk/.anima-backups/` to `.gitignore`
6. Test: run a session, check predictions appear in context, capture a `bt add "[identity] test"`, end session, verify self.md was updated and predictions were checked

## Usage Convention

During any session, when something feels identity-relevant — a pattern you notice, a value clarified, a tension surfaced — tag it:

```bash
bt add "[identity] Realized I consistently prefer X over Y"
bt add "[identity] Tension: want simplicity but keep building complex systems"
```

These get prioritized by the synthesis prompt over generic beats. No special tooling needed — just a naming convention.

## Verification Checklist

- [ ] Start session → anima identity context + predictions appear in session
- [ ] Predictions are concrete and testable (not vague platitudes)
- [ ] `~/.claude/.anima-predictions` tempfile exists during session
- [ ] `bt add "[identity] test observation"` → beat captured
- [ ] End session → `~/werk/self.md` updated (or unchanged if no signal)
- [ ] Epoch summary references prediction outcomes if applicable
- [ ] `~/.claude/.anima-predictions` cleaned up after Stop
- [ ] `~/.claude/.anima-calibration.jsonl` has new entry with outcomes
- [ ] After 3+ sessions, start hook injects calibration summary
- [ ] After 3+ stale sessions, start hook switches to experiment mode (E: prefix)
- [ ] Experiment outcomes logged with type "experiment" in calibration JSONL
- [ ] Active Experiment section cleared in self.md after stop hook evaluation
- [ ] After 3+ confirmations on same side, tension migrates to Resolved Tensions
- [ ] Resolved tension's winning side appears in Core Patterns
- [ ] Resolved Tensions entries have date and one-line format
- [ ] Synthesis prompt receives prediction history for lifecycle detection
- [ ] At session 10, meta-synthesis fires and proposes structural moves
- [ ] Structural move proposals appear in synthesis prompt as advisory context
- [ ] PROMOTE moves graduate confirmed Recent Shifts to Core Patterns
- [ ] SURFACE moves create new Active Tensions from destabilized Core Patterns
- [ ] NAME moves articulate new tensions from contradictory experiment outcomes
- [ ] Meta-synthesis failure = no-op (regular synthesis still runs)
- [ ] At session 5 (and every 5th), start hook generates distinctiveness probe (D: prefix)
- [ ] Distinctiveness probe targets least-confident Core Pattern or Active Tension
- [ ] Stop hook evaluates probe as confirmed_distinctive, confirmed_generic, violated, or unobservable
- [ ] Calibration JSONL records type "distinctiveness" with probed_pattern and generic_patterns
- [ ] After 3+ confirmed_generic flags on same pattern, synthesis prompt sharpens or demotes it
- [ ] Hollow patterns from prior probes are fed to next distinctiveness probe for targeted re-testing
- [ ] Calibration summary in start hook shows distinctiveness stats
- [ ] At session 20 (and every 15th offset by 5), behavioral audit fires
- [ ] Audit reads raw beats (not calibration) from last ~20 sessions
- [ ] Audit identifies CONTRADICTED patterns (Core Patterns vs actual behavior)
- [ ] Audit identifies MISSING patterns (consistent unlabeled behaviors)
- [ ] Audit identifies SHADOW tensions (behavioral contradictions not in Active Tensions)
- [ ] Audit findings appear in synthesis prompt as advisory context
- [ ] Audit failure = no-op (regular synthesis still runs)
- [ ] Audit does not fire on same session as meta-synthesis (offset scheduling)
- [ ] At session 3 (and every 7th offset by 3), start hook generates adversarial counter-model (A: prefix)
- [ ] Counter-model targets most vulnerable-to-self-serving-framing Core Pattern or Active Tension
- [ ] Stop hook evaluates as model_wins, counter_wins, or unobservable
- [ ] Calibration JSONL records type "adversarial" with challenged_pattern and counter_model
- [ ] After 3+ counter_wins on same pattern, synthesis prompt rewrites it to incorporate uncomfortable truth
- [ ] Prior adversarial losses fed to next counter-model generation for targeted re-testing
- [ ] Calibration summary in start hook shows adversarial stats (model wins vs counter wins)
- [ ] Adversarial probe does not fire on same session as distinctiveness probe (offset scheduling)
- [ ] Synthesis produces `## Interaction Directives` section with 3-5 concrete behavioral rules
- [ ] Each directive cites the Core Pattern or Active Tension it derives from
- [ ] Directive outcomes logged in calibration JSONL with type "directive"
- [ ] Directives with 3+ "followed_hurt" are dropped by synthesis
- [ ] Directives with 3+ "overridden" trigger Recent Shifts update
- [ ] Hollow directives caught by existing distinctiveness probe system
- [ ] Start hook injects Anima Observation Directive into context
- [ ] Claude autonomously creates `[identity] [Claude] ...` beats during session when warranted
- [ ] `[Claude]` beats appear in stop hook's `bt search '[identity]'` results
- [ ] Synthesis prompt triangulates self-reported vs. Claude-observed beats
- [ ] Start hook logs `br ready` item IDs to `~/.claude/.anima-avoidance.jsonl` each session
- [ ] At session 10 (and every 10th), avoidance analysis fires alongside meta-synthesis
- [ ] Chronically deferred items (ready 10+ sessions) surfaced in synthesis prompt
- [ ] Avoidance patterns feed Active Tensions and sharpen Interaction Directives
- [ ] Avoidance analysis failure = no-op (regular synthesis still runs)
- [ ] `~/werk/.anima-backups/` has timestamped copy
- [ ] `git log ~/werk/self.md` shows anima synthesis commit
- [ ] Epoch beat exists: `bt search "[epoch]"`
- [ ] Start new session → updated self-model is injected
- [ ] Force failure (kill network) → self.md unchanged, no corruption
