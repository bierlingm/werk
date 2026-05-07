---
name: frontend-integrator
description: Implements werk-tab (Chrome MV3 extension, vanilla JS) integration with the sigil engine.
---

# frontend-integrator

NOTE: Startup and cleanup are handled by `worker-base`. This skill defines the WORK PROCEDURE.

## When to Use This Skill

M5 features only. werk-tab is a Chromium MV3 extension at `werk-tab/`
(not a Cargo workspace member). All work is JS / HTML / CSS.

## Required Skills, Tools, and Dependencies

- **`agent-browser` skill** — required for visual verification. Load the
  extension via "Load unpacked", navigate to the new tab, take screenshots,
  assert DOM content.
- **No bundler. No framework.** werk-tab is intentionally vanilla.
- **Browser:** Brave or Chromium. If neither is installed on Zo, the
  worker installs Chromium via the platform's package manager (apt or brew)
  with explicit user notice in the handoff. **Do NOT install
  proprietary Chrome-branded binaries.**
- **`werk serve` daemon** must be running on port 3749 during testing.
  Start with `cargo run -p werk -- serve --port 3749` in the background;
  stop on exit (`lsof -ti :3749 | xargs kill`).

## Work Procedure

### 1. Read first
- `werk-tab/manifest.json`
- `werk-tab/index.html`
- `werk-tab/app.js` (the existing fetch + render + SSE pattern)
- `werk-tab/style.css`
- The feature description and `fulfills` IDs.
- `library/architecture.md` "Surfaces (M4) > werk-tab (M5)" section.

### 2. Plan the change
- Determine: is this a third toggle (`sigil`) alongside `space` and
  `field`, or a replacement of one? D11 says: third toggle.
- New sections in `index.html`: `<button id="sigil-toggle">sigil</button>`
  in the header; `<section class="sigil" id="sigil" hidden>...</section>`.
- New code in `app.js`:
  - `sigilMode` flag (alongside `fieldMode`).
  - `toggleSigilMode(next)` (mirror of `toggleFieldMode`).
  - `loadSigil()` fetching `${API}/api/sigil?logic=glance`.
  - `renderSigil(svgText)` injecting the SVG into the section.
  - SSE handler additions: listen for `sigil_invalidated` events
    via `es.addEventListener('sigil_invalidated', () => loadSigil())`,
    and route `load()` dispatch in the existing `onmessage` handler so
    the active mode determines what reloads.
- New CSS: scope `.sigil { ... }` for the new section.

### 3. Tests first (manual, scripted)
There is no JS test harness in werk-tab. Tests are scripted manual checks
via `agent-browser`. Write a numbered checklist of expected behaviors
*before* changing code:

```
1. Loading the extension shows three toggle buttons in header
2. Click "sigil" toggle: sigil section appears, space/field hidden
3. Sigil section contains a single inline <svg> element
4. Triggering a tension mutation via curl POST causes sigil to refresh
   within 2 seconds (verify by re-screenshot)
5. Visibility off/on triggers a single reload (no thrashing)
6. With werk serve down, sigil section shows fallback "offline" state
7. Re-toggling between modes does not double-render or break SSE
```

These become `interactiveChecks` entries in the handoff, each with
agent-browser screenshot evidence.

### 4. Implement
- Edit `index.html`, `app.js`, `style.css` in that order.
- Match the existing JS style: vanilla DOM APIs, no `import`, top-level
  `const` for constants, `async function` for I/O. No new transitive
  deps.
- For port discovery: reuse the existing `discoverApi()` — do **not** add
  new probe targets. Sigil endpoints live on the same port as `/api/tensions`.
- For SSE: reuse the existing `EventSource` connection at
  `${API}/api/events`; add a dedicated event-name listener for
  `sigil_invalidated`. Do NOT open a second EventSource.
- The fallback offline state can mirror the existing `space` mode's
  empty-banner pattern; do not invent new chrome.

### 5. Verify with agent-browser
- Start `werk serve` on 3749 in the background.
- Use agent-browser to:
  - Open Brave/Chromium with the unpacked extension.
  - Navigate to `chrome://newtab/` (or whatever the active new-tab URL
    resolves to).
  - Click the sigil toggle.
  - Screenshot.
  - POST a mutation (`curl -X PATCH http://localhost:3749/api/tensions/<id>/desired -H 'content-type: application/json' -d '{"value":"updated"}'`).
  - Wait 3 s.
  - Screenshot again, verify the SVG changed.
- Stop `werk serve`.
- Stop the browser session — do not leave it running.

### 6. Commit
- Stage explicit paths only.
- Message: `feat(werk-tab): add sigil mode rendering glance preset (M5 / werk-tab-sigil-mode)`

## Example Handoff

```json
{
  "salientSummary": "Added werk-tab sigil mode: third toggle in header alongside space/field; renders glance preset via GET /api/sigil; refreshes on sigil_invalidated SSE events without opening a second EventSource. agent-browser checks confirm initial render, mutation-triggered refresh, and offline fallback.",
  "whatWasImplemented": "werk-tab/index.html: added <button id=\"sigil-toggle\">sigil</button> in header (after field-toggle); added <section class=\"sigil\" id=\"sigil\" hidden> as a new top-level section. werk-tab/app.js: added sigilMode flag, toggleSigilMode() mirror of toggleFieldMode, loadSigil() fetches ${API}/api/sigil?logic=glance with text/svg+xml accept header and injects response into #sigil via .innerHTML (sanitization not needed since same-origin and we control the API), renderSigilOffline() shows fallback banner. SSE: added es.addEventListener('sigil_invalidated', () => { if (sigilMode) loadSigil() }). Existing es.onmessage retained, dispatches by active mode. werk-tab/style.css: added .sigil { padding: 24px; display: grid; place-items: center; } and an offline state class.",
  "whatWasLeftUndone": "",
  "verification": {
    "commandsRun": [
      { "command": "cargo run -p werk -- serve --port 3749 (background)", "exitCode": 0, "observation": "daemon listening" },
      { "command": "lsof -ti :3749 | xargs kill (cleanup)", "exitCode": 0, "observation": "daemon stopped" }
    ],
    "interactiveChecks": [
      { "action": "agent-browser: launched Brave with --load-extension=$REPO/werk-tab, opened chrome://newtab", "observed": "header shows three toggles: space (active), field, sigil; click sigil → sigil section visible, space hidden, single inline <svg> rendered with width 600 height 600" },
      { "action": "agent-browser: screenshotted sigil mode with 8 active tensions in fixture", "observed": "screenshot saved at /tmp/sigil-tab-initial.png; 5 outer ring nodes visible, 1 root center" },
      { "action": "curl -X PATCH http://localhost:3749/api/tensions/<id>/reality -d '{\"value\":\"changed\"}'; wait 2s; screenshot again", "observed": "screenshot at /tmp/sigil-tab-after-mutation.png shows visibly different ring composition" },
      { "action": "stop werk serve, refresh new tab", "observed": "sigil section shows 'offline — reconnecting…' banner; existing space/field modes also show offline banner (existing behavior)" }
    ]
  },
  "tests": {
    "added": []
  },
  "discoveredIssues": [
    { "severity": "low", "description": "agent-browser cannot inject extensions automatically without a custom Brave profile dir — required wrapping in a TempDir profile per session.", "suggestedFix": "Document this in library/user-testing.md if not already." }
  ]
}
```

## When to Return to Orchestrator

- agent-browser cannot launch Brave/Chromium on Zo (toolchain/install
  issue beyond this worker's scope).
- The web API `/api/sigil` returns unexpected shapes despite the
  feature description; suggests the M4 web feature was misimplemented.
- The existing extension's discovery probe breaks because port range
  changed (it shouldn't — flag if it does).
- The Brave / Chromium build available on Zo doesn't support MV3 (very
  unlikely in 2026, but possible).
