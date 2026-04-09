# Hook Infrastructure Design

**Status:** Implemented (v2: HookBridge architecture). This document records the rationale.

**Emerged:** 2026-03-25 from Waterlight collaboration (tension #62) driving the need to specify hook payloads for external bridge integration.

---

## Purpose

Hooks allow external systems to react to structural changes in the tension tree. They are the instrument's event boundary — the point where a gesture's effects become visible to the outside world.

Hooks are not an extension mechanism for the instrument itself. They do not modify behavior, add features, or change what the instrument computes. They are a notification surface: "this happened."

---

## The Five Events

| Event | When it fires | Can block? | Purpose |
|---|---|---|---|
| `pre_mutation` | Before any mutation is persisted | Yes (exit non-zero) | External validation gate. Example: block mutations during maintenance windows. |
| `post_mutation` | After any mutation is persisted | No (fire-and-forget) | Universal change notification. Example: auto-commit the database, sync to external system. |
| `post_create` | After a new tension is created | No | Creation-specific notification. Example: initialize external tracking for a new tension. |
| `post_resolve` | After a tension is marked Resolved | No | Resolution-specific notification. Example: send completion notification, close external ticket. |
| `post_release` | After a tension is marked Released | No | Release-specific notification. Example: mark external work item as abandoned, unblock dependents. |

### Why these events and not others

The events correspond to the structural boundaries that external systems care about:

- **pre_mutation** exists because some external constraints are synchronous — they must be checked before the fact is recorded. This is the only blocking hook.
- **post_mutation** exists because every mutation is a structural change. Any system that mirrors the tension structure needs to know about all of them.
- **post_create**, **post_resolve**, **post_release** exist because lifecycle transitions have different semantic weight than field mutations. An external system that creates a tracking item on `post_create` and closes it on `post_resolve` or `post_release` should not have to filter `post_mutation` events by field and value.

Events that were considered and rejected:
- **pre_create**: No use case identified. Creation does not have a meaningful "block" scenario — the user intends to create, and no external system should prevent that.
- **post_reopen**: Reopen fires through `post_mutation` with `event: "active"`. Reopening is rare enough that a dedicated hook adds complexity without demonstrated need. If a bridge needs it, filtering `post_mutation` on `event == "active"` works.
- **Temporal events** (urgency threshold crossed, deadline approaching): Rejected. Urgency is recomputed on read, not stored. The instrument does not generate synthetic events from the passage of time. "Gesture as unit of change" is a sacred principle. External systems that need time-dependent signals should poll `werk survey --json` on an interval.

---

## Payload

All hooks receive a JSON object via stdin:

```json
{
  "event": "mutation" | "create" | "resolved" | "released" | "active",
  "timestamp": "ISO8601",
  "tension_id": "ULID",
  "tension_desired": "string",
  "current_reality": "string | null",
  "parent_id": "string | null",
  "field": "string | null",
  "old_value": "string | null",
  "new_value": "string | null"
}
```

### Field semantics

- **event**: The semantic category. For `post_mutation`, this is "mutation". For status changes, it is the lowercased new status ("resolved", "released", "active"). For `post_create`, it is "create".
- **tension_desired** / **current_reality**: The tension's desired and actual state at the time the event fires. These provide structural context without requiring the hook to query the instrument.
- **parent_id**: The tension's parent in the hierarchy. Enables DAG edge construction in external systems.
- **field** / **old_value** / **new_value**: Present on mutations. Absent on create events. For status changes, field is "status".

---

## Firing Rules Across Interfaces

| Interface | pre_mutation | post_mutation | post_create | post_resolve | post_release |
|---|---|---|---|---|---|
| **CLI** | All mutation commands | All mutation commands | `add` | `resolve` | `release` |
| **MCP** | All mutation tools | All mutation tools | `add` | `resolve` | `release` |
| **TUI** | Not yet implemented | Not yet implemented | Not yet implemented | Not yet implemented | Not yet implemented |

### CLI and MCP symmetry

Every gesture that fires hooks from the CLI fires the same hooks with the same payload from the MCP surface. The events are structurally identical — same `HookEvent` constructors, same fields, same firing order. A bridge consuming hook events cannot and should not distinguish whether the gesture originated from a human (CLI) or an agent (MCP).

### TUI

The TUI does not currently fire hooks. When it does, the same symmetry will apply: same events, same payloads, same constructors. The TUI performs gestures through the same `sd_core` store operations as CLI and MCP; the hook calls will be added at the same structural points.

---

## Configuration

Hooks are configured via `config.toml` in the workspace:

```toml
[hooks]
pre_mutation = "/path/to/script"
post_mutation = "/path/to/script"
post_create = "/path/to/script"
post_resolve = "/path/to/script"
post_release = "/path/to/script"
```

Set via CLI: `werk config set hooks.post_resolve "/path/to/script"`

### Why config.toml

Hooks are workspace-scoped. Different tension structures (different projects) may have different external integrations. Config.toml is the workspace-local configuration surface that already exists. No additional configuration mechanism is needed.

### Execution model

- Hook commands are executed via `sh -c <command>` with the JSON event piped to stdin.
- **pre_mutation**: synchronous. Exit 0 allows the mutation; non-zero blocks it. Stderr is shown to the user as the block reason.
- **All post_* hooks**: fire-and-forget. Failures are logged as warnings but do not block or roll back the gesture.
- Hook failure never corrupts the tension structure. The gesture has already been persisted before post-hooks fire.

---

## What Hooks Are Not

- Hooks are not triggers for instrument behavior. The instrument does not read hook output or change its behavior based on hook results (except pre_mutation blocking).
- Hooks are not an agent orchestration mechanism. The instrument does not launch agents, manage their lifecycle, or route work to them through hooks. Agents that want to react to structural changes subscribe to hooks through their own bridge logic.
- Hooks are not a plugin system. They cannot add gestures, modify the data model, or extend the instrument's vocabulary.

---

## V2: HookBridge Architecture (2026-04-09)

The original five manually-wired events have been replaced by an automatic bridge between the EventBus and the hook system.

### How it works

1. **Event enum** (`sd-core/src/events.rs`): Each variant has `hook_name()` (e.g., `"tension_resolved"`), `is_commandable()` (false for computed signals), and `category()` (e.g., `"mutation"`, `"create"`, `"status_change"`).

2. **HookBridge** (`werk-shared/src/hooks.rs`): Subscribes to the EventBus. When any event is emitted, it converts it to a `HookEvent` payload and fires matching post-hooks. Adding a new Event variant makes it hookable automatically.

3. **Pre-hooks** remain at the command level (CLI/MCP). The Store is a data layer and should not shell out. Commands call `hook_handle.runner.pre_mutation()` before calling Store methods.

4. **Post-hooks** are fully automatic. CLI/MCP commands use `workspace.open_store_with_hooks()` which attaches an EventBus + HookBridge. When Store emits events, the bridge fires post-hooks.

### Hook name resolution

For a given event (e.g., `reality_confronted` with category `mutation`), hooks fire in this order:

1. **Wildcard**: `post_*` — matches all post events
2. **Category**: `post_mutation` — matches all mutations
3. **Specific**: `post_reality_confronted` — matches this exact event
4. **Legacy**: `post_resolve` → matches `tension_resolved` (backward compat)

### Configuration

```toml
[hooks]
# Wildcard: fires for all events
"post_*" = ".werk/hooks/audit-log.sh"

# Category: fires for all mutations
post_mutation = ".werk/hooks/flush.sh"

# Specific: fires only for tension resolution
post_tension_resolved = "./notify.sh"

# Chain: multiple commands per event
post_tension_created = ["./a.sh", "./b.sh"]

# Filter: only fire for children of tension #10
post_tension_resolved = "./team-notify.sh"
"hooks.post_tension_resolved.filter" = "parent:10"
```

Global config (`~/.werk/config.toml`) and workspace config (`.werk/config.toml`) both fire. Global first.

### CLI management

```
werk hooks list [--verbose]
werk hooks add <event> <command> [--filter <f>] [--global]
werk hooks rm <event> [command] [--global]
werk hooks test <event> [--tension ID]
werk hooks log [--tail N]
werk hooks install [--git] [hook-names...]
```

### Shipped hooks

Installed via `werk hooks install <name>`:
- `flush` — calls `werk flush` after mutations
- `readme-tree` — updates README.md tension tree
- `auto-stage` — stages tensions.json and README.md
- `guard-delete` — blocks deletion of tensions with children (pre-hook)
- `audit-log` — appends event JSON to `.werk/audit.jsonl`
