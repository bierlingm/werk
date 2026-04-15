# werk-tab

Browser extension that replaces the new tab page with the werk frontier of action.

A sixth interface surface. Quiet by default — signal by exception.

## Sections

- **overdue** — active tensions past their horizon
- **next** — positioned active tensions, top 5 by position
- **held** — unpositioned active tensions (acknowledged, uncommitted), top 5
- **silent** — shown when there is no signal

Each row shows `#short_code`, desired, actual, and meta (horizon · position). Click jumps to the werk web detail view.

## Install (Brave / Chrome)

1. `brave://extensions` (or `chrome://extensions`)
2. Enable **Developer mode**
3. **Load unpacked** → select this folder
4. Open a new tab

## Requires

`werk daemon` running (or `werk serve` launched manually). The preferred setup is:

```bash
werk daemon install
```

This installs a launchd agent (macOS) or systemd user unit (Linux) that runs
`werk serve --daemon-target --port-range <range>` at login and respawns it on
crash. The active workspace is read from `~/.werk/config.toml` so the in-tab
switcher and `werk daemon point` both take effect on restart.

When nothing is running, the page shows a quiet offline state and silently retries.

## How it fetches

- Probes the canonical port range on load; first one that answers `/api/tensions` becomes the API base. The canonical range is defined in `werk_shared::daemon_net::DEFAULT_PORT_RANGE` and asserted in sync with `app.js` by a Rust parity test.
- Re-probes from scratch on SSE disconnect, tab focus, or 5s after a failure — so daemon restarts on a new port are picked up automatically.
- `GET /api/events` (SSE) for live updates — any event triggers a reload.

No storage, no background worker. All rendering happens in the new tab page itself.
