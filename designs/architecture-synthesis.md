# werk Architecture Synthesis

**Status:** Design draft, 2026-04
**Companion to:** `cli-patterns-study.md`
**Audience:** werk architect (Moritz), future contributors, future implementers of werk-compatible servers.

This document compresses the per-reference study into the architectural picks for werk's evolution toward `werk.moritzbierling.com` as authoritative-replica, multi-device sync, multi-player, and three-SKU distribution.

---

## What "world-class werk" honors

werk's first principles, evident in the existing tension tree:

1. **Structural, not operational.** Holds tensions; does not task-manage.
2. **Provenance-first.** Every gesture has author + underwriter + warranty (`#295`, `#292`).
3. **Activity-derived signals, by exception** (`#207`). The instrument surfaces, doesn't interpret.
4. **Operative grammar as substrate** (`#291`, `#296`). The core is open and portable; institutional features sit on top.
5. **Spaces as unit of visibility and control** — already modeled, becomes the ACL boundary.

A world-class deployment honors all five. Specifically: it can't be "laptop runs a server that phones call." That's a deployment topology, not an instrument. The instrument has to be **local-first, log-authoritative, multi-writer, provenance-preserving** — with the server being one authoritative replica among many.

---

## The 10 architectural picks

### 1. Sync model: Replicache-shape (server-authoritative, typed op log, shared mutators)

Server is authoritative. Clients hold a full local SQLite replica of their visible spaces. All mutations are typed gestures in an op log. Bootstrap is `since=<epoch>` + delta pack. A space's truth = the totally-ordered op log under that space's root. Epoch = a signed reference pointing to a log prefix.

The same Rust gesture-handler code runs on client (speculative, optimistic UI) and server (canonical, authoritative). Conflict resolution is *mutator code* — server re-runs the mutator under canonical state and accepts or rejects with a typed error.

**Why not CRDT.** CRDTs solve merge as an axiom. Werk wants *policy* at merge (warranty levels, provenance contradictions, status-state-machine transitions). That's mutator logic, not a CRDT property. Picking Replicache-shape preserves werk's semantics; picking Automerge/Yjs would force them into the merge function awkwardly.

**This is the single most load-bearing architectural choice in this document.** Get it explicitly committed before building.

### 2. Auth flow: OAuth device-code for humans, service tokens for agents, OS keychain for storage

Humans:
- `werk login` → opens browser on laptop (PKCE) or prints `user_code` for headless (RFC 8628 device code, 5-second poll).
- Tokens stored in OS keychain via Rust `keyring` crate, never plaintext. No fallback.

Agents (agentic-inbox, LaunchAgents, CI):
- `werk token create --space <id> --scope tension:write --ttl 90d --label agentic-inbox@laptop`
- Mints `werk_svc_*` token with Cloudflare-Access-style scoped capabilities. Revocable, attributable, labeled.

Both token types are bearer in `Authorization: Bearer`. Scannable prefixes: `werk_pat_` (personal access), `werk_svc_` (service), `werk_oauth_` (OAuth-issued). TTLs default on; rotation is a first-class command (`werk token rotate <label>`).

### 3. Identity: OIDC-delegated, never build passwords

Account = an external OIDC subject (Google, GitHub, Microsoft, Apple, eventually any OIDC IdP). Email is derived. Passkeys native via WebAuthn on the web surface. **No password field in werk-core ever.**

Authorship in the op log is the OIDC `sub` at gesture time, captured as a DID-shaped identifier: `did:werk:<sub-hash>` so internal IDs don't change if a user re-auths via a different provider. A user's identity record carries all the OIDC subjects they've claimed.

For self-host (ONCE SKU): ship with optional embedded Authelia/Kanidm or instructions for pointing at the buyer's own OIDC. "First user is admin" bootstrap pattern (Campfire-style) for the first 5 minutes; then OIDC required.

### 4. Conflict resolution: per-field LWW + fractional indexing for sibling order

Every field is LWW by server-assigned logical timestamp (Lamport counter per space). One exception: sibling ordering inside a parent tension uses **fractional indexing** (Figma-style) so "move this tension above that one" works correctly under concurrent moves. (Tension `#300` already touches this.)

Mutator code enforces *policy* conflicts (warranty violations, attribution contradictions, illegal status transitions) — rejects the op and emits a typed error the client surfaces. **No silent conflict copies, ever.** Don't ship Dropbox's `(conflicted copy)` pattern.

### 5. Multi-context CLI: `werk link` + hosts.toml + env override, in that precedence

Resolution order (highest precedence first):
1. `--space` / `--host` CLI flag
2. `WERK_SPACE` / `WERK_HOST` env var
3. `./.werk/link.toml` (directory-pinned, `werk link <space>` writes this)
4. `~/.config/werk/hosts.toml` default

Always-available commands:
- `werk auth login [--host <name>]`
- `werk auth switch`
- `werk auth status`
- `werk whoami` — prints active host + space + user + auth method. Copy `gh auth status` exactly.
- `werk link <space>` — pin current dir
- `werk --remote` — read-only escape hatch that skips local replica

### 6. Wire format: HTTP/2 + SSE for pull, JSON POST for push, JSON op envelopes

Endpoints (minimum viable surface):
- `POST /spaces/:id/push` — batch of gestures. Returns `{accepted: [...], rejected: [{op_id, reason}]}`.
- `GET /spaces/:id/pull?since=<epoch>` — SSE stream. Initial catchup frames + live deltas. Long-lived, resumable via `Last-Event-ID`.
- `GET /spaces/:id/snapshot?at=<epoch>` — full state at a point (bootstrap shortcut).

Op envelope (canonical):
```json
{
  "op_id": "01K...",
  "space_id": "01H...",
  "parent_epoch": "<hash>",
  "author_did": "did:werk:...",
  "underwriter_did": "did:werk:..." | null,
  "warranty": "impulsive" | "honest" | "truthful",
  "type": "create_tension" | "update_reality" | ...,
  "payload": { ... },
  "client_ts": "2026-04-17T19:30:00Z"
}
```

**Why SSE not WebSocket.** Proxy-transparent, browser-native, resumable via `Last-Event-ID`, one-way matches the model (client→server is REST POST). Skips gRPC tooling friction and WebSocket statefulness.

### 7. Service tokens: scoped, TTL'd, labeled, rotation-first

Mirror Cloudflare Access service tokens. For systems that can't set `Authorization` (some webhooks, certain iPaaS), provide alternative two-header form: `Werk-Client-Id` + `Werk-Client-Secret`.

Server stores hash + scope + expiry + label. Revocation by label (`werk token revoke agentic-inbox@laptop`). Audit log records every gesture with the token label so you can trace "what did agentic-inbox do last week."

Scope grammar (Cloudflare-style):
- `space:01H...` (resource)
- `tension:read | tension:propose | tension:write | tension:warranty` (action)
- `expires=2026-07-01T00:00:00Z` (TTL)
- `ip=1.2.3.0/24` (optional)

### 8. Invitations: three tiers (capability link → invite → screener)

1. **Ephemeral capability link** (Notion pattern):
   `werk share <tension> --grant view --exp 7d` → signed time-boxed URL. No account needed for *view*; account required for *author*.
2. **Email invite with OIDC-on-accept** (Basecamp pattern):
   Email a future member, they click, OIDC login, role granted.
3. **Agent whitelist Screener** (HEY pattern):
   New service-token caller's writes go to a quarantine queue until space owner one-click approves.

Roles (start minimal): `owner`, `author`, `witness`, `observer`. No "admin" above owner within a space; account-level billing is orthogonal.

### 9. Offline-first, operationally: write-ahead op buffer, optimistic UI, idempotent replay

- Every gesture writes to local SQLite (authoritative for the device) AND to a `pending_ops` table keyed by client-generated `op_id` (ULID).
- UI renders from local SQLite immediately. Field shows "syncing" indicator while op is unacked; never blocks UI on network.
- Background sync task drains `pending_ops` via `POST /push`. ACK clears row. Reject flags the op for user reconciliation with the typed server error.
- On reconnect after long offline: pull with `since=<last_epoch>`, rebase local pending ops on top (Replicache rewind-and-replay). Idempotent by `op_id` so replays are safe.
- **Local replica is a first-class product.** `werk show` works fully offline. `werk epoch` shows the local epoch separately from the remote epoch so the user always knows the sync state.

### 10. Open core + stable log format + three SKUs

API stability contract:
- **`werk-core`** (Apache-2.0): op types, log format, local SQLite schema, gesture validation, bootstrap/pull/push client protocol. Any third party can build a werk-compatible server by implementing the three HTTP endpoints.
- **`werk-serve`** (Apache-2.0): the minimal self-hostable server. Implements the three-endpoint contract. No billing, no org, just spaces + invites + tokens. This is what runs on Umbrel and on `werk.moritzbierling.com`.
- **`werk-hosted`** (closed): billing, org management, rate limits, audit log retention, cross-space search, moderation. Where SaaS revenue lives. None of it required to *use* werk.

**Log format is semver'd.** `werk-log/1.x` is additive op types only. New op types are optional to implement; old clients ignore unknown types (forward-compat). Any breaking change = `werk-log/2` and a documented migration path.

This mirrors Matrix (protocol vs. Element vs. matrix.org) and ActivityPub (spec vs. Mastodon vs. mastodon.social) more than Basecamp (monolith). For a structural-dynamics instrument meant to be held trustworthily for decades, that's the correct posture.

---

## Three-SKU distribution

| SKU | Audience | Price | What they get |
|---|---|---|---|
| **OSS core** (`werk` CLI + `werk-serve`) | devs, tinkerers, self-hosters | free, Apache-2.0 | Run anywhere. No support SLA. |
| **werk.dev hosted** | most practitioners | monthly subscription | Multi-tenant SaaS. We run it. Automatic updates. |
| **werk ONCE** (Docker image + Kamal config) | practitioners wanting permanence + sovereignty | one-time $99–299 | Ship once, own forever. Trust narrative. |

Umbrel fits *inside* the ONCE SKU — it's the appliance flavor of "run werk on your own hardware." Same container image, different config.

**kamal-proxy** is the default reverse-proxy/TLS layer for the ONCE SKU. **Cloudflare Tunnel** is the default for `werk.moritzbierling.com` (no open ports, DDoS protection, Access integration).

---

## Topology diagram

```
                    ┌─ Hetzner: werk authoritative replica ─┐
                    │  werk-serve + logbase + WAL           │
                    │  Litestream → R2 continuous backup    │
                    │  Cloudflare Tunnel → no open ports    │
                    └──────────┬────────────────────────────┘
                               │  events (SSE) / push (POST)
        ┌──────────────────────┼──────────────────────┬────────────────────┐
        │                      │                      │                    │
  ┌─────▼─────┐        ┌───────▼───────┐      ┌───────▼───────┐   ┌───────▼────────┐
  │  Laptop   │        │  Umbrel       │      │  claude@      │   │  Phone PWA     │
  │ werk CLI  │        │ (cold replica │      │  Worker       │   │  werk-web      │
  │ local SQLite│      │  + backup)    │      │ (R2-cached    │   │  IndexedDB     │
  │ replica   │        │ Cloudflare    │      │  logbase)     │   │  replica       │
  │ + sync    │        │ Tunnel        │      │               │   │                │
  └───────────┘        └───────────────┘      └───────────────┘   └────────────────┘
```

Each client keeps its own replica. Offline laptop still works. Phone PWA works on airplane. Hetzner is the rendezvous, not the single writer.

---

## Multi-player semantics, played out

When this becomes multi-player (tensions `#4`, `#297`):

1. **Invitation flow**: create space `waterlight`, invite `mist@…` as `author`. They get email → OIDC login → first sync pulls the logbase slice for `waterlight`.
2. **Author attribution**: every gesture records who. Tree view colorizes by author.
3. **Underwriter** (`#295`): when an agent acts on someone's behalf, the UI shows both. "desired updated by claude (underwriter: mist)".
4. **Warranty enforcement** (`#292`): shared-space gestures default to `honest`; `truthful` requires explicit human attestation. Agent-originated gestures can't rise above `honest` warranty.
5. **Visibility boundaries**: some tensions in a shared space can be `private`, visible only to the author. Logbase encrypts those events client-side with author's key; other replicas see opaque blobs they can verify but not decrypt.

This is the kernel of `#291` — the operative grammar becomes the protocol; institutional features (billing, SSO, audit) sit on top without mutating the core.

---

## What advances by building this

This work directly progresses these existing tensions:
- `#34` — public web surface where others can view part of the tension tree
- `#42` — coherence offering (claude@ + werk tools = the package)
- `#119` — MCP integration guide (canonical pattern for agents)
- `#241` — werk ensures LLMs work well with it
- `#291`, `#292`, `#293`, `#294`, `#295`, `#296`, `#297` — the operative grammar as substrate, warranty levels, engagement first-class, provenance, open vs. institutional layer separation
- `#37` — werk publicly known as the structural intent layer for agentic work

It also creates new tensions to file:
- `werk-log/1.0 spec authored and published`
- `werk-serve sync protocol implemented (push/pull/bootstrap)`
- `werk CLI grows --remote and replica modes`
- `OS keychain credential storage via keyring crate`
- `OIDC integration (Google + GitHub + passkeys minimum)`
- `kamal-proxy + Docker image for ONCE SKU`
- `Cloudflare Tunnel + Hetzner deployment of werk-serve`
- `space ACL model (owner/author/witness/observer)`
- `service-token system with scoped capabilities`
- `agentic-inbox screener integration for service-token approval`

---

## Phased build

**Phase 0 — today/this week (throwaway, but irreplaceable user research):**
- Cloudflare Tunnel from laptop to `werk.moritzbierling.com`, behind existing Access policy.
- Add werk tools to claude@ Worker (wraps existing 11 HTTP endpoints).
- claude@ system prompt for propose-then-execute on werk mutations, auto-send replies.
- **Delivers:** email-to-werk from phone, today. Laptop-on required. Honest limitation. Use for a week to sharpen Phase 1 priorities.

**Phase 1 — next 2 weeks (design-first investment):**
- Author the **`werk-log/1.0` spec** as a markdown document in this repo.
- Author the **`werk-serve` sync protocol spec** (push/pull/bootstrap).
- File these as werk tensions so the design itself is in the instrument.

**Phase 2 — month 1 (engineering bet):**
- Refactor `werk-core` so logbase is authoritative and SQLite materializes from it.
- Audit current implementation — may already be largely true.

**Phase 3 — month 2 (cloud authoritative):**
- `werk-serve` deployed on Hetzner.
- Litestream continuous backup to Cloudflare R2.
- Cloudflare Tunnel + Cloudflare Access in front.
- `werk` CLI grows `--remote` mode (HTTP-only, no local replica). Good for weeks.

**Phase 4 — month 3 (local-first):**
- CLI `replica` mode: laptop has local logbase, syncs deltas in background.
- TUI and CLI work fully offline.
- Conflict resolution via per-field LWW + fractional indexing.
- Umbrel runs `werk-serve --mode replica --follow https://werk.moritzbierling.com` as cold spare.

**Phase 5 — month 4-6 (multi-player):**
- Shared spaces, ACLs, invitations.
- OIDC integration (Google + GitHub + passkeys).
- Author attribution + warranty enforcement.
- First partner: Mist (Waterlight collaboration, tension `#62`).

**Phase 6 — month 6+ (productize):**
- Publish `werk-log/1.0` spec publicly.
- Ship `werk ONCE` Docker image + Kamal config.
- Launch `werk.dev` hosted SaaS.
- `werk.moritzbierling.com` becomes the reference deployment.

---

## Decisions awaiting commitment

Before code, four explicit yeses:

**A. Authority model.** Replicache-shape server-authoritative + typed op log + shared Rust mutators. Strong recommendation. The alternative (full Automerge/Yjs CRDT) loses policy-at-merge.

**B. Log format commitment.** Public, semver'd `werk-log/1.x`. Binds your hands (breaking changes = major bump) but enables third-party clients and the ONCE trust narrative.

**C. Identity stance.** OIDC + passkeys only, no passwords, from day one. Saves months of security work; means self-hosters need to configure an IdP or use embedded.

**D. Three-SKU commitment.** OSS core + hosted SaaS + paid ONCE image, decided now (affects license file on day one, packaging shape from day one). Or punt ONCE to year 2.

---

## What will actually be hard

1. **Logbase-as-authoritative refactor.** If werk-core currently materializes into SQLite as primary and logs to events secondarily, inverting that is real work. Worth auditing first.
2. **CRDT semantics per field.** Reality LWW is easy. Position fractional-indexing you've thought about (`#300`). Notes append-only is easy. Horizon needs care — two people setting different horizons at nearly the same time shouldn't silently lose one.
3. **Warranty enforcement UX.** Designing "this gesture was impulsive, that one honest" without making every write tedious. Probably default-by-context.
4. **Private tensions in shared spaces.** Encrypted events with verifiable metadata is cryptographically doable but operationally subtle (key rotation, revocation, new-collaborator backfill).
5. **The product/practice tension.** Hosted version is revenue; open operative grammar is integrity. Keeping them cleanly separated (`#296`, `#297`) requires discipline — temptation will be to leak institutional features into core.

---

## Companion document

See `cli-patterns-study.md` for the per-reference research that produced these picks.
