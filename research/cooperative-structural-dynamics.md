# Cooperative Structural Dynamics

## The Starting Question

We began with a narrow technical question: should werk workspaces support a git-committable representation of their state? The workspace lives in a SQLite database (`.werk/sd.db`) which is binary and diffs poorly. The appeal of a committed snapshot file is obvious — tension state becomes visible in PRs, collaborators can see structural context without the CLI, and the workspace evolves visibly alongside code.

But exploring this question surfaced something much larger. The snapshot problem is a symptom of a deeper question: how does structural tension work when more than one person — or agent — participates in the same field?

## Why Snapshots Are the Wrong Frame

We evaluated four approaches to making workspace state git-visible:

1. **Commit the .db directly** with a `sqlite3 .dump` textconv driver. This gives readable diffs in local git but not in GitHub PRs. Phantom diffs from WAL checkpointing pollute `git status`. Merges are impossible — binary conflict, pick a side. Repo bloat is severe. Dead end.

2. **`werk snapshot`** — a command that writes a curated `.werk-state.json` to the project root. A lean format: tensions with id, desired, actual, status, parent_id, horizon, phase, tendency, urgency. No mutation history (noise in diffs), no full dynamics (volatile), no projections (derived). The snapshot is a derived artifact like a lockfile. The db remains authoritative.

3. **Auto-export hooks** on every mutation. Creates noise — five reality updates produce five rewrites. Less control than explicit snapshot. Couples every `werk` command to a git-working-tree file write.

4. **Match `werk context --json`** output. Too rich. A PR reviewer doesn't want 200 lines of dynamics JSON changing because someone updated a single reality. The context output is designed for real-time inspection, not diffing.

Option 2 is the most sensible if the goal is git visibility. But something about all four options felt like solving the wrong problem. What we actually want is not a snapshot for one person to commit — we want the workspace to be accessible to multiple participants who are jointly working within it.

## Fritz on Collective Structure

Robert Fritz's framework, which underwrites the entire werk data model, addresses collective work primarily in *Corporate Tides*. His key claims:

**Shared structural tension is more powerful than shared vision.** Vision is only the desired state. Structural tension includes both the vision and the collective ability to assess current reality objectively. An organization that can hold both sides of the gap — desired and actual — together, advances. One that holds only the vision oscillates.

**Authorship of the vision is irrelevant.** Fritz uses the filmmaking analogy: the first-chair violinist doesn't resent Beethoven for not consulting her. People join a shared creative process because they care about the outcome, not because they authored it. "If the vision matters more to us because we had a hand in creating it, then the vision's intrinsic value must be in question."

**Goals must be hierarchically related.** Every goal is the child of a parent goal, up to the organization's purpose. Without this hierarchy, goals compete in a "shotgun approach" that produces structural oscillation — success in one area causes difficulty in others, and the system swings back.

**A senior organizing principle is required.** Without one, the system oscillates. Someone must carry the structural burden — choosing between competing values rather than diplomatically balancing all of them.

**Groups can co-explore reality.** Fritz says structural consulting works better in groups because there are more sources of information. The group process starts without preconception, translates observations into visible forms, surfaces discrepancies, and tests assumptions until clarity emerges for everyone.

## Where Fritz Stops

Fritz's model for collective work is hierarchical. The vision originates from leadership (or organizational purpose) and cascades downward. Each person holds their own local structural tension, nested within the larger one. This is powerful and proven.

But Fritz does not address:

- **Peer-to-peer structural tension.** How equals co-create a shared desired state through dialogue or competition, without a single vision-holder.
- **Emergence.** How many individual tension fields compose into organizational-level structural tension. He asserts the same laws apply at all scales, but doesn't develop a theory of how the scaling actually works.
- **Evolutionary selection among competing sub-tensions.** When multiple plausible paths exist toward a shared desired state, Fritz says the leader must choose. He doesn't describe a process where the system itself discovers the path through differential success of competing approaches.
- **Structural dynamics between organizations** — alliances, ecosystems, markets.

## The War Band Model

There is an image that captures what cooperative structural dynamics might look like beyond Fritz's hierarchical model: the raiding party or war band.

Each participant carries their own tension field — their own desired/actual gap, their own sub-tensions about how to move. The collective doesn't coordinate through centralized command or consensus voting. Instead, it moves in the direction of whatever is actually working.

Sub-tensions function as **theories or assertions** about which way to move toward the overarching desired state. They compete not through argument but through resolution — a sub-tension that successfully closes its gap demonstrates its viability. One that stalls or oscillates reveals its structural inadequacy. The system evolves: successful structural patterns propagate, unsuccessful ones dissolve.

This is closer to variation-selection-retention in evolutionary epistemology than anything in Corporate Tides. It preserves Fritz's core mechanics (structural tension drives movement, oscillation signals structural conflict, the path of least resistance follows the underlying structure) while extending them into a domain Fritz didn't map: cooperative emergence without central authority.

The key structural questions this raises:

**Who holds the senior tension?** Fritz insists one is necessary. In the war band model, the overarching tension might be held by a person, by the group collectively (each maintaining their own understanding of it), or by a document that no one owns but everyone references. The answer determines whether the system needs hierarchy, consensus, or evolutionary competition.

**What does endorsement mean?** If one participant creates a sub-tension and another supports it, what changes structurally? Does endorsement increase urgency? Allocate attention? Or is it purely informational — "I also see this as a viable path"?

**How do competing sub-tensions resolve?** In evolution, competition resolves through differential reproduction. Here, a sub-tension "wins" when its actual converges with its desired. But what happens to losing sub-tensions? Are they released? Absorbed? Does the system choose explicitly, or does the choice emerge from what people actually work on?

**What is the unit of cooperation?** The whole tension tree? A subtree? A single tension? Can someone share one branch of their tension field without exposing the rest?

## The LLM Dimension

This question gains new urgency in an LLM-enabled world because agents can now participate as tension-holders. An agent can:

- Spawn sub-tensions as hypotheses about how to advance toward a desired state
- Attempt resolution autonomously
- Report back on whether the gap actually closed
- Be evaluated by a human or another agent on whether the structural movement was real

This makes the evolutionary model concrete and computational. Sub-tensions are cheap to create, attempt, and discard. The system can explore many paths simultaneously. The human's role shifts from directing every action to holding the senior structural tension and evaluating which sub-tensions actually advanced toward it.

## Technical Landscape

We evaluated three technologies for the cooperative implementation:

**cr-sqlite / Automerge / Loro** — CRDT-based approaches where each participant maintains a local database that can merge conflict-free with others. The append-only mutation log in werk maps naturally to CRDT merge operations. This is the most practical path today.

**Unison** — content-addressed code with algebraic effects ("abilities"). Philosophically aligned: tensions as content-hashed immutable values, mutations as typed operations that can be shipped between nodes, local vs. shared tension fields as different ability handlers for the same interface. But the ecosystem is immature — no CRDT library, no P2P layer, no local-first story, no database drivers, no C FFI. Worth watching, not ready to build on.

**asupersync** — turned out to be a structured concurrency async runtime, not a data synchronization system. Not relevant to this problem.

The honest assessment: technology choice should follow structural model design, not lead it. The harder work is defining what cooperation means within structural dynamics — the mechanics of shared tension fields, endorsement, evolutionary selection among sub-tensions, and the relationship between individual and collective structural tension. The implementation serves the structure.

## What Comes Next

The Foundations for Structural Thinking course (end of April 2026) is a direct input to this design. It provides access to the structural consulting methodology and the community of practitioners who work with Fritz's framework at the organizational level. The questions about collective structural tension — particularly around peer-to-peer dynamics and emergence — can be probed with people who have operational experience.

In the meantime, the immediate technical work on werk (the TUI rebuild, the existing single-user dynamics) continues. The cooperative model is a design exploration that will inform future architecture, not something to implement prematurely.

The `werk snapshot` command may still be worth building as a simple, practical tool — a curated JSON export of current state for git visibility. It solves a real, narrow problem without pretending to solve the cooperation problem. But it should be understood as a stepping stone, not the destination.
