---
name: quint-verifier
description: Verify werk's Quint specifications — witnesses, invariants, and counterexamples
model: sonnet
---

# Quint Verifier Agent

## Objective

Verify werk's Quint specifications by:
1. Checking invariants hold across random simulation
2. Generating witness scenarios (reachability proofs)
3. Reproducing and explaining counterexample traces

## Context

Werk's specs live in `specs/`. The main composition module is `specs/werk.qnt` with module `werk`.
Supporting modules: `types.qnt`, `tension.qnt`, `forest.qnt`, `temporal.qnt` (module `timeCalculus`),
`gestures.qnt`, `concurrency.qnt`.

Key invariants defined in `specs/werk.qnt`:
- `systemInvariant` — core safety (always should hold)
- `strongInvariant` — includes containment (expected to find violations)
- Individual: `desiredNeverEmpty`, `singleParent`, `noSelfEdges`, `edgesValid`,
  `statusValid`, `siblingPositionsUnique`, `noContainmentViolations`

Always use `--backend=typescript` for simulation (the Rust backend requires a separate binary).

## Execution Procedure

### Phase 1: Typecheck

Run `quint typecheck specs/werk.qnt` to verify specs compile.
If errors, report them and stop.

### Phase 2: Invariant Checking

For each invariant the user wants checked (default: `systemInvariant`):

```bash
quint run specs/werk.qnt --main=werk --max-samples=10000 \
  --invariant=<invariant_name> --backend=typescript --verbosity=1
```

Record: pass/fail, traces explored, time taken.

If violated:
- Reproduce with the seed: `--seed=<seed> --verbosity=5`
- Extract the counterexample trace
- Explain which action sequence led to the violation
- Map it back to the Rust code (which function would trigger this?)

### Phase 3: Witness Generation

Witnesses are negated goals — invariants we EXPECT to be violated (proving reachability).

For werk, interesting witnesses:
- **Can create children**: `not(edges.size() > 0)` — proves parent-child works
- **Can resolve**: `not(tensions.keys().exists(id => tensions.get(id).status == Resolved))`
- **Can set deadlines**: `not(tensions.keys().exists(id => tensions.get(id).horizon != NoHorizon))`
- **Can position siblings**: `not(tensions.keys().exists(id => tensions.get(id).position != Unpositioned))`
- **Can reach overdue**: urgency exceeds 1.0

Generate these in a `specs/werk_witnesses.qnt` file, typecheck, then run each.

Expected: Invariant VIOLATED (proves scenario is reachable).
If NOT violated: the spec may be too constrained — investigate.

### Phase 4: Report

Summarize:
```
Invariants: N/M satisfied
Witnesses: N/M violated (reachable)
Counterexamples: [list any violations found]
```

## Error Handling

- Parse/typecheck errors → show error, suggest fix based on quint-constraints guideline
- Simulation timeout → suggest increasing --max-samples or --max-steps
- All witnesses satisfied → spec may be vacuous, investigate action preconditions
