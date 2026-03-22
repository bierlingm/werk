# sc v1.0 — Category Definition & Seed Enrichment

---

## PART I: The New Category

### 1. Working Name

**Operative Instrument**

(From "operative" in the hermetic sense — a system that operates on the operator — and "instrument" in the musical sense — something played, not merely used.)

### 2. Definition

An operative instrument is a persistent software system that holds the structural forces behind a practitioner's work, launches and shapes agent activity within those forces, and updates its model of the practitioner through the practitioner's encounter with reality. It is simultaneously workspace, mirror, and altar: a place you work from, a surface that shows you what you are actually doing, and a consecrated structure whose maintenance is itself the practice. Unlike a tool (which serves goals), a framework (which organizes code), or an agent harness (which orchestrates execution), an operative instrument transforms the operator through use while transforming the world through the operator.

### 3. How It Differs

| Category | What it does | What sc does differently |
|---|---|---|
| **Tool** | Serves a task, then is put down. The user is unchanged. | sc is never put down. It is ambient. The user is changed by confronting their own structural tensions daily. |
| **Framework** | Provides scaffolding others build within. Prescribes architecture. | sc prescribes nothing. It holds forces and surfaces patterns. The practitioner decides what to do about them. |
| **Agent harness** | Orchestrates AI agents toward goals. The harness is infrastructure. | sc does not just launch agents — it shapes what agents see, and what agents produce feeds back into the structure. The harness is the least interesting part. |
| **Practice** | A discipline the practitioner performs. No persistent artifact. | sc is a practice that has a body — a persistent, evolving data structure. The practice and the instrument co-evolve. |
| **Game** | Creates meaning through rules and challenge within a bounded space. | sc has no win condition. The "rules" are the laws of structural tension (Fritz), not designed mechanics. Reality is the opponent, not a simulation of it. |

### 4. Essential Properties

Something belongs to this category if and only if it has ALL of these:

1. **Force-holding**: It makes invisible forces (tensions, contradictions, desires) explicit and persistent. Not tasks, not goals — forces.

2. **Practitioner transformation**: Regular use changes the operator, not just the output. The system is a mirror that cannot be ignored without cost.

3. **Ambient persistence**: It is not opened and closed. It is present — a background structure that shapes foreground activity. It is the room, not a tool on the bench.

4. **Agent origination**: It launches, shapes, and receives from autonomous activity (human or AI). Sessions depart from and return to it. It is the palace, not the quest.

5. **Self-modeling**: It maintains and evolves a model of the practitioner (or the situation, or both). The model predicts, observes, and updates on surprise. It learns.

6. **Structural integrity**: It respects the actual dynamics of the forces it holds. It does not gamify, simplify, or motivate. It confronts.

### 5. Partial Examples

1. **An alchemist's laboratory notebook** — Held the structure of the Great Work across years. Recorded observations, tracked transmutations, served as both workspace record and mirror of the practitioner's development. Lacked: agent origination, computational self-modeling.

2. **Emacs/Vim as a lifelong environment** — Ambient, persistent, the room you work in. Configured to the practitioner over decades; the configuration IS a self-model of sorts. Lacked: force-holding, structural integrity, explicit practitioner transformation.

3. **A Zettelkasten (Luhmann's, specifically)** — A second brain that surprised its operator. The card system surfaced connections the author didn't plan. Long-lived, ambient, self-organizing in ways the practitioner had to confront. Lacked: agent origination, force dynamics, self-modeling in the predictive sense.

4. **Anima (the sc self-modeling system)** — Predicts, observes, updates on surprise. Explicitly transforms the operator's self-knowledge. Has adversarial pressure and honesty mechanisms. Lacked (standalone): force-holding, ambient persistence, agent origination. It is a subsystem of an operative instrument, not one itself.

5. **A monastery's Rule (e.g., Rule of St. Benedict)** — A persistent structure that shaped all activity within it, transformed practitioners through daily confrontation with their own resistance, launched activity (the horarium), and maintained a model of the spiritual life that updated over centuries. Lacked: computational self-modeling, agent origination in the AI sense.

### 6. sc v1.0 as Full Expression

sc v1.0 would be the first software that fully inhabits this category:

**The Palace**: sc is always present. It renders ambient structural state in your terminal — a compact pane, a status bar line, a sigil. You see the shape of your will every time you glance at your terminal. Practice mode is the daily ritual of confrontation, but the ambient presence is the ongoing holding.

**The Spirit**: Inside the palace lives a tension tree — the generative structure of forces behind your work and life. These are not tasks. They are the gap between what is and what you want. The pulse engine reads the dynamics: what is moving, what is stuck, what is in conflict, what is being neglected. The spirit breathes whether or not you tend it.

**The Oracle**: The anima system predicts what you will do, observes what you actually do, and updates its model of you on surprise. It tracks avoidance. It generates adversarial counter-models. It issues interaction directives that change how AI agents collaborate with you. After 50 sessions, the system knows you in ways you do not know yourself.

**The Launcher**: `sc run` departs from a tension. The agent receives full structural context — ancestors, siblings, children, pulse data, conflicts. The agent works within a force field it did not create and cannot escape. When it finishes, its artifacts feed back: reality updates, new sub-tensions, resolved tensions. The structure evolves.

**The Mirror**: Heft emerges from the components — specificity, structural weight, activity, duration, convergence. You see which of your desires have heft and which are wishes. Tellability surfaces which desired states you can actually confront against reality and which are aspirational fog. The mirror does not judge. It shows.

**The Feedback Loop**: Agent sessions update structural state. Structural state shapes the next session. The anima model refines itself. Ambient presence keeps the structure in peripheral awareness. Practice mode forces conscious confrontation. The whole system compounds — not accumulating data, but converging on truth about what you actually want and what you are actually doing about it.

This is not a productivity tool that happens to have AI integration. It is an operative instrument — a consecrated workspace where the practitioner, the structure, and the agents co-evolve toward the practitioner's declared desired states, with the instrument itself serving as the honest witness to whether that evolution is actually happening.

---

## PART II: Seed Enrichment

---

### Seed 1: Language & Vocabulary

**Current essence**: Should sc develop its own vocabulary beyond Fritz's structural dynamics terms?

#### Ultimate Expression

sc develops a living lexicon — a small, precise vocabulary that practitioners learn through use, not study. The terms are not marketing ("Tensions" is already better than "OKRs"). They are compressed pointers to dynamics that Fritz named, that Doolittle illuminated, that daily practice surfaced, and that Fritz never named because he was not building software. The lexicon is versioned. Old terms are not deleted, they are marked as superseded, so the language itself has a visible evolution — a growth log of the instrument's conceptual development. Practitioners who use sc for a year can trace how their understanding deepened through the terms they acquired. The vocabulary IS the pedagogy.

#### Enables What Else

- An onboarding path that does not require reading Fritz. The terms teach the dynamics.
- A shared language for `sc-org` (organizational use). Teams need a common vocabulary; Fritz's books are not a realistic prerequisite.
- Crisp `--robot` API semantics. Agent-facing terms must be unambiguous.
- A distinctive identity for Statecraft Systems that is not derivative.

#### Variations

1. **Glossary-first**: A formal glossary ships with sc. `sc glossary heft` explains the term. The glossary is the manual.
2. **Emergent-only**: No formal vocabulary effort. Terms emerge from use and are documented only when they stabilize. The ideas.md "direction" field already suggests this.
3. **Dual-layer**: Fritz's terms remain the formal layer (the API, the data model). A colloquial layer emerges in the TUI and practice mode — more evocative, less precise, used in prompts and ambient display.

#### Tension With Other Seeds

- **Heft** and **Tellability** are themselves candidate vocabulary terms. If the vocabulary effort is deferred, these terms risk remaining informal and undefined.
- **Composability (sc-core)**: A stable vocabulary is a prerequisite for a stable API. If the terms keep shifting, sc-core consumers cannot rely on them. Tension between "let it emerge" and "stabilize for composability."
- **Prior Art (Fritz)**: Respecting Fritz's rigor while developing native terms requires careful lineage tracking. Risk of accidentally degrading precise terms into vague ones.

#### v1.0 Recommendation

**In v1.0, but minimally.** Ship with Fritz's vocabulary as the foundation. Introduce 2-3 native terms only where Fritz has no coverage (heft, tellability, pulse are candidates — Fritz did not name these dynamics because his work predates software that can compute them). Do not build a formal glossary system yet. Let practice mode surface the need.

---

### Seed 2: The Tool and the Spirit

**Current essence**: sc (the palace/fortress) and the structure inside it (the spirit/generative pattern) are distinct things. The tool serves the spirit.

#### Ultimate Expression

The distinction becomes architecturally real. The spirit — the tension structure, its dynamics, its history — is a portable, transferable, self-contained artifact. It could be exported and imported. It could live in a git repo shared with a team. It could be rendered by different instruments (sc-cli, sc-web, sc-org). The palace is one possible body for the spirit. The spirit could outlive the palace — if sc the software disappeared, the `structure.yaml` and mutation log would still be a legible, valuable artifact. Conversely, the palace without a spirit is an empty room. `sc init` creates the palace; the first `sc add` breathes the spirit into it. Practice mode is communion — the moment the practitioner consciously engages with the spirit. The rest of the time, the palace holds the spirit in ambient awareness, like an icon corner in an Orthodox home.

#### Enables What Else

- **Portability**: If the spirit is a well-defined artifact, it can be version-controlled, backed up, shared, forked. A team's shared spirit (in `.sc/` in a repo) is a living document, not just a config file.
- **Multiple instruments**: The spirit can be rendered by different UIs — terminal, web, mobile. The palace is a viewport, not a container.
- **Succession**: If sc the tool dies, the spirit survives as a structured artifact. No vendor lock-in on your own generative structure.
- **Agent comprehension**: Agents do not need to understand sc. They need to understand the spirit — the tension structure. The `--robot` API is already spirit-first, not palace-first.

#### Variations

1. **Formal separation**: `sc-spirit` as a file format spec. The structure.yaml is the canonical spirit artifact. sc (and future consumers) are all instruments that read/write spirit artifacts.
2. **Metaphor-only**: The distinction remains a design principle but is not architecturally enforced. The code does not change; the maintainer's mental model does.
3. **Liturgical framing**: The relationship between palace and spirit is made explicit in the TUI. Practice mode begins with an invocation ("You are about to confront the structure of your intentions") and ends with a dismissal. The ritual frame makes the spirit/palace distinction felt, not just known.

#### Tension With Other Seeds

- **Composability (sc-core)**: If the spirit is architecturally separated, sc-core IS the spirit layer. These seeds converge.
- **Ambient Presence**: The palace is what provides ambient presence. The spirit does not need to be ambient — it needs to be available. These are complementary, not conflicting.
- **Heft**: Heft is a property of the spirit, not the palace. If heft is computed, it lives in the spirit layer.

#### v1.0 Recommendation

**In v1.0 as a design principle, not a shipping feature.** The architecture should enforce the separation (model/store layers have no TUI/CLI dependencies — this is already the case). The `structure.yaml` export should be treated as the canonical spirit artifact. No need to build a formal spec or multi-instrument support yet. But nothing in v1.0 should make the spirit dependent on the palace.

---

### Seed 3: Heft

**Current essence**: Emergent weight of a tension — composed of specificity, structural weight, activity, duration, depth, convergence. Not a score; a felt quality.

#### Ultimate Expression

Heft becomes a first-class perceptual dimension in sc. Every tension has a felt weight that the practitioner can sense at a glance — not through a number, but through visual density. A tension with heft looks different from one without: its minibar is fuller, its subtree is deeper, its activity sparkline shows sustained motion, its desired state has multiple checkable conditions. Heft is the visual equivalent of picking up the ceramic mug vs. the plastic cup. Over time, the practitioner develops taste for heft — they learn to recognize when a tension lacks it and needs sharpening, or when a tension has too much (overspecified, brittle, unable to adapt). Heft becomes a design sense for one's own intentions.

The most powerful version: heft-awareness changes how the practitioner formulates desires. After months of seeing low-heft tensions stall and high-heft ones resolve, they naturally begin creating tensions with more specificity, more temporal commitment, more structural embedding. The instrument has taught them to want things properly. That is practitioner transformation through instrument use — the defining property of an operative instrument.

#### Enables What Else

- **Practice mode intelligence**: Practice mode can surface low-heft tensions for examination ("This tension has no checkable conditions and no children. Is it a wish or an intention?")
- **Agent prioritization**: `sc --robot ready` can rank by heft, directing agents toward the tensions most likely to benefit from work.
- **Structural health metrics**: Aggregate heft across the tree gives a picture of structural health. A tree full of low-heft tensions is a tree of wishes. A tree with high-heft tensions at every level is a serious generative structure.

#### Variations

1. **Computed composite**: Heft is an explicit multi-dimensional score displayed as a radar chart or minibar per tension. bv's approach — show all components, let the human synthesize.
2. **Visual emergence**: Heft is never computed as a number. Instead, the visual rendering of each tension encodes its components (sparkline, badge, children count, depth indicator). Heft is what the practitioner sees when all components are present. No single "heft" field exists.
3. **Practitioner-declared**: The practitioner can explicitly mark a tension as having heft or lacking it, overriding any computed signal. This preserves the human's authority over meaning.

#### Tension With Other Seeds

- **Tellability**: Tellability is a component of heft (specificity of conditions). A tension can have heft without being fully tellable (deep subtree, high activity, long duration, but vague desired state). These are related but not identical.
- **Language & Vocabulary**: "Heft" is itself a candidate native term — something Fritz does not name. If the vocabulary effort is deferred, heft risks remaining an informal concept.
- **Ambient Presence**: In ambient mode, heft determines visual prominence. High-heft tensions are visually "heavier" in the compact display. Low-heft ones are lighter, less demanding of attention.

#### v1.0 Recommendation

**In v1.0 as visual emergence (variation 2).** Do not compute a heft score. Instead, ensure the TUI renders enough components per tension (activity sparkline, children count, stage badge, age indicator) that heft is perceptible. Practice mode should surface low-component tensions for examination. The word "heft" can appear in documentation and practice prompts without being a formal metric.

---

### Seed 4: Tellability

**Current essence**: Desired states range from aspirational to tellable. Tellable ones create sharper structural tension because reality confrontation is precise.

#### Ultimate Expression

sc becomes a system that teaches practitioners to formulate tellable desires — not by requiring it, but by making the difference between tellable and untellable desires viscerally apparent over time. A tension whose desired state is "be healthy" never develops heft because reality updates are always vague. A tension whose desired state is "blood pressure below 120/80, measured weekly, by June 2027" develops heft rapidly because every reality update is a precise confrontation. The system does not force tellability. It surfaces it as information during practice: "This tension has been active for 90 days. Its desired state contains no conditions you can check. Its reality updates are general descriptions, not measurements. Consider: what would it look like if you could actually tell?"

The most powerful version: tellability becomes a meta-skill the practitioner develops. After six months of sc use, they naturally formulate desires with conditions, dates, measurements — not because the tool requires it, but because they have experienced the difference a hundred times. The instrument has trained their capacity for honest commitment.

#### Enables What Else

- **Automated convergence detection**: If desired states have structured conditions, sc can compute convergence — how close is reality to meeting each condition? This feeds heft.
- **Agent-checkable progress**: Agents can verify whether specific conditions are met, turning `sc --robot` into a genuine progress oracle rather than a context dump.
- **Temporal commitment**: Tellable tensions often have dates. Dates enable deadline awareness, time-pressure signals, and "this tension expires in 3 days" practice mode prompts.

#### Variations

1. **Structured conditions**: Tensions optionally carry a conditions array — each with a description, a target, and a current value. Practice mode walks the conditions. The pulse engine tracks per-condition convergence.
2. **Natural language analysis**: sc parses the desired state text for measurable conditions using heuristics (dates, numbers, "by", "at least", "no more than"). No structured input needed; the system infers tellability from prose.
3. **Practice mode prompt**: Tellability is never formalized. Instead, practice mode asks: "Can you tell whether this is happening? What would you check?" The practitioner articulates tellability in their reality update, not in a metadata field.

#### Tension With Other Seeds

- **Heft**: Tellability is a component of heft but not identical to it. Over-indexing on tellability could make sc feel like a goal-tracking tool rather than a force-holding instrument. Not everything that matters is measurable.
- **The Tool and the Spirit**: Structured conditions risk making the spirit feel like a project plan. The spirit should hold forces, not checklists. Tension between structural rigor and over-specification.
- **Language & Vocabulary**: "Tellable" is another candidate native term. Fritz uses "structural tension" and "current reality" but does not name the quality of precision in the desired state.

#### v1.0 Recommendation

**In v1.0 as practice mode prompt (variation 3).** No structured conditions yet. During practice, when a tension has been active for 30+ days with no convergence, the prompt asks the practitioner to consider tellability. This keeps it human-driven and avoids making sc feel like a goal tracker. If user demand emerges for structured conditions, add them in v1.1.

---

### Seed 5: Composability / sc-core

**Current essence**: The structural dynamics core could be a separable engine (sc-core as a Rust crate) that sc-cli, sc-org, sc-mcp, sc-web consume.

#### Ultimate Expression

sc-core becomes the canonical implementation of computational structural dynamics — a library that any software can use to hold, compute, and evolve tension structures. It is to structural dynamics what SQLite is to embedded databases: small, correct, embeddable, dependency-free. sc-core ships as a Rust crate with C bindings, usable from any language. It contains: the tension model, the store (SQLite-backed), the pulse engine, the mutation log, heft computation, tellability heuristics, and the structural conflict detector. No UI, no CLI, no opinions about rendering. Pure dynamics.

The most powerful version: sc-core becomes infrastructure that other people build instruments on. Someone builds a team instrument on sc-core. Someone builds a therapy instrument on sc-core. Someone builds an educational instrument on sc-core. The structural dynamics are universal — Fritz's insight is that tension resolution is the generative process behind ALL creation. sc-core makes that insight computational.

#### Enables What Else

- **sc-mcp**: An MCP server exposing sc-core to any AI agent natively. Claude, GPT, local models — all can query and update structural state without `--robot` shell calls.
- **sc-org**: Organizational structural dynamics with roles, shared tensions, delegation.
- **sc-web**: A browser UI for practitioners who do not live in the terminal.
- **Third-party instruments**: Other developers building their own operative instruments on the same engine.
- **Embedded sc**: sc-core inside other tools — an IDE plugin, a Slack bot, a mobile app.

#### Variations

1. **Crate extraction now**: Extract sc-core as a separate crate in a workspace. sc-cli depends on sc-core. Clean boundary from day one.
2. **Module boundary only**: Keep everything in one crate but enforce a clean module boundary (model, store, pulse have no TUI dependencies). Extract the crate when a second consumer appears.
3. **API-first**: Instead of a library, sc exposes a local API (Unix socket, HTTP, MCP) and consumers talk to the running sc daemon. The "core" is the API contract, not a crate.

#### Tension With Other Seeds

- **The Tool and the Spirit**: If the spirit is the tension structure and sc-core holds the spirit, then sc-core IS the spirit layer. These seeds are convergent.
- **Jeffrey Emanuel Ecosystem**: If sc-core uses fsqlite, fastmcp-rust, etc., the dependency surface grows. Tension between embeddability (small, dependency-free) and capability (search, MCP, async).
- **Ambient Presence**: Ambient rendering requires TUI code, which lives outside sc-core. But ambient state computation (what to render) lives inside sc-core. The boundary must be clean.

#### v1.0 Recommendation

**In v1.0 as module boundary (variation 2).** The current code already separates model, store, and pulse from tui. Enforce this: model/store/pulse must not import tui or clap. Do not extract a separate crate yet — there is no second consumer. When sc-mcp or sc-org appears, extract then. The boundary is what matters, not the crate structure.

---

### Seed 6: Prior Art (Robert Fritz Software)

**Current essence**: No active structural tension software exists. Fritz teaches courses and consults. The space is unoccupied.

#### Ultimate Expression

sc becomes the definitive computational expression of Fritz's structural dynamics — not a gamification, not a simplification, but a rigorous implementation that Fritz himself would recognize as faithful to the principles. Over time, sc may extend beyond Fritz (Doolittle, the anima system, heft, tellability), but the Fritztian core remains correct and un-compromised. If Fritz or his students ever encounter sc, they see their principles implemented with integrity. This is the moat: not features, but fidelity to the underlying dynamics. Any competitor that gamifies structural tension (streak counters, achievement badges, "resolve 5 tensions to level up") has departed from the principles and sc has not.

#### Enables What Else

- **Credibility with Fritz practitioners**: There are people who have taken Fritz's courses (thousands over 40 years). They are a natural audience if the implementation is faithful.
- **Pedagogical authority**: sc can teach structural dynamics through use, not just apply them. The tool IS the textbook.
- **Licensing/partnership potential**: If sc-core becomes the canonical implementation, a partnership with Fritz's organization (or successors) becomes possible.

#### Variations

1. **Orthodox implementation**: sc implements Fritz exactly. No extensions, no departures. Fritz's vocabulary, Fritz's stages, Fritz's structural conflict detection. Extensions live in separate modules that practitioners can opt into.
2. **Respectful extension**: sc implements Fritz as foundation and extends where practice reveals gaps (heft, tellability, agent integration, anima). Extensions are clearly marked as "beyond Fritz."
3. **Inspired-by**: sc takes Fritz's core insight (structural tension as generative force) and builds freely from there, without attempting fidelity to the full system.

#### Tension With Other Seeds

- **Language & Vocabulary**: Developing native vocabulary inherently departs from Fritz. The question is whether this departure is additive (new terms for things Fritz did not name) or substitutive (replacing Fritz's terms with alternatives).
- **Heft / Tellability**: These are extensions beyond Fritz. They should be framed as such — "Fritz describes structural tension; heft and tellability are properties of tension that emerge from computational tracking, which Fritz did not have."

#### v1.0 Recommendation

**In v1.0 as respectful extension (variation 2).** Fritz's vocabulary and dynamics are the foundation. Extensions (pulse, heft, tellability, agent integration, anima) are clearly rooted in Fritz but go beyond what he articulated. Documentation should cite Fritz where his concepts are used and note where sc extends them. Do not pursue licensing or partnership yet — prove the implementation first.

---

### Seed 7: Ambient Presence

**Current essence**: sc as persistent ambient display — tab 1, tmux panel, status bar. Not a tool you open and close but a room you inhabit.

#### Ultimate Expression

sc is always visible. Not as a distraction, but as a background gravity. The way a clock on the wall does not demand attention but shapes your awareness of time, sc's ambient presence shapes your awareness of structural tension. You glance at it and see: three tensions moving, one stuck for 12 days, a conflict between two siblings you have been avoiding. You did not decide to check. You saw it because it was there, the way you see weather through a window.

The most powerful version: ambient presence creates an unconscious feedback loop. The practitioner who sees their stuck tension in peripheral vision for a week eventually does something about it — not because the system nagged, but because the awareness became unbearable. This is the mechanism by which the instrument transforms the practitioner. Not by instruction, not by notification, but by persistent, honest, inescapable presence. The mirror you cannot cover.

#### Enables What Else

- **Notification-free urgency**: Stuck tensions, approaching deadlines, and structural conflicts become visible without notifications. Ambient awareness replaces interrupt-driven alerts.
- **Session warm-up elimination**: When you start a work session, you do not need to "check sc" — you already know the state from ambient presence. Practice mode becomes deepening, not catching up.
- **Contextual agent launches**: Because you see the structure constantly, `sc run` decisions are more informed. You know which tension needs attention because you have been seeing it all day.

#### Variations

1. **Compact pane mode**: A 20-30 column rendering showing the tree with stage badges, activity minibars, and conflict markers. Lives in a tmux split or terminal tab.
2. **Status bar mode**: A single line: structural health minibar + counts (3 moving, 1 stuck, 1 conflict). Lives in tmux status bar, terminal tab title, or shell prompt.
3. **Sigil mode**: A single glyph or small symbol that encodes overall structural health. Changes color/shape based on aggregate dynamics. Maximum compression — the entire state in one symbol you can read from across the room.
4. **Inline mode (ScreenMode::Inline)**: Renders a compact summary above each shell prompt. You see your structure every time you press Enter.

#### Tension With Other Seeds

- **The Tool and the Spirit**: Ambient presence is a property of the palace, not the spirit. The palace must be designed for ambient rendering. The spirit must be computable into ambient-friendly summaries.
- **Heft**: In ambient mode, heft determines visual weight. High-heft tensions are visually prominent; low-heft ones recede. This is a natural application of heft as a visual dimension.
- **Composability**: Ambient rendering is a TUI/CLI concern, not an sc-core concern. But ambient state computation (what to show in 30 columns) could live in sc-core as a "summary" function.

#### v1.0 Recommendation

**In v1.0 as compact pane mode (variation 1) and status bar mode (variation 2).** These are the highest-value, lowest-complexity ambient modes. The sigil is evocative but possibly premature. Inline mode is technically interesting but may be intrusive. Ship compact pane and status bar; let usage reveal whether sigil or inline modes are wanted.

---

### Seed 8: Jeffrey Emanuel Ecosystem

**Current essence**: Emanuel's 107+ crates are composable and offer smooth upgrade paths for search (frankensearch), formatting (rich_rust), storage (fsqlite), MCP (fastmcp-rust), async (asupersync), and terminal coordination (frankenterm).

#### Ultimate Expression

sc becomes a flagship consumer of the Emanuel stack — not dependent on it, but deeply integrated where the capabilities align. frankensearch-tui gives semantic search over the tension corpus and practice logs ("find the tension about team alignment"). fastmcp-rust gives sc a native MCP interface, making every AI agent a potential consumer of structural state. fsqlite gives concurrent multi-agent writes if sc enters daemon mode. frankenterm coordinates multi-agent swarm sessions launched from `sc run`. The Emanuel stack becomes to sc what the Spring ecosystem is to Java enterprise: the go-to set of well-composed capabilities that handle the hard infrastructure problems so sc can focus on structural dynamics.

The most powerful version: sc's adoption drives improvements in the Emanuel crates, and Emanuel's improvements flow back to sc. A symbiotic ecosystem where both sides compound. If Emanuel's crates become widely adopted (likely, given the quality and breadth), sc is well-positioned as a high-visibility user.

#### Enables What Else

- **Semantic search over practice history**: "When did I last confront this tension?" answered by meaning, not grep.
- **Native MCP integration**: Any MCP-capable agent can query sc without shell-calling `sc --robot`. Cleaner, faster, more capable.
- **Concurrent agent access**: Multiple agents writing to sc simultaneously without corruption.
- **Multi-pane agent orchestration**: `sc run` launches a swarm of agents across terminal panes, each focused on a different sub-tension, with frankenterm managing the coordination.
- **Rich CLI output**: `sc pulse` and `sc tree` in non-TUI mode rendered with rich_rust formatting.

#### Variations

1. **Selective adoption**: Adopt one or two crates where the value is highest (fastmcp-rust for MCP, rich_rust for CLI formatting). Keep the dependency surface small.
2. **Deep integration**: Adopt the full stack where applicable. Design sc's architecture to align with Emanuel's patterns (asupersync for the event loop, fsqlite for storage, ftui for TUI).
3. **Watch and wait**: Adopt nothing now. Monitor the ecosystem. Adopt when specific needs arise (MCP, concurrent writes, search).

#### Tension With Other Seeds

- **Composability (sc-core)**: If sc-core is meant to be small and dependency-free, deep Emanuel integration conflicts. The integration should happen at the sc-cli/sc-mcp layer, not in sc-core.
- **Ambient Presence**: asupersync becomes relevant if ambient mode requires a persistent event loop (background pulse monitoring, notifications). Without ambient mode, async is unnecessary.
- **Prior Art (Fritz)**: No tension. Emanuel's crates are infrastructure, not conceptual. They do not affect fidelity to Fritz's principles.

#### v1.0 Recommendation

**In v1.0 as selective adoption (variation 1).** Adopt fastmcp-rust if MCP integration is a v1.0 feature (recommended — it replaces the `--robot` shell interface with something cleaner). Consider rich_rust for non-TUI output formatting. Do not adopt fsqlite, asupersync, or frankenterm yet — they solve problems sc does not have until daemon mode or multi-agent swarms exist. Keep watching frankensearch-tui for when practice log search becomes a need.

---

## Summary Table

| Seed | v1.0? | Form |
|---|---|---|
| Language & Vocabulary | Yes, minimally | Fritz foundation + 2-3 native terms where Fritz has gaps |
| The Tool and the Spirit | Yes, as design principle | Module boundary enforcement, structure.yaml as spirit artifact |
| Heft | Yes, as visual emergence | TUI renders components; no computed score |
| Tellability | Yes, as practice prompt | Practice mode surfaces untellable tensions after 30+ days |
| Composability / sc-core | Yes, as module boundary | model/store/pulse must not depend on tui/clap |
| Prior Art (Fritz) | Yes, as respectful extension | Fritz is foundation; extensions are clearly marked |
| Ambient Presence | Yes, compact pane + status bar | Two ambient modes shipping in v1.0 |
| Jeffrey Emanuel Ecosystem | Selective | fastmcp-rust for MCP; rich_rust for CLI. Rest deferred. |
