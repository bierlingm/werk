# X Post Draft: Werk Introduction (Jeffrey / Agentic Coding Community)

## Post Text

Every agentic coding setup has the same blind spot. I've been building something about it, and I want to show you what it is — especially @JeffreyEmanuel, whose open source work made it possible for me to build it in the first place.

Jeffrey's Agent Flywheel methodology, his beads system, his skills manager — these tools changed how I work. The 85% planning / 15% execution split, the bead polishing until convergence, the swarm coordination. I use these ideas every day. I'm grateful they exist and that he put them out there for all of us.

But working with these tools intensively, I kept running into the same wall: the plan goes stale.

Not because the plan was bad. Because reality changed. I learned something. My aim shifted. A step I thought was necessary turned out to be irrelevant. A new constraint appeared. The 5,000-line markdown plan is still there, but the structural relationship between what I want and where I actually am has moved, and the plan doesn't know it.

Every agentic builder knows this feeling. You've got agents executing beautifully. Beads being picked up, tasks completing, commits landing. And then you realize: the thing we're building isn't quite the thing we need anymore. The bead graph routes agents to the next task. It doesn't know if the task still matters.

That's the "clockwork deity" problem. The human checks in every 10-30 minutes because something has to hold the structural question: is what we're doing still advancing what we want?

I've been building a tool called werk that holds that question as a living data structure.

werk tracks structural tensions — the gap between what you want (desired outcome) and where you are (current reality). Under each tension, you build a theory of closure: the ordered steps you believe will bridge the gap. Children of a tension aren't tasks. They're your current hypothesis about how to get from here to there.

When you learn something new, you update reality. When your aim evolves, you update desire. When your theory is wrong, you restructure the children. The structure mutates through contact with action — it doesn't go stale because using it IS the practice.

Here's what makes it different from every task tracker and project management tool:

It's not a todo list with extra steps. It's a structural model of directed action. The vertical axis has one law: desired outcome above current reality. The gap between them is the tension that generates energy for creative work. This comes from Robert Fritz's structural dynamics — the same framework that underpins the creative process work taught by Nicholas Catton.

It lives in the terminal. werk is a Rust CLI + TUI. It sits where you already work. Your AI agent uses it via CLI (`werk tree`, `werk show`, `werk context --json`). You inhabit it via TUI. The tension structure is the shared surface between you and your agent.

It's agent-native. My workflow now: I tell Claude to get context from werk at session start, update structure as we accomplish and learn things, and close out the session with a review pass. The agent maintains the structural model as a first-class part of its operating loop. I don't want to work any other way.

It generates the best context your agent has ever had. `werk context --json` outputs structured intent: what you want, where you are, what's at the frontier, what's changed. This is categorically better context than any CLAUDE.md or markdown plan, because it's structurally current by design.

It auto-exports to JSON and you can commit it to git. Every commit is a structural snapshot. You can diff two commits and see not "what files changed" but how your theory of closure evolved. This is structural version control alongside code version control.

My friend Alain had his OpenClaw install werk and now the agent maintains a tension structure on Alain's behalf. Alain never touches the CLI or TUI. He just talks to his agent, and the agent maintains structural coherence via werk. The agent case and the practitioner case are both real and they're complementary.

I think of it as MRP for directed action. Material Requirements Planning transformed manufacturing by creating a structural model of what needs to happen, in what order, with what resources, by when. Before MRP, factories coordinated through bills of materials and physical kanban. MRP didn't replace the factory floor — it provided the structural intelligence that made the factory floor work at scale.

werk does the same for knowledge work in an age where AI agents are the factory floor. It doesn't replace your execution tools. It provides the structural intelligence that makes execution coherent.

How I see it fitting with the Agent Flywheel:

The flywheel's beads solve context at the task level — self-contained packets that any agent can pick up and execute. Brilliant. But where does the strategic context live? The thing that knows why these beads exist, whether the plan they came from is still valid, and when it's time to stop executing and start rethinking?

werk holds that layer. Not above the flywheel in a hierarchy — alongside it, holding the structural question that the bead graph can't hold because bead graphs route mechanistically (PageRank, betweenness centrality) while structural intent requires honest assessment of desire vs. reality.

The "clockwork deity" role — the human checking in every 10-30 minutes — is doing werk work manually. werk makes that structural awareness persistent and queryable.

I'm going to open-source the core (data format, CLI, sd-core library). The TUI instrument and the dynamics engine will be the product. Open format, proprietary instrument — like SQLite is a file format anyone can read, but the tools built on it are where the value compounds.

If you're building with agents and drowning in context management, or if your plans go stale faster than you can update them, or if you're tired of maintaining big markdown documents that are supposed to tell your agent what you're working on — this is for you.

werk.sh (coming soon) | github.com/[repo] (coming soon)

Thank you @JeffreyEmanuel for building in the open. It's because of that generosity that tools like this can exist.

---

## Video Script (60-90 seconds, Remotion)

### Visual: Terminal screen, dark background, werk TUI or tree output

**[0-5s]** Text on screen: "Every agentic coding setup has the same blind spot"

**[5-15s]** Show a typical setup: big markdown plan file, CLAUDE.md, scattered todo files. Text overlay: "Plans go stale. Context drifts. Agents stay busy while the goal moves."

**[15-25s]** Terminal: `werk tree` output appears, showing a tension structure with desire/reality/children. Narrator or text: "werk holds the structural question: is what we're doing still advancing what we want?"

**[25-40s]** Split screen. Left: desire at top, reality at bottom, theory of closure between them. Right: terminal showing `werk show <id>` with the frontier, closure ratio, window. Text: "Each tension is a gap between what you want and where you are. Children are your theory of how to close it."

**[40-50s]** Terminal: `werk context --json` output piped, showing structured intent data. Text: "Your agent gets structural context, not stale markdown. Updated through use, not through maintenance."

**[50-60s]** Animation: epoch transition. The tree restructures — desire evolves, some steps release, new steps appear. Text: "When you learn something new, the structure mutates. Plans aren't contracts. They're hypotheses."

**[60-70s]** Terminal: `werk tree` showing the updated structure. Text overlay: "MRP for directed action. The intent layer your execution tools are missing."

**[70-80s]** Closing frame: "werk" logo/name. "Open format. Terminal-native. Agent-ready." Links. "Thank you @JeffreyEmanuel for building in the open."

### Production Notes
- Style: Clean terminal aesthetic, Berkeley Mono or similar monospace font
- No music or ambient only
- Text-driven, no voiceover (or optional voiceover)
- Color palette: match werk TUI glyphs (diamond family, muted colors)
- Pacing: let each frame breathe, don't rush
