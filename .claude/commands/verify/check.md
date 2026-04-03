---
command: /verify:check
description: Run invariant checks on werk's Quint specifications
---

# Verify Specifications

Run all invariant checks on the werk Quint specs.

## Procedure

1. **Typecheck all modules**
   ```bash
   quint typecheck specs/werk.qnt
   ```

2. **Check core invariant** (should always pass)
   ```bash
   quint run specs/werk.qnt --main=werk --max-samples=10000 \
     --invariant=systemInvariant --backend=typescript --verbosity=1
   ```

3. **Check strong invariant** (expected to find containment violations)
   ```bash
   quint run specs/werk.qnt --main=werk --max-samples=5000 \
     --invariant=strongInvariant --backend=typescript --verbosity=1
   ```

4. **Check individual invariants** (for detailed diagnostics)
   For each of: `desiredNeverEmpty`, `singleParent`, `noSelfEdges`, `edgesValid`,
   `statusValid`, `siblingPositionsUnique`:
   ```bash
   quint run specs/werk.qnt --main=werk --max-samples=5000 \
     --invariant=<name> --backend=typescript --verbosity=1
   ```

5. **Report results**
   - List pass/fail for each invariant
   - For violations: reproduce with `--seed=<seed> --verbosity=5`
   - Explain what the counterexample means for the Rust code

If `$ARGS` contains a specific invariant name, check only that one.
