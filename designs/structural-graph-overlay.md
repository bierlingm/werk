# Structural Graph Overlay: Typed Cross-Cutting Relationships

Design document for Phase 2/3 of FrankenNetworkX integration (#141 under #140).

Written 2026-03-31. For review before implementation.

## What exists after Phase 1

Forest holds a FrankenNetworkX DiGraph alongside its HashMap. The DiGraph has parent→child edges only. Four structural signals are computed from it:

- **Betweenness centrality** — identifies structural routing hubs
- **Topological generations** — concurrent possibility waves
- **Longest path** — the deepest structural chain (spine)
- **Transitive descendants** — blast radius of any tension

Signals surface in `show` (CLI, MCP, JSON) and TUI survey via ◉ HUB, ┃ SPINE, ◎ REACH glyphs. All computation is on-demand from the current tree structure.

## What this design adds

**Typed edges between tensions that aren't parent-child.** The practitioner can say "this enables that" or "these compete" — articulations of their own creative process, not management dependencies. The instrument computes from these relationships without enforcing them.

## The conceptual tension

The foundation (line 173) says: "The instrument does not track dependencies between tensions."

This is correct. We are not adding dependencies. Dependencies are management constraints ("B is blocked by A" — enforced, workflow-gating). What we add are **operative observations** — the practitioner's own reading of their creative field:

- "A enables B" = in my creative process, progress on A opens possibilities for B
- "A competes with B" = these draw from the same finite energy/attention
- "A informs B" = learning from A shapes how I approach B

The instrument computes from these observations (centrality shifts, community detection, flow analysis) without enforcing them. No tension is "blocked." No gesture is "gated." The user supplies the relationship; the instrument reasons from it, per the standard of measurement principle.

## Edge types

Three types, deliberately minimal:

| Type | Direction | Meaning | Example |
|------|-----------|---------|---------|
| `enables` | A → B | Progress on A opens possibility for B | #141 enables #142 (graph substrate enables visualization) |
| `competes` | A ↔ B | A and B draw from the same finite resource (attention, time, energy) | #3 (TUI) competes with #82 (GUI) |
| `informs` | A → B | Learning from A shapes approach to B | #13 (foundation) informs #89 (logbase) |

**`enables`** is directed: A enables B, not necessarily B enables A.

**`competes`** is symmetric: if A competes with B, B competes with A. Stored as two directed edges internally (A→B and B→A with type "competes") but presented as one relationship to the user.

**`informs`** is directed: A informs B means learning flows from A to B.

### Why not more types?

Minimalism. Three types cover the structural relationships a solo practitioner encounters. Adding types like "blocks", "requires", "depends-on" would cross into management territory. Adding types like "precedes", "follows" would duplicate temporal ordering (which the position system already handles). If a fourth type proves necessary through practice, add it then.

## Schema

```sql
CREATE TABLE edges (
    id TEXT PRIMARY KEY,
    from_id TEXT NOT NULL REFERENCES tensions(id),
    to_id TEXT NOT NULL REFERENCES tensions(id),
    edge_type TEXT NOT NULL CHECK(edge_type IN ('enables', 'competes', 'informs')),
    created_at TEXT NOT NULL,
    gesture_id TEXT,
    UNIQUE(from_id, to_id, edge_type)
);
CREATE INDEX idx_edges_from ON edges(from_id);
CREATE INDEX idx_edges_to ON edges(to_id);
```

No `attrs_json` column yet. The three edge types are sufficient. If attributes become necessary (e.g., strength, confidence), add the column then.

The UNIQUE constraint prevents duplicate edges of the same type between the same pair. Different types between the same pair are allowed (A enables B AND A informs B).

## Gestures

### `link`

```
werk link <from> <to> --type enables
werk link <from> <to> --type competes
werk link <from> <to> --type informs
```

Creates a typed edge. For `competes`, creates both directions (A→B and B→A) in a single gesture.

Validates:
- Both tensions exist
- No self-link
- No duplicate (same from, to, type)
- No linking resolved/released tensions (they're structurally inert)

Produces a mutation record on both tensions: `field: "link"`, `new_value: "enables #42"`.

Triggers signal recomputation (centrality, communities change when edges are added).

### `unlink`

```
werk unlink <from> <to> --type enables
werk unlink <from> <to>                   # removes all edge types between pair
```

Removes the edge. For `competes`, removes both directions.

Produces a mutation record: `field: "unlink"`, `new_value: "enables #42"`.

### Palette integration

When the practitioner resolves a tension that has `enables` edges pointing to other tensions, the instrument could offer a pathway palette: "This tension enabled #42 and #50. Update their reality to reflect the new possibility?" This is a signal-by-exception moment — the practitioner articulated the relationship, and now the instrument surfaces the structural consequence of resolution.

Not implemented in Phase 3. Noted for future consideration.

## Combined graph construction

`Forest::from_tensions_and_edges()` (or a separate builder) constructs the DiGraph with both containment edges and typed edges. Each edge carries an attribute distinguishing its type:

- Parent→child edges: `{"type": "contains"}`
- User-created edges: `{"type": "enables"}`, etc.

Forest methods (`children()`, `ancestors()`, `siblings()`) continue to filter to "contains" edges only. A new `forest.graph()` already exposes the full DiGraph — once overlay edges are added, algorithms run on the complete graph.

## New signals from the full graph

With typed edges, new computations become meaningful:

### Community detection (Louvain)

`louvain_communities()` identifies clusters of tensions that are densely connected by typed edges. In a field with 20 active tensions linked by `enables`/`informs` edges, Louvain might find 3 natural clusters. This reveals implicit project boundaries the practitioner hasn't explicitly created with containment.

Surfaced as: a new signal when a tension belongs to a community that doesn't match its containment parent. "This tension clusters with #36's children by relationship, but lives under #13 structurally." A structural tension — the containment says one thing, the relationships say another.

### Centrality shift

With typed edges, betweenness centrality changes. A leaf tension with many `enables` edges becomes a routing hub that the tree structure alone couldn't reveal. The HUB signal gains new meaning.

### Competing tension pairs

For `competes` edges: when both tensions in a competing pair have high urgency, surface a signal. "These compete and both are due soon." The practitioner articulated the competition; the instrument surfaces the temporal collision.

### Enabling chains

For `enables` edges: transitive closure of enabling relationships. "Resolving A enables B, which enables C." A progress chain the practitioner can reason about.

## What this does NOT do

- **No blocking/gating.** `enables` does not prevent the enabled tension from being worked on. It's an observation, not a constraint.
- **No automatic status changes.** Resolving an enabling tension doesn't auto-resolve or auto-advance the enabled one.
- **No weight/priority on edges.** The relationship either exists or it doesn't. Strength is the practitioner's judgment, not the instrument's.
- **No graph visualization (yet).** FrankenMermaid (#142) will handle structural visualization. This design is about the data model and computation, not rendering.

## Migration

The `edges` table is new and additive. No existing data changes. Schema version bump. Empty table on first migration — the practitioner builds the overlay graph over time through `link` gestures.

## Evidence ledger

When edges are added to the DiGraph via `link`/`unlink`, the DiGraph's `EvidenceLedger` records each mutation. This feeds into the logbase (#89) as structural change history: "On March 31, the practitioner articulated that #141 enables #142. On April 5, they removed the competition link between #3 and #82."

## Open questions

1. **Should `link` trigger an epoch?** Adding a structural relationship changes the practitioner's articulation of their creative field. Is that significant enough for a narrative beat? Probably not for every link, but maybe for the first link on a tension (the moment it becomes structurally connected beyond containment).

2. **Should resolved tensions keep their edges?** When A is resolved and A enables B, does the edge persist (for historical analysis) or get pruned (for signal cleanliness)? Recommendation: persist but exclude from signal computation. The edge is a historical fact; the signal is about current structure.

3. **Multi-workspace edges.** If the practitioner has multiple workspaces, can edges cross workspace boundaries? Not in Phase 3. This connects to #96 (global workspace) and #100 (cross-workspace addressing).

4. **Community detection threshold.** At what point does a community signal fire? How many typed edges are needed before Louvain produces meaningful communities? Needs empirical testing with real practice data. Connects to #139 (user-defined thresholds).

## Evaluation findings (2026-03-31)

This design was evaluated against the conceptual foundation before implementation. Findings:

### The reframing is partially honest

The distinction between "operative observations" (no enforcement) and "management dependencies" (gating, blocking) is real. `enables` without enforcement IS structurally different from `blocks`. But the *topology and computation* are management regardless of vocabulary. Community detection, enabling chains, and centrality shifts on a cross-tension relationship graph answer coordinator questions ("what clusters with what?", "what's the critical chain?"), not practitioner questions ("what's my reality and where can I move?").

The foundation's "Operative Not Managerial" section (line 173) doesn't say "the instrument does not *enforce* dependencies." It says "the instrument does not *track* dependencies between tensions." This design tracks cross-tension relationships and computes from them. Softer vocabulary doesn't change the topology.

### Locality violation

Section 7: "Meaningful signal propagates one level, not globally." Typed edges explicitly create non-local relationships. Community detection is explicitly global. This design expands the definition of "neighborhood" without acknowledging the cost to the locality guarantee.

### Standard of measurement is satisfied

Section 10 is clean — the user supplies the edge, the instrument computes from it. No instrument-originated standards. But satisfying one principle while straining two others (operative identity, locality) isn't sufficient justification.

### Marginal value at current scale

The owner's GitHub issue #2 response: "you don't need a formal link to see it — you need honest reality updates." At 70 active tensions, this remains true. The practitioner knows their field well enough to update reality directly. The marginal value appears at 200+ active tensions with long time horizons where the practitioner forgets prior articulations — but designing for that hypothetical violates the project's build-for-now principle.

The one genuinely valuable signal is **competing pair collision** (both urgent, practitioner said they compete). But one signal doesn't justify the full schema, gesture set, and computation pipeline.

### Correctly deprioritized

#142 (visualization), #143 (search), #144 (MVCC), #145 (ftui depth) all address current friction. #154 addresses a theoretical analytical gap. Repositioned from position 2 to position 7 under #140.

### Prerequisites identified

- #139 (user-defined thresholds) should be implemented first — typed-edge signals need configurable thresholds
- The current Phase 1 signals (HUB/SPINE/REACH) should be made configurable before expanding the signal taxonomy

### Disposition

Design preserved. Not rejected — deferred until practice reveals the need. Revisit when: (a) workspace exceeds ~150 active tensions, (b) multi-player sessions exist and coordination structures emerge organically, or (c) a practitioner reports that reality updates alone don't capture cross-tension patterns they need to see.
