---
name: rust-implementer
description: Implements Rust backend changes for the sigil engine — werk-core, werk-sigil, werk-cli, werk-web.
---

# rust-implementer

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

All Rust feature work in this mission, across `werk-core`, `werk-sigil`,
`werk-cli`, and `werk-web`. The same conventions apply across crates.

## Required Skills, Tools, and Dependencies

- **Toolchain:** `cargo` (nightly, edition 2024). Never `rustup default` or
  modify `rust-toolchain.toml`.
- **Test framework:** standard `#[test]` + `tempfile` for fixtures + `assert_cmd`
  + `predicates` for CLI integration tests.
- **External crates introduced in this mission:** `rhai`, `quick-xml`,
  `blake3`, `notify`, `rand_chacha`. Versions/features per
  `library/environment.md`.
- **Workspace surgery:** when adding a new crate, edit the root `Cargo.toml`
  `[workspace] members` list. When adding a path dep, add the line to the
  consuming crate's `Cargo.toml` and verify `cargo build` from repo root
  resolves it.
- **No skill invocations** — Rust work doesn't need agent-browser or
  tuistory.

## Work Procedure

### 1. Read first
- Re-read the feature description, `expectedBehavior`, `verificationSteps`,
  and `fulfills` IDs.
- Open the relevant file(s) the feature touches. Read enough surrounding
  code to match the established style (~50-100 lines around the change
  site).
- For new files in `werk-sigil`, mirror the structure documented in
  `library/architecture.md`. For new code in existing crates, match the
  patterns of nearby modules.
- For features that touch shared registries (attribute names, channel
  names), re-read `library/sigil-engine-decisions.md` D1, D2 — the
  decisions are load-bearing.

### 2. Tests first (TDD)
- Write failing tests in a single edit. Run `cargo test -p <crate> --
  <test_name>` to confirm RED.
- Test what the feature must achieve, not how it's implemented. Reference
  the `fulfills` assertion IDs in test names where they line up.
- Determinism tests (for sigil engine work): pass a fixed `Ctx::now` and
  fixed `seed`; assert byte-equality on output.
- For featurizers: use `Store::new_in_memory()`; build small known
  tensions; assert specific attribute values for specific tension IDs.
- For stages: build a fixture IR by hand; pass it through one stage;
  assert specific output structure.

### 3. Implement
- Smallest change that makes tests pass.
- `#![forbid(unsafe_code)]` if creating a new lib crate.
- No `unwrap()` outside tests. Use `?` and `Result`. Crate-local error
  enums via `thiserror`.
- Add `pub use` re-exports at lib root for new public types.
- For workspace `Cargo.toml` edits: also confirm the consuming crate's
  `Cargo.toml` lists the dep with the right path/version.

### 4. Verify
- `cargo fmt --all -- --check` — must pass. Run `cargo fmt --all` if it
  doesn't.
- `cargo test -p <crate>` — must pass for the crate you touched.
- `cargo test --workspace` — must pass before handoff.
- `cargo clippy -p <crate> --all-targets` — read warnings; fix any new
  ones your change introduced. Pre-existing warnings outside the change
  are fine.
- For CLI changes: invoke the binary manually
  (`cargo run -p werk -- sigil 2 --logic contemplative --out /tmp/x.svg`)
  and inspect the output to confirm it matches expected behavior.
- For web changes: start `werk serve` on port 3749; curl the new endpoint;
  confirm response. Stop the server (`lsof -ti :3749 | xargs kill`).
  Do not leave processes running.

### 5. Snapshot discipline
- If your change should produce new SVG output, run the relevant snapshot
  test once with `WERK_UPDATE_SNAPSHOTS=1` to seed the snapshot file,
  then run again **without** the env var to confirm equality.
- Diff the snapshot file in your favorite SVG viewer (or any text editor)
  to convince yourself the output is correct before declaring done.
- Record the diff observation in the handoff (`interactiveChecks`).

### 6. Architecture discipline
- The engine `(Scope, Logic) -> Sigil` is **pure**. Stages may not call
  `Utc::now()`, may not write to disk, may not make network calls. The
  `Ctx::now` field is the only clock.
- Sigils are **artifacts, not gestures**. Do not add code paths that emit
  events into werk-core's gesture log when rendering.
- The 4 IRs live in `werk-core` (not `werk-sigil`). Featurizers in
  `werk-sigil` build them via `werk-core` APIs.
- Loud at construction; graceful at render — see `library/architecture.md`
  "Error Handling Discipline".

### 7. Commit
- Stage explicit paths only (`git add path/...`). Never `git add -A`.
- Commit message: imperative mood, one-line summary; body if non-trivial.
  Reference feature ID. Example:
  `feat(werk-sigil): add RadialMandala layouter (M2 / sigil-radial-mandala)`

## Example Handoff

```json
{
  "salientSummary": "Implemented werk-core::ir::TensionTree IR (feature ir-tension-tree). Wraps Forest with a side-table HashMap<String, Attributes>; AttributeBuilder pulls 12 design-registry attributes (urgency, staleness, depth, child_count, status, is_held, etc.) by joining store reads with computations from werk_core::temporal and werk_core::projection. cargo test -p werk-core passes (47 tests, +6 new). cargo clippy clean.",
  "whatWasImplemented": "Created werk-core/src/ir.rs with public types: IrKind enum, Ir trait, Attributes struct, AttributeValue enum (Number, Text, Bool, Categorical), TensionTree struct, TensionTreeBuilder. The builder accepts (Store, Vec<Tension>, Forest, Ctx::now) and emits a fully-attributed TensionTree. is_held attribute uses Frontier::compute scoped to the tree's roots. Edge type translation contains→parent_child, merged_into→merge_into is wired but not exercised yet (no AttributeGraph yet — separate feature). Status mapping follows D1 (3 enum variants + held derivation). Re-exported TensionTree, IrKind, Attributes, AttributeValue from lib.rs.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo test -p werk-core ir", "exitCode": 0, "observation": "6 new tests pass: builds_empty_tree, builds_three_node_tree, computes_urgency_clamped, derives_is_held_for_unstarted_horizon, status_categorical_includes_held, edge_type_translation_in_attribute_join (the last one currently empty since AttributeGraph builder not yet implemented; placeholder)" },
      { "command": "cargo test --workspace", "exitCode": 0, "observation": "all 384 tests pass (was 378 before)" },
      { "command": "cargo fmt --all -- --check", "exitCode": 0, "observation": "clean" },
      { "command": "cargo clippy -p werk-core --all-targets", "exitCode": 0, "observation": "no new warnings; 2 pre-existing on store.rs untouched" }
    ],
    "interactiveChecks": [
      { "action": "manually built a 5-node tree with mixed status, ran the builder, dumped attributes via dbg!", "observed": "urgency clamped to 1.0 for past-horizon node; is_held=true for node with horizon starting next month; child_count matches expected" }
    ]
  },
  "tests": {
    "added": [
      {
        "file": "werk-core/src/ir.rs",
        "cases": [
          { "name": "builds_empty_tree", "description": "Empty tension list produces an empty TensionTree with zero entries." },
          { "name": "builds_three_node_tree", "description": "Root + two children produces a tree with attributes populated for all 3, depths 0/1/1." },
          { "name": "computes_urgency_clamped", "description": "Past-horizon tension produces urgency = 1.0 in the registry attribute (clamped per D4); urgency_raw available unclamped." },
          { "name": "derives_is_held_for_unstarted_horizon", "description": "Active tension whose horizon range starts in the future is is_held=true and status='held' in the categorical attribute." },
          { "name": "status_categorical_includes_held", "description": "When fed a held tension, attributes['status'] == AttributeValue::Categorical('held'); is_held bool is also true." },
          { "name": "rejects_unknown_attribute_in_request_list", "description": "AttributeBuilder::compute(['urgency', 'not_real']) returns Err with the unknown name in the message." }
        ]
      }
    ]
  },
  "discoveredIssues": []
}
```

## When to Return to Orchestrator

- The feature requires expanding `TensionStatus` enum or otherwise touching
  the sacred core (`designs/werk-conceptual-foundation.md`).
- `services.yaml` has a wrong command and the right command isn't obvious.
- A pre-existing test outside the change area starts failing and is
  unrelated to the feature.
- The TOML schema needs to break compatibility with `contemplative.toml`
  (the reference preset is the schema authority — see `mission.md`).
- The Rhai dep tree pulls something blocked on Zo or licensed
  incompatibly.
- A snapshot diff looks wrong but you cannot tell whether your
  implementation or the snapshot is the truth.
