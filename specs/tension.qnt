// tension.qnt — Tension lifecycle state machine and sacred invariants.
//
// Specifies:
//   - Valid state transitions (Active → Resolved|Released, reopen)
//   - Field update guards (no mutation on inactive tensions)
//   - Gap magnitude (binary, not fuzzy)

module tension {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var tensions: str -> Tension

  // =========================================================================
  // State Transitions
  // =========================================================================

  // Valid transition check (no tuple match — use nested if).
  pure def canTransition(fromStatus: TensionStatus, toStatus: TensionStatus): bool = {
    match fromStatus {
      | Active => match toStatus {
        | Resolved => true
        | Released => true
        | Active => false
      }
      | Resolved => match toStatus {
        | Active => true
        | _ => false
      }
      | Released => match toStatus {
        | Active => true
        | _ => false
      }
    }
  }

  action resolve(id: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { status: Resolved, ...tensions.get(id) }),
  }

  action release(id: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { status: Released, ...tensions.get(id) }),
  }

  action reopen(id: str): bool = all {
    id.in(tensions.keys()),
    or {
      tensions.get(id).status == Resolved,
      tensions.get(id).status == Released,
    },
    tensions' = tensions.set(id, { status: Active, ...tensions.get(id) }),
  }

  // =========================================================================
  // Field Updates
  // =========================================================================

  action updateDesired(id: str, newDesired: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    newDesired != "",
    tensions' = tensions.set(id, { desired: newDesired, ...tensions.get(id) }),
  }

  action updateActual(id: str, newActual: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    newActual != "",
    tensions' = tensions.set(id, { actual: newActual, ...tensions.get(id) }),
  }

  action updatePosition(id: str, newPos: Position): bool = all {
    id.in(tensions.keys()),
    tensions' = tensions.set(id, { position: newPos, ...tensions.get(id) }),
  }

  action updateHorizon(id: str, newHorizon: Horizon): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { horizon: newHorizon, ...tensions.get(id) }),
  }

  action create(id: str, desired: str, actual: str, ts: int): bool = all {
    not(id.in(tensions.keys())),
    desired != "",
    tensions' = tensions.put(id, {
      id: id,
      desired: desired,
      actual: actual,
      status: Active,
      horizon: NoHorizon,
      position: Unpositioned,
      created_at: ts,
    }),
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  // INV-1: No tension with empty desired state.
  val desiredNeverEmpty: bool =
    tensions.keys().forall(id => tensions.get(id).desired != "")

  // INV-2: Gap magnitude is binary.
  pure def gapMagnitude(t: Tension): int = {
    if (t.desired != t.actual) 1 else 0
  }

}
