# Synthesis Map: Tools, Systems, and Structures for sc v1.0

> Research artifact. For each tool/system: movement grammar, affordance shaping, extractable idea for sc.
> The core question: how do diverse systems move people from current-state to desired-state, and how does the system's architecture shape the person using it?

---

## 0. The Anima Pattern: Prediction-Observation-Surprise as Structural Tension Engine

The anima plan (`research/anima_plan.md`) describes a self-modeling loop:

1. **Predict** — from a self-model (core patterns, active tensions), generate testable predictions about what will happen next session.
2. **Observe** — session runs; beats accumulate.
3. **Surprise** — stop hook compares predictions to evidence. Confirmed predictions weakly reinforce. Violated predictions trigger model updates ("Recent Shifts").

This is structurally identical to sc's tension primitive but applied to *identity* rather than *projects*:
- `desired` = "who I think I am / what I think I'll do" (the prediction)
- `actual` = "what actually happened" (the evidence)
- `resolution` = model update (the surprise-driven synthesis)

Key anima innovations relevant to sc v1.0:
- **Distinctiveness probes**: "Would this prediction be true for anyone, or specifically for this person/project?" Prevents the model from converging on platitudes. sc equivalent: a tension where desired="clean code" and actual="messy code" is generic; the system should nudge toward specificity.
- **Behavioral audit vs. calibration**: Two epistemic channels. Calibration refines what you know is uncertain. Audit surfaces what you don't know you're wrong about. sc needs both: tracking whether you're closing the gap (calibration) AND whether you've named the right gap (audit).
- **Shadow tensions**: Contradictions in behavior not yet captured as explicit tensions. The system detects them from patterns, not from user declaration. This is the deepest idea: sc should eventually *propose* tensions the user hasn't articulated.

---

## 1. Goal/Outcome Tools

### 1.1 Gtmhub / Quantive (OKR platform)

- **One-line**: Enterprise OKR software that cascades objectives through organizational hierarchy.
- **Movement grammar**: Define Objective (qualitative desired-state) + Key Results (quantitative current-to-target measures). Progress is measured by KR metric movement. The gap between KR baseline and KR target IS the tension. Quarterly cadence forces re-evaluation.
- **Affordance shaping**: By requiring numeric KRs, it forces users to operationalize vague desires into measurable proxies. This is both powerful (you can't lie about numbers) and distorting (you optimize for the measurable, not the meaningful). The cascade structure means individual tensions are always framed as contributions to someone else's tension above.
- **Extractable idea**: **Operationalization pressure** — sc could have an optional `measure` field that asks "how would you know the gap is closing?" without requiring it. The act of trying to answer that question sharpens both desired and actual.

### 1.2 Ally.io (now Microsoft Viva Goals)

- **One-line**: OKR tool emphasizing alignment visualization across teams.
- **Movement grammar**: Same OKR structure, but with explicit "alignment" views showing how your objectives connect to others'. Movement = update check-ins on KRs, which roll up.
- **Affordance shaping**: The alignment visualization makes dependencies visible. You can see when your tension depends on someone else closing theirs. This creates social pressure but also social awareness.
- **Extractable idea**: **Dependency visibility** — sc's parent/child tree already implies dependency. v1.0 could make it explicit: "this tension can't resolve until its children resolve" vs. "this tension can resolve independently." The constraint itself is informative.

### 1.3 Weekdone

- **One-line**: Weekly OKR check-in tool with PPP (Plans, Progress, Problems) reporting.
- **Movement grammar**: Weekly rhythm. You declare plans (desired actions), report progress (actual movement), flag problems (obstacles). The PPP structure is a micro tension-resolution cycle: plan=desired, progress=actual movement toward it, problem=what's blocking.
- **Affordance shaping**: The weekly cadence is the key affordance. It's not continuous monitoring; it's punctuated reflection. The "problems" field normalizes naming obstacles without shame.
- **Extractable idea**: **Punctuated reflection rhythm** — sc's practice mode already has ceremony. v1.0 could formalize a "weekly pulse" that asks for each active tension: "what moved? what's stuck? what changed about what you want?"

### 1.4 Notion Life OS Templates

- **One-line**: Personal operating systems built in Notion's block/database primitives — areas, projects, goals, habits.
- **Movement grammar**: Users build their own movement grammar from generic database primitives. Goals link to projects link to tasks. The movement is: define areas of life -> set goals per area -> break into projects -> execute tasks. Reality updates happen when you check boxes.
- **Affordance shaping**: The infinite flexibility is the affordance AND the trap. Users spend more time building the system than using it. The relational database model makes everything linkable, which creates the illusion of coherence. The real shaping: Notion teaches you to think in properties and relations, which is a particular (and limited) ontology.
- **Extractable idea**: **Resist premature structure** — sc should NOT let users build arbitrary metadata schemas. The tension primitive (desired/actual/parent/rank/status) is deliberately minimal. Adding fields should require proving the existing ones are insufficient.

### 1.5 Obsidian Periodic Reviews

- **One-line**: Markdown-based knowledge management with daily/weekly/monthly/quarterly review templates.
- **Movement grammar**: Time-based nesting: daily notes feed weekly reviews feed monthly reviews feed quarterly reviews. Each higher level asks "what patterns emerge from the lower level?" Movement is through progressive synthesis — you don't just track, you reflect on what tracking reveals.
- **Affordance shaping**: Plain text files in a folder. No server, no API, no lock-in. The affordance is ownership and longevity. The backlink graph creates emergent structure without imposed hierarchy. The periodic review pattern means the system only works if you show up.
- **Extractable idea**: **Progressive synthesis** — sc's pulse system infers lifecycle stages from mutation history. v1.0 could add explicit "review" events that capture user-authored synthesis, not just field updates. The system already knows what changed; the review captures what the changes *mean*.

### 1.6 Roam Research

- **One-line**: Networked thought tool built on block-level references and daily notes.
- **Movement grammar**: Start with daily notes (current reality captured as stream of thought). Develop structure through block references (ideas link to ideas). Movement is emergent — you don't plan the destination, you discover it through connection density. High-connection nodes become natural attractors.
- **Affordance shaping**: Block-level addressing means every thought is linkable. This shapes thinking toward atomicity — you learn to think in composable units. The bidirectional links mean creating a thought automatically creates a context for it. The daily note entry point normalizes starting from "where I am" rather than "where I should be."
- **Extractable idea**: **Atomic addressability** — every tension in sc already has a ULID. v1.0 could allow tensions to *reference* other tensions (not just parent/child but lateral "see also" or "blocks" or "feeds into"). The link vocabulary matters more than the link mechanism.

### 1.7 PARA Method (Tiago Forte)

- **One-line**: Organizational system: Projects (with deadlines), Areas (ongoing responsibilities), Resources (reference), Archives (done).
- **Movement grammar**: Everything starts as a Resource or Area input. When you commit to an outcome with a deadline, it becomes a Project. When finished, it moves to Archive. The movement is through commitment — declaring that something has a deadline and an outcome.
- **Affordance shaping**: The four-bucket structure forces a decision: is this active or reference? Does it have an end state or is it ongoing? This distinction (Project vs. Area) maps directly to sc's tension (has a desired state to resolve toward) vs. an ongoing concern (desired state is maintenance, not arrival).
- **Extractable idea**: **Tension vs. Concern distinction** — sc currently treats everything as a tension. Some "tensions" are really ongoing concerns with no resolution state (e.g., "stay healthy"). v1.0 could distinguish `tension` (gap to close) from `concern` (standard to maintain). Different lifecycle, different pulse logic.

### 1.8 Getting Things Done (David Allen)

- **One-line**: Stress-free productivity through complete capture, clarification, and context-based organization.
- **Movement grammar**: Capture everything -> Clarify (is it actionable? what's the next action? what's the desired outcome?) -> Organize by context -> Review weekly -> Engage. The transformation mechanism is the **clarification step**: forcing "stuff" through the "what's the next action?" filter converts anxiety into agency.
- **Affordance shaping**: The weekly review is the engine. Without it, the system degrades. The "next action" focus shapes users toward concreteness — you can't have a next action that's vague. The context tagging (@home, @computer, @calls) shapes awareness of when/where you can act. Projects are defined as "any outcome requiring more than one action" — a very low bar that catches things that would otherwise be implicit.
- **Extractable idea**: **Next action forcing function** — sc tensions have desired and actual but no "next action." v1.0 could optionally capture "what's the smallest next move?" — not as a task manager, but as a concreteness check. If you can't name a next action, the tension might be too abstract to generate movement.

---

## 2. Visualization/Sensemaking Tools

### 2.1 Miro

- **One-line**: Infinite canvas for collaborative visual thinking — sticky notes, diagrams, frameworks.
- **Movement grammar**: Start with divergent capture (stickies everywhere). Cluster into themes. Arrange into structures (timelines, matrices, flows). The canvas IS the sensemaking — spatial arrangement creates meaning. Movement is from chaos to pattern through physical-spatial organization.
- **Affordance shaping**: The infinite canvas removes premature structure. You can always zoom out. Stickies are deliberately ephemeral-feeling, which lowers commitment anxiety. The spatial metaphor means proximity = relationship, which is a powerful but implicit grammar.
- **Extractable idea**: **Spatial tension mapping** — sc's tree view is hierarchical. A complementary view could show tensions as nodes in 2D space where proximity indicates relationship strength, and the user can manually cluster. This would surface patterns the hierarchy obscures.

### 2.2 Figma FigJam

- **One-line**: Collaborative whiteboard built into Figma's design ecosystem.
- **Movement grammar**: Similar to Miro but tighter integration with design artifacts. The movement grammar is: brainstorm -> structure -> hand off to design. FigJam boards become the "why" documentation for design decisions.
- **Affordance shaping**: Stamps and emoji reactions make agreement/disagreement visible without verbal debate. The timer and voting widgets impose process structure on freeform canvas. The connection to Figma proper means ideas have a clear path to materialization.
- **Extractable idea**: **Lightweight agreement signals** — in multi-user sc (if it ever goes there), the ability to "stamp" a tension as "I see this too" or "I disagree with this framing" without editing it. Even for single-user: marking a tension as "I believe this" vs. "I'm not sure about this framing."

### 2.3 Kumu (Systems Mapping)

- **One-line**: Web-based tool for mapping complex systems — stakeholders, causal relationships, feedback loops.
- **Movement grammar**: Name elements -> define connections -> assign attributes -> discover feedback loops. The transformation is: you start thinking the system is a list of parts, and the tool reveals it's a web of relationships. Emergent properties become visible through the map, not through the elements.
- **Affordance shaping**: The decoration system (size, color, clustering by attribute) means the same map tells different stories depending on which lens you apply. This teaches that any system has multiple valid framings. The loop detection feature surfaces structure you didn't intentionally create.
- **Extractable idea**: **Feedback loop detection** — sc already detects sibling conflicts (one moving, one stuck). v1.0 could detect more patterns: reinforcing loops (tension A's progress enables tension B, which enables A), balancing loops (closing one gap opens another), and oscillation (actual keeps bouncing between two states).

### 2.4 Loopy (Causal Loop Diagrams)

- **One-line**: Nicky Case's minimal tool for drawing and simulating causal loop diagrams.
- **Movement grammar**: Draw nodes (variables) -> draw arrows (causal links, + or -) -> simulate to watch behavior emerge. The transformation: you think you understand the system, then simulation shows you the second and third-order effects you missed. The gap between your mental model and the simulation IS the learning.
- **Affordance shaping**: Extreme simplicity (just circles, arrows, +/-) means you can't hide behind complexity. If you can't express the relationship as "more A leads to more/less B," you haven't understood it yet. The simulation is immediate — draw and watch, no compilation step.
- **Extractable idea**: **Tension interaction simulation** — sc could allow users to declare that "progress on tension A increases/decreases the gap on tension B." Then a simple simulation could show: "if you focus on A, here's what happens to B, C, D over time." This makes structural tension structural — not just individual gaps but a system of gaps.

### 2.5 Observable Notebooks

- **One-line**: Reactive JavaScript notebooks for data visualization and exploratory analysis.
- **Movement grammar**: Write a cell -> see output immediately -> chain cells together reactively. The movement is from data to insight through progressive, visible transformation. Each cell is a step in an argument.
- **Affordance shaping**: Reactivity means changing an input upstream instantly propagates. This teaches dependency thinking — you learn to structure analysis as a DAG. The public-by-default sharing model means your analysis is also your communication.
- **Extractable idea**: **Reactive tension views** — sc's TUI could have computed views that automatically update when underlying tensions change. "Show me all tensions where actual hasn't changed in 14 days" is a reactive query, not a manual filter.

### 2.6 TheBrain

- **One-line**: Dynamic mind mapping where any node can be parent, child, or sibling of any other — no fixed hierarchy.
- **Movement grammar**: Create a thought -> link it to existing thoughts in any direction -> navigate by activating a thought (which recenters the view). The movement is: you don't traverse a fixed structure, you constantly recenter on what matters now.
- **Affordance shaping**: The "activate and recenter" navigation means context is always relative to your current focus. There's no God's-eye view; there's only the view from where you are. This mirrors actual cognition: you think from a center, not from above.
- **Extractable idea**: **Focus-relative view** — sc's tree view always shows the whole tree. A complementary mode could "focus" on one tension, showing only its parents, children, and linked tensions. The focused view IS the working context.

### 2.7 Scapple

- **One-line**: Literature & Latte's freeform note-arranging tool — no enforced hierarchy, just notes and connections on a canvas.
- **Movement grammar**: Type notes anywhere -> drag to arrange -> optionally connect. The lack of hierarchy IS the point: you arrange spatially until structure emerges, then export to Scrivener (which has hierarchy).
- **Affordance shaping**: No templates, no structure suggestions. The empty canvas is both freedom and anxiety. It trusts you to find your own arrangement. The deliberate lack of features forces you to do the cognitive work rather than delegating it to the tool.
- **Extractable idea**: **Pre-structural capture** — sc currently requires tensions to be well-formed (desired + actual). A "scratch" mode could capture half-formed tensions — maybe you know the desired state but can't articulate current reality, or vice versa. The tool would hold the incomplete thing until it crystallizes.

---

## 3. CAD/Design Software

### 3.1 Fusion 360

- **One-line**: Parametric CAD with timeline-based modeling history — every operation is recorded and replayable.
- **Movement grammar**: Sketch (2D desired shape) -> Extrude/Revolve (3D form) -> Constrain (dimensions, relationships) -> Iterate (edit any prior step, history replays forward). The movement is from idea to physical specification through progressive constraint. Each operation narrows the space of possible forms.
- **Affordance shaping**: The parametric timeline means the past is always editable. This shapes a particular relationship to decisions: nothing is permanent, everything is a parameter. But it also means you must think about the order of operations — the sequence of constraints matters. The "fully constrained sketch" indicator teaches you to notice when something is under- or over-determined.
- **Extractable idea**: **Constraint awareness** — sc could show when a tension is "under-constrained" (desired is vague, actual is vague, no children decomposing it) or "over-constrained" (too many sub-tensions competing for attention, conflicting desired states). The constraint status is metadata about the tension's readiness for action.

### 3.2 Blender (Sculpture Mode)

- **One-line**: 3D sculpting with brushes that push, pull, smooth, and carve digital clay.
- **Movement grammar**: Start with a blob -> sculpt toward a vision using additive and subtractive tools. The movement is embodied and iterative — you don't specify, you shape. The gap between what you see and what you want drives each brush stroke.
- **Affordance shaping**: The physical metaphor (clay, brushes, pressure) creates an embodied relationship to digital creation. Multires sculpting (work at different detail levels) teaches zoom-level awareness: rough out the big forms before detailing. Symmetry tools mean you specify half and get the whole.
- **Extractable idea**: **Zoom-level awareness** — sc's tree already has depth (parent/child). v1.0 could make zoom level explicit in the practice ceremony: "Are you working at the right level of detail? Should you zoom out to a parent tension or zoom into a child?"

### 3.3 OpenSCAD (Declarative CAD)

- **One-line**: Programmer's CAD — you write code that describes geometry, compile to see the shape.
- **Movement grammar**: Write a description of what you want -> compile -> see what you got -> adjust the description. The movement is linguistic: you must be able to SAY what you want in the tool's language. If you can't express it, you can't build it.
- **Affordance shaping**: The code-as-specification paradigm means you can diff, version, and parameterize designs. But it also means you can't "feel your way" to a shape — you must articulate before you see. This is both a discipline (forces clarity) and a limitation (prohibits intuitive exploration).
- **Extractable idea**: **Tension as code** — sc already stores tensions as YAML. v1.0 could lean into this: tensions as diffable, versionable, composable text. The `sc export` could produce a format that other tools can consume. The constraint is that everything must be articulable in text — and that constraint is a feature.

### 3.4 Rhino/Grasshopper (Parametric Design)

- **One-line**: Visual programming environment for generative/parametric architecture and design.
- **Movement grammar**: Define relationships between parameters -> adjust inputs -> watch outputs change across the entire design. The movement is: you don't design a thing, you design a *system that generates things*. Then you explore the possibility space by tweaking parameters.
- **Affordance shaping**: The node-graph visual programming means you literally see the flow of causation. Changing one input ripples visually through the graph. This teaches systems thinking: every design decision is connected to every other. The "bake" operation (freezing a parametric design into a static one) is the commitment moment.
- **Extractable idea**: **Tension parameters** — some tensions have a continuous dimension (not binary open/closed but a sliding scale). "How fit am I?" has a current value and a target value. sc could support numeric tensions where the gap is literally measurable, and progress is a time series.

### 3.5 TinkerCAD

- **One-line**: Autodesk's beginner-friendly 3D modeling with drag-and-drop primitive shapes.
- **Movement grammar**: Combine simple shapes (add/subtract) to make complex forms. The movement is compositional: the vocabulary is simple (box, cylinder, sphere), but composition creates complexity. You build up from atoms.
- **Affordance shaping**: The constraint to primitives and boolean operations (union, difference) means you think in terms of addition and subtraction. "What do I add? What do I remove?" This is a surprisingly powerful framing for any creative process.
- **Extractable idea**: **Additive vs. subtractive tension operations** — when decomposing a tension into children, the question could be: "Is closing this gap about adding something that's missing, or removing something that's in the way?" This shapes the action vocabulary.

---

## 4. Magical/Ceremonial Systems

### 4.1 Golden Dawn Ritual Structure

- **One-line**: Systematic ceremonial magic with grade initiations mapping to the Tree of Life sephiroth.
- **Movement grammar**: The grade system (Neophyte -> Zelator -> Theoricus -> etc.) maps to Kabbalistic sephiroth. Each grade has prescribed study, practice, and initiation ritual. Movement is through structured transformation — you don't just learn, you are *initiated* (identity transformation through ceremony). The ritual creates a container for psychological change that ordinary learning doesn't.
- **Affordance shaping**: The elaborate ceremonial apparatus (robes, implements, temple layout) creates separation from ordinary consciousness. This separation IS the affordance: by making the practice feel different from daily life, it signals "this is a context where transformation is possible." The grade system provides a map of development stages that is both motivating (you can see what's ahead) and potentially constraining (you might skip what the map doesn't show).
- **Extractable idea**: **Container ceremonies** — sc's practice mode is already a ceremony. v1.0 could make the ceremonial aspect more explicit: entering practice mode is crossing a threshold. The TUI could have a different visual mode, a different interaction grammar. The ceremony says: "in this context, we look at reality honestly."

### 4.2 I Ching (Hexagram Consultation)

- **One-line**: Chinese divination system: generate a hexagram (6 lines, yin/yang), read its commentary as guidance for a situation.
- **Movement grammar**: Formulate a question (articulate the tension) -> generate randomness (yarrow stalks or coins) -> receive a hexagram -> interpret its image and commentary in context. The transformation is not in the answer but in the *question*: the act of formulating what you're really asking clarifies the situation more than the response does. Changing lines show movement: "this is where you are; this is where it's tending."
- **Affordance shaping**: The 64 hexagrams provide a vocabulary of situations — archetypes of structural positions. Having names for situations ("Difficulty at the Beginning," "Breakthrough," "Retreat") gives you cognitive handles for recognizing where you are. The randomness element prevents you from choosing the answer you want to hear.
- **Extractable idea**: **Situation archetypes** — sc's lifecycle stages (germination, assimilation, completion) are already situation archetypes. v1.0 could expand the vocabulary: "stagnation" (no movement, no energy), "conflict" (sibling tensions pulling against each other), "breakthrough" (sudden movement after long stagnation), "retreat" (deliberately stepping back from a tension). Named states create recognition.

### 4.3 Tarot (Sensemaking Practice)

- **One-line**: 78-card symbolic system used for divination, self-reflection, and narrative construction.
- **Movement grammar**: Shuffle (let go of control) -> draw cards into a spread (positional meaning: past/present/future, or situation/obstacle/advice) -> interpret the symbols in context of the question. The transformation is narrative construction: random symbols + a question + positional structure = a story about your situation that you didn't author consciously. The spread structure IS a grammar for decomposing a situation.
- **Affordance shaping**: The rich visual symbolism activates associative thinking rather than analytical thinking. The spread positions force you to consider aspects you'd otherwise skip (what's your unconscious motivation? what's the external influence? what's the likely outcome if nothing changes?). The practice of reading for others teaches empathetic perspective-taking.
- **Extractable idea**: **Positional decomposition** — instead of free-form child tensions, sc could offer "spread templates" for decomposing a tension: "What's the desired state? What's actually true right now? What's the obstacle? What resource do you have that you're not using? What's the next move?" The positions ask questions the user wouldn't ask themselves.

### 4.4 Enochian System

- **One-line**: Elizabethan-era angelic magic system with its own language, hierarchical cosmology, and systematic ritual framework.
- **Movement grammar**: The system maps reality into a hierarchy of increasingly abstract/powerful levels (elemental -> planetary -> zodiacal -> angelic). Movement through the system is movement up the hierarchy of abstraction. You work with concrete elements before abstract ones. Each level has its own "language" (calls, tables, sigils).
- **Affordance shaping**: The extreme systematicity (tables, grids, linguistic rules) means the practice is as much about mastering a formal system as about spiritual development. The tabular structure (the Great Table) organizes all possible workings into a grid — you can always locate yourself in the system.
- **Extractable idea**: **Locatability** — sc should always be able to answer "where am I in this?" Users should be able to locate any tension in the tree, any moment in the lifecycle, any point in the practice rhythm. Being lost is the enemy of creative process.

### 4.5 Chaos Magick Sigil Practice

- **One-line**: Write a desire -> remove duplicate letters -> abstract into a symbol -> charge it (ecstatic state) -> forget it -> let the unconscious work.
- **Movement grammar**: Articulate desire (make the tension explicit) -> abstract it (remove the conscious attachment to outcome) -> charge it (invest energy) -> release it (let go of tracking). The transformation mechanism is paradoxical: you achieve the goal by forgetting you want it. The abstraction step (desire -> letters -> symbol) is a progressive encoding that strips away narrative baggage.
- **Affordance shaping**: The practice forces you to distill a desire to its essence. You can't sigil-ize "I kinda want things to be better" — you have to say specifically what you want. The forgetting step is an affordance against obsessive monitoring. The "fire and forget" model is the opposite of OKR-style continuous tracking.
- **Extractable idea**: **Desire distillation** — sc's `desired` field currently accepts free text. The tool could have a mode where it helps you compress your desired state to its essence: "What's the shortest possible statement of what you want here?" Shorter = clearer = more generative tension.

### 4.6 Astrological Chart Reading

- **One-line**: Map of planetary positions at a specific moment, interpreted as a pattern of potentials, tensions, and tendencies.
- **Movement grammar**: A natal chart is a snapshot of structural tensions (squares, oppositions) and structural supports (trines, sextiles) at a moment in time. Transits (current planetary positions relative to the natal chart) indicate which natal tensions are being activated NOW. Movement is not linear progress but cyclic activation — your fundamental tensions don't resolve, they get activated and worked in different contexts at different times.
- **Affordance shaping**: The aspects (angular relationships between planets) create a vocabulary of tension types: conjunction (fusion), opposition (polarization), square (friction/growth), trine (flow), sextile (opportunity). Having multiple named tension-types means you can recognize WHAT KIND of tension you're in, not just that you're in one.
- **Extractable idea**: **Tension types** — sc currently has one kind of tension (gap between desired and actual). But tensions feel different: some are friction (I want X but Y blocks me), some are polarization (I want both X and not-X), some are potential (X is possible but not yet begun). Naming tension types would help users understand what kind of action each tension calls for.

### 4.7 Alchemical Opus Stages (Nigredo/Albedo/Citrinitas/Rubedo)

- **One-line**: Four-stage transformation model: blackening (dissolution), whitening (purification), yellowing (awakening), reddening (integration).
- **Movement grammar**: Nigredo = the current state must be destroyed/dissolved before transformation. Albedo = separation of elements, clarity about what's what. Citrinitas = new connections, dawning understanding. Rubedo = integration into a new whole. The key insight: transformation requires destruction of the old form FIRST. You can't get to the new state by adding to the old one.
- **Affordance shaping**: The four-stage model normalizes the "it gets worse before it gets better" experience. Nigredo is uncomfortable but necessary. Without this framing, people abandon transformations during the dissolution phase because it feels like failure.
- **Extractable idea**: **Dissolution as progress** — sc's lifecycle stages (germination -> assimilation -> completion) are linear and progressive. But real creative process often involves a regression phase where the actual gets WORSE before it gets better (you tear apart the old approach before building the new one). sc should recognize and normalize this: "actual moved further from desired" isn't always failure — it might be nigredo.

---

## 5. Kitchen/Chef Games and Systems

### 5.1 Overcooked (Video Game)

- **One-line**: Cooperative cooking game where coordination under time pressure IS the game.
- **Movement grammar**: Orders arrive (desired state = completed dishes) -> current reality (raw ingredients, stations in various states) -> players must coordinate who does what. The gap closes through division of labor and communication. The chaos increases with each level, forcing better coordination or failure.
- **Affordance shaping**: The split kitchen layouts (counters between players, moving platforms, fire) create artificial constraints that make coordination harder and more important. The time pressure prevents over-planning — you must act with incomplete information and adjust. The cooperative structure means your movement on one tension creates or resolves tensions for others.
- **Extractable idea**: **Constraint-driven coordination** — when multiple tensions share resources (time, attention, energy), the constraint is as important as the goal. sc could model resource contention: "these three tensions all require deep focus time, and you only have X hours."

### 5.2 Cooking Mama (Video Game)

- **One-line**: Mini-game collection that decomposes cooking into discrete skill challenges (chop, stir, fold, etc.).
- **Movement grammar**: A recipe is decomposed into atomic operations -> each operation is a self-contained skill challenge -> success at each step contributes to the final dish. Movement is through sequential skill execution. Failure at one step degrades the final result but doesn't prevent completion.
- **Affordance shaping**: The decomposition into mini-games teaches that complex outcomes are sequences of simple operations. The scoring (gold/silver/bronze per step) provides granular feedback on which sub-skills need work. "Mama" provides emotional scaffolding — encouragement even on failure.
- **Extractable idea**: **Graceful degradation** — sc tensions are currently all-or-nothing (open/resolved). v1.0 could model partial resolution: "You didn't reach the full desired state, but you got 70% there, and that's a meaningful state to acknowledge and name."

### 5.3 Mise en Place (Methodology)

- **One-line**: "Everything in its place" — the chef's practice of preparing and organizing all ingredients before cooking begins.
- **Movement grammar**: Before any cooking (action toward desired), you do ALL preparation (understand and arrange current reality). The transformation is: you convert chaotic raw materials into organized, ready-to-use components. Then execution becomes smooth because every decision was already made.
- **Affordance shaping**: The practice shapes attention toward preparation as a first-class activity. It reframes "not yet cooking" from "wasting time" to "essential phase." The physical arrangement (each ingredient in its own container, in order of use) is an externalized plan.
- **Extractable idea**: **Preparation as explicit phase** — sc's germination stage is already this. v1.0 could make it more explicit: "Before you can work on this tension, what needs to be in place? What's your mise en place?" This could be captured as "precondition" sub-tensions.

### 5.4 Brigade de Cuisine (Organizational Structure)

- **One-line**: Escoffier's kitchen hierarchy — chef de cuisine, sous chef, chef de partie, commis — each with defined responsibilities.
- **Movement grammar**: The brigade takes an order (desired outcome) and decomposes it by station: saucier makes sauce, poissonnier prepares fish, patissier makes dessert. Movement is through parallel specialized execution with hierarchical coordination. The sous chef manages the flow; the chef de cuisine manages the vision.
- **Affordance shaping**: The rigid role structure means no one has to decide what to do — they know their station. This eliminates coordination overhead at the cost of flexibility. The "pass" (where dishes are assembled and inspected before serving) is a quality gate — a checkpoint where reality is compared to the standard.
- **Extractable idea**: **Quality gates** — sc could have explicit checkpoints in a tension's lifecycle where you compare actual to desired and make a decision: "close enough to resolve," "needs more work," or "desired has changed." Currently this happens implicitly; making it a ceremony adds rigor.

### 5.5 Recipe Development Iteration

- **One-line**: Professional recipe development: cook, taste, adjust, record, repeat until the recipe is reliable and delicious.
- **Movement grammar**: Envision a dish (desired) -> cook a version (actual) -> taste the gap -> adjust one variable -> cook again. The key discipline: change ONE thing at a time so you know what caused the difference. Movement is through controlled experimentation with clear feedback (taste).
- **Affordance shaping**: The feedback loop is immediate and sensory. You can't lie to your tongue. The "change one variable" discipline teaches experimental rigor. The written recipe (capture what worked) is the artifact of resolved tension.
- **Extractable idea**: **One-variable-at-a-time discipline** — when a tension isn't closing, sc could prompt: "What one thing will you change? Just one. Change it and observe what happens before changing anything else."

---

## 6. Musical Instruments and Practice

### 6.1 Deliberate Practice Methodology (Ericsson)

- **One-line**: Structured practice at the edge of current ability with immediate feedback and specific goals.
- **Movement grammar**: Identify a specific skill gap (tension) -> design a practice exercise that targets exactly that gap -> execute with full attention -> get feedback -> adjust. The key: the practice must be at the boundary of ability — too easy and you're not growing, too hard and you're not learning.
- **Affordance shaping**: The methodology reframes practice from "playing through songs" (enjoyable but slow growth) to targeted exercises (uncomfortable but fast growth). The requirement for immediate feedback means you must have a way to know if you're improving. The specificity requirement forces you to decompose "get better at guitar" into "improve left-hand stretch between frets 5-9 at 120 BPM."
- **Extractable idea**: **Edge-of-ability targeting** — sc could help users identify which tensions are at their "practice edge" — not so easy they'll resolve without effort, not so hard they're paralyzed. The pulse system's lifecycle stages are a proxy for this, but a more explicit "difficulty/readiness" signal could help prioritize.

### 6.2 Ableton Live (Session vs. Arrangement View)

- **One-line**: Music production software with two views: Session (nonlinear clips, improvisation) and Arrangement (linear timeline, composition).
- **Movement grammar**: Session view: launch clips in any combination, explore possibilities, find what works together. Arrangement view: commit a sequence, refine details, produce a finished piece. The two views represent different phases of creative process: divergent exploration (Session) and convergent commitment (Arrangement). You record from Session into Arrangement when you're ready to commit.
- **Affordance shaping**: Session view's grid of clips teaches combinatorial thinking — music as modular blocks. Arrangement view's timeline teaches sequential thinking — music as narrative. Having BOTH in one tool means the transition from exploration to commitment is low-friction. The "record session to arrangement" action IS the commitment moment.
- **Extractable idea**: **Dual-mode interface** — sc could have an "exploration" mode (brainstorm tensions, rearrange freely, no commitment) and a "commitment" mode (rank, prioritize, status-track). The transition between modes is itself meaningful: "I'm done exploring, I'm committing to these tensions."

### 6.3 Modular Synthesis (Patch-as-Architecture)

- **One-line**: Electronic music made by connecting modules (oscillators, filters, envelopes) with patch cables — the architecture IS the instrument.
- **Movement grammar**: Imagine a sound (desired) -> select modules that could produce aspects of it -> patch them together (build the system) -> tweak parameters -> listen to the gap between what you hear and what you want -> re-patch or re-tweak. The key insight: you don't play notes, you build an instrument and then interact with it. The instrument-building IS the creative act.
- **Affordance shaping**: The patch cable is physical and visible — you can SEE the signal flow. This makes system architecture tangible. The modular format means you can always add, remove, or rearrange components. But complexity accumulates: a 50-cable patch is hard to understand even for its creator. The "happy accident" (unplanned sounds) is a feature, not a bug — the system can surprise you.
- **Extractable idea**: **System-building as the work** — sc is itself a system people build (their tension tree). The act of structuring your tensions IS the sensemaking, not just a precursor to action. v1.0 should treat tree-building and tree-editing as first-class creative acts, not just administrative setup.

### 6.4 Guitar Practice Routines

- **One-line**: Structured daily practice covering technique, repertoire, theory, and ear training.
- **Movement grammar**: Warm up (prepare the instrument and body) -> technique exercises (targeted gap-closing) -> repertoire (integrate skills into music) -> cool down/review. The routine creates a container: you don't decide what to practice each day, the routine decides. Movement is through consistency: the daily routine accumulates into transformation over weeks and months.
- **Affordance shaping**: The metronome is the key affordance — it provides objective measurement of tempo capability. "Yesterday I could play this at 100 BPM, today at 104 BPM" is concrete progress. The routine structure reduces decision fatigue. The separation of technique from repertoire teaches that skill-building and skill-application are different activities.
- **Extractable idea**: **Routine as container** — sc's practice mode is a routine. v1.0 could make the routine more structured: always start with pulse review (what's moving? what's stuck?), then focus on one tension, then step back to tree view. The routine removes the "what should I do?" question.

### 6.5 Sight-Reading Development

- **One-line**: Learning to play music at first sight — progressive exposure to increasingly complex notation.
- **Movement grammar**: Start with simple patterns (whole notes, C major) -> gradually increase complexity (rhythmic variety, key signatures, accidentals). The key: you must read slightly ahead of where you're playing. The gap between "where my eyes are" and "where my hands are" IS the skill. Movement is through tolerating imperfection — you keep going even when you make mistakes, because stopping to correct teaches the wrong skill.
- **Affordance shaping**: Graded difficulty (method books with progressive exercises) means the challenge is always calibrated. The "don't stop" rule shapes a particular relationship to mistakes: errors are data, not failure. The difference between reading and memorizing teaches the difference between pattern recognition and recall.
- **Extractable idea**: **Keep moving through imperfection** — sc's resolution model is currently binary (tension is open or resolved). A "keep moving" philosophy would suggest: update the actual field regularly even if the update is "still stuck" or "made it worse." The practice of updating IS the practice of engagement, regardless of progress direction.

---

## 7. Agent Harnesses and AI Orchestration

### 7.1 Claude Code (Hooks, MCP, Session Context)

- **One-line**: Anthropic's CLI for Claude with hook system (pre/post actions), MCP tool integration, and persistent context.
- **Movement grammar**: User states intent (desired) -> agent reads codebase (understands actual) -> agent proposes and executes changes -> user reviews and accepts/rejects -> iterate. Hooks allow pre/post session behaviors (like anima's prediction-observation loop). MCP extends the agent's action space. The movement is collaborative: human holds the vision, agent executes, human evaluates.
- **Affordance shaping**: The conversational interface means intent must be articulable in natural language. The hook system allows meta-level behavior (behavior about behavior). The context window creates a recency bias — what was discussed recently is more salient. The tool permission system creates explicit boundaries on agent autonomy.
- **Extractable idea**: **Hook-driven meta-loops** — sc v1.0 could have hooks that fire on tension events: on-create (prompt for desired/actual clarity), on-update-actual (check: is the gap closing?), on-stagnation (prompt: is this still a real tension?), on-resolve (prompt: capture what you learned). The hooks are where the system's intelligence lives.

### 7.2 Cursor

- **One-line**: VS Code fork with AI-native code editing — inline generation, chat, and codebase-aware completions.
- **Movement grammar**: Write code (or describe what you want) -> AI suggests completions/edits -> accept, reject, or modify -> the codebase evolves as a collaboration between human intent and AI capability. Movement is through continuous micro-negotiations: each suggestion is a proposal that the human evaluates.
- **Affordance shaping**: The inline suggestion (appearing in the editor, not in a chat window) means AI proposals are in the context of the work, not separate from it. Tab-to-accept creates a low-friction acceptance path — it's easier to accept than to reject. This subtly shifts the power dynamic toward the AI's framing.
- **Extractable idea**: **In-context suggestions** — sc could generate suggestions inline with the tension view: "This tension has been in germination for 30 days — consider decomposing into smaller tensions" appearing right next to the tension, not in a separate notification.

### 7.3 Windsurf (Codeium)

- **One-line**: IDE with "Cascade" — an agentic flow that maintains context across multiple file edits.
- **Movement grammar**: Describe a change at high level -> agent cascades through affected files -> human reviews the cascade. The movement grammar is: one intent, many consequences. The agent handles the propagation; the human handles the intent and quality check.
- **Affordance shaping**: The "cascade" metaphor teaches that changes propagate — editing one thing affects many things. The multi-file diff view shows the full impact of a change. This trains awareness of systemic effects.
- **Extractable idea**: **Cascade awareness** — when a tension's actual is updated in sc, what other tensions are affected? If "launch product" has child tensions for "build feature X" and "write docs" and "set up CI," updating one child's actual changes the parent's effective actual. sc could show these cascades.

### 7.4 Devin (Cognition)

- **One-line**: Autonomous software engineer agent with its own browser, terminal, and editor — works independently on tasks.
- **Movement grammar**: Receive task specification (desired) -> plan -> execute autonomously (browse docs, write code, run tests, debug) -> deliver result. The human is removed from the loop during execution. Movement is through autonomous problem-solving with human evaluation at checkpoints.
- **Affordance shaping**: Full autonomy means the human must specify desired state clearly enough for unsupervised execution. This forces extremely precise intent articulation. The sandbox environment (Devin's own machine) creates safe autonomy — it can't break your production systems.
- **Extractable idea**: **Precision of intent** — sc could measure how precisely a tension's desired state is specified. "Make the code better" is low-precision. "All API endpoints return proper error codes with HTTP status >= 400" is high-precision. Higher precision = more actionable tension.

### 7.5 AutoGPT / BabyAGI

- **One-line**: Early autonomous agent frameworks: decompose a goal into tasks, execute them, evaluate results, repeat.
- **Movement grammar**: High-level goal (desired) -> LLM decomposes into sub-tasks -> agent executes tasks sequentially -> evaluates result -> generates more tasks if needed. The movement is through recursive decomposition and execution. The key failure mode: task generation outruns task execution (infinite to-do list growth).
- **Affordance shaping**: The recursive decomposition reveals a fundamental problem: decomposition without constraint is infinite. Every task can be decomposed further. Without a stopping criterion, the system spins. The "task prioritization" step (which of the generated tasks actually matters?) is where the real intelligence is needed.
- **Extractable idea**: **Decomposition limits** — sc should have an opinion about maximum tree depth. Past 3-4 levels of nesting, you're probably over-decomposing. The tool should resist infinite decomposition and push instead toward action.

### 7.6 CrewAI

- **One-line**: Framework for orchestrating multiple AI agents with defined roles, goals, and backstories working on collaborative tasks.
- **Movement grammar**: Define roles (agent identities with goals) -> define tasks (units of work) -> define process (sequential or hierarchical) -> agents negotiate and hand off work. Movement is through role-based collaboration: each agent advances its own tensions, and the orchestrator manages conflicts.
- **Affordance shaping**: The "role + backstory" pattern means agents have identity, not just instructions. This shapes their behavior holistically rather than per-task. The "delegation" mechanism means an agent can decide "this is not my tension" and hand it off.
- **Extractable idea**: **Tension ownership** — sc currently has one user. If it ever supports multiple users (or AI agents), tensions need owners. Even for single-user: "whose tension is this really?" Sometimes the tensions in your tree belong to someone else and you're carrying them.

### 7.7 LangGraph

- **One-line**: Framework for building stateful, multi-step agent workflows as directed graphs with conditional edges.
- **Movement grammar**: Define states (nodes) -> define transitions (edges) -> define conditions (when to take which edge) -> execute the graph. Movement is state-machine-like: you're always in a defined state, and transitions are explicit. The graph is the plan; execution follows the graph.
- **Affordance shaping**: The graph visualization makes the workflow's logic visible and debuggable. Conditional edges (if X then go to state A, else state B) teach decision-point awareness. The checkpointing system (save state, resume later) means long-running work is persistent.
- **Extractable idea**: **Explicit state transitions** — sc's tension status (seed/active/resolved/released) already defines a state machine. v1.0 could make transitions more intentional: you must explicitly move a tension from seed to active (a commitment), and the system records when and why each transition happened.

---

## 8. Contemplative/Reflective Practices

### 8.1 Ignatian Examen

- **One-line**: Daily prayer practice: review the day for moments of consolation (life-giving) and desolation (life-draining).
- **Movement grammar**: Stillness -> Gratitude -> Review of the day (what happened) -> Identify consolation and desolation -> Look ahead to tomorrow. The movement is through affective review: not "what did I do?" but "what gave life and what drained life?" This surfaces values you hold but haven't articulated.
- **Affordance shaping**: The consolation/desolation framework gives you exactly two lenses. This binary simplifies the infinite complexity of a day into a pattern you can read. The daily cadence means patterns emerge over weeks. The gratitude step prevents the review from becoming purely analytical.
- **Extractable idea**: **Affective signal tracking** — when reviewing tensions in sc's practice mode, the question isn't just "did the actual move?" but "how do I feel about this tension?" Energy/vitality toward a tension is signal. Persistent dread is signal. sc could track not just state changes but the user's felt relationship to each tension.

### 8.2 Zen Koans

- **One-line**: Paradoxical questions or stories (e.g., "What is the sound of one hand clapping?") used to provoke insight beyond conceptual thinking.
- **Movement grammar**: Receive the koan -> sit with it (don't try to solve it analytically) -> present your understanding to the teacher -> get rejected or accepted. The movement is NOT linear progress toward an answer. The koan dissolves the question-answer framework itself. You don't solve a koan; the koan solves you.
- **Affordance shaping**: The koan is deliberately unsolvable by conventional means. This trains a different kind of knowing — direct perception rather than analysis. The teacher relationship means you can't self-certify your understanding. The long sitting practice (months or years with one koan) teaches patience with non-resolution.
- **Extractable idea**: **Tensions that don't resolve** — some tensions in sc should be recognized as koans: tensions where the desired state and actual state create a productive paradox that you don't resolve but learn from. "I want complete freedom AND deep commitment" is not a problem to solve but a polarity to hold. sc could have a "paradox" status alongside "active" and "resolved."

### 8.3 Vipassana Noting Practice

- **One-line**: Meditation technique: note each arising experience with a simple label ("thinking," "hearing," "pain," "pleasant").
- **Movement grammar**: Sit -> experience arises -> note it ("thinking") -> return to bare attention -> next experience arises -> note it. There is no "desired state" in the conventional sense; the practice is about seeing clearly what IS. Movement is toward finer discrimination: at first you note "thinking," later you note "planning" vs. "remembering" vs. "fantasizing."
- **Affordance shaping**: The noting labels create categories of experience. The vocabulary shapes what you notice: if you have a label for "aversion," you start noticing aversion. The simplicity of the labels (one word) prevents the noting from becoming another form of thinking. The instruction to note without judgment teaches observation without reactivity.
- **Extractable idea**: **Noting vocabulary for tension states** — sc's lifecycle stages are a noting vocabulary. v1.0 could expand it: note not just the stage but the quality. "This tension feels stuck." "This tension has energy." "This tension feels irrelevant." The noting practice IS the data collection.

### 8.4 Morning Pages (Julia Cameron)

- **One-line**: Three pages of longhand stream-of-consciousness writing first thing each morning.
- **Movement grammar**: Wake up -> write three pages of whatever comes -> don't read them for 8 weeks. The movement is drainage: you drain the mental clutter so that what remains is clearer. The desired state is not in the pages but in the consciousness that's been cleared. The pages are exhaust, not product.
- **Affordance shaping**: Three pages is specific and non-negotiable — not "write until you feel done" but "write three pages." This eliminates the decision of when to stop. The longhand requirement slows you down enough that you can't outrun your thoughts. The "don't read them" rule prevents self-editing and performance.
- **Extractable idea**: **Drainage as practice** — sc's practice mode could include a "dump" phase: before reviewing tensions, free-write everything on your mind. The dump clears the noise so that the subsequent tension review engages with what matters, not what's loudest.

### 8.5 The Work (Byron Katie's 4 Questions)

- **One-line**: Inquiry method: take a stressful thought and ask: Is it true? Can you absolutely know it's true? How do you react when you believe it? Who would you be without it? Then turn it around.
- **Movement grammar**: Identify a stressful belief -> question its truth -> notice how it affects you -> imagine yourself without it -> find turnarounds (opposite statements that are equally true). The movement is from fusion with a belief to freedom from it. The "turnaround" is the key mechanism: it reveals that the opposite of your painful belief might be as true as the original.
- **Affordance shaping**: The four questions are a fixed protocol — you don't improvise, you follow the steps. This constraint prevents the mind from escaping into intellectualization. The turnaround forces perspective reversal, which is cognitively difficult and productive.
- **Extractable idea**: **Tension inquiry** — sc could offer a structured inquiry for stuck tensions: "Is this desired state actually what you want? Can you absolutely know? How does holding this tension affect you? Who would you be without it? What if the opposite of your desired state were the actual goal?" This dissolves false tensions.

### 8.6 Focusing (Eugene Gendlin)

- **One-line**: Somatic awareness practice: attend to the body's felt sense of a problem, let it form into a "handle" (word or image), and check the handle against the feeling.
- **Movement grammar**: Clear a space (like mise en place) -> choose an issue -> find the felt sense (body-level knowing) -> get a handle (word/image) -> resonate (check the handle against the felt sense) -> ask (what is it about this?) -> receive. The movement is from vague unease to articulate understanding through somatic attention rather than analysis.
- **Affordance shaping**: The practice insists that the BODY knows something the MIND doesn't yet. This redirects attention from conceptual thinking to somatic experience. The "felt shift" — a physical release when the handle fits — provides unmistakable feedback. The resonance step (checking word against feeling) teaches iterative approximation.
- **Extractable idea**: **Handle-finding for tensions** — sc's `desired` and `actual` fields are conceptual. But sometimes you know something is off before you can articulate it. v1.0 could support a "felt tension" — a tension where you can name the feeling but not yet the desired or actual state. The tool holds the space until articulation arrives.

### 8.7 IFS (Internal Family Systems)

- **One-line**: Therapeutic framework: the psyche contains "parts" (protectors, exiles, managers) and a core "Self" that can lead with curiosity and compassion.
- **Movement grammar**: Notice a part (a reaction, a pattern) -> get curious about it (what is it protecting?) -> find the exile underneath (the original wound/need) -> unburden the exile -> the protector relaxes because it's no longer needed. Movement is through relationship: you don't eliminate parts, you understand them and offer what they need.
- **Affordance shaping**: The "parts" framework means internal conflicts are dialogues, not pathologies. Having multiple parts means having multiple tensions simultaneously is NORMAL. The "Self" (curious, calm, compassionate) provides a vantage point from which to relate to parts — you are not your parts; you are the one relating to them.
- **Extractable idea**: **Tensions as parts** — sc tensions might represent different "parts" of the user's psyche or project, and they can be in conflict with each other. v1.0's conflict detection (siblings where one is moving and the other is stuck) is already an IFS-like insight. The tool could ask: "What is this stuck tension protecting? What would happen if it moved?"

---

## 9. Games with Emergent Strategy

### 9.1 Dwarf Fortress

- **One-line**: Colony simulation where emergent complexity arises from simple systems interacting — legendary for producing unplanned narratives.
- **Movement grammar**: Embark (start with limited resources and a desired fortress vision) -> dig, build, assign (manipulate reality) -> crises emerge (goblin sieges, vampire dwarves, flooding) -> adapt the vision to what's actually happening. The movement grammar is: plan meets reality, and reality always wins. Your actual desired state evolves as you discover what's possible and what's threatening.
- **Affordance shaping**: The ASCII/text interface forces imagination — you must mentally construct the world from symbols. The extreme depth of simulation (every dwarf has preferences, relationships, and trauma) means you can never fully understand the system you're managing. The lack of a win condition means YOU define what success looks like.
- **Extractable idea**: **Desired-state evolution** — sc currently lets you edit `desired`, but it should maybe track how desired changes over time. The history of desire-revisions tells you something important: "I thought I wanted X, then realized I wanted Y" is itself a pattern of growth.

### 9.2 Factorio

- **One-line**: Factory building game: automate the production of increasingly complex items to launch a rocket.
- **Movement grammar**: The end goal is known (launch a rocket). The gap is the entire production chain between raw resources and a rocket. Movement is through progressive automation: you manually do what will later be automated. Each automation frees attention for higher-level design. The factory is a physical representation of resolved tensions (each production line is a gap between "I need X" and "I have X" that's been automated away).
- **Affordance shaping**: The "main bus" design pattern (a central transport line feeding all factories) emerges from player communities, not from the game's instructions. The game teaches systems thinking by forcing you to deal with throughput, bottlenecks, and ratios. The satisfaction of watching a factory run autonomously is the reward for good structural design.
- **Extractable idea**: **Automation of resolved tensions** — when a tension in sc resolves, the pattern of how it was resolved could be captured and offered as a template for similar future tensions. "Last time you had a tension about launching a feature, you decomposed it into these 5 sub-tensions and they resolved in this order."

### 9.3 Civilization (Series)

- **One-line**: 4X strategy game (explore, expand, exploit, exterminate) spanning human history.
- **Movement grammar**: Choose a win condition (domination, science, culture, diplomacy, religion) -> manage the tension between short-term survival and long-term strategy -> every turn is a resource allocation decision among competing tensions. The movement grammar is: you're always underfunded, so you must choose which tensions to advance and which to let languish.
- **Affordance shaping**: The tech tree shows you the entire possibility space and what you haven't yet unlocked. This creates a tension between depth (investing in one branch) and breadth (covering multiple branches). The fog of war means your information about other players is always incomplete. The turn structure creates natural review points.
- **Extractable idea**: **Explicit tradeoff awareness** — sc's ranking system implicitly prioritizes tensions. v1.0 could make tradeoffs explicit: "Advancing tension A comes at the cost of tension B (because they share resources). Which do you choose?" Making the cost visible changes the decision quality.

### 9.4 Chess

- **One-line**: Two-player perfect-information strategy game on 64 squares.
- **Movement grammar**: From an opening position (known start) through increasingly complex middlegame positions to endgame (simplified, approaching resolution). Movement is through piece coordination toward checkmate. Every move changes the tension landscape: creating threats, removing options, opening/closing lines.
- **Affordance shaping**: The fixed rules and perfect information mean the only variable is skill. The opening theory provides named patterns for recurring structural positions. The concept of "prophylaxis" (preventing your opponent's desired moves) teaches that tension management isn't just pursuing your goals but preventing threats. The time control adds resource constraint.
- **Extractable idea**: **Named positions/patterns** — chess openings have names. sc could develop a vocabulary for recurring tension patterns: "the feature-debt squeeze" (shipping features while managing tech debt), "the priority inversion" (urgent displacing important). Named patterns are recognizable and teachable.

### 9.5 Go

- **One-line**: Two-player territory game with minimal rules producing extraordinary strategic depth.
- **Movement grammar**: Place stones to claim territory and connect groups. The tension is between expansion (claiming more) and defense (keeping what you have). Movement is through reading: projecting sequences of moves and counter-moves to evaluate which tensions to pursue. The concept of "sente" (initiative) is key: forcing your opponent to respond to you means you choose which tensions matter.
- **Affordance shaping**: The simple rules (place stone, capture surrounded stones) create emergent complexity — a bottom-up complexity generator. The concept of "thickness" (robust, flexible positions) teaches that strength often comes from resilience, not direct aggression. The "ko" rule (you can't immediately recapture) prevents infinite loops and forces creative alternatives.
- **Extractable idea**: **Thickness vs. thinness** — a tension with good decomposition, clear desired state, and active movement is "thick." A tension that depends on many uncertain factors and hasn't been well-articulated is "thin." sc could assess tension thickness as a health metric.

### 9.6 Poker

- **One-line**: Incomplete-information card game where optimal play requires reasoning about others' hidden states.
- **Movement grammar**: You have cards (your actual) and a pot goal (desired). But you don't know others' cards (hidden information). Movement is through betting (committing resources based on probabilistic assessment) and folding (accepting loss to preserve resources). The key: you're not playing your cards, you're playing your opponents' uncertainty about your cards.
- **Affordance shaping**: Incomplete information means you must reason probabilistically. Position (when you act relative to others) is a structural advantage. The "pot odds" calculation teaches cost/benefit analysis. Tilt (emotional dysregulation after bad outcomes) is the enemy — the game teaches emotional regulation as a strategic skill.
- **Extractable idea**: **Uncertainty acknowledgment** — sc tensions currently state desired and actual as facts. But sometimes you're uncertain: "I think the current state is X, but I might be wrong." v1.0 could support confidence levels on actual (and even desired). Low confidence = you need information, not action.

### 9.7 Kerbal Space Program

- **One-line**: Rocket building and space flight simulation with realistic orbital mechanics.
- **Movement grammar**: Design a rocket (plan) -> launch (execute) -> observe physics (reality) -> iterate (redesign based on what happened). The gap between "I think this rocket will reach orbit" and "it exploded at 10km" IS the learning. Movement is through iterative experimentation with fast, clear feedback (explosion = failure, orbit = success).
- **Affordance shaping**: The physics simulation is honest — it doesn't cheat for you. This teaches that reality has laws and your plans must respect them. The "staging" system (which parts activate in which order) teaches sequential planning. The "revert to launch" option means failure is cheap, encouraging experimentation.
- **Extractable idea**: **Cheap failure** — sc should make it safe to create, modify, and abandon tensions. The "released" status is already this — you can release a tension without resolving it (admitting "this isn't the right tension"). v1.0 should normalize releasing tensions as a positive act, not a failure.

---

## 10. Fitness/Body Practices

### 10.1 Starting Strength (Progressive Overload)

- **One-line**: Barbell training program: squat, deadlift, press, bench press — add weight every session.
- **Movement grammar**: Today's actual (weight lifted) -> add 5 lbs -> tomorrow's desired. The movement grammar is brutally simple: do the work, add weight, repeat. The gap is always exactly 5 lbs. When you can't add weight (stall), you deload and rebuild. Stalls are expected, not failures.
- **Affordance shaping**: The barbell is honest — either you lift the weight or you don't. There's no subjective evaluation. The program's simplicity (4 exercises, 3 sets of 5) removes all decision-making. The progressive overload principle teaches that growth is incremental and measurable. Deloads teach that regression is part of progression.
- **Extractable idea**: **Incremental gap-closing with expected stalls** — sc could model expected stalls: "This tension has been closing steadily for 3 weeks. It's normal for progress to plateau. When it does, consider: do you need to decompose differently (deload and rebuild)?"

### 10.2 Yoga Sequencing

- **One-line**: Designing a yoga class as a movement sequence: warm-up, build, peak pose, cool-down, savasana.
- **Movement grammar**: The peak pose is the desired state (e.g., full wheel backbend). The sequence prepares the body progressively: warm up the relevant muscles, open the relevant joints, build strength, attempt the peak, then integrate. Every pose in the sequence is a sub-tension that prepares for the peak.
- **Affordance shaping**: The sequence structure teaches that you can't jump to the desired state — you must prepare through ordered steps. The breath integration (ujjayi, kapalabhati) creates a parallel process: you're working on the physical form AND the internal state simultaneously. Savasana (final rest) is integration — you stop doing and let the changes settle.
- **Extractable idea**: **Integration pauses** — after resolving a tension, sc could prompt a "savasana" — a pause before jumping to the next tension. "You resolved this. What changed? What did you learn? Let it settle before moving on."

### 10.3 Martial Arts Kata

- **One-line**: Predefined sequences of movements practiced solo to internalize combat patterns.
- **Movement grammar**: The kata is a fixed form (desired) and your execution is the actual. The gap is between the ideal form and your current execution. But the deeper movement is: initially you learn the external form, then you understand the applications (bunkai), then the form teaches you principles that transcend the specific movements.
- **Affordance shaping**: Repetition is the affordance. Doing the same form 10,000 times changes not just your body but your understanding of the form. The same kata means something different at white belt and black belt. The fixed sequence frees attention from "what do I do next?" and directs it to "how do I do this better?"
- **Extractable idea**: **Same structure, deepening understanding** — sc's practice ceremony could be the same sequence every time (review pulse, pick a tension, examine it, update actual, step back). The power is in the repetition: the 100th time you run the ceremony, you see things the 1st time missed.

### 10.4 Climbing Route-Reading

- **One-line**: Analyzing a climbing route before attempting it — identifying holds, sequences, crux moves, and rest positions.
- **Movement grammar**: Stand at the base -> read the route (map desired sequence of moves) -> attempt the climb -> discover where reading was wrong -> fall/succeed -> re-read with new knowledge -> re-attempt. The tension is between the plan (read from below) and reality (discovered on the wall). Movement is through iterative plan-reality reconciliation.
- **Affordance shaping**: The physical wall is the environment — you can't change it, only your approach to it. Route-reading from the ground is always an approximation. The grading system (V-scale, YDS) provides coarse difficulty calibration. The "project" (a route you can't yet complete) is a named, ongoing tension.
- **Extractable idea**: **The project as practice** — climbers call a route they're working on "a project." It's a tension with a name and a history of attempts. sc could track attempt history for recurring tensions: "You've updated this actual 7 times. Here's the trajectory."

### 10.5 Alexander Technique

- **One-line**: Somatic re-education: notice habitual tension patterns and learn to "inhibit" (not suppress) automatic reactions.
- **Movement grammar**: Notice a habitual reaction (the actual) -> inhibit it (don't do the habitual thing) -> direct (consciously choose a different use of self) -> the new use gradually becomes natural. The key mechanism is inhibition: the space between stimulus and response is where freedom lives.
- **Affordance shaping**: The technique is taught through touch (teacher's hands on student). The primary affordance is the teacher's hands providing feedback on tension you can't feel yourself. "Directions" (instructions like "let the neck be free, let the head go forward and up") are not things you DO but things you ALLOW. This teaches the difference between efforting and allowing.
- **Extractable idea**: **Inhibition as strategy** — sometimes the action on a tension is to NOT do the habitual thing. sc could recognize a pattern: "Every time this tension type appears, you do X. Consider doing nothing and seeing what happens." The suggestion to inhibit a habit is itself a generative intervention.

---

## 11. Financial/Trading Systems

### 11.1 Portfolio Theory (Markowitz)

- **One-line**: Optimal allocation across assets to maximize return for a given level of risk (efficient frontier).
- **Movement grammar**: Define desired return -> assess risk tolerance -> allocate across uncorrelated assets to achieve the desired return at minimum risk. The tension is between return (desire) and risk (reality). Movement is through diversification — spreading bets reduces risk without proportionally reducing return, IF the assets are uncorrelated.
- **Affordance shaping**: The efficient frontier visualization shows the tradeoff between risk and return as a curve. You can see that there's a boundary you can't cross — you can't get high return with zero risk. This teaches the reality of tradeoffs. Correlation analysis teaches that the RELATIONSHIP between your tensions matters as much as the tensions themselves.
- **Extractable idea**: **Tension portfolio view** — sc's tension tree could be viewed as a portfolio. Are your active tensions correlated (all in one life area) or diversified? Are you taking on more total tension than you can manage (over-leveraged)? A portfolio view would surface imbalance.

### 11.2 Kelly Criterion

- **One-line**: Mathematical formula for optimal bet sizing: bet proportional to your edge, never more.
- **Movement grammar**: Assess your edge (how likely are you to win?) -> calculate optimal bet size -> bet that amount. The key insight: even with a positive edge, overbetting leads to ruin. Bet too big and you go broke even when you're right. Movement is through disciplined sizing, not just direction.
- **Affordance shaping**: The formula teaches that how MUCH you commit matters as much as WHAT you commit to. Full Kelly is optimal but volatile; half Kelly is safer with modest sacrifice. This creates a vocabulary for commitment sizing.
- **Extractable idea**: **Commitment sizing** — sc currently has rank (priority) but not commitment level. How much of your time/energy/attention are you committing to this tension? Over-commitment to even the right tensions leads to burnout. sc could track energy allocation and warn about over-commitment.

### 11.3 Technical Analysis Charting

- **One-line**: Reading price chart patterns (support/resistance, trend lines, indicators) to predict future price movement.
- **Movement grammar**: Plot historical data -> identify patterns (head and shoulders, double bottom, trend channels) -> project future movement from patterns -> act on projections -> evaluate. The movement is from raw data to pattern to prediction to action. The feedback loop is the market itself.
- **Affordance shaping**: Charts make time-series data visual. The pattern vocabulary (support, resistance, breakout, reversal) gives names to recurring situations. Indicators (moving averages, RSI, MACD) are computed overlays that extract signal from noise. The danger: seeing patterns that aren't there (apophenia). The discipline: waiting for confirmation.
- **Extractable idea**: **Tension sparklines** — sc already has sparklines for tension activity. v1.0 could extend them: visualize the trajectory of the actual field over time, relative to the desired. Patterns in the trajectory (convergent, divergent, oscillating, flat) are diagnostic.

### 11.4 Double-Entry Bookkeeping

- **One-line**: Every transaction recorded as equal debit and credit entries — the books must always balance.
- **Movement grammar**: Event occurs -> record it as both a debit and a credit -> the balance sheet always balances. The constraint (debits = credits) means errors are self-revealing. Movement is through accurate recording — the system doesn't tell you what to do, but it tells you exactly where you are.
- **Affordance shaping**: The mandatory balancing creates integrity — you can't fudge one number without visibly breaking the balance. The account structure (assets, liabilities, equity, revenue, expenses) provides a fixed ontology for categorizing all economic events. Historical records enable trend analysis.
- **Extractable idea**: **Integrity constraints** — sc could have constraints that must be satisfied: "every tension must have both a desired and an actual" (already true), but also "a resolved tension must have a resolution_reason," "a parent tension can't be resolved if active children exist." Constraints that catch inconsistency.

### 11.5 YNAB (You Need A Budget)

- **One-line**: Envelope budgeting software: give every dollar a job before you spend it.
- **Movement grammar**: Income arrives -> allocate every dollar to a category (job) -> when you overspend a category, move dollars from another category (explicit tradeoff) -> age your money (increase the gap between earning and spending). The movement is toward intentionality: every resource is allocated before use.
- **Affordance shaping**: "Give every dollar a job" is a forcing function for intentionality. The explicit reallocation when overspending means you FEEL the tradeoff — money for eating out comes from somewhere. The "age of money" metric reframes the goal from "spend less" to "increase the buffer." Rolling with the punches (adjusting budget mid-month) normalizes plan changes.
- **Extractable idea**: **Attention budgeting** — sc could borrow YNAB's model: you have X hours of attention this week. Allocate them to tensions. When you overspend on one, you must explicitly take from another. This makes the zero-sum nature of attention visible.

---

## 12. Gardening/Agriculture

### 12.1 Companion Planting

- **One-line**: Growing certain plants together because they mutually benefit (nitrogen fixers near heavy feeders, pest repellents near vulnerable crops).
- **Movement grammar**: Desired state (thriving garden) -> plant combinations that support each other -> the plants do work for each other (beans fix nitrogen for corn; corn provides trellis for beans). Movement is through designing beneficial relationships, not just optimizing individual plants.
- **Affordance shaping**: The companion planting chart (what grows well together, what doesn't) is a relationship map. It teaches that elements don't exist in isolation — their context determines their thriving. The "three sisters" pattern (corn, beans, squash) is a named, reusable relationship template.
- **Extractable idea**: **Tension companionship** — some tensions support each other (working on fitness supports working on mental health). sc could allow marking tensions as companions (mutually reinforcing) or antagonists (mutually undermining). The relationship map surfaces systemic design opportunities.

### 12.2 Permaculture Design

- **One-line**: Ecological design system based on observing and replicating natural ecosystems — zones, sectors, guilds.
- **Movement grammar**: Observe the site for a full year (understand actual deeply) -> design in zones (most attention-intensive elements closest to you) -> stack functions (each element serves multiple purposes) -> work with succession (plant for the future, not just now). Movement is through patient observation before action, then systemic design.
- **Affordance shaping**: The "observe for a year" principle teaches that understanding current reality takes time and patience. The zone system (Zone 0 = your house, Zone 5 = wilderness) organizes by attention frequency, not importance. The "stacking functions" principle teaches efficiency through integration.
- **Extractable idea**: **Zone-based attention** — sc's rank system could be replaced or supplemented by zones: Zone 1 (daily attention), Zone 2 (weekly), Zone 3 (monthly), Zone 4 (quarterly), Zone 5 (watch but don't act). Different tensions need different attention rhythms.

### 12.3 Bonsai Cultivation

- **One-line**: Shaping living trees through years of pruning, wiring, repotting, and patient observation toward an aesthetic vision.
- **Movement grammar**: Envision the mature tree (desired, 20 years from now) -> assess the current material -> make one or two moves per season (prune a branch, wire a trunk line) -> wait for growth -> assess again. The movement is through extreme patience and minimal intervention. Each intervention is high-stakes (a wrong cut can't be undone) and long-feedback-loop (results visible in months or years).
- **Affordance shaping**: The living material has its own agency — it grows where it wants, and you negotiate with it. The long time horizon teaches a different relationship to progress: "fast" in bonsai is 5 years. The seasonal rhythm means there are right times to act and right times to wait. The aesthetic tradition provides a vocabulary of styles (formal upright, cascade, literati) that guide vision.
- **Extractable idea**: **Seasonal timing** — some tensions have natural rhythms. Acting at the wrong time is wasted effort. sc could support "seasonal" metadata: "this tension is best worked on in Q1" or "wait until after the product launch to address this." Timing is a dimension of strategy that sc currently ignores.

### 12.4 Succession Planting

- **One-line**: Planting crops in staggered intervals so harvest is continuous rather than all-at-once.
- **Movement grammar**: Desired = continuous yield over the season. Plant wave 1 now -> plant wave 2 in two weeks -> plant wave 3 in four weeks. Each wave is at a different lifecycle stage at any given time. Movement is through temporal distribution — you manage a pipeline, not a single crop.
- **Affordance shaping**: The pipeline view teaches that you need things at different stages simultaneously. If everything is in germination, you'll have nothing to harvest for months. If everything is in completion, you'll have nothing coming next.
- **Extractable idea**: **Tension pipeline balance** — sc's pulse view shows lifecycle stages per tension. v1.0 could show the pipeline: "You have 5 tensions in germination, 2 in assimilation, 0 in completion. Your pipeline is back-loaded — consider which germination tensions are ready to push into active work."

### 12.5 Soil Building

- **One-line**: Building healthy soil through composting, cover crops, mulching, and microbial cultivation — the foundation for all growth.
- **Movement grammar**: You don't grow plants; you grow soil, and soil grows plants. The desired state (healthy soil) is invisible and underground. Movement is through decomposition: organic matter breaks down into humus, feeding microbes, which feed plants. The process is slow, invisible, and foundational.
- **Affordance shaping**: Soil building reframes "what's the real work?" The visible work (growing plants) depends on the invisible work (building soil). This teaches infrastructure thinking: the unglamorous foundational work that makes everything else possible.
- **Extractable idea**: **Infrastructure tensions** — sc could distinguish "soil" tensions (foundational, enabling, invisible) from "crop" tensions (visible, deliverable, outcome). Neglecting soil tensions is a common failure mode that sc's practice ceremony could surface: "You have no active infrastructure tensions. What foundational work are you neglecting?"

---

## 13. Navigation/Wayfinding

### 13.1 Polynesian Star Navigation

- **One-line**: Traditional Pacific wayfinding using stars, swells, clouds, birds, and mental models of island-moving-toward-you.
- **Movement grammar**: The navigator doesn't move; the island moves toward the canoe (conceptual inversion). You hold your star course and the destination approaches. The reference stars (rising/setting points of specific stars over specific islands) provide direction. Swell patterns, cloud formations, and bird behavior provide position feedback. Movement is through maintaining orientation, not through measuring distance.
- **Affordance shaping**: The "moving island" reframe is profound: instead of "I am traveling toward my goal," it's "my goal is arriving as I maintain my course." This reduces anxiety about progress and focuses attention on staying oriented. The navigational knowledge is held in memory and chants, not in instruments — the navigator IS the instrument.
- **Extractable idea**: **Orientation over measurement** — sc's pulse system measures progress (days since update, mutation count). But the Polynesian model suggests an alternative: are you oriented toward your desired state? Do you know which direction to face? Sometimes orientation is more important than velocity. sc could ask: "Forget how far you've come. Are you pointed the right way?"

### 13.2 Orienteering

- **One-line**: Competitive navigation sport: use map and compass to find control points in terrain as fast as possible.
- **Movement grammar**: Study the map (plan a route) -> run the route -> compare what you see to what the map says -> adjust. The tension is between map (model of terrain) and territory (actual terrain). Movement is through continuous map-territory reconciliation. Route choice (which path between control points) is the strategic dimension — the fastest straight line may cross a swamp.
- **Affordance shaping**: The map is a model — it's always an abstraction. Learning to read a map IS learning which details to ignore and which are load-bearing. The compass provides absolute direction reference. The "control point" system means you must visit specific locations in order — you can't skip.
- **Extractable idea**: **Map-territory reconciliation** — sc's tension tree IS a map. The actual fields are the territory. Practice mode is the reconciliation. v1.0 should make this explicit: "Your map says X. When you look at reality, is that still true?" Regular map-territory checks prevent the system from becoming fiction.

### 13.3 Dead Reckoning

- **One-line**: Estimating current position from last known position plus estimated speed and direction — no external reference.
- **Movement grammar**: Start from a known position (last verified actual) -> estimate movement (direction + speed + time = new position) -> accumulate errors over time -> eventually you MUST get a fix (external verification) to reset accumulated error.
- **Affordance shaping**: Dead reckoning teaches that error accumulates without correction. The longer you go without verification, the more uncertain your position. This creates urgency for periodic reality-checking.
- **Extractable idea**: **Error accumulation warning** — if a tension's actual hasn't been verified (updated with a real observation) in a long time, confidence degrades. sc could show "staleness" — not just "days since update" but "confidence in current actual" decaying over time. Stale tensions need reality checks before action.

### 13.4 GPS-Assisted Hiking (AllTrails)

- **One-line**: Mobile app providing trail maps, GPS position, elevation profiles, and user reviews for hiking.
- **Movement grammar**: Choose a trail (desired experience) -> follow GPS position on map (continuous actual) -> the blue dot shows you exactly where you are relative to the trail. Movement is through wayfinding-without-navigation-skill: the technology provides the orientation that Polynesian navigators develop internally.
- **Affordance shaping**: The GPS blue dot gives instant location certainty — you always know where you are. This removes wayfinding anxiety but also removes wayfinding skill development. The user reviews create social proof (others have done this; you can too). The elevation profile sets expectations for difficulty.
- **Extractable idea**: **"You are here" indicator** — sc's TUI already shows the tree and pulse. v1.0 could make the "you are here" more prominent in practice mode: "Of all your tensions, THIS is where you're standing right now. This is the one with the most energy / most urgency / most readiness."

### 13.5 Flight Planning

- **One-line**: Pilots plan flights with weather analysis, fuel calculation, route selection, alternates, and NOTAMs — comprehensive risk management.
- **Movement grammar**: Desired = arrive at destination safely. Actual = current position, weather, fuel, aircraft state. Planning = identify all the ways reality could deviate from desire and prepare for each. Movement is through comprehensive contingency planning: "What if the weather closes in? What if I burn more fuel than planned? What's my alternate?"
- **Affordance shaping**: The checklist culture (pre-flight, pre-takeoff, cruise, descent, landing) ensures nothing is skipped. The "personal minimums" concept means each pilot defines their own limits, which is a formalized self-knowledge practice. The requirement to file a plan and update it creates accountability.
- **Extractable idea**: **Contingency and alternates** — sc tensions could have optional "alternate" plans: "If this desired state turns out to be impossible, what's my alternate desired state?" Having a named alternate reduces the psychological cost of admitting the original plan won't work.

---

## Cross-Cutting Synthesis: Patterns Across All Categories

### Pattern 1: The Gap Must Be Named Before It Can Be Closed

Every system begins with articulating the tension — GTD's "clarify," the I Ching's question formulation, Starting Strength's "how much can you lift today?", Focusing's "find the felt sense." sc's core is right: the primitive is the named gap. But the naming can be supported: positional decomposition (Tarot), inquiry protocols (The Work), felt-sense tracking (Focusing), and distillation practices (sigil work).

### Pattern 2: The Architecture of the System Shapes What You Can See

Notion's relational databases make you think in properties. Go's simple rules produce emergent complexity. Ableton's dual views support different creative phases. sc's tree-with-pulse is already an architecture that shapes perception. v1.0 decisions: what views to add (spatial? portfolio? pipeline?), what metadata to track (confidence? energy? timing?), what the practice ceremony contains — these decisions shape what users can notice about their own situations.

### Pattern 3: Rhythm Creates the Container for Transformation

Weekly OKR check-ins, daily Ignatian Examen, seasonal bonsai pruning, guitar practice routines, morning pages. EVERY transformative system has a rhythm. sc's practice mode is this rhythm. The question for v1.0: what is the natural cadence? Daily pulse check (light), weekly deep review (medium), monthly/quarterly restructuring (heavy)?

### Pattern 4: Regression Is Part of Progression

Starting Strength deloads, alchemical nigredo, Alexander Technique inhibition, bonsai seasonal dormancy. Systems that don't account for regression teach users that going backwards means failure. sc should normalize: actual moving further from desired can be progress (dissolution before reformation).

### Pattern 5: The System Must Eventually Propose, Not Just Record

Anima's shadow tensions, Kumu's loop detection, companion planting charts, chess named patterns. Mature systems don't just hold your data — they surface what you can't see. sc v1.0 should move from pure recording toward gentle pattern detection: stagnation alerts, conflict detection (already exists), pipeline imbalance, and eventually proposed tensions.

### Pattern 6: Commitment Is a Phase Transition, Not a Setting

Ableton's Session-to-Arrangement, PARA's Resource-to-Project, Fusion 360's "fully constrained," the Golden Dawn's grade initiation. There's a meaningful difference between "thinking about this" and "committed to this." sc's seed->active transition is this, but it could be more ceremonial: "Are you committing to closing this gap?"

### Pattern 7: Orientation Matters More Than Velocity

Polynesian star navigation, orienteering, flight planning. Knowing you're pointed the right way matters more than knowing how fast you're going. sc's desired-state field IS the orientation. The practice of regularly checking "is this still what I want?" is more important than tracking how fast actual is approaching desired.

---

## Anima Pattern Integration: How Prediction-Observation-Surprise Maps to Structural Tension

The anima plan's core loop is:

```
self-model → predictions → session → evidence → surprise → updated self-model
```

This IS structural tension applied to identity:

| Anima Concept | sc Tension Equivalent |
|---|---|
| Self-model | The tension tree itself (your map of your situation) |
| Prediction | Implied desired movement ("I expect actual to move toward desired") |
| Session evidence | Reality updates (new actual values) |
| Surprise | Gap between expected movement and actual movement |
| Model update | Revising desired, actual, or tree structure |

**Key insight for sc v1.0**: The tension tree is itself a model that should be subject to the prediction-observation-surprise loop. Each practice session, sc could implicitly or explicitly predict: "Based on recent trajectory, here's what I expect happened since last session." Then the user updates actuals. Discrepancies between predicted and observed movement are the highest-signal moments — they indicate either the tension is mis-specified, the approach isn't working, or something unexpected happened.

**Specific anima patterns to steal**:

1. **Distinctiveness probes** → "Is this tension specific to YOU, or would anyone in your position have it?" Generic tensions produce generic actions.

2. **Behavioral audit vs. calibration** → Two modes of practice: (a) "Is the gap closing?" (calibration) and (b) "Is this even the right gap?" (audit). These are different activities and sc should distinguish them.

3. **Shadow tensions** → After enough history, sc could suggest: "You keep updating actuals in area X but have no tension for it. Should there be one?"

4. **Hollow pattern detection** → If a tension's desired state hasn't changed in months and the actual keeps bouncing around, the desired might be hollow — aspirational but not load-bearing. sc should surface this.

5. **Meta-synthesis** → Periodically synthesize across all tensions: "Looking at your whole tree, what's the pattern? What's the thing you're not seeing?"
