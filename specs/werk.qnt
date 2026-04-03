// werk.qnt — Top-level composition: the full werk state machine.
//
// Composes the specification into a single system with a step relation
// and combined invariants for model checking.
//
// Simulate:    quint run specs/werk.qnt --main=werk --max-samples=10000
// Verify:      quint run specs/werk.qnt --main=werk --invariant=systemInvariant
// Explore:     quint -r specs/werk.qnt::werk

module werk {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var tensions: str -> Tension
  var edges: Set[Edge]
  var now: int

  // =========================================================================
  // Constants (bounded model checking)
  // =========================================================================

  pure val IDS = Set("t1", "t2", "t3", "t4", "t5")
  pure val DESIRES = Set("ship feature", "fix bug", "write docs", "refactor")
  pure val ACTUALS = Set("", "started", "halfway", "ship feature")
  pure val DEADLINES = Set(500, 700, 900, 1100)
  pure val POSITIONS = Set(1, 2, 3, 4, 5)

  // =========================================================================
  // Init
  // =========================================================================

  action init = all {
    tensions' = Map(),
    edges' = Set(),
    now' = 100,
  }

  // =========================================================================
  // Actions
  // =========================================================================

  action createTension(id: str, desired: str, actual: str): bool = all {
    id.in(IDS),
    not(id.in(tensions.keys())),
    desired != "",
    tensions' = tensions.put(id, {
      id: id, desired: desired, actual: actual,
      status: Active, horizon: NoHorizon,
      position: Unpositioned, created_at: now,
    }),
    edges' = edges, now' = now,
  }

  action createChild(childId: str, parentId: str, desired: str): bool = all {
    childId.in(IDS),
    not(childId.in(tensions.keys())),
    parentId.in(tensions.keys()),
    parentId != childId,
    desired != "",
    tensions' = tensions.put(childId, {
      id: childId, desired: desired, actual: "",
      status: Active, horizon: NoHorizon,
      position: Unpositioned, created_at: now,
    }),
    edges' = edges.union(Set({ from_id: parentId, to_id: childId, edge_type: Contains })),
    now' = now,
  }

  action resolveTension(id: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { status: Resolved, ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action releaseTension(id: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { status: Released, ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action reopenTension(id: str): bool = all {
    id.in(tensions.keys()),
    or { tensions.get(id).status == Resolved, tensions.get(id).status == Released },
    tensions' = tensions.set(id, { status: Active, ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action setDesired(id: str, newDesired: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    newDesired != "",
    tensions' = tensions.set(id, { desired: newDesired, ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action setActual(id: str, newActual: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    tensions' = tensions.set(id, { actual: newActual, ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action setHorizon(id: str, deadline: int): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    deadline.in(DEADLINES),
    tensions' = tensions.set(id, { horizon: SomeHorizon(deadline), ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action setPosition(id: str, pos: int): bool = all {
    id.in(tensions.keys()),
    pos.in(POSITIONS),
    // No sibling collision.
    val parentEdges = edges.filter(e => e.edge_type == Contains and e.to_id == id)
    if (parentEdges.size() == 1) {
      val parentId = parentEdges.fold("", (_, e) => e.from_id)
      val siblings = edges.filter(e => e.edge_type == Contains and e.from_id == parentId)
                          .map(e => e.to_id).exclude(Set(id))
      siblings.forall(sib => {
        match tensions.get(sib).position {
          | Positioned(p) => p != pos
          | Unpositioned => true
        }
      })
    } else true,
    tensions' = tensions.set(id, { position: Positioned(pos), ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action moveTension(childId: str, newParentId: str): bool = {
    val oldParentEdges = edges.filter(e => e.edge_type == Contains and e.to_id == childId)
    all {
      childId.in(tensions.keys()),
      newParentId.in(tensions.keys()),
      childId != newParentId,
      oldParentEdges.size() == 1,
      val oldEdge = oldParentEdges.fold(
        { from_id: "", to_id: "", edge_type: Contains },
        (_, e) => e
      )
      all {
        edges' = edges.exclude(Set(oldEdge)).union(Set({ from_id: newParentId, to_id: childId, edge_type: Contains })),
        tensions' = tensions, now' = now,
      }
    }
  }

  action tick: bool = all {
    now' = now + 100,
    tensions' = tensions, edges' = edges,
  }

  // =========================================================================
  // Step
  // =========================================================================

  action step = any {
    nondet id = IDS.oneOf()
    nondet desired = DESIRES.oneOf()
    nondet actual = ACTUALS.oneOf()
    nondet deadline = DEADLINES.oneOf()
    nondet pos = POSITIONS.oneOf()
    nondet parentId = IDS.oneOf()
    any {
      createTension(id, desired, actual),
      createChild(id, parentId, desired),
      resolveTension(id),
      releaseTension(id),
      reopenTension(id),
      setDesired(id, desired),
      setActual(id, actual),
      setHorizon(id, deadline),
      setPosition(id, pos),
      moveTension(id, parentId),
      tick,
    }
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  val containsEdges: Set[Edge] = edges.filter(e => e.edge_type == Contains)

  // Sacred Core: desired is never empty.
  val desiredNeverEmpty: bool =
    tensions.keys().forall(id => tensions.get(id).desired != "")

  // Forest: at most one parent per tension.
  val singleParent: bool =
    tensions.keys().forall(id =>
      containsEdges.filter(e => e.to_id == id).size() <= 1
    )

  // No self-referencing edges.
  val noSelfEdges: bool =
    edges.forall(e => e.from_id != e.to_id)

  // Edges reference existing tensions.
  val edgesValid: bool =
    edges.forall(e => e.from_id.in(tensions.keys()) and e.to_id.in(tensions.keys()))

  // Status is always valid.
  val statusValid: bool =
    tensions.keys().forall(id => {
      val s = tensions.get(id).status
      or { s == Active, s == Resolved, s == Released }
    })

  // Gap magnitude: binary.
  pure def gap(t: Tension): int =
    if (t.desired != t.actual) 1 else 0

  // Sibling position uniqueness.
  val siblingPositionsUnique: bool =
    tensions.keys().forall(pid => {
      val children = containsEdges.filter(e => e.from_id == pid).map(e => e.to_id)
      children.forall(c1 =>
        children.forall(c2 =>
          (c1 == c2) or
          match tensions.get(c1).position {
            | Unpositioned => true
            | Positioned(p1) => match tensions.get(c2).position {
              | Unpositioned => true
              | Positioned(p2) => p1 != p2
            }
          }
        )
      )
    })

  // Containment: child deadline <= parent deadline.
  val noContainmentViolations: bool =
    containsEdges.forall(e => {
      val parent = tensions.get(e.from_id)
      val child = tensions.get(e.to_id)
      match child.horizon {
        | NoHorizon => true
        | SomeHorizon(ch) => match parent.horizon {
          | NoHorizon => true
          | SomeHorizon(ph) => ch <= ph
        }
      }
    })

  // =========================================================================
  // Composite Invariants
  // =========================================================================

  // Core invariant: always holds, enforced by action guards.
  val systemInvariant: bool = and {
    desiredNeverEmpty,
    singleParent,
    noSelfEdges,
    edgesValid,
    statusValid,
    siblingPositionsUnique,
  }

  // Strong invariant: includes containment (detected but not prevented in Rust).
  val strongInvariant: bool = and {
    systemInvariant,
    noContainmentViolations,
  }

}
