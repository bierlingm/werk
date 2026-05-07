# werk-web and werk-tab investigation (research artifact)

## werk-web

- **Framework:** axum 0.8 (single-file crate at `werk-web/src/lib.rs`,
  ~1080 lines).
- **Runtime:** tokio (full features).
- **Front-end:** the only static surface is an embedded HTML file at
  `werk-web/index.html`, included via `include_str!`. No `ServeDir`, no
  bundler.
- **Internal deps:** `werk-core`, `werk-shared`, plus
  `tokio-stream`, `tower-http` (cors only), `futures-core`.
- **Routes registered at `lib.rs:351`** (`build_router`):
  - GET /, /api/tensions, /api/workspace, /api/workspaces, /api/field/...
  - POST /api/tensions, /api/workspace/select
  - PATCH /api/tensions/{id}/desired, .../reality
  - POST .../resolve, .../release, .../reopen
  - GET /api/views/* (focus, horizon, deadlines, epoch, tree)
  - GET /api/events (SSE)
- **SSE infrastructure (NOT bridged to werk-core's EventBus):**
  - Private `tokio::sync::broadcast::Sender<SseEvent>` channel
    (`lib.rs:354`, capacity 64).
  - `SseEvent { kind: String }` (`lib.rs:332-335`); data payload is
    hard-coded `"{}"`.
  - Each mutation handler explicitly publishes a kind string after
    success: `tension_created`, `tension_updated`, `tension_resolved`,
    `tension_released`, `tension_reopened`.
  - sse_handler at `lib.rs:560-575` subscribes a `BroadcastStream` and
    maps to axum SSE events.

**Where new sigil routes go:** After existing route registrations in
`build_router` (`lib.rs:354-373`), add:
```
.route("/api/sigil", get(get_sigil))
.route("/api/sigil/stream", get(sse_sigil_handler))
```
Handler placement: in the "Views (consumer-agnostic projections)"
section.

**SSE invalidation strategy (D12):** Additive, not bus bridge. Add a
second `state.tx.send(SseEvent { kind: "sigil_invalidated".into() })`
after each existing mutation emit. New `sse_sigil_handler` clones the
existing `BroadcastStream` and filters to only `kind == "sigil_invalidated"`.

**Tests:** None currently in werk-web. Workers must add at least basic
smoke coverage for the new endpoints.

## werk-tab

- **Form factor:** Chromium MV3 extension. Not a Cargo workspace member.
- **Files:** `manifest.json`, `index.html`, `app.js` (~430 lines vanilla
  JS), `style.css`.
- **Permissions:** `chrome_url_overrides.newtab = "index.html"`,
  `host_permissions` for `localhost:3749..3759`.
- **API discovery:** `discoverApi()` (app.js:117) probes ports 3749–3762
  hitting `/api/tensions`. First responder = active API.
- **Initial load:** `load()` (app.js:140) fetches `/api/tensions` (or
  `/api/field/*` in field mode).
- **SSE consumer:** `connectStream()` (app.js:382-407) opens
  `EventSource(${API}/api/events)`. Treats any message as
  "something changed, refetch" — ignores `event:` name and `data`.
- **Modes:**
  - `space` (default): top of band overdue/next/held.
  - `field`: per-space vitals + pooled bands.
- **Workspace switching:** POST `/api/workspace/select`, the daemon
  exits, supervisor restarts, the tab rediscovers.

**Sigil mode (M5) plan:** Third toggle in the header
(`<button id="sigil-toggle">`). New `<section class="sigil">` block.
New `sigilMode` flag in `app.js`. New `loadSigil()` and
`renderSigil(svgText)`. SSE: add a dedicated event-name listener
`es.addEventListener('sigil_invalidated', ...)` on the existing
EventSource — do NOT open a second connection.

**Ports:** No new range needed; sigil endpoints are on the same daemon.

**Test approach:** No JS test harness exists. Use `agent-browser` skill
to automate Brave/Chromium with the unpacked extension. See
`skills/frontend-integrator/SKILL.md`.
