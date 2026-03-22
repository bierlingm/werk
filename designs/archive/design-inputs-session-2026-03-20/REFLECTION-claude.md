# Design System Reflection
## Lessons from Building the Operative Instrument TUI Design

---

## I. What Was Easy and What Was Hard

### Easy Wins

**Excellent Foundation Work**: The existing design documents were exceptional quality. The "Operative Instrument" interaction design doc provided a clear vision and philosophy. The current implementation showed ftui being used effectively in practice. The domain model in sd-core was well-structured with clear types and relationships.

**Visual Grammar Inheritance**: Most of the visual system was already established - glyphs for phases, temporal indicators, color semantics, the six-color palette. I could build on proven foundations rather than inventing from scratch.

**Basic Widget Mapping**: Straightforward mappings were obvious - tension lines become `Paragraph` widgets, cards become `Panel` widgets, input fields become `TextInput`. The core widget vocabulary was sufficient.

**Spatial Metaphor Clarity**: The "reality = ground / desire = sky" principle was well-defined and provided clear direction for every layout decision. Having this fundamental spatial law made many design choices obvious.

### Hard Challenges

**Implementation Gap**: Bridging the gap between high-level design vision ("an instrument that disappears during use") and concrete implementation details ("what exact ftui widgets with what configuration") required significant interpretation and assumption-making.

**Widget Capability Uncertainty**: I had to infer ftui's capabilities from usage examples rather than complete API documentation. Questions like "Can Panel contain TextInput widgets?" or "What are Grid's exact constraint options?" required educated guessing.

**Responsive Complexity**: Handling graceful degradation across terminal widths while preserving the spatial metaphor proved surprisingly complex. The reality=ground/desire=sky law needs to hold whether the terminal is 60 or 150 columns wide.

**Progressive Disclosure Engineering**: Ensuring the three depth layers (scanning, gaze, analysis) work together seamlessly while maintaining performance and consistent navigation patterns required careful state management design.

**Edge Case Proliferation**: Every feature spawned edge cases - What happens during gaze when parent changes? How does cursor behave during mode transitions? What if dynamics computation fails? The combinatorial complexity grew quickly.

---

## II. Ambiguities That Forced Assumptions

### Widget Architecture Assumptions

**Panel Composition**: I assumed `Panel` widgets can contain other widgets like `TextInput` for inline editing. This may not be true - ftui might require different composition patterns.

**Layout Constraints**: I designed around assumed Grid and Flex constraint behaviors. The exact constraint resolution algorithm and priority handling might differ from my expectations.

**Event Handling**: I assumed keyboard events can be routed differently based on current navigation mode. The actual event system might be more restrictive.

### Performance Characteristics Unknowns

**Dynamics Computation Cost**: How expensive is `compute_full_dynamics_for_tension()`? Should it be cached aggressively, computed on-demand, or pre-computed for visible items? I assumed moderate cost requiring smart caching.

**Rendering Performance**: What's the cost of complex Panel layouts versus simple Paragraph rendering? I assumed Paragraph is cheaper and used it for high-frequency updates like cursor movement.

**Memory Management**: How much state should be held in memory versus recomputed? I assumed a middle ground with targeted caching for expensive operations.

### Terminal Capability Questions

**Unicode Support**: Can I assume full Unicode support including the specific glyphs (◇◆◈◉)? I provided ASCII fallbacks but had to guess at the right detection strategy.

**Color Degradation**: How should the six-color palette degrade on terminals with limited color support? I assumed a specific degradation hierarchy but this needs validation.

**Extreme Sizes**: What's the minimum viable terminal size? I assumed 40x5 as absolute minimum but this is untested.

### State Management Patterns

**Navigation Preservation**: Should cursor position be preserved when switching views? Should gaze state survive mode changes? I made consistency assumptions that need validation.

**Error Recovery**: How should the system recover from dynamics computation failures or store corruption? I assumed graceful degradation patterns that might not match user expectations.

---

## III. Core Implementation Decision Hierarchy

### 1. Foundation Layer (Prerequisites for Everything)

**Widget Inventory Audit** ← *Critical First Step*
- Catalog every available ftui widget
- Test composition capabilities (Panel + TextInput, etc.)
- Document constraints, styling options, event handling
- Build reference implementation for each widget

**Terminal Capability Detection**
- Color support detection (TrueColor → ANSI256 → Basic)
- Unicode glyph availability testing
- Size constraint validation
- Performance characteristics on different terminals

**Basic Rendering Pipeline**
- Rect-slicing architecture implementation
- Element dispatch system
- Style application and theme management
- Scroll offset and viewport management

### 2. Core Navigation (Enables All User Interaction)

**State Machine Implementation**
- Navigation mode definitions and transitions
- Input event routing based on current mode
- State preservation rules across mode changes
- Error recovery and invalid state handling

**Virtual List Foundation**
- Dynamic height calculation per element
- Efficient scrolling with large item counts
- Cursor visibility management
- Selection state preservation

### 3. Basic Display (Minimal Viable Product)

**Tension Line Rendering**
- Complete annotation system (horizon, temporal indicator)
- Word-wrapping for selected items
- Truncation strategies for narrow terminals
- Responsive layout adaptation

**Field View Structure**
- Proper vertical element stacking
- Header/footer rendering in descended view
- Rule and separator display
- Alert placement and numbering

### 4. Progressive Disclosure (Core Value Proposition)

**Gaze Card Implementation**
- Panel-based expansion inline
- Children preview with positioning indication
- Reality section display
- Toggle between quick and full gaze

**Depth Layer Coordination**
- Smooth transitions between depths
- State preservation during depth changes
- Navigation consistency across depths
- Information hierarchy maintenance

### 5. Interaction Features (User Actions)

**Inline Editing System**
- Panel + TextInput coordination
- Tab cycling between fields
- Input validation and feedback
- Cancel/confirm patterns

**Creation Workflow**
- Inline card creation at cursor position
- Multi-field input flow
- Smart defaults and suggestions
- Error handling and validation

**Alert Response System**
- Number key to alert action mapping
- Direct action execution
- Feedback and confirmation
- Alert state updates

### 6. Advanced Features (Sophistication Layer)

**Agent Integration**
- Session mode visual transformation
- Context display for agent consumption
- Mutation proposal card system
- Apply/reject/modify workflows

**Reordering System**
- Grab-and-drop visual feedback
- Position boundary enforcement
- Undo/cancel capabilities
- Smooth animation or transitions

### 7. Polish Layer (Professional Quality)

**Responsive Behavior**
- Breakpoint-based layout switching
- Graceful feature degradation
- Content prioritization strategies
- Minimum viable layouts

**Performance Optimization**
- Smart caching strategies
- Efficient re-rendering
- Background computation
- Memory usage optimization

---

## IV. Recommendations to Harden Further Work

### 1. Widget API Deep Dive (Highest Priority)

**Create Definitive Widget Inventory**: Build test programs for every ftui widget to understand exact capabilities, limitations, and edge cases. Document composition rules, styling options, and event handling patterns.

**Prototype Critical Interactions**: Build minimal implementations of the hardest features first:
- Gaze card with dynamic Panel content
- Inline editing with nested TextInput widgets
- Agent session with proposal cards
- Virtual scrolling with heterogeneous heights

### 2. Terminal Reality Testing

**Cross-Platform Validation**: Test on representative terminals across platforms:
- macOS: iTerm2, Terminal.app, Alacritty
- Linux: GNOME Terminal, Konsole, xterm
- Windows: Windows Terminal, ConEmu
- SSH contexts with varying capabilities

**Capability Detection Refinement**: Build robust detection for:
- True color vs 256-color vs 16-color support
- Unicode glyph availability (test specific chars: ◇◆◈◉●◦◌◎)
- Terminal size extremes and resize behavior
- Keyboard event support variations

### 3. Performance Baseline Establishment

**Dynamics Computation Profiling**: Measure actual costs:
- Time to compute full dynamics for 1, 10, 100, 1000 tensions
- Memory usage patterns during intensive computation
- Cache hit ratios and effectiveness
- Background vs foreground computation tradeoffs

**Rendering Performance Analysis**: Profile rendering costs:
- Simple Paragraph vs complex Panel layouts
- Scroll performance with large lists
- Style application and color computation overhead
- Frame rate under heavy interaction

### 4. User Validation Early and Often

**Core Navigation Testing**: Get basic field navigation + gaze working and test with real users immediately. Validate:
- Spatial metaphor intuition (do users understand reality=ground/desire=sky?)
- Progressive disclosure effectiveness (do the three depths make sense?)
- Navigation flow and muscle memory development
- Cognitive load and learning curve

**Workflow Integration Testing**: Test the TUI alongside CLI and agent tools to ensure the multi-tool experience is seamless, not fragmented.

### 5. State Management Architecture

**Define Clean Patterns**: Establish consistent approaches to:
- Navigation state preservation across mode transitions
- Undo/redo for all user actions (not just mutations)
- Error recovery without losing user context
- Graceful handling of external data changes

**Build State Management Primitives**: Create reusable patterns for common scenarios rather than ad-hoc state handling throughout the codebase.

### 6. Integration Hardening

**CLI/TUI Coordination**: Ensure the TUI can handle:
- External mutations from CLI commands
- File system watching and live updates
- Concurrent access patterns
- Data consistency during multi-tool workflows

**Agent System Integration**: Validate the agent session design with actual LLM interactions:
- Context size and token usage patterns
- Response parsing reliability
- Error handling for malformed agent responses
- Session state management and recovery

### 7. Documentation and Knowledge Transfer

**Decision Documentation**: Record the reasoning behind major design decisions, especially where multiple approaches were possible. Future maintainers need to understand *why* choices were made.

**Implementation Patterns**: Document the established patterns for common tasks (adding new widgets, extending the navigation system, handling new dynamics types) so the codebase can grow consistently.

**User Mental Model Documentation**: Describe the intended user understanding of the spatial metaphor, progressive disclosure, and interaction patterns. This guides both implementation decisions and user onboarding design.

---

## V. Final Thoughts

Creating this design system highlighted the tension between design vision and implementation reality. The existing design work provided excellent north stars, but translating philosophical principles ("the instrument disappears during use") into concrete widget configurations required significant interpretation.

The most critical insight: **the spatial metaphor must be consistent at the implementation level, not just the conceptual level**. Every widget placement, animation direction, and layout decision either reinforces or undermines the reality=ground/desire=sky principle. This consistency is what transforms a collection of terminal widgets into a coherent instrument.

The progressive disclosure architecture is both the system's greatest strength and its greatest implementation challenge. Done well, it creates the "instrument that grows with you" experience. Done poorly, it becomes confusing mode soup. The key is ensuring that each depth layer adds value without invalidating the previous layer's mental model.

Success will be measured not by feature completeness but by whether the instrument truly disappears during use—whether a practitioner can engage with their structural dynamics through computation as naturally as a pianist engages with music through keys.

---

*This reflection captures the state of understanding at design completion. It should be revisited and updated as implementation reveals new insights and challenges.*