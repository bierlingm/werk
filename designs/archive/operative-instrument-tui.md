# The Operative Instrument: Interaction Design Document

## I. Design Philosophy

The best TUIs share a quality with musical instruments: the interface disappears during use. A pianist doesn't think "press the key at position 43." They think a note and their hands produce it. The Operative Instrument must achieve this same transparency.

**Three principles:**

1. **The instrument is a mirror, not a dashboard.** Dashboards display data. Mirrors reveal something about the person looking. Every view should provoke recognition, not just inform.

2. **Density is earned, not given.** A blank terminal with a single tension is more powerful than a screen full of dynamics nobody asked about. Information appears when the user's attention reaches for it.

3. **Every interaction is an act.** Adding a tension, resolving one, invoking the agent — these are not CRUD operations. They are deliberate gestures. The interface should give each gesture weight proportional to its meaning.

---

## II. The Primary View: The Field

When you open the instrument, you see **The Field** — a vertical list of your root-level tensions, rendered as single lines. Nothing else. No header. No sidebar. No status bar (yet). Just your tensions, breathing in your terminal.

```
  ◇ Write the novel                                    ○○○●
  ◆ Fix relationship with brother                       ○●
  ◇ Build the company                                   ○○○○○●
  ◇ Learn to rest                                       ●
  ◈ Get the apartment sorted                            ○○
```

**What you see:**

- **The glyph** (leftmost) encodes the phase without labels:
  - `◇` — Germination (open, new, still forming)
  - `◆` — Assimilation (solid, being worked on, actively digested)
  - `◈` — Completion (textured, complex, nearing resolution)
  - `◉` — Momentum (full, radiating, on fire)

- **The name** — written by the user, in their own language. This is the most important element on screen. It gets the most space.

- **The trail** (rightmost) — a sequence of `○` and `●` dots showing recent mutation activity. Each dot is a time unit (week by default). Filled means something changed. This is **neglect** and **tendency** made visible without a single label. A long trail of `○○○○○` is damning. A pattern of `●○●○●` shows oscillation. Solid `●●●●●` shows momentum. The eye learns to read these in days.

**What you do NOT see:** dynamics labels, percentages, progress bars, timestamps, IDs, tree structure, magnitude numbers, conflict indicators. All of those exist. None of them belong on the first screen.

**Why this works:** The Field is a confrontation. You open the instrument and you see your life's tensions named and pulsing (or not pulsing). The trails are a silent accusation or a silent encouragement. You don't need to understand "dynamics" to feel the weight of five empty circles next to "Learn to rest."

---

## III. Navigation Model: Descent

The spatial metaphor is **depth, not breadth.** You are always looking at one level of a tree. You descend into a tension to see its children. You ascend to see its parent's siblings. The entire navigation model is four motions:

| Key | Action | Metaphor |
|-----|--------|----------|
| `j/k` or `↑/↓` | Move between siblings | Scanning the field |
| `l` or `Enter` or `→` | Descend into selected tension | Going deeper |
| `h` or `Backspace` or `←` | Ascend to parent level | Pulling back |
| `/` | Search across all tensions | Cutting across |

**That's it.** Four directions. The entire forest is navigable with these four motions. There is no "tree view" that shows everything at once — that would be a map, and a map is not the territory. You are always *in* a specific place in your tension forest, and you see only what's around you.

When you descend into "Write the novel," you see:

```
  Write the novel                                       ◇
  ─────────────────────────────────────────────────────────

  ◆ Finish the first draft                              ○●●●
  ◇ Find an agent                                       ○○○
  ◇ Research the setting                                ●●
  ◈ Resolve the ending                                  ○●○●
```

The parent sits at the top as a header, separated by a thin rule. Its children are below. The same visual language applies. You can descend further into any child.

---

## IV. The Lever: Context Line

At the **bottom** of the terminal, a single line tells you where you are. This is the Lever — your grip on the instrument.

```
◇ Write the novel › ◆ Finish the first draft                          3 of 4
```

It shows your path (breadcrumb) and your position among siblings. That's all. When you're at the root level (The Field), the Lever shows:

```
The Field                                                              5 tensions
```

The Lever is **always visible** and is the only persistent chrome. It occupies exactly one line. It uses dim/muted color so it doesn't compete with content.

---

## V. Progressive Disclosure: The Three Depths

Information about a tension is revealed in three depths. The user controls which depth they're seeing.

### Depth 0: The Line (default)
What you see in The Field or any sibling list. Name, glyph, trail. One line per tension.

### Depth 1: The Gaze (press `Space` on any tension)
An expanded inline view appears below the selected tension, pushing siblings down. This shows the tension's **current state** without leaving context:

```
  ◇ Write the novel                                    ○○○●
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  desire    A completed novel I'm proud of, published
  reality   42,000 words. Stuck on the third act.

  children  4 (1 advancing, 2 stagnant, 1 oscillating)
  gap       ████████░░░░░░░░                           large
  conflict  with "Learn to rest"
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  ◆ Fix relationship with brother                       ○●
```

**Depth 1 reveals:**
- The desire and reality statements (the user's own words — the most important data)
- A one-line children summary (how many, what's their tendency)
- The gap/magnitude as a bar (visual, not numeric)
- Any conflicts (which other tensions pull against this one)

This is where dynamics start to surface — but only the ones that matter. No dynamic is shown if it's in a "normal" or uninteresting state. Conflict only appears if there is one. Gap only appears because it's always relevant. The children summary uses tendency words (advancing/stagnant/oscillating) because those are the actionable signals.

**Press `Space` again to collapse.** The Gaze is a toggle. It respects the user's attention.

### Depth 2: The Study (press `Enter` then `Tab`)
After descending into a tension (so it's the header), pressing `Tab` switches from seeing children to seeing **the tension's full dynamics and history.** This is a dedicated analytical view:

```
  Write the novel                                       ◇
  ═══════════════════════════════════════════════════════════

  DESIRE    A completed novel I'm proud of, published
  REALITY   42,000 words. Stuck on the third act.

  ───── dynamics ─────

  phase         germination         still forming, not yet solid
  tendency      oscillating         attention comes and goes
  magnitude     large               significant gap between desire and reality
  orientation   creative            generative, not reactive
  conflict      with "Learn to rest"    zero-sum on time and energy
  neglect       moderate            3 of 4 children untouched in 2+ weeks
  momentum      low                 few recent changes
  coherence     high                children align with parent intent
  volatility    moderate            desire has shifted twice
  dependency    none                no blocking external tensions
  maturity      young               created 3 weeks ago
  saturation    low                 few mutations relative to scope
  drift         notable             reality description hasn't updated in 18 days

  ───── history ─────

  Mar 14   reality updated          "42,000 words. Stuck on the third act."
  Mar 08   child resolved           "Outline the structure"
  Mar 01   desire updated           was: "A finished novel"
  Feb 22   child added              "Resolve the ending"
  Feb 15   created
```

All 13 dynamics. Full mutation history. No abbreviation. This is the view for someone who wants the complete picture. It scrolls if it needs to.

**The key insight:** most users will spend 90% of their time at Depth 0 and Depth 1. Depth 2 exists for the moments when you want to sit with a single tension and understand it fully. The fact that it exists doesn't mean it intrudes.

---

## VI. Interaction Patterns: Acts

Every modification is called an **act.** The language matters — these are not edits, they are deliberate gestures.

### Adding a tension: `a`
Pressing `a` anywhere opens a minimal inline prompt at the cursor position:

```
  ◇ Write the novel                                    ○○○●
  ◆ Fix relationship with brother                       ○●
  │
  └─ new tension: _

  ◇ Build the company                                   ○○○○○●
```

You type the name and press Enter. That's all that's required. The tension exists.

Immediately after creation, a second prompt appears:

```
  └─ desire: _
```

You can type the desire statement, or press `Esc` to skip (you can always add it later). If you type a desire, a third prompt:

```
  └─ reality: _
```

Same — type or skip. This three-beat rhythm (name → desire → reality) is the fundamental act of creation. It mirrors the concept: you name what matters, you articulate what you want, you name what is.

If you're inside a tension (viewing its children), `a` adds a child of that tension. If you're at The Field, `a` adds a root tension. Context determines scope.

### Editing: `e`
Press `e` on a selected tension. A minimal editor opens — just the desire and reality fields, inline. You edit in place. Press `Enter` to confirm, `Esc` to cancel.

```
  ◆ Finish the first draft                              ○●●●
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
  desire ▎ A completed first draft by end of April_
  reality▎ 42,000 words. Stuck on the third act.
  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
```

The cursor `▎` shows which field is active. `Tab` moves between fields. This is a micro-editor, not a modal form.

### Annotating: `n`
Press `n` to add a note (a mutation that is just text, a journal entry for that tension). A single-line or multi-line input appears. Notes are visible in the history at Depth 2. They are the primary way the user talks to themselves across time.

### Resolving: `r`
Press `r` on a selected tension. A confirmation appears:

```
  ◈ Resolve the ending                                  ○●○●

      resolve this tension?
      this marks the gap as closed — desire met reality.

      (y)es   (n)o
```

If confirmed, the tension's glyph changes to `✦` and it dims. It remains visible in its position but is clearly finished. It doesn't disappear — resolved tensions are achievements, not garbage. They fade but persist.

### Releasing: `x`
Press `x` to release a tension — to consciously let it go. Different from resolving: this is surrender, not achievement. The confirmation says:

```
      release this tension?
      this lets it go — acknowledging the gap without closing it.

      (y)es   (n)o
```

Released tensions show as `·` (a small dot) and are the dimmest elements. They're still in the history. The act of releasing is important and the instrument should make you feel the weight of it.

### Moving / Reparenting: `m`
Press `m` to move a tension. It enters a "placing" mode where you navigate the forest and press `Enter` to drop it as a child of whatever tension is selected (or at the root level). The Lever changes to:

```
placing: "Resolve the ending" → select new parent (Enter to place, Esc to cancel)
```

---

## VII. Visual Language

### Color Palette

The instrument uses a **restrained** palette. Not every dynamic gets a color. Colors are reserved for states that demand attention.

| Color | Meaning | Where it appears |
|-------|---------|-----------------|
| **White/default** | Normal, active tensions | Names, desire/reality text |
| **Dim/gray** | Resolved, released, or stable | Resolved tension names, stable dynamics |
| **Amber/yellow** | Stagnation or neglect | Trail dots turn amber after prolonged inactivity; neglect warnings |
| **Red** | Conflict | Conflict indicators, competing tension names when referenced |
| **Cyan** | Agent session active | The agent prompt, suggested mutations |
| **Green** | Recent positive change | Briefly flashes on a trail dot when something advances |

No gradients. No backgrounds. No bold except for the selected line's name. The palette should feel like ink on paper, not like a SaaS dashboard.

### Symbols

The full symbol vocabulary:

| Symbol | Meaning |
|--------|---------|
| `◇` | Phase: Germination |
| `◆` | Phase: Assimilation |
| `◈` | Phase: Completion |
| `◉` | Phase: Momentum |
| `✦` | Resolved |
| `·` | Released |
| `●` | Trail: active (mutation occurred) |
| `○` | Trail: quiet (no mutation) |
| `█░` | Gap bar |
| `⚡` | Conflict marker (used sparingly in Depth 1/2) |
| `┄` | Light separator |
| `─` | Medium separator |
| `═` | Heavy separator (Depth 2 only) |

### Density and Whitespace

**Rule: The left margin is sacred.** Tension names always start at the same column (column 3, after the glyph and a space). Child views indent by 0 — they are not indented, because you have already descended. The parent is the header. Indentation is a crutch for showing tree structure on one screen; this design uses *navigation* instead of indentation.

**Rule: One tension per line, always.** No wrapping. If a name is too long, it truncates with `…`. The user can see the full name in Depth 1 or Depth 2. The line is a unit of attention.

**Rule: Blank lines are semantic.** A blank line between tensions means they're in different clusters (if clustering is ever added). No blank lines between siblings means they're peers in the same space.

---

## VIII. The Agent Session

This is the most distinctive interaction pattern. The agent is not a chatbot floating in a sidebar. It is **invoked within the scope of a single tension** and its session has a formal structure: opening, dialogue, and a closing act.

### Invocation: `@`

Press `@` on any tension. The entire screen transforms. The siblings and chrome fade. The selected tension becomes the center of attention:

```
                    ◆ Finish the first draft

     desire    A completed first draft by end of April
     reality   42,000 words. Stuck on the third act.

     4 children · oscillating · large gap · 3 weeks old

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  agent session                                    @ to end

  ▎ _
```

The top half is the tension's context, presented cleanly for the agent to see and for the user to reference. The bottom half is the conversation space.

The user types naturally. The agent responds. But the agent's responses are not just text — they can include **proposed mutations**, rendered as actionable cards:

```
  You've been oscillating on this for three weeks. The third act
  problem might be upstream — your desire shifted from "a finished
  novel" to "a novel I'm proud of." That's a different standard.

  I'd suggest:

  ┌─ proposed ──────────────────────────────────────────────┐
  │ update desire                                           │
  │ "A completed first draft — imperfect is fine"           │
  │                                          (y)es  (n)o   │
  └─────────────────────────────────────────────────────────┘

  ┌─ proposed ──────────────────────────────────────────────┐
  │ add child                                               │
  │ "Write the bad version of act three"                    │
  │                                          (y)es  (n)o   │
  └─────────────────────────────────────────────────────────┘

  ┌─ proposed ──────────────────────────────────────────────┐
  │ update reality                                          │
  │ "42,000 words. Act three drafted but not trusted yet."  │
  │                                          (y)es  (n)o   │
  └─────────────────────────────────────────────────────────┘
```

Each proposed mutation is a discrete card the user can accept or reject. The agent cannot make changes without explicit acceptance. This is crucial — the instrument protects the user's sovereignty over their own tensions.

### Closing: `@` again

When the session ends (user presses `@` again, or types "done"), the agent session closes with a summary:

```
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  session closed

  accepted    update desire, add child "Write the bad version..."
  declined    update reality

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The screen transitions back to the normal view. The tension's dynamics have updated to reflect the accepted mutations. The trail has a new `●`.

**Why `@`?** It's mnemonic (invoke the **a**gent) and it's a single key. The symmetry of `@` to open and `@` to close creates a ritual bracket — the session is a bounded container, not an ambient presence.

---

## IX. The Top of Screen

Nothing. The Field starts at line 1. There is no header, no title bar, no menu bar, no breadcrumb at the top. The Lever at the bottom is the only chrome.

**Why:** Every line at the top is a line stolen from content. The top of the terminal is the most valuable real estate. htop puts content there. vim puts content there. The instruments that waste the top on chrome (many Electron apps, many TUIs that imitate GUIs) feel bloated.

The application name never appears on screen. If you need to know what program you're running, you're not using it yet.

---

## X. Keyboard Map (Complete)

The full keymap fits on one screen. If it doesn't fit on one screen, there are too many commands.

```
 NAVIGATION                    ACTS                       VIEWS

 j/k  ↑/↓   move up/down      a   add tension             Space   gaze (expand/collapse)
 l  Enter →  descend            e   edit desire/reality     Tab     study (full dynamics)
 h  Bksp  ←  ascend             n   add note                /       search
                                r   resolve                 ?       show this keymap
 g          jump to top         x   release                 q       quit
 G          jump to bottom      m   move/reparent
                                @   agent session
                                u   undo last act
```

`?` shows this keymap as an overlay. It disappears on any keypress. A new user presses `?` once, learns the layout, and rarely needs it again. The keys are borrowed from vim (`j/k/g/G`), adapted for this domain (`r` for resolve, `x` for release, `@` for agent).

There is no command palette. There is no `:` mode. The instrument has ~15 actions and each one has a dedicated key. If you need a command palette, the design has too many commands.

---

## XI. Empty State: The First Encounter

When a user opens the instrument for the first time, with no tensions, they see:

```





                        nothing here yet.

                        press  a  to name what matters.





```

Centered. Quiet. No tutorial, no onboarding wizard, no "Welcome to The Operative Instrument!" The single instruction teaches the first gesture. After they press `a` and create their first tension, they have learned the core loop. The rest follows from `?` and exploration.

---

## XII. Search: `/`

Search is a flat list of all tensions across the entire forest, filtered by keystroke. It uses a fuzzy-match algorithm and renders results as they type:

```
  /nov_

  ◇ Write the novel                                     root
    ◆ Finish the first draft                            › Write the novel
    ◇ Resolve the ending                                › Write the novel
  ◇ November trip planning                               root
```

Each result shows the tension name and its parent path (dimmed, right-aligned). Pressing `Enter` on a result navigates directly to that tension (descends to its level, with it selected). `Esc` cancels.

Search is the escape hatch from the descent model. If you know what you want, you cut straight to it.

---

## XIII. Responsive Layout

The instrument works at any terminal width from 60 columns to 200+. The layout adapts:

- **60-80 columns**: Tension names truncate earlier. Trails shorten to 4 dots. Depth 1 stacks fields vertically.
- **80-120 columns**: The ideal range. Full names, 8-dot trails, Depth 1 fits comfortably.
- **120+ columns**: Extra space becomes whitespace. The content does NOT expand to fill — it stays centered or left-aligned with generous right margin. Wide terminals should feel spacious, not sprawling.

Height: The Field shows as many tensions as fit. Scrolling is implicit (the selection cursor moves and the viewport follows, like vim). There is no visible scrollbar.

---

## XIV. What This Design Rejects

Explicit rejections, because what you leave out defines a design as much as what you include:

1. **No tabs or panes.** You are in one place at a time. Split attention is the opposite of what this instrument is for.

2. **No progress percentages.** "42% complete" is a lie. The gap bar in Depth 1 is visual and approximate. The instrument never pretends to quantify the unquantifiable.

3. **No due dates or deadlines.** This is not a project manager. Tensions are alive or resolved or released. Time appears only in the trail and the mutation history.

4. **No colors for "good" and "bad."** Green does not mean good. Red does not mean bad. Amber means stagnant, which might be wise rest or might be avoidance — the user must decide. The instrument observes; it does not judge.

5. **No notifications or badges.** The instrument does not interrupt. You come to it when you're ready. The trail pattern is visible when you look; it doesn't pulse or alert.

6. **No customizable themes.** One palette. One look. The instrument has an aesthetic position and holds it. Theming is a concession that says "we don't know what this should look like." This design knows.

7. **No mouse support.** The keyboard is the instrument. A mouse invites pointing and clicking, which is browsing. This is not a browser. (If mouse support is added later, it should feel like a concession, not a feature.)

8. **No export or reporting.** The instrument is not for showing others. It is a mirror. You don't export your mirror.

---

## XV. A Session, End to End

To ground all of this, here is what using the instrument looks like in practice:

**Morning. Open the instrument.**

You see The Field. Five tensions. Two have amber trails (stagnant). One has a bright recent dot. You press `j` twice to select "Fix relationship with brother." You press `Space`.

The Gaze opens. Desire: "A relationship where we can talk honestly." Reality: "Haven't spoken in six weeks." The gap bar is large. No conflict. Two children: "Write the letter" (stagnant) and "Call on his birthday" (resolved last month).

You press `l` to descend. You see the two children. "Write the letter" has an all-empty trail. You select it and press `e`. You update reality: "Drafted something. Not sure it's right." Press Enter.

You press `@`. The agent session opens. You type: "I wrote a draft of this letter but I keep revising it. I think I'm afraid to send it."

The agent responds. It observes the oscillating pattern. It proposes: add a child tension "Send the letter without revising it again" and update desire to "An honest letter sent, not a perfect letter." You accept the first, think about the second, accept it too.

You press `@` to close the session. The session summary appears. You press any key. You're back in the children of "Fix relationship with brother." There's a new child. The trail for "Write the letter" has a fresh `●`.

You press `h` to return to The Field. You press `q` to quit.

**The whole interaction took 90 seconds.** You confronted something real, recorded a change, and received an observation that reframed your stuckness. Tomorrow the trail will tell you whether you followed through.

---

## XVI. Implementation Notes for the Builder

These are not design decisions but pragmatic notes for whoever builds this:

- **Rendering**: Use a reactive TUI framework (Ratatui in Rust, Bubbletea in Go, blessed/ink in JS). The transitions between views should be instant — no animation, no fade, just cut. Snappy is respectful of attention.

- **State**: The backing store is append-only mutations. Every screen is a computed view of the current state. There is no "save." Every act is immediately persisted. `u` for undo appends a reversal mutation, it does not delete.

- **The trail**: Compute from mutation timestamps. Configurable bucket size (default: 7 days). Show last 8 buckets. This is the single most important visual innovation — it makes time and attention visible in a way that numbers and labels cannot.

- **Agent integration**: The agent receives: the tension's desire, reality, children summary, dynamics, and recent mutation history. It returns structured proposals (JSON with mutation type + payload). The TUI renders these as the interactive cards described above. The agent never directly modifies state.

- **Startup time**: Under 50ms or the design fails. The instrument must open as fast as vim. Anything slower and the user will stop reaching for it.

---

This is the complete design. It is an instrument with one view (The Field), one navigation model (descent), three depths of information (line, gaze, study), a small set of deliberate acts, and a bounded agent session. It fits in a terminal. It asks you to name what matters and then shows you, without judgment, whether you're tending to it.
