# CLI Patterns Study: Reference Architecture for Remote/Multi-Player werk

**Status:** Research draft, 2026-04
**Audience:** werk architect (Moritz)
**Purpose:** Extract architectural patterns from comparable tools to inform werk's evolution toward `werk.moritzbierling.com` as authoritative-replica, multi-device sync, and eventual multi-player + hosted SaaS.

This document is the per-reference study. The cross-cutting picks live in `architecture-synthesis.md`.

---

## 1. Git + GitHub

**The distinctive move.** Content-addressed immutable objects + refs as moveable pointers. Truth is *the object graph*; "branches" are tiny labels. Sync is negotiation over the graph (`have`/`want`), not a diff of state.

- **Remote/local model.** Every clone is a full authoritative replica; the "remote" is just another peer that happens to be the social convention for truth. Conflicts never happen at the object layer (hashes can't collide meaningfully) — they happen at the ref layer (non-fast-forward) and at the merge layer (tree 3-way merge), both handled by clients.
- **Wire format.** `pkt-line` framing (4-byte hex length + payload, `0000` terminator) over SSH or smart-HTTP. Two-phase: ref advertisement (server lists `<sha> <ref>` + capabilities like `multi_ack thin-pack side-band-64k ofs-delta shallow`), then `want`/`have`/`done` negotiation, then a packfile stream. Smart-HTTP is stateless: `GET .../info/refs?service=git-upload-pack` then `POST .../git-upload-pack`.
- **Auth.** GitHub layers: SSH keys (device-like, one per machine), PATs (user-scoped bearer tokens, `ghp_...`/`github_pat_...`), **GitHub Apps** (installation tokens, per-org/per-repo, short-lived, signed JWT → installation access token — this is the enterprise-grade pattern), OAuth apps (user-on-behalf-of).
- **Multi-context.** `.git/config` per repo pins `remote.origin.url`. No global "which repo am I in" — `cwd` is the context. `gh` layers a `hosts.yml` for multi-host.
- **Multi-player.** Attribution is *in the object* (`author`/`committer`); ACLs are at the ref layer (GitHub's branch protection) and at the repo/org layer (not in git itself).
- **Steal for werk.** (a) Content-addressed tension-operations (each gesture is an immutable op with a stable hash; epoch = signed reference to a root op). (b) `have`/`want` negotiation — when a device reconnects it sends its epoch heads; server sends the delta pack. (c) **GitHub App model** for agentic-inbox and other non-human agents: short-lived installation tokens scoped to a space, not long-lived user PATs.
- **Avoid.** (a) Per-file text merge (werk's data is structured, not lines). (b) GitHub's "org = flat namespace of repos" — it doesn't compose. Werk spaces need real hierarchy/nesting. (c) Push-time conflicts as UX — works for programmers, fatal for a practice instrument.

## 2. GitButler

**The distinctive move.** Virtual branches: the working tree belongs to *N simultaneously-applied* branches, and the tool reconciles which hunks belong to which. Git becomes a storage backend, not a UX.

- **Remote/local.** Local is a superset of git state plus GB's own metadata (which hunks → which virtual branch). On push, a virtual branch materializes as a real ref. Remote truth is still git; GB is a local overlay.
- **Auth.** Reuses git credentials (SSH/HTTPS). gitbutler.com layer exists for cloud-side features but isn't required for the core tool.
- **Steal.** The *overlay* pattern: werk's advanced features (warranty enforcement, provenance chains, private-within-shared) can be **local metadata layered over a simpler canonical log**, so a minimal client can still participate. Also: the "apply multiple streams at once" insight maps to werk's horizons — you can be holding several tensions in active practice simultaneously without forced serialization.
- **Avoid.** Tool-specific metadata that *can't* round-trip through the canonical format. If a vanilla git-only client loses your GB state, GB has failed the compat contract. Werk must keep the canonical log self-sufficient.

## 3. Railway

**The distinctive move.** Context is *explicit and sticky per directory*: `railway link` writes a project/environment/service triple into `.railway` and then every command implicitly targets it.

- **Auth.** `railway login` (browser) or `railway login --browserless` (paste pairing code). Two token types: `RAILWAY_API_TOKEN` (account-scoped, for CI on your behalf) and `RAILWAY_TOKEN` (project-scoped service token).
- **Multi-context.** `railway link`, then `railway environment` / `-e` / `-s` flags override. Config file, not env-var-driven — so `cd` into another project Just Works.
- **Steal.** The **directory-pinned space** pattern: `werk link <space>` writes `.werk/link.toml` with `{space_id, remote, default_horizon}`. Override with `--space` or `WERK_SPACE`. Two token classes mirror Railway cleanly: *account tokens* (user-on-behalf-of, for CLI) and *space tokens* (service tokens, for agentic-inbox and LaunchAgents). Use `werk_pat_` / `werk_svc_` prefixes (scannable like Cloudflare's `cfut_`, GitHub's `ghp_`).
- **Avoid.** Railway hides the project ID behind opaque URL slugs; reversing a slug to a stable ID is painful for tooling. Werk space IDs should be stable ULIDs exposed in the URL: `werk.moritzbierling.com/s/01H.../t/01K...`.

## 4. Cloudflare (Wrangler, Access, Tunnel)

**The distinctive move.** Scoped capability tokens — a token is a (resource set × permission set × optional IP/TTL) tuple, not a role. And Tunnel flips the NAT problem: the origin dials out, the edge becomes the front door.

- **Auth.** `wrangler login` runs browser OAuth 2.0 PKCE; stores OAuth refresh token locally. Alternative: API tokens created in dashboard with explicit scopes (`Account:Workers Scripts:Edit` × zone `example.com`). Scannable `cfut_` prefix.
- **Multi-context.** `CLOUDFLARE_ACCOUNT_ID` + `wrangler.toml` per project. No "switch account" — one account per project directory.
- **Multi-player.** Cloudflare Access: identity-aware proxy. Users get Access JWTs; services use Access service tokens (`CF-Access-Client-Id` + `CF-Access-Client-Secret` headers).
- **Steal.** (a) **Tunnel pattern for Umbrel**: home-server instance of werk dials out to `werk.moritzbierling.com` — no port-forwarding, no dynamic DNS, origin IP invisible. This is the cleanest "my laptop + my home server + my cloud are one network" story. (b) Scoped token grammar: `scope=space:01H.../tension:write` not `role=editor`. (c) Access service tokens as the *exact shape* agentic-inbox uses: two headers, rotatable, attributable to an identity.
- **Avoid.** The CF dashboard's conflation of Account API tokens vs. User API tokens is genuinely confusing; document werk's version with one crisp diagram.

## 5. Basecamp

**The distinctive move.** *Projects as first-class permission roots.* Not users → roles → resources; users are **invited into a project** and the project carries the access list. No global "admin of everything" by default.

- **Auth.** Launchpad SSO across 37signals products (OAuth 2.0). Personal access for API: Basecamp API uses OAuth 2.0 only — no API keys, no PATs. Deliberate.
- **Multi-player.** Invitation by email. A non-member receiving an @mention gets a gated invite. Clients (people you work for) are a separate permission class from employees.
- **Steal.** (a) **Project-rooted ACL** maps directly onto werk spaces: the space carries its member list; there's no global werk.moritzbierling.com admin who can read every space. (b) Invitation-by-email with optional account creation on accept — don't require signup before invite-accept. (c) **Deliberate OAuth-only API** means every integration has a revocable, identifiable token with a human behind it. Consider this for werk instead of long-lived PATs for humans.
- **Avoid.** Basecamp's message/todo/card schema is rigid; when you want to bend it, you can't. Werk's operative grammar must be extensible (new gesture types) without server schema migrations — pushes toward a log-of-typed-events model, not relational tables.

## 6. HEY.com

**The distinctive move.** The Screener: default-deny for new senders. Inbox membership is an explicit grant.

- **Steal for agent whitelist.** This is *the* model for agentic-inbox ↔ werk. When an email agent tries to file a tension into your space, it's in The Screener until you approve. First-approve creates a service credential scoped to that space. Revocation = one click.
- **Steal for multi-player invites.** "This person wants to join your space" is a Screener-style decision, not an instant-accept.
- **Avoid.** HEY's all-or-nothing Screener; werk needs partial grants (read-only observer, tension-proposer-but-not-author, warranty-witness).

## 7. Linear

**The distinctive move.** A *sync engine*, not a REST client: clients hold a full local model of everything visible to them, mutations are local-first with an op log, and the server broadcasts deltas to all connected clients subscribed to the same workspace.

- **Remote/local.** Server is authoritative. Client maintains IndexedDB replica. Bootstrap = "give me everything since transactionId X." Deltas over WebSocket.
- **Conflict.** Per-field LWW with server as referee. Mutations are idempotent by operation ID; offline writes queue, replay on reconnect, server may reject, client reconciles.
- **Auth.** API keys or OAuth. WebSocket auth via initial handshake bearer.
- **Steal.** (a) **Op-log first, REST second.** Every gesture in werk is a typed op; the HTTP API is a projection. (b) **Bootstrap cursor + delta stream**: client sends `since=<epoch_hash>`, server replies with deltas, then opens SSE/WS for live. (c) Keyboard-native surface — werk's TUI and web should share a command palette and shortcut table.
- **Avoid.** Linear hides its op log from users. Werk is explicitly a practice instrument where the log *is* the product (that's what "epochs" and "gestures" are). Surface it.

## 8. Figma / FigJam

**The distinctive move.** One server process per document. Not CRDT, not OT — centralized-authority LWW per property over WebSocket, with fractional indexing for ordered children.

- **Remote/local.** Client downloads full doc, edits locally, sends property-level ops over WS, server serializes and rebroadcasts. On reconnect: fresh download + replay unacked local ops. Clients *discard* incoming server updates that contradict their unacked local ops (prevents flicker).
- **Multi-player.** Team → Project → File hierarchy with ACLs per level. Presence via ephemeral channel (not persisted).
- **Steal.** (a) **Fractional indexing for tension ordering** within a parent — crucial because werk tensions in a tree have user-meaningful order, and reordering under concurrent edits is nasty without it. (b) Per-property LWW — werk field-level (desired_outcome, current_reality, warranty) resolves independently. (c) Flicker suppression: don't re-render a field you just edited until the server ACKs past your op.
- **Avoid.** Figma's one-process-per-doc scales for interactive canvases, but for a long-lived tension tree that's mostly idle, a per-space Durable Object or actor model fits better than always-on per-doc processes.

## 9. Obsidian + Obsidian Sync

**The distinctive move.** E2E-encrypted sync where the server literally cannot read your notes. Vault password ≠ account password. Scrypt KDF → AES-256-GCM, WebSocket transport (wss://). Lose password, lose data.

- **Steal.** (a) **Separate vault encryption key from account auth.** For private-within-shared tensions in werk: the tension body can be client-encrypted with a per-space key; the server holds only encrypted blobs plus a clear envelope (author, epoch, parent). The server can enforce ACLs on the envelope without reading contents. (b) Publishing as a public site (Obsidian Publish) — werk-as-portfolio is a real emergent surface; architect for "render an epoch slice as static HTML" from day one.
- **Avoid.** Obsidian's conflict files (`foo (conflicted copy).md`) are a confession that sync gave up. Don't ship that pattern — it's what you get when the data model is opaque to the sync engine. Werk's typed op log sidesteps this.

## 10. Automerge / Yjs / Jazz / Replicache

**The distinctive move per library.**
- **Automerge**: CRDT library, network-agnostic, automerge-repo handles storage/sync separately. You bring the server.
- **Yjs**: similar, with y-websocket/y-webrtc providers. Lighter wire format, less history by default.
- **Jazz**: CoValues (typed CRDTs) + opinionated cloud + passkey auth + cascading permissions. The most "batteries included."
- **Replicache**: *Not a CRDT.* Server-authoritative. Client runs mutators speculatively, then the sync loop: client sends pending mutations to `push` endpoint, pulls with an opaque `cookie` (server state version), server returns `(new_cookie, patch, lastMutationIDChanges)`; client **rewinds to last server state, applies patch, replays pending mutators on top**. Conflict resolution is whatever the mutator code does — server re-runs it canonically.

**Steal for werk.** The **Replicache model is the right fit** for werk, not a CRDT library. Reasons:
1. Werk is server-authoritative in spirit — warranties and provenance chains need a referee.
2. Gestures are already discrete, typed, replayable ops — that's exactly what Replicache mutators are.
3. CRDTs solve merge, but werk wants *policy* at merge (e.g., "you can't release a tension someone else is warrantying"), which is mutator-logic, not a CRDT axiom.
4. You avoid carrying Automerge's history-tail bloat (real issue for long-lived docs).

Keep `opaque cookie = epoch hash`, `patch = list of ops since that epoch`, mutators = gesture handlers shared between Rust client and Rust server (same code, same result). **This is the single most important architectural pick in this document.**

**Avoid.** Don't pick Automerge/Yjs just because they're the famous local-first names — their sweet spot is "any peer can be authoritative" (collab text editors, no server). That's not werk.

## 11. Tailscale / WireGuard

**The distinctive move.** Control plane issues short-lived capabilities; data plane is direct peer-to-peer WireGuard. Identity is OIDC-delegated. Device = node key.

- **Auth.** User logs in via Google/GitHub/Okta OIDC → control plane mints node key binding → device gets ACL policy + peer pubkeys pushed.
- **MagicDNS.** `<machine>.<tailnet>.ts.net` — stable, human, owned-subdomain under ts.net.
- **Steal.** (a) **MagicDNS pattern for werk spaces**: every space gets `<space>.werk.moritzbierling.com` (personal) or `<space>.<tailnet>.werk.dev` (hosted). Stable URL, human-readable, no UUIDs in surface URLs. (b) **Device concept separate from user**: your laptop, Umbrel, phone, and agentic-inbox are each a *device* with its own credential, all owned by you. Revoking one device doesn't log you out everywhere. (c) **OIDC-delegated identity** — don't build a password system. Start with Google/GitHub/passkeys.
- **Avoid.** Tailscale's ACL DSL is powerful but opaque; for werk's human/small-team use, keep ACLs as typed Rust enums with a plain-text YAML surface.

## 12. Fly.io

**The distinctive move.** `fly.toml` in the repo defines the app; `flyctl deploy` reconciles. Machines are individually addressable (`flyctl machine ...`).

- **Steal.** `werk.toml` in any werk-adjacent repo pins `{space, horizon}` — useful for integrations (e.g., a project repo that emits tensions via CI). Mirrors the `fly.toml`/`railway link` pattern.
- **Avoid.** Fly's org/app/machine hierarchy is three levels where two would do; keep werk to `space → tension`.

## 13. Supabase CLI

**The distinctive move.** `supabase link --project-ref <ref>` pins remote project. `supabase db push` applies local migrations to remote; `supabase db pull` generates a migration from remote diff. Migrations are SQL files in git, numbered by timestamp — the op log *is* the migration history.

- **Steal.** Timestamp-prefixed migration files mapped onto werk's epoch log: `epochs/20260417T1430_<hash>.werk` as a human-diffable serialization, so werk state is git-compatible if someone wants to version a space in git.

## 14. gh CLI

**The distinctive move.** `gh auth` manages a `hosts.yml` with multiple hosts (github.com, enterprise.example.com), each with its own token. `gh auth switch` flips active. `gh extension` lets third parties publish `gh-*` binaries that inherit gh's auth.

- **Steal.** (a) `~/.config/werk/hosts.toml` with multiple remotes (personal cloud, Umbrel, hosted SaaS, client space). `werk auth switch`. (b) **Extension protocol**: any binary named `werk-<x>` on PATH becomes `werk x` and inherits the active credential. This is how prose/beats/agentic-inbox can extend werk without being in the core repo.
- **Avoid.** gh's fallback to plaintext credential storage. Require OS keychain (keyring crate on Rust), no insecure fallback.

## 15. Dropbox / iCloud Drive

**The distinctive move.** Filesystem-shaped sync with last-writer-wins *and* conflict copies as the escape hatch.

- **Avoid entirely.** This is the anti-pattern. "`Tension — Moritz's conflicted copy 2026-04-17`" must never exist in werk. The typed op log exists precisely so this is impossible.

## 16. Notion / Coda

**The distinctive move.** Block tree with per-block permission inheritance + shareable URLs with granular grants (`page.readonly`, `page.comment`, `page.edit`). Workspaces are the billing/ACL root.

- **Steal.** (a) **Shareable-link grants**: `https://werk.moritzbierling.com/t/01K.../share?grant=witness&exp=...` issues a signed, time-boxed capability token — no account required to *view*; account required to *author*. (b) Block-level inheritance maps onto tension-tree inheritance: a tension inherits ACL from parent unless overridden.
- **Avoid.** Notion's block model made search and offline nearly impossible for years. Keep werk's canonical representation friendly to grep, SQL, and static rendering.

---

## 17. 37signals' ONCE (Campfire, Writebook)

> Caveat: this section was written from training knowledge without live source-fetching. Specifics about license terms, Kamal mechanics, and current pricing should be verified against once.com and DHH's posts before depending on them.

**The distinctive move.** Self-contained Docker image the buyer runs on their own server, with **zero runtime dependency on 37signals**. Pay once, own forever. If 37signals vanished, every Campfire/Writebook keeps working.

The corollary: **aggressively boring single-process Rails**. SQLite + the "Solid Trifecta" (Solid Queue, Solid Cache, Solid Cable) — no Redis, no Postgres, no Kubernetes. One container, one volume, one port. Small enough ops surface that a non-expert can run it for a decade.

- **Deployment.** Built around **Kamal** (DHH's deploy tool, post-cloud-exit). Kamal SSHes into target host, pulls image, runs behind **kamal-proxy** (Traefik replacement, also OSS), terminates TLS via Let's Encrypt automatically. Recommended targets: Hetzner, DigitalOcean. Updates: `kamal deploy` (or admin-UI button wrapping the same).
- **Licensing.** Perpetual. License verified at install/pull, **no runtime phone-home**. Air-gappable. 37signals ships updates for some unspecified period, then you keep your last image forever.
- **Data model.** **One workspace per deploy.** No `account_id` scoping in the schema — every row implicitly belongs to *this* instance. Backups = copy the volume.
- **Auth.** Self-contained, instance-local. First user is admin. No 37signals account. No SSO in the base product.
- **Multi-player.** Email invites via instance's own SMTP (BYO Postmark/Resend). No SSO/SAML.
- **Open-source posture.** **Source-available, not OSS** — buyers get the Rails source in the image, can read/modify locally, but redistribution is forbidden. The *plumbing* (Kamal, kamal-proxy, Solid Queue/Cache/Cable, Thruster, Mission Control: Jobs) is genuinely OSS MIT.
- **Steal.**
  - Single-binary, single-volume deploy. `werk serve` + SQLite is the whole thing for self-host.
  - **kamal-proxy for TLS + zero-downtime deploys** in the self-host SKU.
  - "First user is admin" bootstrap. No external IdP required for v1.
  - Instance-local email invites with BYO-SMTP — don't make self-hosters depend on werk.dev to send an invite.
  - Solid-Trifecta equivalent in Rust: SQLite for queue/cache/pubsub, no Redis dependency.
  - **Air-gappable by design.** Marketing honesty of "you own it, we can't take it away" — *real* differentiator for a structural-dynamics tool where archive longevity matters.
- **Avoid.**
  - Source-available license for werk's core. Werk has a smaller addressable market than Campfire; restrictive license kills the contributor flywheel. Apache-2.0 on `werk-core` and `werk-serve`.
  - No SSO story. ONCE gets away with it for 10-person teams; werk needs SAML/OIDC eventually.
  - Pay-once for the hosted tier (only works because the buyer pays their own ops bill).
  - Vague update SLA. Solo-maintained werk needs explicit "security updates for N years" or OSS fallback.
  - Multi-tenancy amputation. Keep codebase multi-tenant-capable, *run* it single-tenant in the self-host SKU via config, not schema.

**The three-SKU shape this implies:**

| SKU | Audience | Price | What they get |
|---|---|---|---|
| **OSS core** (`werk` + `werk-serve`) | devs, tinkerers | free, Apache-2.0 | Run anywhere. No support SLA. |
| **werk.dev hosted** | most people | monthly | Multi-tenant SaaS. Updates automatic. |
| **werk ONCE** (Docker image + Kamal config) | practitioners wanting permanence | one-time $99–299 | Ship once, own forever. Trust narrative. |

Umbrel fits *inside* the ONCE SKU as its appliance flavor. Same container image, different config.

---

## Sources

- Git Transfer Protocols (Pro Git book) — https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols
- OAuth 2.0 Device Authorization Grant (RFC 8628) — https://datatracker.ietf.org/doc/html/rfc8628
- Replicache: How It Works — https://doc.replicache.dev/concepts/how-it-works
- How Figma's multiplayer technology works — https://www.figma.com/blog/how-figmas-multiplayer-technology-works/
- How Tailscale Works — https://tailscale.com/blog/how-tailscale-works
- Tailscale MagicDNS — https://tailscale.com/kb/1081/magicdns
- Obsidian Sync end-to-end encryption — https://obsidian.md/blog/verify-obsidian-sync-encryption/
- Cloudflare API tokens — https://developers.cloudflare.com/fundamentals/api/get-started/create-token/
- Railway CLI docs — https://docs.railway.com/guides/cli
- gh auth login manual — https://cli.github.com/manual/gh_auth_login
- Jazz docs — https://jazz.tools/docs
- Automerge: Hello — https://automerge.org/docs/hello/
- GitButler virtual branches — https://docs.gitbutler.com/features/virtual-branches/branch-lanes
- Linear sync engine (talk announcement) — https://linear.app/blog/scaling-the-linear-sync-engine
- 37signals' once.com (verify before depending on)
- DHH posts on Kamal, Solid Trifecta, cloud exit — https://world.hey.com/dhh
