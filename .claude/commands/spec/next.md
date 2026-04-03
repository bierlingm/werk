---
command: /spec:next
description: Suggest next steps for werk's Quint specification workflow
---

# Suggest Next Steps

Analyze the current state of werk's Quint specifications and suggest concrete next steps.

## Procedure

1. **Check spec health**
   - Run `quint typecheck specs/werk.qnt` — do all modules compile?
   - Count modules: `ls specs/*.qnt`
   - Check for witness files: `ls specs/*_witnesses.qnt`

2. **Analyze current coverage**
   - Read `specs/werk.qnt` to list defined invariants
   - Check which invariants have been verified recently
   - Compare spec types against `sd-core/src/` types for drift

3. **Detect gaps**
   - New Rust types/fields not yet in `specs/types.qnt`?
   - New actions in `sd-core/src/engine.rs` not modeled in specs?
   - Invariants from `designs/werk-conceptual-foundation.md` not yet specified?

4. **Suggest prioritized next steps**
   - Fix errors (if any)
   - Add missing types/actions
   - Generate witnesses (if none exist)
   - Run invariant checks
   - Evolve specs for new features

Output concrete `quint` commands the user can run.
