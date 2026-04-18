# werk-log/1.0 — Draft Specification

**Status:** DRAFT, not committed
**Date:** 2026-04
**Scope:** Wire format and semantics for the werk operative log
**Authority claim:** This document proposes the log as authoritative source of truth for werk state. **Moritz has not yet committed to this premise.** The spec stands on its own merits — if log-as-authoritative is rejected, large portions still apply (the log can remain a derivative audit trail of SQLite writes rather than the canonical substrate).

---

## 0. Purpose

`werk-log/1.0` defines a portable, durable, content-addressed event log for werk gestures. Its goals:

1. **Replication**: any device holding the log slice for a space can reconstruct identical materialized state.
2. **Provenance**: every gesture records author, underwriter, warranty, and parent-causation.
3. **Auditability**: the log is human-diffable, grep-able, and survives the death of any particular implementation.
4. **Forward-compat**: unknown op types are ignored by old clients; new ops are additive.
5. **Network-friendly**: gesture envelopes are self-contained, idempotent by `op_id`, and replayable.

Non-goals:
- Defining the storage engine (SQLite, Postgres, JSONL, content-addressed blobs all conform).
- Defining the network protocol (covered in `werk-serve-protocol-1.0-draft-spec.md`).
- Mandating a specific identity system (operates on opaque `did:` strings).

---

## 1. Identifiers

### 1.1 `op_id`
- Format: ULID (Crockford's base32, 26 chars, lex-sortable).
- Generated client-side at gesture creation.
- Globally unique. Two ops with the same `op_id` MUST be byte-identical (idempotency contract).

### 1.2 `space_id`
- Format: ULID.
- Assigned at space creation. Stable for the life of the space.
- A space's log is the totally-ordered set of ops with this `space_id` in the envelope.

### 1.3 `tension_id`
- Format: ULID, scoped within a space (`{space_id}:{tension_ulid}` if global referencing is needed).
- Assigned by the `create_tension` op. Stable for the life of the tension.

### 1.4 `epoch`
- Format: BLAKE3 hash (32 bytes, hex-encoded for transport) of a canonical encoding of an op-log prefix.
- Each op produces a new epoch by hashing `(prev_epoch || canonical_op_bytes)`. The first op's `prev_epoch` is the all-zero hash.
- Epochs serve as opaque sync cursors. `since=<epoch>` means "send me everything after this point."

### 1.5 `did` (decentralized identifier)
- Format: `did:werk:<32-hex>` for werk-internal, or any RFC-compliant DID (e.g. `did:web:moritzbierling.com`) for external authors.
- Resolved via `werk-identity/1.0` (separate spec).
- Used for `author_did` and `underwriter_did` fields.

---

## 2. The op envelope

Every gesture is a single op object. Canonical encoding is JSON with sorted keys, no whitespace, UTF-8.

```json
{
  "op_id":         "<ULID>",
  "space_id":      "<ULID>",
  "prev_epoch":    "<32-byte hex>",
  "author_did":    "did:werk:...",
  "underwriter_did": "did:werk:..." | null,
  "warranty":      "impulsive" | "honest" | "truthful",
  "type":          "<op_type>",
  "payload":       { ... type-specific ... },
  "client_ts":     "<RFC3339 UTC>",
  "client_meta":   { ... optional, ignored by canonical hash ... }
}
```

`client_meta` is excluded from the canonical encoding used to compute the new epoch — clients can carry arbitrary annotations (e.g. local device id, app version) without affecting the cryptographic chain.

`prev_epoch` allows the server to detect concurrent ops: if a client submits with a stale `prev_epoch`, the server may either fast-forward the op (if no conflict) or reject (if a policy mutator says no). See §6 (mutator semantics).

---

## 3. Op types (1.0 set)

### 3.1 Space gestures

#### `create_space`
- payload: `{ name: string, description?: string, metadata?: object }`
- Constraints: `name` unique within authoring identity's namespace.
- Must be the first op in a space's log. `prev_epoch` is the all-zero hash.

#### `archive_space` / `unarchive_space`
- payload: `{}`
- Soft-state; the log keeps growing under it.

### 3.2 Tension lifecycle

#### `create_tension`
- payload: `{ tension_id, desired: string, reality: string, parent_tension_id?: string|null, position?: string }`
- `position` uses fractional indexing (§5).

#### `update_desired`, `update_reality`
- payload: `{ tension_id, value: string }`
- Field-level LWW: receiver records the new value; previous values stay in log history (queryable).

#### `update_status`
- payload: `{ tension_id, status: "active" | "resolved" | "released" | "held" | "snoozed" | "deadlined" }`
- State-machine transitions enforced by mutator (§6.2).

#### `set_horizon`
- payload: `{ tension_id, horizon: string|null }`  (RFC3339 date or human shorthand like `2026-Q3`)

#### `snooze_until` / `recur`
- payload: `{ tension_id, until?: string, interval?: string }`

#### `move_tension`
- payload: `{ tension_id, new_parent_tension_id: string|null, position: string }`

#### `delete_tension`
- payload: `{ tension_id, reparent_strategy: "to_grandparent" | "to_root" | "delete_subtree" }`

### 3.3 Compositional gestures

#### `compose_up`
- payload: `{ new_tension_id, desired, reality, child_tension_ids: [string], position }`
- Creates a parent and reparents existing children atomically.

#### `split_tension`
- payload: `{ source_tension_id, new_tensions: [{ tension_id, desired, reality }] }`
- Provenance link: each new tension records `provenance: { kind: "split", from: source_tension_id }` in its initial state (computed, not stored separately).

#### `merge_tensions`
- payload: `{ source_tension_ids: [string], target: { tension_id, desired, reality } }`
- Provenance link: target records `provenance: { kind: "merge", from: source_tension_ids }`.

### 3.4 Notes & testimony

#### `add_note`, `retract_note`
- payload: `{ note_id, tension_id, body: string }` / `{ note_id }`
- Append-only by semantics; retract sets a flag, doesn't remove from log.

### 3.5 Position & ordering

#### `reposition`
- payload: `{ tension_id, position: string }`
- `position` is a fractional index (§5).

### 3.6 Field/agg gestures

(Reserved for future: `mark_attention`, `clear_attention` — agent-driven signals.)

### 3.7 Forward-compat: unknown op types

Receivers MUST:
1. Validate envelope shape (all required fields present, correct types).
2. If `type` is unrecognized: store the op, advance the epoch, but do not apply to materialized state. Mark in the local index as "unapplied: unknown_type".
3. Surface unapplied-op count in `werk epoch --verbose` so users can detect version-skew.

This makes `werk-log/1.x` strictly additive: any 1.x.y client can sync with any 1.x.z server.

---

## 4. Warranty levels

Each op carries `warranty ∈ {impulsive, honest, truthful}`.

- **impulsive**: written without reflection. Default for casual capture (email-to-claude flows, voice memos). May be revised silently.
- **honest**: written deliberately, the author believes it. Default for CLI/TUI explicit gestures.
- **truthful**: written under explicit attestation by a human author. Required for certain mutator-policy-protected gestures (e.g. resolving a tension someone else is warrantying).

Mutators (§6) MAY reject ops below a required warranty floor for the operation. Agent-originated ops MUST NOT exceed `honest` warranty (a service token cannot mint truthful gestures).

---

## 5. Fractional indexing for position

Sibling tensions under a parent are ordered by a `position` string. Positions are compared lexicographically. To insert between two siblings:
- Between positions `A` and `B`: generate any string `X` such that `A < X < B`.
- We adopt the algorithm in [Fractional Indexing](https://gist.github.com/wolever/3c3fa1f23a7e2e19dcb3) (Crockford base32 alphabet, midpoint generation).

This ensures concurrent reorders never collide and never require renumbering.

---

## 6. Mutators: policy at apply time

When a server (or any conformant verifier) applies an op, it runs the corresponding **mutator function**. The mutator is deterministic given (current-state, op). Mutators MAY:

1. Accept the op, producing the new state.
2. Reject the op with a typed error.

### 6.1 Standard mutator errors

```
INVALID_ARGUMENT          - payload doesn't validate
NOT_FOUND                 - referenced tension/space doesn't exist
PRECONDITION_FAILED       - state machine transition illegal (e.g. resolve a released tension)
WARRANTY_FLOOR_VIOLATED   - op warranty below required level
ATTRIBUTION_CONFLICT      - underwriter attribution doesn't match author's declared scope
RACE_CONDITION            - prev_epoch is too stale and policy says retry
```

### 6.2 Status state machine

```
       create_tension          resolve
   ────────────────────►     ────────►
            │                          │
            ▼                          ▼
        Active ◄──────reopen──── Resolved
        │  ▲                          │
        │  │                          │
   release  reopen                    │
        │  │                          │
        ▼  │                          │
       Released ◄────────────────────┘
                                (release-from-resolved)

held / snoozed are orthogonal sub-states of Active
deadlined is a derived signal, not a stored status
```

### 6.3 Replicache parallelism

Mutators are run client-side speculatively (optimistic UI) and server-side canonically. The same Rust function, compiled for both targets. On reconnect, the client rewinds local pending ops, applies server state, replays pending ops. This is the Replicache pattern — see `architecture-synthesis.md` §1.

---

## 7. Sync semantics (preview)

Full sync protocol lives in `werk-serve-protocol-1.0-draft-spec.md`. Summary:

- **Bootstrap**: `GET /spaces/:id/snapshot?at=<epoch>` returns full state at a point. Or `GET /spaces/:id/pull?since=<all-zero>` returns the entire log.
- **Pull (live)**: `GET /spaces/:id/pull?since=<epoch>` is an SSE stream. Initial frames replay the log delta; new frames stream as ops are accepted.
- **Push**: `POST /spaces/:id/push` with an array of op envelopes. Server returns `{accepted: [op_id, ...], rejected: [{op_id, error_code, message}, ...]}`.

Resumability via SSE `Last-Event-ID` header set to the last received epoch.

---

## 8. The non-authoritative-log alternative

If werk decides **not** to make the log authoritative, this spec still has value:

- **As an audit log**: SQLite remains the source of truth; every mutation also writes a log envelope. The log becomes the export/portability/replication mechanism without the storage refactor.
- **As a sync ferrying format**: even with SQLite-as-truth, the sync protocol can ship log envelopes between replicas; each receiver replays them against its own SQLite.
- **As an interop format**: third-party tooling can read the log without understanding SQLite schema.

The cost of the non-authoritative path: drift between SQLite and log is possible, and detecting/repairing it requires tooling that the authoritative-log path makes unnecessary.

The benefit: less invasive refactor of `werk-core`, faster to ship.

---

## 9. Open questions

1. **Encoding choice.** JSON-with-sorted-keys is human-diffable but bulky. CBOR or BARE would be smaller and faster, at the cost of needing a viewer tool. Recommendation: ship JSON for 1.0, evaluate binary encodings for 1.1 if size/speed matters.
2. **Signing.** Should op envelopes be signed by the author's key for cryptographic non-repudiation? Strong yes for multi-player; arguable for single-player. Proposal: optional `signature` field, server records it in 1.0 but doesn't verify; verification becomes mandatory in 2.0 for shared spaces.
3. **Compaction.** Long-lived spaces accumulate huge logs. Compaction (replace a log prefix with a snapshot) needs clear semantics. Proposal: defer to 1.1 — initial spaces are small enough that this isn't pressing for personal use, but matters for enterprise.
4. **Garbage collection of retracted notes / deleted tensions.** GDPR-style "right to be forgotten" requires log entries to be deletable. Conflict with cryptographic chain. Proposal: tombstone-style — replace payload with `{redacted: true, reason}` while keeping op-level metadata, so chain stays valid.
5. **Cross-space references.** Can a tension in space A link to a tension in space B? If yes, what happens when the linker doesn't have access to B? Proposal: defer; for 1.0 keep tensions space-local.

---

## 10. Status of this draft

This is a starting point for discussion. Decisions that block adoption:

- [ ] Commit to log-as-authoritative (or commit to log-as-audit-trail per §8)
- [ ] Confirm op type set covers v1 needs
- [ ] Pick encoding (JSON vs. CBOR vs. BARE)
- [ ] Decide signing posture (required / optional / deferred)
- [ ] Schedule compaction story (ship in 1.0 or defer to 1.1)

Reference: companion to `cli-patterns-study.md` and `architecture-synthesis.md` in this directory.
