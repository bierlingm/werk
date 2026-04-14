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

`werk daemon` running (or `werk serve --global` launched manually). The preferred setup is:

```bash
werk daemon install
```

This installs a launchd agent (macOS) or systemd user unit (Linux) that keeps `werk serve --global` running across reboots, against the global workspace at `~/.werk/`.

When nothing is running, the page shows a quiet offline state and silently retries.

## How it fetches

- Probes ports `3749-3759` on load; first one that answers `/api/tensions` becomes the API base.
- Re-probes from scratch on SSE disconnect, tab focus, or 5s after a failure — so daemon restarts on a new port are picked up automatically.
- `GET /api/events` (SSE) for live updates — any event triggers a reload.

No storage, no background worker. All rendering happens in the new tab page itself.
