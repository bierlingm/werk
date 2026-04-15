---
name: werk-triage
description: "Tend the tension field. Surfaces signal-bearing tensions, classifies each into a triage bucket with a default structural gesture, grounds the diagnosis in logbase history, then executes approved gestures. Use when the field has drifted — stale tensions accumulating, signals unattended, momentum unclear. Not a dev session opener; not a per-tension brief."
disable-model-invocation: false
allowed-tools: Bash, Read, Agent
argument-hint: "[optional: --scope <root-id> | --overdue | --stale | --signals | --since <window>]"
---

# werk-triage — Tend the Field

You are performing triage on the practitioner's tension field. Triage is the recurring practice gesture of surveying what has drifted, proposing structural gestures, and — after approval — executing them.

This is **not** a dev session — `/werk-session` does that. Triage operates on the field itself as a living structure.

## Operating principles

- **Signal by exception.** Only surface tensions that are actually exceptional. A healthy tension is invisible here.
- **Desired above actual.** Frame every diagnosis as the gap between desire and reality, not as "this is behind schedule."
- **Locality.** Propose gestures on the tension, its parent, or its children — never cascading reorganizations across the field.
- **Gesture as unit of change.** Every proposal maps to exactly one werk gesture (or a tight sequence).
- **History earns the claim.** Every candidate must cite at least one logbase fact (last epoch, last reality, last note, ghost geometry). Triage without history is a todo list.

## Step 1 — Orient

Parse `$ARGUMENTS`. Supported scopes:

- no args → full active field
- `--scope <id>` → only the subtree rooted at `<id>`
- `--overdue` → only overdue tensions
- `--stale` → only stale tensions (uses configured threshold)
- `--signals` → only tensions with active structural signals
- `--since <window>` → only tensions changed in the window (e.g. `7d`, `2w`, `2026-03`)

Run these reads in parallel (one Bash block, multiple calls):

```bash
werk stats --temporal --drift --health
werk list --signals --json
werk list --stale --json
werk list --changed "7 days ago" --json
werk log --since 7d
werk tree
```

Argument formats that matter:
- `werk list --changed` takes natural language or dates — `"7 days ago"`, `yesterday`, `today`, `2026-03-10`. **Not** `7d` (rejected).
- `werk log --since` accepts `Nd` / `Nw` / `YYYY-MM` / `YYYY-MM-DD` / `today` / `yesterday`.

If a scope was given, prefer `werk list --parent <id> ...` and `werk log <id> ...` as appropriate.

Read enough to answer: which tensions have active signals, which are stale, where did the last week's momentum actually land, and which branches are quiet.

## Step 2 — Classify

Assign each candidate to exactly one of six buckets. Each bucket has a default gesture.

| Bucket | Condition | Default gesture |
|---|---|---|
| **Overdue** | deadline passed, status Active | `resolve` / `release` / `horizon` (with note explaining the push) |
| **Containment** | child deadline > parent deadline | `horizon` on parent or child, or `split` the parent |
| **Critical path** | on the path to an approaching parent deadline | confirm priority via `position`, or split if overloaded |
| **Rollup-stale** | parent whose children have produced epochs since parent's last reality update | `reality` update on the parent — synthesize a draft from intervening child epochs, offer edit-or-accept |
| **Stale** | no mutation in N days (config) | `reality` update, `snooze`, `release`, or `rm` |
| **Drift** | desire mutations outpacing reality (ghost geometry) | `split`, `reality`, or `epoch` to mark the rewrite |
| **Held-but-live** | unpositioned yet activity nearby | `position` into the sequence |

Detecting **Rollup-stale**: for each parent tension, compare the timestamp of its last reality-mutating epoch against the max timestamp of reality-mutating epochs on its descendants. If descendants have moved and the parent hasn't, the rollup is behind. Parents with no children are not candidates.

Drafting the new reality: **do not write a ship-list**. The child epochs tell you what's been done; the reality must say what's *true now* about the parent against its *desire*. Use the child epochs as evidence, not as content. A good parent reality answers: where are we, honestly, against the desire? What is solid, what is the open gap, what substrate is queued? A bad parent reality enumerates what shipped. Present the draft for the practitioner to accept, edit, or reject.

Note: `werk reality` auto-records an epoch boundary. Do not pair it with a separate `werk epoch` call.

Rules:

- A tension goes into **one** bucket — the most severe (overdue > containment > critical path > drift > rollup-stale > stale > held-but-live).
- Drift outranks rollup-stale because drift may call for a structural split; rollup-stale is information hygiene.
- Cap the total queue at 7. If more qualify, keep the most severe and note the overflow count.
- Skip tensions already under active work (mutations in the last 48h) unless they are overdue or containment-violating.

## Step 3 — Historize

For each survivor, pull 1–2 logbase facts that explain why it's stuck:

```bash
werk show <id> --json        # current state, notes, signals
werk log <id> --since 30d    # recent epochs, reality/desire evolution
```

Extract: last reality update date, last epoch boundary, last note, whether desire has been rewritten, whether children have progressed. Keep the fact to one phrase in the diagnosis.

## Step 4 — Propose

Output format — tight, one tension per row, newest/most-severe first:

```
## Triage queue (N candidates)

1. #42 [OVERDUE]       <desired, truncated>
   Gap: <reality one-liner>.  Last reality 2026-02-11, no notes.
   Proposed: werk horizon 42 2026-05-01 && werk note 42 "pushed — waiting on X"
   Reason: <one sentence>

2. #154 [CONTAINMENT]  <desired, truncated>
   Child #177 due 2026-07, parent due 2026-06.
   Proposed: werk horizon 154 2026-08-01
   Reason: <one sentence>

... up to 7 ...
```

Each candidate: ID, bucket tag, short desire, one-line diagnosis grounded in history, the **exact commands** you will run, one-sentence reason.

End with:

```
Apply which? (all / <numbers> / none / skip <numbers>)
```

If none qualify, say so plainly — the field is healthy for this scope — and stop.

## Step 5 — Apply or defer

Wait for the user's response. Then:

- For each approved candidate: run the proposed commands. If a command fails (validation, missing arg), stop, report, do not continue the queue — the field state may have shifted.
- For each deferred candidate: run `werk note <id> "triage <YYYY-MM-DD>: deferred — <brief reason from user or 'not now'>"` so the logbase records the decision. This prevents cold rediscovery next pass.
- After each gesture, do **not** re-print the updated tension unless the user asks — the gesture commands echo enough.

Destructive gestures (`rm`, `release`) — confirm once more before running even if "all" was said. Irreversible structural changes deserve a second look.

## Step 6 — Record

After the queue is processed, write a single triage summary note on the scope anchor, then re-check signals.

**Resolving the anchor**, in order:
1. If `--scope <id>` was given, the anchor is `<id>`.
2. Else check `werk config get triage.anchor` — if set, use that short code.
3. Else, field-wide: use the tension that was actually touched most often by the pass (most applied gestures), falling back to the root of the touched subtree. Avoid writing the triage note on an untouched tension — that pollutes unrelated history.
4. If nothing was applied at all (all deferred or empty queue), skip the summary note and print one line: `no anchor note written (no gestures applied)`.

If the user runs `/werk-triage` field-wide and has no `triage.anchor` set, mention once that they can set one: `werk config set triage.anchor <id>` — ideally a meta-tension whose desire is literally the field's health. Do not nag.

Write the summary:

```bash
werk note <anchor> "triage <YYYY-MM-DD>: applied <N>, deferred <M>, skipped <K>. <one-line notable>"
```

Then run `werk flush` if any mutations were applied, so the triage pass lands in git-trackable state. A "no changes" response from flush is fine — the pre-commit hook may have already flushed.

Finish with a one-paragraph debrief: what was applied, what shifted in the signal counts (run `werk list --signals` quickly to confirm), what the next triage should watch.

## Quality checks

Before presenting the queue, verify:

- [ ] Every candidate has a bucket tag and a default gesture matching the bucket table
- [ ] Every candidate cites at least one logbase fact (date, epoch number, or note reference)
- [ ] Every proposed command is complete and runnable — no placeholders like `<id>` or `<date>`
- [ ] The queue is ≤7 items; severity order is respected
- [ ] No tension is diagnosed as "behind schedule" — always framed as a desire-reality gap or a structural violation
- [ ] Destructive gestures (`rm`, `release`) are flagged for explicit reconfirmation
- [ ] A healthy field produces "nothing to triage" and stops — no manufactured work
