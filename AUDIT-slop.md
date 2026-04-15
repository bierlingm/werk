# AUDIT-slop: AI slop / LARP / in-motion-work comment audit

Branch: `worktree-agent-a107aafa`
Scope: all `*.rs` files in the werk workspace (110 files, ~55k LOC).
Approach: codedb + ripgrep searches for known slop patterns, then per-match
triage against the "keep if it might help a new reader" bar.

## Headline

- **0** commented-out code blocks found.
- **0** `unimplemented!()` / `todo!()` stubs in source or tests.
- **0** `TODO:` / `FIXME:` / `XXX:` / `HACK:` markers in source or tests.
- **0** placeholder `it_works`-style tests.
- **0** marketing-language adjectives in rustdoc (grep for
  "elegantly|seamlessly|robustly|intuitively|effortlessly|state-of-the-art"
  returned nothing).
- **0** emojis in source positions where they are slop; all occurrences are
  intentional: domain glyphs (`✓`, `✦`, `✧`, `⚡`) used per CLAUDE.md's
  "glyphs carry meaning without color" rule, or Unicode test fixtures
  (`🎵 compose 音楽`) exercising multibyte handling.

This codebase is unusually clean. Most "slop" audit categories found zero
matches.

## Remediations landed

### Commit 1 — remove narration comments (`889d0113`)

15 deletions across 17 files. Pure narration — the identifier on the next
line already carries the meaning.

Representative examples removed:

| file:line | comment |
|---|---|
| `werk-core/tests/integration.rs:21` | `// Create a tension` above `engine.create_tension(...)` |
| `werk-mcp/src/tools.rs:2191` | `// Delete the tension` above `store.delete_tension(...)` |
| `werk-core/src/tree.rs:516` | `// Set the root` above `subtree.roots.push(...)` |
| `werk-cli/src/editor.rs:34` | `// Open the editor` above `Command::new(&editor)...` |
| `werk-cli/src/commands/split.rs:187` | `// Execute the split` above `store.begin_gesture(...)` |
| `werk-cli/src/commands/undo.rs:77` | `// Apply the undo` above `store.undo_gesture(...)` |
| `werk-cli/tests/nuke.rs:237` | `// Create a subdirectory` above `std::fs::create_dir_all(...)` |
| `werk-tui/tests/tui_flows.rs:111,146` | `// Add a tension` above `send(&mut sim, Msg::StartAdd)` |

Full set of touched files:
`werk-cli/src/commands/split.rs`,
`werk-cli/src/commands/undo.rs`,
`werk-cli/src/editor.rs`,
`werk-cli/tests/add_show.rs` (×2),
`werk-cli/tests/config.rs`,
`werk-cli/tests/discovery.rs`,
`werk-cli/tests/lifecycle.rs`,
`werk-cli/tests/nuke.rs` (×3),
`werk-cli/tests/reality_desire.rs`,
`werk-core/src/store.rs`,
`werk-core/src/tree.rs`,
`werk-core/tests/epoch_integrity.rs`,
`werk-core/tests/integration.rs`,
`werk-mcp/src/tools.rs`,
`werk-shared/src/config.rs`,
`werk-shared/src/workspace.rs`,
`werk-tui/tests/tui_flows.rs` (×2).

### Commit 2 — rewrite in-motion-work comments (`e54ab2ca`)

7 rewrites. Each one narrated migration history ("replaces X", "preserves
the old API so the mechanical replacement...", "for now, just return the
basic error") rather than describing present-tense intent.

| file:line | before → after |
|---|---|
| `werk-shared/src/error.rs:138` | "For now, just return the basic error…" → doc comment explaining why path args are accepted but unused |
| `werk-core/src/temporal.rs:30–32` | "This replaces the former text-similarity based magnitude computation which pretended to quantify…" → "Binary by design: string similarity would be an arbitrary quantification of something that is semantically present-or-absent." |
| `werk-core/src/store.rs:3365` | "This replaces direct parent_id column reads." → doc noting edges are source of truth |
| `werk-core/src/store.rs:3382` | "This replaces direct parent_id = ? queries." → deleted (sibling docstring already covers it) |
| `werk-core/src/edge.rs:6` | "(replaces the old parent_id column)" → "(source of truth for hierarchy; the `parent_id` column is retained only for backward compat)" |
| `werk-tui/src/app.rs:170,1332` | "replaces TransientMessage" / "replaces old TransientMessage" → present-tense role description |
| `werk-tui/src/toast.rs:1` | "Toast notification system — replaces TransientMessage." → "Toast notification system." |
| `werk-tui/src/theme.rs:83` | "Field names preserve the old `Styles` API so the mechanical replacement across deck.rs/render.rs/survey.rs is straightforward." → "Field names mirror the `Styles` API used across deck.rs, render.rs and survey.rs so callers can reach for styles by role (amber, selected, dim…) without knowing about theme resolution." |

## Categories inspected, nothing removed

### Grandiose section banners (`// ===...===`)

283 occurrences across 29 files. Inspected samples in `werk-shared/src/hooks.rs`,
`werk-shared/src/cli_display/glyphs.rs`, `werk-core/src/temporal.rs`,
`werk-core/src/events.rs`, `werk-cli/src/serialize.rs`, `werk-cli/src/commands/hooks.rs`,
and the integration test files. In every sampled case the banners group
structurally distinct regions (e.g. "Urgency" / "Horizon Drift" / "Gap Detection"
in temporal.rs; "STATUS glyphs" / "SIGNAL glyphs" / "TREE glyphs" in glyphs.rs;
test suites by subject area). They have real organizing value and aren't
redundant with the docstring below. Left in place.

The one banner label that WAS slop — "Gap Detection (honest — binary, not
text-similarity)" — got cleaned up in commit 2 (the parenthetical was
in-motion-work narration).

### Docstrings with parenthetical "(something)" qualifiers

Many present-tense docstrings use parentheticals for context (e.g.
`/// Get the parent ID for a tension (from contains edges).`). These read as
intentional present-tense documentation, not slop. Left alone.

### `// NOTE:` comments (7 total)

All 7 occurrences genuinely explain non-obvious constraints (test parallelism,
Rc internals in fsqlite, why tests use a temp HOME). Kept.

### Emoji / glyph characters

All occurrences are either (a) status/signal glyphs used with intent per
the "legibility without color" design rule, or (b) Unicode test fixtures
exercising multibyte string handling. Kept.

## Stubs / placeholders flagged

**None.** Exhaustive searches for `unimplemented!()`, `todo!()`,
`panic!("not `, `panic!("TODO`, and `it_works` returned zero matches.

## Excluded from scope

- `specs/*.qnt` — Quint convention, out of scope per task.
- `.werk/backups/*.json` — not source, ignored.
- `AUDIT-*.md` — other agents' outputs, not present in this worktree.
- Other worktrees under `.claude/worktrees/` — not this agent's branch.
- `designs/**/*.md` — text/markdown, not code comments.

## Verification

- `cargo check --workspace` passes after each commit.
- `cargo test --workspace --lib` — 494 + 131 + 25 tests pass, 0 fail.

## Commits on this branch

```
e54ab2ca Rewrite in-motion-work comments as present-tense intent
889d0113 Remove narration comments that restate code
```
