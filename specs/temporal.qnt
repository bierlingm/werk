// temporal.qnt — Calculus of Time: deadline propagation, urgency,
//                containment violations, and temporal signals.
//
// Specifies:
//   - Urgency as elapsed/total ratio (standard of measurement)
//   - Containment: child deadline must not exceed parent deadline
//   - Sequencing consistency between positioned siblings
//   - The standard of measurement principle (no horizon → no urgency)

module timeCalculus {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var tensions: str -> Tension
  var edges: Set[Edge]
  var now: int

  // =========================================================================
  // Urgency
  // =========================================================================

  // Urgency = elapsed / total window.
  // Returns -1 if no horizon (standard of measurement: no basis).
  // Fixed-point x1000: 500 = 0.5, 1000 = 1.0, 1500 = overdue.
  pure def urgency(t: Tension, currentTime: int): int = {
    match t.horizon {
      | NoHorizon => -1
      | SomeHorizon(deadline) => {
        val totalWindow = deadline - t.created_at
        if (totalWindow <= 0) 1000
        else (currentTime - t.created_at) * 1000 / totalWindow
      }
    }
  }

  pure def isOverdue(t: Tension, currentTime: int): bool = {
    urgency(t, currentTime) > 1000
  }

  // =========================================================================
  // Containment
  // =========================================================================

  val containsEdges: Set[Edge] = edges.filter(e => e.edge_type == Contains)

  pure def hasContainmentViolation(ts: str -> Tension, es: Set[Edge], childId: str): bool = {
    val parentEdges = es.filter(e => e.edge_type == Contains and e.to_id == childId)
    if (parentEdges.size() != 1) false
    else {
      val parentId = parentEdges.fold("", (_, e) => e.from_id)
      val child = ts.get(childId)
      val parent = ts.get(parentId)
      match child.horizon {
        | NoHorizon => false
        | SomeHorizon(ch) => match parent.horizon {
          | NoHorizon => false
          | SomeHorizon(ph) => ch > ph
        }
      }
    }
  }

  // =========================================================================
  // Sequencing Consistency
  // =========================================================================

  // If sibling A has position < sibling B, and both have horizons,
  // then A's horizon should not exceed B's.
  pure def sequencingConsistent(ts: str -> Tension, es: Set[Edge], parentId: str): bool = {
    val children = es.filter(e => e.edge_type == Contains and e.from_id == parentId).map(e => e.to_id)
    children.forall(c1 =>
      children.forall(c2 => {
        val t1 = ts.get(c1)
        val t2 = ts.get(c2)
        match t1.position {
          | Unpositioned => true
          | Positioned(p1) => match t2.position {
            | Unpositioned => true
            | Positioned(p2) => match t1.horizon {
              | NoHorizon => true
              | SomeHorizon(h1) => match t2.horizon {
                | NoHorizon => true
                | SomeHorizon(h2) => if (p1 < p2) h1 <= h2 else true
              }
            }
          }
        }
      })
    )
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  // INV-T1: Urgency is -1 for tensions without a horizon.
  val noPhantomUrgency: bool =
    tensions.keys().forall(id => {
      val t = tensions.get(id)
      match t.horizon {
        | NoHorizon => urgency(t, now) == -1
        | SomeHorizon(_) => urgency(t, now) >= 0
      }
    })

  // INV-T2: Containment violations are detectable.
  val containmentViolations: Set[str] =
    tensions.keys().filter(id =>
      tensions.get(id).status == Active and hasContainmentViolation(tensions, edges, id)
    )

  // INV-T3: Overdue active tensions form the must-act set.
  val mustActSet: Set[str] =
    tensions.keys().filter(id => {
      val t = tensions.get(id)
      t.status == Active and isOverdue(t, now)
    })

  // =========================================================================
  // Horizon Drift
  // =========================================================================

  type DriftType = Stable | Tightening | PostponementDrift
                 | RepeatedPostponement | Loosening | Oscillating

  pure def classifyDrift(horizons: List[int]): DriftType = {
    if (horizons.length() <= 1) Stable
    else {
      val last = horizons[horizons.length() - 1]
      val prev = horizons[horizons.length() - 2]
      if (last > prev) {
        if (horizons.length() >= 3) {
          val prevprev = horizons[horizons.length() - 3]
          if (prev > prevprev) RepeatedPostponement
          else PostponementDrift
        } else PostponementDrift
      } else if (last < prev) Tightening
      else Stable
    }
  }

}
