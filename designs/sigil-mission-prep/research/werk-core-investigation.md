# werk-core investigation (research artifact)

Conducted during prep. Used to inform the four IR designs and the
attribute registry mapping.

## Mapping: design attribute → werk-core source

| Group | Attribute | Source | Notes |
|---|---|---|---|
| Identity | `id` | `Tension.id` (ULID) | exists |
| Identity | `short_code` | `Tension.short_code: Option<i32>` | exists |
| Identity | `space` | NOT on Tension | inject from Ctx::workspace_name (D6) |
| Text | `desired` / `actual` | `Tension.desired` / `Tension.actual` | exists |
| Status | `status` | `TensionStatus` enum (Active/Resolved/Released) | derive 'held' via Frontier (D1) |
| Status | `is_held` | `Frontier::compute(...).held` | derive |
| Status | `is_resolved` / `is_released` | direct enum check | derive |
| Time | `created_at` | `Tension.created_at` | exists |
| Time | `updated_at` | NOT a column | derive: MAX(mutations.timestamp WHERE tension_id = ?) |
| Time | `deadline` | `Tension.horizon` (`Horizon::range_end()`) | exists; design conflates deadline/horizon |
| Time | `last_pulse_at` | undefined; map to last mutation timestamp (D5) | |
| Time | `age_seconds` | now - created_at | derive |
| Time | `time_to_deadline_seconds` | `Urgency.time_remaining` | exists |
| Computed | `urgency` | `compute_urgency(...).value` | clamp 0..1; expose `urgency_raw` (D4) |
| Computed | `staleness` | `Horizon::staleness(last_mutation, now)` | requires both inputs |
| Computed | `gap_magnitude` | `gap_magnitude(desired, actual)` | binary 0/1 (D3) |
| Trajectory | all | `MutationPattern.*` from `extract_mutation_pattern` | exists |
| Structure | `depth` | `Forest::depth(id)` | exists |
| Structure | `child_count` | derived | exists via Forest |
| Structure | `descendant_count` | `StructuralSignals.descendant_count` | exists |
| Structure | `parent_id` / `parent_short_code` | direct + lookup | |
| Structure | `note_count` | NOT a column; aggregate over mutations where field='note' | derive |
| Structure | `has_children` | derived | |
| Edge | `edge_type` | `Edge.edge_type` | translate names (D2) |
| Edge | `edge_weight` | does NOT exist | derive or drop in v1 |

## Key facts

- werk-core has **no schema_version table**; migrations are inline
  `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE ADD COLUMN` blocks in
  `Store::create_schema()` (`store.rs:362-757`).
- `Store::list_tensions()` returns `Vec<Tension>`.
- `Forest::from_tensions_and_edges()` is the canonical constructor for
  the tree IR. It already wraps an FNX `DiGraph`.
- `Store::get_epochs(tension_id) -> Vec<EpochRecord>` is the per-tension
  epoch reader. EpochRecord has `desire_snapshot`, `reality_snapshot`,
  `children_snapshot_json`.
- Edge type constants in werk-core: `EDGE_CONTAINS = "contains"`,
  `EDGE_SPLIT_FROM = "split_from"`, `EDGE_MERGED_INTO = "merged_into"`.

## sqlite migration site

Append `CREATE TABLE IF NOT EXISTS sigils (...)` at the end of
`Store::create_schema()`. Add indexes with `CREATE INDEX IF NOT EXISTS
...`. Update the schema docstring at `store.rs:14-73`.

## *N short-code parser changes

In `werk-core/src/address.rs`:
1. Add `Sigil(i32)` to the `Address` enum.
2. Add `Display` arm.
3. Add a `*N` branch in `parse_address_inner` (strip `*`, parse rest as
   short code).
4. Tests in the existing `mod tests` block.

## Test conventions

- `Store::new_in_memory()` for fixtures.
- `tempfile::TempDir` for filesystem.
- `#[test]` only; no `#[tokio::test]` (werk-core is sync).
- `std::thread::sleep(Duration::from_millis(10))` for monotonic
  timestamps in time-sensitive tests.

## Risks / surprises

- TensionStatus enum is fixed at 3 variants. "Held" is a Frontier
  classification, not a status. "Frozen" has no analogue. Resolved by D1.
- Edge type names diverge between werk-core and the design. Featurizer
  is the translation boundary (D2).
- `gap_magnitude` is binary, not continuous. Documented (D3).
- `urgency` can exceed 1.0; design says 0..1. Clamped to registry,
  unclamped exposed as `urgency_raw` (D4).
- `last_pulse_at` is undefined in werk-core; maps to last mutation
  timestamp (D5).
- `space` is workspace-level, not on Tension; inject from Ctx (D6).
- `note_count` requires aggregation over mutations table (no notes
  table).
- No cross-tension EpochSeries helper; v1 ships per-tension only (D9).
