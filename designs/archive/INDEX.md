# werk Feature Designs — Index

**Date:** 2026-03-08
**Status:** All specs complete, ready for implementation planning

## Overview

Five designs that improve `werk` CLI usability and agent integration. These enable the Hermes Hackathon to use `werk` for structured tension management with AI agent integration.

---

## Designs

### Design A: Agent Command Resolution ⚡ **P0 BLOCKER**
**File:** `a-agent-command-resolution.md`
**Status:** Ready to implement (30 minutes)
**Problem:** Shell aliases don't work in subprocess calls. Error: "command not found: cdang"
**Solution:** Resolve commands in order: absolute path → full command with flags → PATH lookup

**Why it matters:**
- Unblocks all other agent integration features
- Required for Design D (werk run with inline prompt)
- Users can configure custom agents

**Tension:** `01KK68Z62QVZ3ZTG43GW8E1SS3`

---

### Design B: Interactive Config
**File:** `b-interactive-config.md`
**Status:** Ready to implement (2-3 hours)
**Priority:** P2 (Nice-to-have, can defer)
**Problem:** `werk config` is non-interactive. Users must know valid keys.
**Solution:** TUI menu for browsing and editing config with descriptions

**Why it matters:**
- Improves onboarding for new users
- Makes configuration discoverable
- Reduces lookup friction

**Tension:** `01KK68Z98EFXQAG5Q9AQQW64E4`

---

### Design C: ID Collision Disambiguation ⚡ **P0**
**File:** `c-id-collision-disambiguation.md`
**Status:** Ready to implement (1-2 hours)
**Problem:** Short IDs (8 chars) collide when tensions created within ~5 seconds
```
$ werk show 01KK461Y
error: ambiguous prefix '01KK461Y' matches 6 tensions
```

**Solution:** When ambiguous, show interactive menu for user selection

**Why it matters:**
- UX blocker for large tension forests (Hermes will have 20+ tensions)
- Avoids manual full-ID lookup
- Makes exploration natural

**Tension:** `01KK68Z93X28AH1HV972V2VYBA`

---

### Design D: `werk run` with Inline Prompt ⚡ **P0 CORE**
**File:** `d-werk-run-inline-prompt.md`
**Status:** Ready to implement (1-2 hours)
**Problem:** No way to pass user prompt + tension context in one command
**Solution:** `werk run <tension-id> "<prompt>"` returns agent response with optional update suggestion

```bash
# One-shot: context + prompt + agent response
werk run 01KK461YBDBEX3W3N2MCWR880A "I offered Dylan to do the video"

Agent Response:
───────────────────────────────────
Great! Delegation is smart.
SUGGESTED REALITY: Delegated to Dylan Thomas (ETA Saturday)
───────────────────────────────────

Accept? (y/n): y
✓ Updated tension
```

**Why it matters:**
- Enables agent integration with Hermes
- Hermes can read tension context and return one-shot responses
- Dylan can use this to record the video efficiently

**Tension:** `01KK68Z95AG4GRWZPYC4F78HK1`
**Depends on:** Design A (agent command resolution)

---

### Design E: Structured Suggestions ⭐ **P1**
**File:** `e-one-shot-with-structured-suggestions.md`
**Status:** Concept (Future enhancement to D)
**Priority:** P1 (High-value feature)
**Problem:** Agent gives prose advice; users must manually apply suggestions
**Solution:** Agent returns YAML with structured mutations; user reviews and applies with one click

```bash
werk run 01KK461Y "I delegated video to Dylan"

Agent Response:
───────────────────────────────────
Smart delegation! Track separately.
───────────────────────────────────

Suggested Changes:
1. Update actual: "Delegated to Dylan Thomas"
2. Create child: "Dylan's video meets quality standards"
3. Add note: "Confirmed via Telegram"

Apply all? (y/n): y
✓ Applied 3 changes
```

**Why it matters:**
- Closes the feedback loop: agent advice → structural change → measurement
- Enables exploratory structural iteration with AI
- Creates audit trail of agent suggestions

**Tension:** `01KK68Z96C65QTPMDQ8YKK5M5X`
**Depends on:** Design D (inline prompt)

---

## Implementation Order

### Phase 1: Blockers (Must complete before Phase 2)
1. **Design A** (agent command resolution) — ~30 minutes
   - Unblocks all agent-dependent features
   - Required by D

2. **Design C** (ID collision) — ~1-2 hours
   - UX improvement
   - Not strictly blocking, but necessary for large forests

### Phase 2: Core (Hermes integration)
3. **Design D** (inline prompt) — ~1-2 hours
   - Enables Hermes to pass context to agents
   - Unblocks Dylan video recording
   - Foundation for E

4. **Design E** (structured suggestions) — ~2-3 hours
   - High-value enhancement to D
   - Can follow immediately after D

### Phase 3: Polish (Can defer)
5. **Design B** (interactive config) — ~2-3 hours
   - Nice-to-have
   - Can be deferred post-launch

---

## Integration with Hermes Hackathon

### Using Design D (inline prompt)
Once `werk run <id> "<prompt>"` is available, Hermes can:

```python
# In Hermes' agent loop
context = get_werk_context(tension_id)
agent_response = call_claude(context + user_message)
# User can accept suggestions in one command
```

### Recording Dylan's Video
Dylan needs Design D to efficiently:
1. Receive Hermes context via `werk run`
2. Understand the state of all tensions
3. Record a contextual demo
4. Mark completion via `werk run`

**Tension:** `01KK68Z97HVW218153KTT97J20`
**Blocked by:** Design D (`01KK68Z95AG4GRWZPYC4F78HK1`)

---

## File Locations

All designs are in: `/Users/moritzbierling/werk/desk/werk/designs/`

```
designs/
├── INDEX.md                           (this file)
├── a-agent-command-resolution.md      (30 min, P0 blocker)
├── b-interactive-config.md            (2-3 hr, P2 polish)
├── c-id-collision-disambiguation.md   (1-2 hr, P0)
├── d-werk-run-inline-prompt.md        (1-2 hr, P0 core)
├── e-one-shot-with-structured-suggestions.md  (2-3 hr, P1)
├── i-time-travel-field-replay.md      (proposed, next major feature)
└── j-field-resonance.md               (proposed → v0.5 Phase 7, 14th sd-core dynamic, field-level coupling detection)
```

---

## Hackathon Work Tensio ns

In `~/werk/desk/hermes-hackathon/`, tensions created:

**Parent:** `01KK68ZXVP7Q1ADYSN6GFY8HJV` — Establish werk improvements for Hermes integration

**Children:**
- `01KK68Z62QVZ3ZTG43GW8E1SS3` — Fix werk agent command resolution (Design A)
- `01KK68Z93X28AH1HV972V2VYBA` — Fix werk ID collision (Design C)
- `01KK68Z95AG4GRWZPYC4F78HK1` — Implement inline prompt (Design D)
- `01KK68Z96C65QTPMDQ8YKK5M5X` — Structured suggestions (Design E)
- `01KK68Z98EFXQAG5Q9AQQW64E4` — Interactive config (Design B)

**Special:**
- `01KK68Z97HVW218153KTT97J20` — Record Dylan Thomas video demo (blocks on D)

View hierarchy:
```bash
cd ~/werk/desk/hermes-hackathon
werk tree --open  # Shows full dependency structure
```

---

## Next Steps

1. ✅ Designs complete and documented
2. ✅ Dependencies represented in werk tensions
3. ⏭️ Implementation planning (prioritize by deadline)
4. ⏭️ Code implementation with tests
5. ⏭️ Integration verification with Hermes

---

## Questions?

Each design includes:
- Problem statement
- Solution approach
- Implementation details with code sketches
- Testing checklist
- Related designs

Start with Design A (agent command) → D (inline prompt) → E (structured suggestions) → C (ID collision) → B (config).
