# User Testing — surface, prerequisites, concurrency

## Validation Surface

The mission has three user-facing surfaces. They are validated through
different tools.

### Surface 1: CLI (primary)

`werk sigil <scope> [...]` produces SVGs. Validation runs the binary
directly and checks:
- Exit code (0 on success, 1 on user error, 2 on internal).
- Stdout / file contents (SVG bytes, JSON in `--json` mode).
- Determinism (same args → byte-identical SVG).
- Provenance metadata embedded.

**Tool:** raw `cargo run -p werk -- sigil ...` invocations,
`assert_cmd` for automated tests, `xmllint` for SVG sanity, `diff` for
byte-equality, manual eyeball in any SVG viewer for shape review.

### Surface 2: Web API (secondary)

`werk-web` exposes `GET /api/sigil` and `GET /api/sigil/stream`.
Validation:
- HTTP 200 + `Content-Type: image/svg+xml` for `/api/sigil`.
- SSE event `sigil_invalidated` after triggering a mutation.
- Cache hit/miss verified via filesystem inspection.

**Tool:** `curl`, `curl -N` for SSE, manual inspection. The `werk-web`
crate has no existing tests — workers must add at least basic smoke
coverage.

### Surface 3: werk-tab (M5 only)

Chrome MV3 extension. New-tab page renders glance preset.

**Tool:** `agent-browser` skill — load extension, navigate to new tab,
wait for sigil mode to be available, screenshot, assert SVG element
visible.

## Validation Prerequisites

| Prerequisite                | Verified during readiness check       |
|-----------------------------|---------------------------------------|
| Rust nightly + cargo        | `cargo --version` succeeds            |
| Crates.io reachable         | `cargo add --dry-run rhai` succeeds   |
| `~/.werk/sigils/` writable  | `mkdir -p $HOME/.werk/sigils/cache && touch test && rm test` |
| `tensions.json` present     | `test -f $REPO_ROOT/tensions.json`    |
| `xmllint` available         | `xmllint --version` (nice to have)    |
| For M5 only: Brave/Chromium | `which chromium-browser \|\| which brave-browser \|\| which google-chrome` |

For M5 specifically, the orchestrator may need to install a Chromium-class
browser on Zo (apt-get / brew) before validation. Document this in the
M5 user-testing handoff.

## Validation Concurrency

| Surface       | Per-instance footprint                              | Max concurrent | Rationale |
|---------------|-----------------------------------------------------|----------------|-----------|
| CLI           | ~50 MB (cargo run + sigil render)                   | **5**          | Lightweight; bounded by cargo lock contention which `cargo` handles. Shared `target/`. |
| Web API       | shared `werk serve` process (~100 MB) + curls       | **3**          | One daemon, multiple curls. Avoid race on cache writes (atomic rename mitigates). |
| werk-tab      | one Brave/Chromium per browser session (~500 MB)    | **2**          | Heaviest surface; browser instances multiply RAM. |

These are conservative for an 8 GB Zo machine. Adjust upward if Zo has
more headroom (check during readiness).

## Eyeball-review milestones

Three milestones gate on user eyeball review. The orchestrator must
collect SVGs and present them to the user before declaring the milestone
sealed.

| Milestone | What user reviews                                              |
|-----------|----------------------------------------------------------------|
| M2        | First contemplative render of a fixture subtree (one SVG)      |
| M3        | All five presets rendered against a fixture (five SVGs)        |
| M5        | Screenshot of werk-tab new-tab page in sigil mode (one PNG)    |

Validators commit these review artifacts under
`{missionDir}/validation/<milestone>/eyeball/` so the orchestrator can
locate them.

## Known testing gotchas

- **Time sensitivity.** Featurizers compute `urgency`, `staleness`,
  `last_pulse_at` against `Ctx::now`. Tests must pass a fixed `now` to
  get deterministic SVGs. The CLI uses `Utc::now()` at the boundary; the
  golden snapshot tests use a fixed `Ctx::now = "2026-01-01T00:00:00Z"`
  (or similar — workers pick a stable date and document it).
- **Hash stability.** `seed = blake3(canonical_inputs)`. The canonical
  serialization of `Scope` and `Logic` must be stable. Workers must
  use a deterministic serializer (sorted keys, no whitespace) — see
  `library/architecture.md#determinism`.
- **Rhai version pin.** Rhai's expression mode is stable, but minor
  versions can affect parse-error message text. Pin to `1.x` and assert
  parse-error messages by structure, not full text.
