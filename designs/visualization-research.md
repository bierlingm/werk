# Visualization Research: Structure x Time x State on a 2D Surface

**Context:** Research for werk TUI — a terminal UI (character grid, limited color, ~80-200 columns, ~24-60 rows) showing a lattice of hierarchical tensions evolving over time, where the user is at the center and the field moves around them.

**Date:** 2026-03-26

---

## Part I: Techniques Organized by Underlying Principle

The techniques below are grouped not by domain of origin but by the structural principle they embody. Many techniques appear in multiple groups because they combine principles. The grouping reveals what is transferable.

---

### Principle 1: Flow as Width — Encoding Quantity in the Thickness of a Path

**Techniques:** Minard's map, Sankey diagrams, alluvial diagrams

#### How they work

Minard's 1869 map of Napoleon's Russian campaign encodes six variables on a 2D surface: army size (band width), geographic position (x/y), direction of travel (color/path direction), temperature (aligned chart below), and time (implicit in the path). The band narrows as soldiers die. You read catastrophe in the thinning of a line.

Sankey diagrams generalize this: nodes connected by flows whose width is proportional to quantity. Flows can split and merge. Alluvial diagrams are a temporal variant where the x-axis is time and flows show how categorical memberships change across discrete time slices.

#### What makes them powerful

- **Width as quantity is pre-attentive.** You don't count — you *see* that something is shrinking or growing. The eye tracks proportion without conscious effort.
- **Branching and merging show structural change.** When a flow splits, something divided. When flows merge, something consolidated. The topology of the flow *is* the story.
- **Causality through spatial alignment.** Minard's temperature chart below the map lets you *see* that temperature drops coincide with army losses. Juxtaposition creates causal inference without asserting it.

#### Terminal applicability: MEDIUM-HIGH

Width encoding translates to character-grid work: a band 3 characters wide vs. 1 character wide is immediately readable. Unicode box-drawing and block characters (▏▎▍▌▋▊▉█) give 8 levels of sub-character width. For werk:

- **Tension health as band width.** A tension with 8/10 steps resolved is a thick band; one with 2/10 is thin. The thinning/thickening over time (scrolling vertically through epochs) shows progress or regression.
- **Theory of closure as branching flow.** When a tension splits (phase transition), the flow visually bifurcates. When steps resolve, sub-flows merge back into the parent.
- **Accumulated zone as alluvial summary.** The compressed representation of resolved/released/noted items could use width-proportional bands rather than counts: `✓████░░ ~ ██░ ※ █` tells you more than `✓ 5 resolved · ~ 2 released · ※ 1 note`.

---

### Principle 2: Slope as Rate — Using Angle to Encode Speed or Velocity

**Techniques:** Marey's train schedule, burndown/burn-up charts, behavior-over-time graphs

#### How they work

Marey's 1878 train schedule places stations on the y-axis (proportional to distance) and time on the x-axis. Each train is a line. Steep slope = fast train. Horizontal segment = stopped at station. Lines crossing = trains passing each other. The entire Paris-Lyon schedule fits on one sheet.

Burndown charts plot remaining work (y) against time (x). The slope of the actual line vs. the ideal line tells you whether you're ahead or behind without reading numbers.

#### What makes them powerful

- **Slope is a natural metaphor for rate.** Steep = fast, shallow = slow, horizontal = stalled, vertical = instantaneous. The human visual system processes angle effortlessly.
- **Crossings encode interactions.** In Marey's chart, where lines cross, trains pass. In a multi-tension burndown, where lines cross, one tension overtook another in progress. Crossings are *events* that emerge from the representation rather than being explicitly marked.
- **Deviation from expected slope is a signal.** A burndown line that flattens shows stall. A Marey line that goes horizontal shows delay. The *change in slope* is where the information lives.

#### Terminal applicability: HIGH

This is one of the most terminal-friendly principles. Character-grid diagonal lines using `/`, `\`, `╱`, `╲`, and Unicode line-drawing characters can represent slopes at discrete angles. For werk:

- **Epoch progress as slope.** Within a descended view, the vertical axis is order-of-operations and the frontier moves upward as steps resolve. The *rate* at which the frontier advances could be shown as a sparkline slope in the right margin: `╱╱╱─╱` (steady progress, stall, resumed).
- **Multi-tension Marey chart at survey level.** Each root tension is a line on a time-vs-progress grid. Where they are steep, work happened fast. Where they flatten, attention went elsewhere. Where they cross, one overtook another. This is a powerful survey view.
- **Deadline approach as slope.** The urgency computation (elapsed fraction of deadline window) could be visualized as a slope approaching a horizontal deadline line. Steep approach = running out of time. Shallow = comfortable.

---

### Principle 3: Strata as Accumulated Time — Layering to Show History

**Techniques:** Stratigraphic columns, horizon charts, geological cross-sections, icicle plots

#### How they work

Geological stratigraphic columns show layers of rock from bottom (oldest) to top (youngest). Each layer's thickness represents duration. Composition, color, and texture encode what happened during that period. Cross-sections show how strata vary across space.

Horizon charts take a time series, divide it into bands by value range, and fold the bands on top of each other with color intensity encoding the band. A chart that would normally need 100px of vertical space fits in 25px with 4 bands. You read magnitude by color saturation/hue and direction by position.

Icicle plots show hierarchy as horizontal layers: root at top, full width; children below, width proportional to their share; recursively deeper.

#### What makes them powerful

- **Compression without loss.** Horizon charts achieve 4:1 or 8:1 vertical compression while preserving the ability to detect peaks, troughs, and trends. The principle: use color to encode what the y-axis previously encoded, freeing vertical space.
- **Bottom-up reading is natural for time.** We read geological columns and icicle plots from bottom to top or outside to inside. Older things support newer things. This maps directly to werk's spatial law: reality (accumulated, past) at bottom, desire (aimed-at future) at top.
- **Layer thickness encodes duration.** A thick stratum = long period. A thin stratum = brief. This is information that traditional timelines waste a full dimension on.

#### Terminal applicability: HIGH

The stratigraphic metaphor is directly applicable to werk's epoch model. For werk:

- **Epochs as strata.** Each epoch in the log is a layer. Thickness (number of lines allocated) can be proportional to epoch duration or activity density. The current epoch is the top layer, the active surface. Prior epochs compress downward.
- **Horizon-chart-style compression for the route.** Instead of showing every step at full height, use color/shading intensity to encode urgency while maintaining a single-line-per-step layout. Steps near their deadline get intense color; steps with breathing room get dim. This keeps the route compact while encoding temporal pressure.
- **Reality trace as geological cross-section.** The pattern of reality updates — steady advance, oscillation, regression — could be shown as a compressed stratigraphic mini-view: `▁▂▃▄▅▆▇█` for steady climb, `▅▃▅▃▅▇` for oscillation. This encodes the *shape* of the trace (which the conceptual foundation calls diagnostic).

---

### Principle 4: Focus + Context — Showing Detail at Center, Structure at Periphery

**Techniques:** Fisheye views, hyperbolic trees, DOI (Degree-of-Interest) trees, semantic zoom

#### How they work

Fisheye views apply a distortion function: items near the focal point are shown at full size; items farther away are progressively compressed. Hyperbolic trees embed a tree in hyperbolic space and project it onto a disk — the focused node appears large at center, with exponentially more context fitting at the edges.

DOI trees (Furnas, Card & Nation) compute each node's Degree of Interest = intrinsic importance + distance from focus. Nodes below a threshold are elided. This is not geometric distortion but *semantic* filtering — what matters gets shown, what doesn't gets hidden.

Semantic zoom changes not just scale but *representation* at different zoom levels. A city at country-zoom is a dot; at city-zoom it shows districts; at street-zoom it shows buildings. The content changes qualitatively, not just quantitatively.

#### What makes them powerful

- **The eye can only focus on one thing.** These techniques align the display with the structure of human attention: sharp center, fuzzy periphery. They make the display *behave like vision itself.*
- **Exponential compression.** Hyperbolic geometry is native to trees: circumference grows exponentially with radius, matching the exponential growth of tree breadth with depth. A 1000-node tree fits on screen with the focused subtree readable and the rest providing orientation.
- **DOI is user-model-aware.** Unlike geometric fisheye (which is purely spatial), DOI incorporates *what matters* — recently visited nodes, nodes with anomalies, nodes on the critical path. The distortion serves the user's task, not just their position.

#### Terminal applicability: VERY HIGH — this is werk's existing core strategy

Werk already implements semantic zoom (orient/normal/focus) and the envelope-as-focus concept. The research suggests deepening this:

- **DOI-driven elision of route steps.** Instead of compressing route by position (first/last bookends), compute DOI: steps on the critical path, steps with deadline pressure, steps recently mutated get high DOI. Steps with no signals get elided. The route shows *what matters*, not *what's next*.
- **Fisheye within the route itself.** The step the cursor is on shows full detail (desire text, deadline, children count, age). Adjacent steps show text + deadline. Steps 3+ away show just the glyph + ordinal. This is character-grid fisheye: allocating *lines* rather than pixels proportionally to proximity.
- **Hyperbolic survey.** At the root level (survey view), the focused tension gets multi-line treatment. Adjacent tensions get single-line. Distant tensions get compressed to glyph + name fragment. The exponential compression of hyperbolic space maps to the exponential compression of text: from 5 lines to 1 line to a single character.

---

### Principle 5: Parallel Tracks — Multiple Independent Sequences Aligned on a Common Axis

**Techniques:** Conductor's score, Labanotation, piano roll, timing diagrams, parallel coordinates, swimlanes

#### How they work

A conductor's score stacks instrument parts vertically, aligned on a common time axis (left to right). The conductor reads vertically to see what everyone plays at one moment, horizontally to follow one instrument's journey. Beat lines create a grid that syncs all parts.

Labanotation uses a vertical staff read bottom-to-top (time flows upward). The center line is the body's center; columns left and right represent body parts. Shape = direction, shading = level, length = duration, column = body part. Four dimensions encoded per symbol, plus temporal alignment across all body parts.

Timing diagrams in circuit design show digital signals as horizontal tracks, each oscillating between HIGH and LOW, aligned on a common time axis. Transitions, setup times, and causal relationships between signals are visible through vertical alignment.

Piano rolls map pitch to y-axis, time to x-axis, and encode duration as bar length. Multiple voices use colors. The vertical slice shows the chord; the horizontal slice shows the melody.

#### What makes them powerful

- **Vertical alignment reveals simultaneity.** When you can see that the oboe enters exactly when the violins fade, that's a vertical read. The common axis makes coincidence visible.
- **Horizontal continuity reveals narrative.** Following one track left-to-right shows its story. The two reading directions (vertical = moment, horizontal = journey) serve different questions from the same display.
- **Multiple independent sequences on one surface.** Each track is autonomous — it has its own state, its own rhythm, its own transitions. But the alignment makes their *relationships* visible without explicit linking.

#### Terminal applicability: HIGH

This is directly relevant to the survey view and multi-tension awareness. For werk:

- **Survey view as conductor's score.** Each root tension is a horizontal track. Time flows left to right. The current moment is a vertical line. You can read vertically: "right now, tension A is active at step 3, tension B is overdue, tension C is held." You can read horizontally: "tension A has been progressing steadily."
- **Labanotation-inspired encoding.** Labanotation's four-dimensional symbol (shape + shading + length + position) maps to terminal glyphs: `▸` (active, positioned), `·` (held), `✓` (resolved), `~` (released). Add color for urgency, width for importance, position for sequence. Each glyph carries multiple channels.
- **Timing-diagram-style state transitions.** Each tension's state over time as a single track: `───▸▸▸═══✓` where `───` is quiescent, `▸▸▸` is active, `═══` is overdue, `✓` is resolved. Multiple tensions stacked vertically, aligned on time, make patterns visible: which tensions move together, which are anti-correlated, which are neglected.

---

### Principle 6: Topology as Information — Letting Shape Encode Structure

**Techniques:** Phase portraits, causal loop diagrams, PERT/dependency graphs, syntax trees, Feynman diagrams

#### How they work

Phase portraits plot one state variable against another (e.g., position vs. velocity for a pendulum). The resulting trajectory reveals the system's qualitative behavior without solving equations: spirals = damped oscillation, closed loops = periodic motion, fixed points = equilibria.

Causal loop diagrams show variables as nodes and causal relationships as arrows marked + (reinforcing) or - (balancing). Loops are the structure: reinforcing loops drive exponential growth/collapse; balancing loops drive oscillation or equilibrium.

PERT charts show tasks as nodes and dependencies as edges. The critical path — longest chain of dependent tasks — determines minimum project duration. The shape of the graph (wide vs. narrow, deep vs. shallow) reveals parallelism vs. serial bottlenecks.

Syntax trees show hierarchical decomposition of a sentence. Each node is a constituent; the tree shape reveals the structure of meaning.

Feynman diagrams use topology (which lines connect to which vertices) rather than geometry (exact positions) to encode particle interactions. A diagram's meaning is in its connectivity, not its layout.

#### What makes them powerful

- **Shape carries meaning independent of scale.** A spiral in a phase portrait means "damped oscillation" whether drawn large or small. A reinforcing loop in a causal diagram means "exponential behavior" regardless of which variables are involved. Topology is scale-invariant.
- **Qualitative behavior from quantitative data.** Phase portraits reveal stability, periodicity, chaos — *categories* of behavior — without requiring precise numerical reading. The question shifts from "what is the value?" to "what kind of behavior is this?"
- **Dependencies are edges, not annotations.** In PERT/dependency graphs, the relationship *is* the visual element (the line), not a label on some other element. Structure is first-class.

#### Terminal applicability: MEDIUM

These are harder to render on a character grid but the *principles* transfer powerfully. For werk:

- **Tension phase portrait as sparkline.** Plot reality-progress (x) against desire-evolution (y) as a micro-chart. A diagonal line = steady convergence. A horizontal line = reality advancing without desire changing (execution phase). A vertical line = desire shifting without reality advancing (re-envisioning). An oscillation = structural tension unresolved. This tiny chart (6-8 characters wide) placed next to a tension name tells the practitioner the *qualitative character* of their engagement with that tension.
- **Theory-of-closure as dependency topology.** Instead of a flat list, show which steps depend on which. In terminal: indentation + connecting lines. `├──` and `└──` already encode tree topology. Adding dependency arrows (even as annotation: `←3`) shows non-tree dependencies.
- **Causal loop awareness.** When tensions reference each other (step in A depends on outcome of B), this is a loop. The instrument could detect and surface these: `⟳ A ↔ B` signals a reinforcing or balancing relationship between tensions.

---

### Principle 7: Contour and Isoline — Equal-Value Curves on a Field

**Techniques:** Topographic maps, isochrone maps, choropleth maps, contour plots, heatmaps

#### How they work

Topographic maps use contour lines connecting points of equal elevation. Closely spaced lines = steep terrain. Widely spaced = gentle slope. The shape of contours reveals ridges, valleys, saddles, peaks.

Isochrone maps show contours of equal travel time from a point. The shape distorts based on transport networks: 15 minutes by car reaches farther along highways. The map shows *accessibility*, not distance.

Choropleth maps color regions by a quantitative value. Heatmaps generalize this to a grid.

#### What makes them powerful

- **Gradient without explicit annotation.** Closely spaced contour lines *mean* steep gradient. You read the density of lines, not labels. The spacing *is* the data.
- **Fields, not points.** These techniques show a *continuous field* over a space, not discrete data points. This reveals structure (ridges, basins, saddle points) that discrete representations miss.
- **Isochrones redefine distance.** An isochrone map centered on "you" shows what you can reach — not what's geometrically close, but what's *functionally* close. This directly maps to "you are the center, field moves around you."

#### Terminal applicability: MEDIUM-HIGH

Character-grid contours use block characters and Braille dots (⠁⠂⠃⠄⠅⠆⠇⡀⡁...) for density. For werk:

- **Urgency isochrones.** Center the display on "now." Draw contours of equal urgency radiating outward. Steps at urgency 0.9 are in the innermost ring; steps at 0.1 are in the outermost. The "terrain" is the landscape of temporal pressure. This inverts the normal list view: instead of ordering by sequence, order by urgency-distance-from-now.
- **Activity density as heatmap.** In the survey view, the background shade of each tension reflects how much mutation activity it received recently. Active tensions are bright; neglected tensions are dim. This is signal-by-exception: you don't annotate "neglected" — the darkness *is* the neglect.
- **Contour lines in the route.** Between route steps, the "spacing" (number of empty lines or dash characters) could reflect the implied execution window. Tightly spaced = compressed timeline. Widely spaced = breathing room. The spacing *is* the temporal data.

---

### Principle 8: Compression Through Folding — Reducing Dimensions by Overlaying

**Techniques:** Horizon charts, small multiples, interlinear glossing, exploded views

#### How they work

Horizon charts fold a tall chart into bands, overlaying them with color differentiation. 4:1 compression with minimal perceptual loss.

Small multiples (Tufte) repeat the same frame across values of one variable. Each individual chart is simple; the collection reveals pattern. "Small multiples enforce local comparisons within our eye span."

Interlinear glossing in linguistics aligns three+ parallel texts line by line: source language, morpheme-by-morpheme gloss, free translation. Each line provides a different *view* of the same content, aligned for cross-reference.

Exploded views in architecture separate layers of a building vertically, maintaining their x-y alignment but adding z-separation. You see each layer independently while understanding their stacking.

#### What makes them powerful

- **Repetition with variation is how humans detect pattern.** Small multiples work because the eye can compare 16 structurally identical charts faster than it can parse one chart with 16 overlaid series. The frame becomes invisible; the variation is the signal.
- **Interlinear alignment connects heterogeneous views.** Glossing doesn't just show source and translation — it shows them *aligned*, so you can see which source word maps to which meaning. The alignment *is* the analysis.
- **Folding preserves shape while compressing range.** Horizon charts work because the human visual system can decode "dark blue at height 0.5 = value 3.5" once trained. The shape of the time series is preserved; only the y-axis encoding changes.

#### Terminal applicability: VERY HIGH

These are among the most terminal-native techniques. For werk:

- **Small multiples for multi-tension comparison.** Instead of a single deck view, show 3-4 tensions side by side, each as a compressed mini-deck: desire on top, reality on bottom, a few route steps between, console zone highlighted. At 50 columns each, 4 tensions fit in 200 columns. The eye compares structure across tensions instantly.
- **Interlinear glossing for the deck itself.** The current deck layout is already an interlinear structure: left column (deadline) | main column (text) | right column (ID, children, age). Each column is a different "language" describing the same step. The principle suggests: ensure strict vertical alignment so the eye can read across columns for one step, or down columns for one dimension.
- **Horizon-style route compression.** Instead of allocating one line per route step, allocate half a line: two steps per line, using left-half and right-half characters or color to distinguish them. This doubles route capacity in the same vertical space.

---

### Principle 9: The Observer at Center — Egocentric Projection

**Techniques:** Isochrone maps, fisheye views, radar/spider charts, military situation maps, head-up displays

#### How they work

Military situation maps (Common Operating Picture) center on the commander's area of operations. Overlays layer tactical information: friendly positions (blue), enemy positions (red), boundaries, objectives, movement arrows. The map moves as the situation moves; the commander's perspective is the origin.

Head-up displays project critical information onto the pilot's forward view. The information is spatially registered to the real world — an altitude indicator overlays the actual horizon. The pilot doesn't look away from the world to read instruments.

Radar charts (spider/web) place variables on radial axes from a center point. A data point's values form a polygon. The polygon's shape = the entity's profile. Multiple polygons overlay for comparison.

#### What makes them powerful

- **You don't navigate to the information; the information navigates to you.** The display is organized around your position, your priorities, your current heading. Everything is relative to you.
- **Overlays add dimensions without changing the base.** Military maps use acetate overlays: one for terrain, one for friendly forces, one for enemy, one for planned movements. Each overlay is independent, toggleable, and registered to the same coordinate system.
- **Heading implies what's ahead.** In egocentric displays, "forward" = "what's coming." "Behind" = "what's past." Direction has temporal meaning.

#### Terminal applicability: HIGH — this is werk's core metaphor

The conceptual foundation already establishes this: "When the user opens the instrument, the cursor lands at the envelope. They look up to see what's next. They look down to see what's done." The research suggests:

- **The deck IS a head-up display.** The envelope/console is the "forward view." Route steps above are the flight path ahead. Reality below is the ground already covered. The deck should feel like looking through a window at one's own creative structure, not like reading a report.
- **Overlay-style toggling.** Different "layers" of information on the same spatial layout: default view shows text + glyphs. Toggle deadline layer to add urgency shading. Toggle trace layer to add sparkline histories. Toggle dependency layer to add relationship indicators. Same spatial positions, different information depth.
- **Heading as temporal direction.** The cursor's movement direction (pitch up/down) should feel like steering through time: moving up = looking further into the future (route), moving down = looking further into the past (log). The user's heading is their temporal attention direction.

---

### Principle 10: Glyph Alphabets — Encoding Multiple Dimensions in a Single Symbol

**Techniques:** Labanotation symbols, Chernoff faces, military map symbols (MIL-STD-2525), weather station plots, Unicode block elements

#### How they work

Labanotation encodes four dimensions per symbol: shape = direction, shading = level, length = duration, column = body part. A trained reader decodes all four simultaneously.

Military symbology (MIL-STD-2525) encodes affiliation (shape: circle=unknown, rectangle=friendly, diamond=hostile), echelon (size indicator), function (icon inside), status (fill: present=filled, planned=outline), and modifiers (text/number annotations). A single symbol on a map communicates 5+ dimensions.

Weather station plots encode temperature, dew point, wind speed (barbs), wind direction (shaft angle), cloud cover (circle fill), pressure, and weather type — all in one station circle.

#### What makes them powerful

- **Bandwidth of a trained reader is enormous.** A meteorologist reads a station plot in a glance, extracting 8+ variables. The learning curve is real but the payoff is permanent.
- **Consistent alphabet enables pattern recognition.** When every entity uses the same glyph system, anomalies pop: the one diamond (hostile) among rectangles (friendly). The one filled circle (overcast) among open circles (clear). Deviation from the pattern *is* the signal.
- **Spatial position is freed for other use.** When dimensions are encoded in the glyph itself, position on the display can encode geography, hierarchy, or time — rather than being consumed by data representation.

#### Terminal applicability: HIGH

Werk already uses a glyph vocabulary (▸, ·, ✓, ~, ※). The research suggests enriching this:

- **Phase-encoded glyphs.** Currently, glyphs encode status. They could additionally encode urgency through surrounding characters or color: `▸` (normal active), `▸` in cyan (approaching deadline), `▸` in bold/red (overdue). The glyph system already works; adding color as a second channel multiplies information density.
- **Composite glyphs for compound state.** A step that is active, on the critical path, and approaching its deadline might be: `▸!` or `▸▸` (doubled = emphasis) or `▶` (filled triangle = urgent variant of ▸). The glyph alphabet can grow to handle 2-3 dimensions per symbol position.
- **Sparkline micro-glyphs.** `▁▂▃▄▅▆▇█` encode 8 levels in one character width. `⣀⣤⣶⣿` use Braille for 4 levels. These can appear inline to show history: `▁▃▅▇` = steady progress, `▇▅▃▁` = regression, `▃▃▃▃` = stall.

---

## Part II: Domain-Specific Techniques and Their Transferable Ideas

### From Cartography

**Key transfer:** Contour lines for urgency, isochrone for accessibility, flow maps for movement direction.

The most powerful cartographic idea for werk is the **isochrone**: what can I reach from here within N units of effort? Applied to tensions, this becomes: "given my current position in all active tensions, what steps are within one session's reach?" This is a different slicing of the data than the current per-tension view — it's a *cross-tension frontier* defined by proximity to action, not by hierarchy.

### From Music Notation

**Key transfer:** The conductor's score model (parallel tracks, aligned time axis, vertical reading for simultaneity) and piano roll (pitch-as-hierarchy, duration-as-bar-length).

The deepest music-notation insight is **the rest**. In music, silence is notated. A rest is not absence — it is a deliberate act of not-playing. In werk terms: a tension that is deliberately not being worked on is different from one that is neglected. The notation should distinguish between held-silence (intentional pause) and dead-air (fallen off the radar). The glyph vocabulary could include a rest marker.

### From Scientific Visualization

**Key transfers:**

- **Feynman diagrams:** Topology over geometry. The *connection pattern* of tensions matters more than their exact position. A tension that feeds into three others is structurally different from one that stands alone, regardless of where they appear on screen.
- **Spacetime diagrams:** The light cone concept. From any point in the tension lattice, some future states are reachable and some are not. The "light cone" of a step is determined by dependencies and deadlines. Steps outside the light cone are structurally inaccessible from here.
- **Bifurcation diagrams:** Phase transitions in werk (desire changes, reality shifts) are literally bifurcation points. The behavior of the tension *qualitatively changes* at these boundaries. Visualizing where past bifurcations occurred (as markers on an epoch timeline) shows the structural history of the tension's character.

### From Military Planning

**Key transfers:**

- **Situation map overlay model.** Base map (tension structure) + overlays (urgency, dependencies, activity recency, epoch boundaries). Each overlay is independently toggleable and adds information to the same spatial layout. In terminal: this maps to key-toggled display modes that recolor/annotate the same positions rather than switching to different screens.
- **Movement arrows.** The planned movement of attention — which tension will be worked next, what the intended sequence is — as directional indicators on the survey view. Not just "where things are" but "where things are going."

### From Systems Dynamics

**Key transfers:**

- **Reinforcing vs. balancing loops.** When two tensions have mutual dependencies (progress on A enables progress on B enables progress on A), that's a reinforcing loop — and a leverage point. When they compete for resources (progress on A requires neglecting B), that's a balancing loop — and a structural tension. The instrument could detect and annotate these patterns from the mutation history.
- **Stock and flow.** Unresolved steps are stock (accumulated). Resolution rate is flow (throughput). Inflow is step creation. The ratio of stock to flow is the backlog pressure. This is computable from the data and could be shown as a single number or sparkline per tension.
- **Behavior over time graphs.** The simplest systems dynamics tool: plot a variable over time and look at the shape. For werk: plot "number of active steps" over time per tension. Rising = scope creep. Falling = progress. Oscillating = rework. Flat = stall. This is the reality-trace shape that the conceptual foundation calls diagnostic.

### From Dance Notation (Labanotation)

**Key transfer:** The staff model — a vertical timeline with parallel columns for different body parts, where each symbol simultaneously encodes direction, level, duration, and part.

Applied to werk: a Labanotation-style log view where each column represents a different aspect of a tension (desire, reality, theory-of-closure composition, step activity, notes), time flows upward, and symbols in each column encode what happened. The entire history of a tension as a single multi-columnar vertical staff, where trained reading reveals the pattern of engagement.

### From Architecture

**Key transfer:** The section drawing — a cut through a building revealing internal structure.

Applied to werk: a "section" through the tension lattice at a particular moment, showing all tensions at their current state, cutting through hierarchy to reveal the cross-sectional structure. The exploded view — separating layers — maps to the zoom levels: orient (collapsed), normal (one layer expanded), focus (fully separated into constituent parts).

### From Circuit Design

**Key transfer:** Timing diagrams show state transitions as step functions on parallel tracks.

Applied to werk: each tension's lifecycle as a timing-diagram track — `───╥═══╥───╥═══╥───✓` where `═══` is active periods and `───` is quiescent periods. The transitions (╥) are gesture events. Multiple tensions stacked and time-aligned reveal patterns: which tensions activate together, which are anti-correlated (when one rises another falls = attention competition), and where the dead zones are.

### From Geology

**Key transfer:** The law of superposition (oldest at bottom, youngest at top) and the concept of unconformity (a gap in the geological record indicating missing time).

Applied to werk: epochs as strata are already natural. The unconformity idea is powerful: when there's a gap in mutation history (weeks of no gestures on a tension), that gap is *structural information*. It's the geological equivalent of erosion — time passed but nothing was deposited. The display could mark unconformities: a visible gap or marker indicating "nothing happened here for N days."

### From Genealogy

**Key transfer:** The ahnentafel numbering system — a deterministic way to assign unique numbers to every ancestor (2n = father, 2n+1 = mother). And the pedigree chart's exponential fan-out.

Applied to werk: a deterministic addressing scheme for positions in the theory of closure. The ahnentafel principle is that structure determines address. A step's position in the hierarchy *is* its identity, not an arbitrary ID. This is more of a data model insight than a visualization one, but it informs display: structural position can replace explicit labels.

### From Linguistics

**Key transfer:** Interlinear glossing — multiple aligned parallel representations of the same content at different levels of analysis.

Applied to werk: the deck's column layout is already interlinear. The principle suggests pushing further: three "lines" per step that are always aligned:
1. Intent line: what is this step for (desire text)
2. Status line: what is its current state (glyphs, urgency, deadline)
3. Trace line: what has happened (micro-history sparkline)

Normally, only line 1 shows (normal zoom). Focus zoom reveals all three. The alignment ensures you can read across (one step, three views) or down (one view, all steps).

### From Project Management

**Key transfer beyond Gantt:**

- **PERT's critical path as visual emphasis.** The critical path through the theory of closure should be visually distinct — not annotated but *structurally emphasized*. Bold text, brighter color, or additional characters. The critical path is the load-bearing structure; everything else is margin.
- **Kanban's "WIP limits" as visual constraint.** The operating envelope is already a WIP concept — it bounds what's action-relevant. The visual design should make the envelope feel like a container with finite capacity, not an infinite scroll.

---

## Part III: Synthesis — The 10 Most Promising Ideas for Werk's TUI

Ranked by (a) information density, (b) terminal feasibility, (c) alignment with werk's conceptual foundation, (d) novelty over current design.

---

### 1. Sparkline Micro-Histories Inline with Each Tension

**Source principles:** Tufte's sparklines, behavior-over-time graphs, Marey's slope-as-rate, geological trace shape

**The idea:** Every tension, at every level of the hierarchy, carries a small inline sparkline (4-8 characters) showing its activity pattern over recent time. Using `▁▂▃▄▅▆▇█` characters, encode one of: resolution rate, mutation frequency, or urgency trajectory. The sparkline appears in the right column alongside the ID and age.

**What it reveals:** The *shape* of engagement — steady progress (`▁▃▅▇`), stall (`▃▃▃▃`), burst (`▁▁▇▁`), oscillation (`▃▇▃▇`), abandonment (`▇▅▃▁`). This is the "trace shape is diagnostic" principle from the conceptual foundation, made visible without consuming any additional vertical space.

**Terminal implementation:** 6 characters wide, right-aligned, dimmed (low-priority visual channel). Computed from gesture timestamps bucketed into time windows.

```
  Mar 28  ▸ Draft proposal          #42 →3  ▂▄▆▇ 2d
  Apr 05  ▸ Review with team        #43 →0  ▁▁▂▃ 5d
           · Contingency plan       #44      ▁▁▁▁ 12d
```

---

### 2. Conductor's Score Survey View

**Source principles:** Musical score, parallel tracks, timing diagrams, Marey's train schedule, small multiples

**The idea:** The survey view (showing all root tensions) uses a conductor's-score layout: each tension is a horizontal track. Time flows left to right. The leftmost column is the tension name. The body of the track shows state over time using compact glyph sequences. A vertical cursor line marks "now." The user reads vertically to see all tensions at one moment; horizontally to see one tension over time.

**What it reveals:** Temporal patterns *across* tensions — which ones move together, which compete for attention, where dead zones are, which are approaching deadlines. This is the cross-tension correlation the logbook query system aspires to, made visual.

**Terminal implementation:**

```
  Now ─────────────────────┐
  Book     ───▸▸▸═══▸▸✓───│───────
  Course   ▸▸▸▸▸▸▸────────│▸▸▸════
  Garden   ──────▸▸▸───▸──│▸──────
  Release  ────────────────│▸▸▸▸▸▸▸
                           │
```

Where `▸` = active progress, `═` = overdue, `─` = quiescent, `✓` = resolved step, and the density of `▸` marks reflects activity intensity.

---

### 3. DOI-Driven Route Compression

**Source principles:** Degree-of-Interest trees, fisheye, semantic zoom, signal by exception

**The idea:** Instead of showing route steps in flat list order and compressing by position (first/last bookends), compute a Degree of Interest for each step: DOI = f(critical path membership, deadline proximity, mutation recency, child complexity, cursor distance). Steps above a threshold get full display. Steps below threshold get single-character representation. Steps far below get elided entirely with a count.

**What it reveals:** The *structurally significant* steps rather than the *sequentially adjacent* steps. A step that is 8th in sequence but on the critical path with an approaching deadline is more important than the 2nd step that has no deadline and no signals.

**Terminal implementation:**

```
  Apr 15  ▸ Core API implementation          #31 →5  ▃▅▇█ 1d   <- high DOI: critical path + recent
          ⋯ 3 steps                                              <- elided: low DOI
  Apr 22  ▸ Security audit                   #35 →2  ▁▁▁▁ 8d   <- high DOI: deadline pressure
          ⋯ 2 steps
  May 01  ▸ Launch preparation               #38 →4  ▁▁▁▂ 14d  <- high DOI: last step
```

---

### 4. Urgency Contour Spacing

**Source principles:** Topographic contour lines, isochrone maps, gradient-as-spacing

**The idea:** In the route and console zones, the vertical spacing between steps encodes temporal pressure. Steps with tight implied execution windows (little time between predecessor deadline and own deadline) are rendered with no blank lines between them — packed tight. Steps with breathing room get an empty line of separation. The density of the display *is* the urgency field.

**What it reveals:** Temporal pressure zones without explicit annotation. A visually "compressed" section of the route means "everything here is time-pressed." A visually "spacious" section means "there's room." The user feels the pressure through spatial density, the way a hiker feels slope through contour line spacing.

**Terminal implementation:** Zero, one, or two blank lines between consecutive route steps, determined by the ratio of implied execution window to step complexity. No additional characters needed — the whitespace is the data.

---

### 5. Horizon-Style Urgency Shading

**Source principles:** Horizon charts, heatmaps, choropleth maps, signal-by-exception

**The idea:** Instead of annotating urgency as text ("OVERDUE", "3d to deadline"), encode it as background color intensity or foreground dimming on a per-step basis. Steps in the comfortable zone: default dim foreground. Steps entering the deadline window: normal brightness. Steps approaching deadline: bright/bold. Steps overdue: accent color (red or configured warn color). The entire route becomes a gradient from dim (distant) to bright (urgent) to colored (overdue).

**What it reveals:** The urgency landscape at a glance. The eye is drawn to the bright/colored items without needing to read any text. This is signal-by-exception implemented through a continuous visual channel rather than a discrete text label.

**Terminal implementation:** ANSI color codes for foreground intensity. Four levels: dim (8-color: dark gray), normal (default), bold (bright), accent (red/configured). No additional characters — the existing text just changes color. Works within the current monochrome-plus-one-accent palette by using the accent for overdue and brightness gradations for approach.

---

### 6. Epoch Strata in the Log View

**Source principles:** Stratigraphic columns, geological cross-sections, bottom-up time, unconformity

**The idea:** The log view (navigated to via `↓`) shows epoch history as a stratigraphic column. Each epoch is a horizontal band. Band height is proportional to epoch duration or activity density. Color/shading encodes epoch type: productive epochs (many resolutions) are one shade, pivot epochs (desire changed) are another, stall epochs (long duration, few gestures) are dim. Unconformities (gaps between epochs) are marked with a distinct separator.

**What it reveals:** The structural history of a tension's evolution. Was it a steady series of productive epochs? A long stall followed by a burst? A series of pivots searching for the right desire? The pattern of epoch types *is* the pattern of the user's relationship with the tension.

**Terminal implementation:**

```
  Epoch 4  ████████████████████ desire: "Ship v2.0"   12d, 8 gestures
           ── unconformity: 9d silence ──
  Epoch 3  ████████             desire: "Ship v2.0"   5d, 3 gestures
  Epoch 2  ██████████████████   desire: "Refactor"    10d, 12 gestures
  Epoch 1  ████                 desire: "Prototype"   3d, 6 gestures
```

The band width (number of █ characters) encodes activity density. The visual pattern is immediately diagnostic.

---

### 7. Overlay-Mode Information Layers

**Source principles:** Military situation map overlays, head-up display, cartographic overlays

**The idea:** The deck view has a base representation (current default: text + glyphs + deadline + age). Additional information layers toggle on/off with keybindings, adding data to the *same spatial positions* without rearranging the display:

- `d` — **Dependency layer:** shows which steps depend on others via inline indicators (`←#31`, `→#35`)
- `t` — **Trace layer:** adds sparkline micro-histories to each step
- `u` — **Urgency layer:** adds urgency fraction as colored bar or number
- `e` — **Epoch layer:** shows which epoch each step belongs to, with boundaries marked

**What it reveals:** Each layer answers a different question about the same structure. The spatial stability means the user's mental map of step positions is never disrupted — only enriched.

**Terminal implementation:** Each layer replaces or augments the right-column content (where ID, children count, and age currently live). Since these right-column items are already variable, replacing them contextually is natural. Or: overlay data appears *below* each step as a second line, shown only when the layer is active.

---

### 8. Phase Portrait Micro-Glyph

**Source principles:** Phase portraits, state space diagrams, qualitative dynamics classification

**The idea:** For each tension, compute a 2D micro-characterization: x = reality progress rate (are steps getting resolved?), y = desire stability (has the desire changed recently?). Plot this as a single composite glyph that encodes the *qualitative character* of the tension's current dynamics:

- `→` Executing (reality advancing, desire stable)
- `↑` Re-envisioning (desire shifting, reality paused)
- `↗` Converging (both advancing — approaching closure)
- `↺` Oscillating (reality advancing then retreating — rework)
- `◯` Equilibrium (nothing moving — either complete or stalled)
- `⇉` Accelerating (increasing resolution rate)

**What it reveals:** The *type* of engagement, not just the state. A tension marked `↺` needs attention not because it's overdue but because it's in a rework loop. A tension marked `↑` isn't stuck — it's reconsidering its aim. The glyph tells you *what kind of thing is happening.*

**Terminal implementation:** A single 1-2 character glyph placed before or after the tension name, computed from recent gesture history. Updates automatically as new gestures occur.

---

### 9. Interlinear Multi-Resolution Steps

**Source principles:** Interlinear glossing, exploded views, Labanotation staff

**The idea:** Each step in the route/console can render at three resolutions depending on zoom and DOI:

**Compressed (1 line, ~40 chars):** glyph + truncated text + urgency indicator
```
  ▸ Draft proposal...        ▃▅▇
```

**Normal (1 line, full width):** deadline + glyph + full text + right-column metadata
```
  Mar 28  ▸ Draft proposal for review        #42 →3  2d
```

**Expanded (3 lines, interlinear):** intent + status + trace
```
  Mar 28  ▸ Draft proposal for review        #42 →3  2d
           urgency: 0.72  critical path  epoch 4
           ▁▂▃▄▅▆▇█▇▆ created 14d ago, 8 gestures
```

The expansion is driven by cursor proximity (fisheye) or explicit focus (Enter). The three lines are always vertically aligned, forming a "gloss" of the step from three perspectives.

**What it reveals:** Detail on demand without navigation. The user doesn't leave the current view to get more information — the information unfolds at the point of attention.

---

### 10. Alluvial Flow for Theory-of-Closure Evolution

**Source principles:** Alluvial diagrams, Sankey diagrams, Minard's flow

**The idea:** In the log view or a dedicated "theory evolution" view, show how the theory of closure has changed across epochs. Each epoch is a vertical column. Steps that persist from one epoch to the next are connected by flow lines. Steps that are added appear as new branches. Steps that are resolved disappear (their flow width drops to zero). Steps that are removed or released have their flow diverted.

**What it reveals:** The stability and evolution of the user's theory. A theory that remains mostly the same across epochs (many persistent flow lines) is stable. A theory that completely restructures each epoch (few persistent lines) is volatile. This visualizes "does my theory of closure still make sense?" over time.

**Terminal implementation:** This is the most challenging to render in a terminal, but a simplified version works:

```
  Epoch 1    Epoch 2    Epoch 3    Epoch 4
  ─Step A───►Step A───►Step A───►✓
  ─Step B───►Step B──┘
  ─Step C───►Step C───►Step C───►Step C──►
             ─Step D───►Step D──┘
                        ─Step E───►Step E──►
```

Using `─`, `►`, `┘`, `┐`, `│` to show persistence, resolution, and removal. The visual pattern shows theory stability at a glance.

---

## Part IV: Principles That Didn't Make the Top 10 But Should Inform Design

### The Rest (from music notation)
Silence should be notated. A tension where nothing is happening should be visually distinguishable from a tension that doesn't exist. The absence of signal is itself a signal — but only if the notation acknowledges it. Consider a "quiescent" glyph that marks intentional non-activity.

### The Unconformity (from geology)
Gaps in the record are information. When no gestures occur for an extended period, that gap should be *visible* — not smoothed over. The log view should show unconformities as distinct markers, not elide them.

### The Light Cone (from physics)
From any given step, only some future states are reachable given the dependency and deadline structure. Steps outside the "light cone" of the current position are structurally inaccessible. This could inform route display: steps that are reachable from here vs. steps that require prerequisites not yet met.

### The Reinforcing Loop (from systems dynamics)
When progress on tension A creates conditions for progress on tension B, and vice versa, that's a leverage point. The instrument could detect these from dependency structure and highlight them — not as interpretation but as structural fact.

### The Weather Station Model (from meteorology)
A single compact multi-dimensional glyph at each tension in the survey view: combining status, urgency, activity level, and child complexity into one 3-4 character symbol. Requires a legend and training, but maximizes information density per character cell.

### The Ahnentafel Principle (from genealogy)
Structure determines address. A step's position in the hierarchy is already informative. The numbering scheme (1.1, 1.2, 1.2.1) could be made more visible as a navigation aid, using indentation and ordinals to convey depth and sequence simultaneously.

---

## Part V: Terminal-Specific Technical Palette

These are the character-level building blocks available for implementing the above ideas:

### Block elements for micro-charts
`▁▂▃▄▅▆▇█` — 8-level vertical bar (sparklines)
`▏▎▍▌▋▊▉█` — 8-level horizontal fill (progress bars, width encoding)
`░▒▓█` — 4-level density fill (background intensity)

### Braille patterns for high-resolution mini-plots
`⠁⠂⠃⠄⠅⠆⠇⡀⡁⡂⡃⡄⡅⡆⡇` — 256 Braille patterns enable 2x4 dot-matrix per character cell, allowing mini scatter plots or fine-grained sparklines at effectively double horizontal and quadruple vertical resolution.

### Box-drawing for structure
`─│┌┐└┘├┤┬┴┼` — tree structure, borders, separators
`═║╔╗╚╝╠╣╦╩╬` — heavy variants for emphasis
`┄┆┈┊` — dashed variants for ephemeral/optional

### Arrows and flow
`→←↑↓↗↘↙↖⟶⟵⟷` — direction indicators
`▸▹▾▿◂◃` — small directional triangles
`►◄▲▼` — heavy directional triangles

### ANSI color as information channel
- **Foreground brightness** (4 levels: dim, normal, bold, bright) — encodes urgency/recency
- **Accent color** (1, configurable) — encodes exception/alert
- **Background shade** (subtle) — encodes zone membership
- **Underline/strikethrough** — encodes secondary states
- **Inverse** — encodes cursor/selection

### Spatial whitespace as data
- Line spacing (0, 1, or 2 blank lines between items) — encodes temporal pressure
- Indentation depth — encodes hierarchy
- Right-margin position — encodes metadata column
- Centering vs. left-alignment — encodes structural role (desire centered, steps left-aligned)

---

## Sources

- [Analyzing Minard's Visualization](https://thoughtbot.com/blog/analyzing-minards-visualization-of-napoleons-1812-march)
- [Minard Map - Big Think](https://bigthink.com/strange-maps/229-vital-statistics-of-a-deadly-campaign-the-minard-map/)
- [Marey Charts for Analyzing Work Flow](https://www.3cs.ch/analyze-work-flow-with-marey-chart/)
- [Visualizing Metra - Nicholas Rougeux](https://www.c82.net/blog/?id=66)
- [Hyperbolic Tree Focus+Context - Lamping & Rao](http://prior.sigchi.org/chi95/Electronic/documnts/papers/jl_bdy.htm)
- [Sizing the Horizon - Heer, Kong, Agrawala](https://idl.uw.edu/papers/horizon)
- [Labanotation Fundamentals - Dance Notation Bureau](https://www.dancenotation.org/labanotation-fundamentals/)
- [Labanotation - Wikipedia](https://en.wikipedia.org/wiki/Labanotation)
- [Sankey and Alluvial Diagrams - EU Data Viz Guide](https://data.europa.eu/apps/data-visualisation-guide/sankey-and-alluvial-diagrams)
- [Effective Visualization of Hierarchies - Bamberg](https://vis-uni-bamberg.github.io/hierarchy-vis/)
- [Flame Graphs vs Treemaps vs Sunburst - Brendan Gregg](https://www.brendangregg.com/blog/2017-02-06/flamegraphs-vs-treemaps-vs-sunburst.html)
- [Flame Graphs - Brendan Gregg](https://www.brendangregg.com/flamegraphs.html)
- [Flame Charts: Time-Aware Sibling - Polar Signals](https://www.polarsignals.com/blog/posts/2025/05/28/flamecharts-the-time-aware-sibling-of-flame-graphs)
- [Phase Portraits - PySD Cookbook](https://pysd-cookbook.readthedocs.io/en/latest/analyses/visualization/phase_portraits.html)
- [Hive Plots - Krzywinski](https://hiveplot.com/)
- [Hive Plots - Mike Bostock](https://bost.ocks.org/mike/hive/)
- [Isochrone Map - Wikipedia](https://en.wikipedia.org/wiki/Isochrone_map)
- [Causal Loop Diagrams - 6Sigma](https://www.6sigma.us/systems-thinking/causal-loop-diagram-in-systems-thinking/)
- [Stock and Flow Diagrams - Creately](https://creately.com/guides/stock-and-flow-diagram/)
- [Degree-of-Interest Trees - Card & Nation](https://davenation.com/doitree/doitree-avi-2002.htm)
- [Semantic Zoom - EmergentMind](https://www.emergentmind.com/topics/semantic-zoom)
- [Sparkline Theory and Practice - Tufte](https://www.edwardtufte.com/notebook/sparkline-theory-and-practice-edward-tufte/)
- [Tufte's Principles](https://thedoublethink.com/tuftes-principles-for-visualizing-quantitative-information/)
- [Feynman Diagrams - MIT Press](https://direct.mit.edu/posc/article/26/4/423/15455/How-Do-Feynman-Diagrams-Work)
- [Bifurcation Diagram - Wikipedia](https://en.wikipedia.org/wiki/Bifurcation_diagram)
- [Chord Diagram - Data to Viz](https://www.data-to-viz.com/graph/chord.html)
- [Stratigraphic Column - Wikipedia](https://en.wikipedia.org/wiki/Stratigraphic_column)
- [Temporal Mapping Visualization - Map Library](https://www.maplibrary.org/1582/data-visualization-techniques-for-temporal-mapping/)
- [Sparklines for Terminal - deeplook](https://github.com/deeplook/sparklines)
- [Parallel Coordinates - Wikipedia](https://en.wikipedia.org/wiki/Parallel_coordinates)
- [Chord Diagram - Wikipedia](https://en.wikipedia.org/wiki/Chord_diagram_(information_visualization))
- [Military Map Overlays](https://www.globalsecurity.org/military/library/policy/army/fm/3-25-26/ch7.htm)
