---
command: /verify:witnesses
description: Generate and run witness scenarios for werk's Quint specs
---

# Generate and Run Witnesses

Witness scenarios are negated goals — invariants we EXPECT to be violated,
proving that interesting states are reachable.

## Procedure

1. **Analyze spec for interesting states**
   Read `specs/werk.qnt` and identify:
   - Can tensions be created? (tensions map non-empty)
   - Can children be attached? (edges contain Contains)
   - Can tensions be resolved/released? (status != Active exists)
   - Can deadlines be set? (SomeHorizon exists)
   - Can siblings be positioned? (Positioned exists)
   - Can time advance past deadlines? (urgency > 1.0)
   - Can containment violations occur? (child deadline > parent deadline)

2. **Generate witness module**
   Write `specs/werk_witnesses.qnt`:
   ```quint
   module werkWitnesses {
     import types.* from "./types"
     import werk.* from "./werk"

     // Witnesses: negated goals, expect VIOLATION
     val canCreateTension = tensions.keys().size() == 0
     val canAttachChild = edges.size() == 0
     val canResolve = tensions.keys().forall(id => tensions.get(id).status == Active)
     val canSetDeadline = tensions.keys().forall(id =>
       match tensions.get(id).horizon { | NoHorizon => true | _ => false })
     val canPosition = tensions.keys().forall(id =>
       match tensions.get(id).position { | Unpositioned => true | _ => false })
   }
   ```

3. **Typecheck**
   ```bash
   quint typecheck specs/werk_witnesses.qnt
   ```

4. **Run each witness**
   For each witness:
   ```bash
   quint run specs/werk_witnesses.qnt --main=werkWitnesses \
     --invariant=<witness_name> --max-samples=5000 \
     --max-steps=20 --backend=typescript --verbosity=1
   ```
   Expected: **VIOLATED** (proves reachability)

5. **Report**
   - Violated = scenario reachable (good)
   - Satisfied = scenario NOT reached (investigate spec constraints)
