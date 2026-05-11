# werk doctor — agent handbook

`werk doctor` diagnoses and (optionally) repairs a werk workspace. It is
designed for AI agents calling without a TTY: safe-by-default, idempotent,
reversible, and self-describing.

## Five things to know

1. `werk doctor` (no flags) NEVER mutates user state. It writes an
   append-only run artifact at `.werk/.doctor/runs/<ULID>/` and updates the
   `.werk/.doctor/latest` symlink. That is the only filesystem effect.
2. Every mutation flows through one chokepoint (`DoctorRun`) that records a
   verbatim backup, BLAKE3 hashes, and an `actions.jsonl` line per write.
3. `werk doctor --fix` and `werk doctor undo <run-id>` are a true inverse
   pair. The undo path restores from `runs/<id>/backups/` byte-for-byte.
4. Exit codes are a fixed dictionary. Always check the exit code first,
   then the JSON envelope.
5. The contract is pinned by `werk doctor capabilities --json`. If you're
   ever unsure what this binary can detect or fix, call that.

## Exit-code dictionary

```
0  healthy                 1  findings_present      2  partial_fix
3  fix_failed_rolled_back  4  refused_unsafe        5  concurrency_lost
6  online_required         64 usage_error           66 no_input
73 cannot_create_output    74 io_error
```

## Verbs

```
werk doctor                        # diagnose; exit 0/1/4
werk doctor --fix --yes            # repair (records backup + actions.jsonl)
werk doctor --dry-run --fix        # print the plan, no execution
werk doctor --robot-triage         # one-call mega-command (always JSON)
werk doctor --json                 # any verb above as machine-readable
werk doctor --explain <id>         # expand evidence for one finding-id
werk doctor undo <run-id|latest>   # restore from runs/<id>/backups/
werk doctor capabilities --json    # full contract (detectors, fixers, codes)
werk doctor health                 # one-line liveness (cheap, for CI cron)
werk doctor robot-docs             # this handbook
werk doctor ls                     # list runs (newest first)
werk doctor diff [<ref>]           # diff stored reports between runs
werk doctor gc --before <ISO> --yes # prune old run dirs (latest preserved)
```

## JSON envelope (read verbs)

```json
{
  "schema_version": 1,
  "verb": "doctor",
  "run_id": "<ulid|null>",
  "exit_code": <int>,
  "data": { /* verb-specific */ }
}
```

`verb: "robot-triage"` is FLAT (no nested `data`). Its keys are
`{schema_version, verb, run_id, exit_code, summary, findings,
actions_planned, recommended_command, capabilities_command}`.

## Run-artifact layout

```
.werk/.doctor/
├── runs/<ULID>/
│   ├── report.json         findings + exit_code + werk_version
│   ├── report.md           human-readable companion
│   ├── actions.jsonl       one line per mutation (op, target, hashes)
│   ├── backups/<path>      verbatim per-file backups
│   └── stderr.log          (future) captured stderr
├── latest -> runs/<ULID>/  atomic symlink
└── scorecard_history.jsonl one line per finalized run
```

## Safety envelope (what doctor will NEVER do)

- Touch `.werk/tensions.json` directly. The DB is authority; flush is the
  recovery substrate.
- Invoke `werk nuke` or delete `.werk/`.
- Edit user-configured `[hooks]` in `werk config`.
- Make network calls without `--online` (no probe currently uses `--online`).
- Bypass the `DoctorRun` chokepoint.

## Current detector / fixer surface (pass-3)

| Detector | Subsystem | Available |
|----------|-----------|-----------|
| `noop_mutations` | store | yes |
| `singleParent` `noSelfEdges` `edgesValid` `siblingPositionsUnique` `noContainmentViolations` | edges | reserved for R-005 |
| `undoneSubsetOfCompleted` | gestures | reserved for R-005 |

| Fixer | Detector | Op | Inverse |
|-------|----------|----|---------|
| `purge_noop_mutations` | `noop_mutations` | `purge_noop_mutations` | `restore_db_from_backup` |

The six Quint-named detectors are wired into `capabilities --json` with
`available: false`. They become available in a follow-up PR (R-005); no
agent-visible contract change is needed when they land.

## Backwards-compat aliases

- `werk stats --health` → equivalent to `werk doctor --only=store`
- `werk stats --health --repair --yes` → equivalent to
  `werk doctor --fix --only=store --yes`

The legacy `--json` envelope of `stats --health` (`{noop_mutations,
purged?, doctor_run_id?}`) is pinned and will not change without a major
version bump.

## Typical agent recipe

1. `werk doctor --robot-triage` → one JSON object with everything needed.
2. If `exit_code == 0`: done.
3. If `exit_code == 1`: parse `recommended_command`, run it.
4. If anything went sideways: `werk doctor undo latest`.
