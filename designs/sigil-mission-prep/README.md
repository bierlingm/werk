# Sigil Engine v1 — Mission Prep Package

This directory is **mission preparation material**, drafted on a different machine
and committed to the repo so the mission can be executed on a Zo (BYOM) Droid
Computer.

It is **not** a live missionDir. The orchestrator that runs on Zo will:

1. Read every file in this directory as authoritative input.
2. Do a quick review pass (one round, focused on the validation contract).
3. Call `propose_mission` (which allocates a real `missionDir` on Zo).
4. Copy the contents of this directory into that `missionDir` (or symlink — see
   the kickoff prompt).
5. Call `start_mission_run`.

## How to start the mission on Zo

In the Factory **desktop app**:

1. Open a new session.
2. Backend: select your **Zo** Droid Computer.
3. Working directory: the `werk` repository on Zo, on the branch
   `sigil-engine-v1` (this branch).
4. Paste the contents of `kickoff-prompt.md` as your first message.

The orchestrator on Zo takes it from there.

## Files in this package

| File                           | What it is                                                    |
|--------------------------------|---------------------------------------------------------------|
| `kickoff-prompt.md`            | The exact prompt to paste in the Zo session                   |
| `mission.md`                   | Mission proposal — title, scope, milestones, infrastructure   |
| `validation-contract.md`       | Behavioral assertions defining "done"                         |
| `features.json`                | Decomposed feature list with `fulfills` mappings              |
| `AGENTS.md`                    | Operational guidance for workers (boundaries, conventions)    |
| `services.yaml`                | Commands + service definitions                                |
| `init.sh`                      | Idempotent environment setup                                  |
| `library/architecture.md`      | How the engine works — components, IRs, pipeline              |
| `library/environment.md`       | Env vars, deps, platform notes                                |
| `library/user-testing.md`      | Validation surface, prerequisites, concurrency                |
| `library/sigil-engine-decisions.md` | Decisions resolved during prep (status, edges, expr lib) |
| `skills/rust-implementer/SKILL.md`     | Worker procedure for Rust backend work             |
| `skills/frontend-integrator/SKILL.md`  | Worker procedure for werk-tab JS work              |
| `research/`                    | Reference reports from prep investigation                     |
| `validation/`                  | Empty; mission state lives here on Zo at runtime              |

## Source authorities (read these first)

- `designs/sigil-engine.md` — the architectural authority for the engine
- `designs/werk-conceptual-foundation.md` — sacred core; do not violate
- `werk-sigil/presets/contemplative.toml` — reference preset; TOML schema crystallizes against it

## Notes for the orchestrator on Zo

- The validation contract was drafted on a single pass during prep. Run **one
  review pass** (per-area subagents) on it before `propose_mission`. If
  reviewers surface significant gaps, add assertions and re-check.
- The decisions captured in `library/sigil-engine-decisions.md` are
  load-bearing — particularly the status taxonomy resolution (`is_held` is a
  derived featurizer attribute, not a `TensionStatus` enum variant). Do not
  expand `TensionStatus` without separately consulting the sacred core.
- `tensions.json` at the repo root is a tracked snapshot of werk-state and is
  available on Zo via git. Workers can use it as fixture data without needing a
  live `.werk/` database.
