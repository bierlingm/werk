# Kickoff prompt for the Zo orchestrator

Paste the message below as the first message in your Zo session.

---

You are the orchestrator for the **werk sigil engine v1** mission.

**Source of truth:**
- `designs/sigil-engine.md` is the architectural authority for the engine.
- `designs/werk-conceptual-foundation.md` is werk's sacred core — do not
  violate (`desire above actual`, `theory of closure`, `signal by exception`,
  `gesture as unit of change`, `locality`).
- `werk-sigil/presets/contemplative.toml` is the reference preset. The TOML
  schema crystallizes against it.

**Prep package:** A complete mission prep package was authored on a different
machine and committed to this branch at `designs/sigil-mission-prep/`. It
contains:

- `mission.md` (proposal text)
- `validation-contract.md` (behavioral assertions, organized by area)
- `features.json` (decomposed features with `fulfills` mappings)
- `AGENTS.md`, `services.yaml`, `init.sh`
- `library/architecture.md`, `library/environment.md`, `library/user-testing.md`,
  `library/sigil-engine-decisions.md`
- `skills/rust-implementer/SKILL.md`, `skills/frontend-integrator/SKILL.md`
- `research/` — investigation reports (read for context; do not commit
  edits to them)

**Your job, in order:**

1. **Read** the prep package end-to-end. Start with `README.md`, then
   `library/sigil-engine-decisions.md`, then `mission.md`,
   `validation-contract.md`, `features.json`. Skim the rest.

2. **Run one review pass on `validation-contract.md`** before proposing.
   Spawn one subagent per top-level area (Foundation, Engine, Vocabulary,
   Surfaces, Combinators, Hardening) plus one for cross-area flows. Each
   reviewer reads the full draft + the design doc + the relevant code areas
   and reports missing assertions (especially user-flow gaps, consequential
   behaviors, consistency expectations). Synthesize findings; update the
   contract; rerun if any pass turns up significant additions.

3. **Verify mission readiness.**
   - Confirm `cargo` toolchain works: `cd /path/to/repo && cargo --version`
     and `cargo build -p werk-core` succeeds.
   - Verify `tensions.json` exists at the repo root (fixture data; ~302
     tensions tracked).
   - Confirm `~/.werk/` is writable (workers will write archive files here).
   - Spawn a **dependency readiness** subagent that runs `cargo add --dry-run`
     for `rhai = { version = "1", features = [...] }` in a temp directory to
     confirm registry access.
   - Spawn a **validation readiness** subagent that confirms the Rust test
     toolchain works (`cargo test -p werk-core --no-run`) and that
     `tuistory` and `agent-browser` skills load if needed for any
     surface-level validation.

4. **Call `propose_mission`** with the contents of `mission.md` (after any
   adjustments from steps 1-3).

5. **Copy / symlink the prep into the live missionDir.** Concretely, after
   `propose_mission` returns the missionDir path, copy:
   - `prep/mission.md` is already used as the proposal — fine.
   - `prep/validation-contract.md` → `{missionDir}/validation-contract.md`
   - Initialize `{missionDir}/validation-state.json` with all assertion IDs
     in `pending`.
   - `prep/features.json` → `{missionDir}/features.json`
   - `prep/AGENTS.md` → `{missionDir}/AGENTS.md`
   - `prep/services.yaml` → `{missionDir}/services.yaml`
   - `prep/init.sh` → `{missionDir}/init.sh` (chmod +x)
   - `prep/library/*` → `{missionDir}/library/`
   - `prep/skills/*` → `{missionDir}/skills/`

6. **Run the artifact checklist** from your orchestrator role:
   - Every assertion ID in `validation-contract.md` is claimed by exactly one
     feature's `fulfills` (no duplicates, no orphans).
   - `services.yaml` has `commands.test`, `commands.lint`, `commands.build`.
   - `skills/*/SKILL.md` exists for each skillName referenced in
     `features.json`.

7. **Call `start_mission_run`.**

**Constraints you must enforce on workers:**

- Branch: stay on `sigil-engine-v1`. All commits land here. PR to `main`
  happens at end of mission.
- Sacred core: `designs/werk-conceptual-foundation.md` is untouchable.
  Sigils are *artifacts*, not *gestures* — rendering does not enter the
  gesture log.
- Stable registries (attribute-name, channel-name) are public API once
  shipped. Treat additions as deprecation-cycled rather than free.
- No external glyph assets in v1 — workers compose SVG path data inline
  (per user decision during prep). Glyph asset hot-reload is also out of
  scope (recompile required per design Part IX).

**Out of scope for v1 (do not build):** terminal renderer, `EpochRange`
animation axis, `AnimatedSvg` output, composite rules beyond `Concentric`,
E4 full scripting / WASM plugins, cross-tension `EpochSeries` builder, multi-host
concerns. See `designs/sigil-engine.md` Part IX.

**Mission size:** ~35 features across 6 milestones. Expect this mission to
run for many worker sessions. Validate at every milestone seal. If anything
in the prep contradicts the design doc or the sacred core, the design doc and
sacred core win — flag the contradiction and pause for orchestrator review
before continuing.

Begin by reading the prep package now.
