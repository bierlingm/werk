# werk-tui Rewrite: Implementation Plan (final)

## Overview

Replace the existing TUI with a clean implementation of the Operative Instrument design. Single navigation model (descent), two-depth Gaze (quick + full), one-shot agent integration, ~2000 lines total.

---

## Core Design Decisions

### Navigation: Descent
You are always viewing one level of the tension forest. `Enter`/`l` descends into children. `Backspace`/`h` ascends to parent. No tree view. No dashboard/detail split.

### Information: The Gaze (two depths)
Space expands a tension inline. First press: desire, reality, gap, children preview, conflict. Tab inside Gaze: expands further to show all dynamics + history. Space again collapses. No separate "Study" view — Gaze is progressive.

### Acts: All from the Gaze
When a tension is gazed, all acts (edit, note, resolve, release, agent) target the gazed tension. When no Gaze, acts target the cursor selection. `action_target()` centralizes this.

### Agent: Two patterns
1. **One-shot (`@`)**: Write a prompt, werk sends it with tension context, response + mutations render inline. Primary pattern.
2. **Clipboard handoff (`@!`)**: Werk composes prompt + context, copies to clipboard, shows message. User pastes into their agent terminal. Changes flow back via CLI + file watcher.

### Chrome: Minimal
- Lever (bottom line): breadcrumb + position. Also serves as transient message surface (3s timeout: "tension resolved", "note added", "copied to clipboard").
- Hints (bottom line): just `? help`.
- No status bar. No app name on screen. Content starts at line 1.

### Layout: Capped width
Content capped at 100 columns, 2-char left indent. Wide terminals get right margin.

---

## State Machine

```
Navigation:
  parent_id: Option<String>     // None = root (The Field)
  siblings: Vec<FieldEntry>     // children of parent, or roots
  cursor: usize                 // selected sibling
  gaze: Option<GazeState>       // which sibling is expanded, and how deep

GazeState:
  index: usize                  // which sibling
  full: bool                    // false = quick (desire/reality/gap), true = full (+ dynamics + history)

Input Mode:
  Normal
  Adding { step: AddStep, parent_id: Option<String> }
  Editing { tension_id: String, field: EditField }
  Annotating { tension_id: String }
  Confirming { kind: ConfirmKind }
  Moving { tension_id: String, search: SearchState }
  Searching { search: SearchState }
  AgentPrompt { tension_id: String }     // writing the @ prompt
  ReviewingMutations { mutations: Vec<AgentMutation>, selected: Vec<bool>, cursor: usize }
  Help

AddStep:
  Name { buffer }
  Desire { name, buffer }           // Backspace on empty -> back to Name
  Reality { name, desire, buffer }   // Backspace on empty -> back to Desire
  // Esc on Name: cancel. Esc on Desire/Reality: create with what we have.
```

---

## File Plan (10 files, ~2000 lines)

### `lib.rs` (~60 lines)
Module declarations. `load_field()` (workspace discovery, engine, activity buckets). `run()` entry point.

### `state.rs` (~150 lines)
All types: `FieldEntry`, `GazeState`, `GazeData`, `FullGazeData`, `InputMode` + sub-enums, `SearchState`, `SearchResult`, `TransientMessage`.

### `vlist.rs` (~70 lines)
Variable-height virtual list. Tracks cursor-to-rendered-line mapping. Handles scroll. Rebuilt when Gaze toggles.

### `app.rs` (~200 lines)
`InstrumentApp` struct + all data methods:
- Navigation: `load_siblings()`, `descend()`, `ascend()`
- Gaze: `compute_gaze()`, `compute_full_gaze()` (dynamics + history)
- Targeting: `action_target()` (gazed if active, else selected)
- Agent: `build_agent_prompt()`, `send_oneshot()`, `copy_handoff()`
- Lever: `breadcrumb()`, `set_transient_message()`

### `render.rs` (~550 lines)
All rendering:
- `render_content()` — dispatch to empty/field/mutation-review
- `render_field()` — parent header (if descended), tension lines, inline gaze expansion
- `render_tension_line()` — glyph + name + trail
- `render_gaze()` — quick: desire/reality/children-preview/gap/conflict
- `render_full_gaze()` — adds dynamics section + history section below quick gaze
- `render_children_preview()` — mini-lines inside gaze (just glyph + name, no trail)
- `render_lever()` — breadcrumb or transient message (with 3s timeout)
- `render_empty()` — centered ◇ + "nothing here yet"
- `render_help()` — keymap overlay
- `render_search()` — input + results with parent breadcrumb
- `render_add_prompt()`, `render_edit()`, `render_confirm()` — inline
- `render_agent_prompt()` — inline text input for @ message
- `render_mutation_review()` — dedicated screen: tension, response text, mutation cards

### `update.rs` (~600 lines)
Model trait impl. Key routing by InputMode. Message dispatch.

### `agent.rs` (~100 lines)
- `send_oneshot(agent_cmd, tension_context, prompt) -> Result<(String, Vec<AgentMutation>)>`
  Calls configured agent command with context + prompt. Parses structured response.
  Runs via `Cmd::Task` (background thread, piped stdio — no terminal handoff needed).
- `build_handoff_text(tension_context, prompt) -> String`
  Composes rich text for clipboard. Includes `werk show <id>`, `werk reality <id> "..."` examples so the agent knows how to interact.
- `copy_to_clipboard(text)` — `pbcopy` on macOS, `xclip` on Linux.

### `search.rs` (~80 lines)
Fuzzy search across all tensions. `(root level)` entry for move-to-root. Parent breadcrumb in results.

### `glyphs.rs` (~50 lines)
`phase_glyph()` ◇◆◈◉, `status_glyph()` ✦·, `trail()` ○●, `gap_bar()`, separators.

### `theme.rs` (~80 lines)
6-color palette. Pre-computed styles.

---

## Key Interactions

### The Gaze (progressive expansion)

```
  ◇ Write the novel                               ○○○●

  [Space]

  ◇ Write the novel                               ○○○●
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  desire   A completed novel I'm proud of
  reality  42,000 words. Stuck on the third act.

    ◆ Finish first draft                    ○●●●
    ◇ Find an agent                         ○○○
    ◈ Resolve the ending                    ○●○●

  gap      ████████░░░░                     large
  conflict with "Learn to rest"
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

  [Tab — expand further]

  ...everything above, plus:

  ───── dynamics ─────
  phase        germination      still forming
  tendency     oscillating      attention comes and goes
  magnitude    large            significant gap
  orientation  creative         generative, not reactive
  ...

  ───── history ─────
  Mar 14  reality updated    "42,000 words..."
  Mar 08  child resolved     "Outline the structure"
  ...
```

Children inside the Gaze are a preview — just glyph + name. Press Enter on a gazed tension to descend into those children as a full list.

### One-shot agent (`@`)

```
  ◇ Write the novel                               ○○○●
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  desire   A completed novel I'm proud of
  reality  42,000 words. Stuck on the third act.
  ...

  @ I keep oscillating on the third act. What am I avoiding?_

  [Enter — sends to agent]

  [Agent response streams in, then mutations appear:]

  ┌ update desire ──────────────────────────────────┐
  │ "A completed first draft — imperfect is fine"   │
  └─────────────────────────────────────────────────┘
  [x] add child: "Write the bad version of act three"
  [ ] update reality: "Third act blocked by perfectionism"

                              a apply    Esc dismiss
```

### Clipboard handoff (`@!`)

```
  @ prompt: I need to do deep research on act three structures_

  [! — switches to clipboard mode]

  Lever: copied to clipboard — paste into your agent

  [User switches to Hermes terminal, pastes, works, runs `werk reality ...`]
  [File watcher picks up changes, Field updates live]
```

---

## Build Order

### Phase 1: Static Field (~400 lines)
state.rs → theme.rs → glyphs.rs → vlist.rs → msg.rs → app.rs → render.rs (field + lever + empty) → update.rs (j/k/g/G/q) → lib.rs

**Ship: tensions on screen, navigation works, glyphs and trails visible.**

### Phase 2: Descent (~200 lines)
Descend/ascend. Parent header. Breadcrumb lever. Cursor restore on ascend.

**Ship: full forest navigation via descent.**

### Phase 3: Gaze (~350 lines)
Quick gaze (Space). Full gaze (Tab inside gaze). Children preview. VirtualList integration. Gaze as action surface (action_target).

**Ship: progressive information disclosure working.**

### Phase 4: Acts (~350 lines)
Add (a) with back-navigation. Edit (e). Note (n). Resolve (r) + Release (x) with confirm. Undo (u). Filter (f). Transient lever messages.

**Ship: full CRUD.**

### Phase 5: Search + Help + Move (~250 lines)
Search (/). Help (?). Move-by-search (m).

**Ship: discoverability + reparenting.**

### Phase 6: Agent (~250 lines)
One-shot (@). Clipboard handoff (@!). Agent prompt input. Mutation review screen. Background task for agent call.

**Ship: complete Operative Instrument.**

### Phase 7: Polish (~100 lines)
File watcher. Amber trail tinting. Edge cases. `werk /query` CLI launch mode.

---

## What to Adapt vs Write Fresh

**Adapt from existing:**
- `phase_char()`, `activity_trail()`, `render_bar()` → glyphs.rs
- `build_detail_dynamics()` → compute_full_gaze()
- `resolve_agent_command()`, `execute_agent_capture()` → agent.rs
- Workspace discovery + DynamicsEngine → lib.rs
- Activity bucket computation → lib.rs
- `StructuredResponse::from_response()` → agent mutation parsing

**Write fresh:**
- VirtualList abstraction
- Descent navigation
- All rendering (Paragraph-based, no Table/List widgets)
- Gaze with progressive expansion + children preview
- Move-by-search
- Agent prompt input + clipboard handoff
- Mutation review screen
- Transient lever messages
- Content width cap
- Everything else
