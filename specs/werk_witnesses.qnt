// werk_witnesses.qnt — Reachability witnesses for the werk specification.
//
// These are negated goals: invariants we EXPECT to be VIOLATED.
// Violation proves the scenario is reachable (good!).
// No violation means the scenario may be unreachable (investigate).
//
// Run: quint run specs/werk_witnesses.qnt --main=werkWitnesses \
//        --invariant=<name> --max-samples=5000 --backend=typescript

module werkWitnesses {

  import types.* from "./types"

  // =========================================================================
  // State (mirrors werk.qnt)
  // =========================================================================

  var tensions: str -> Tension
  var edges: Set[Edge]
  var now: int

  // =========================================================================
  // Constants
  // =========================================================================

  pure val IDS = Set("t1", "t2", "t3", "t4", "t5")
  pure val DESIRES = Set("ship feature", "fix bug", "write docs", "refactor")
  pure val ACTUALS = Set("", "started", "halfway", "ship feature")
  pure val DEADLINES = Set(500, 700, 900, 1100)
  pure val POSITIONS = Set(1, 2, 3, 4, 5)

  // =========================================================================
  // Init + Step (copied from werk.qnt — Quint doesn't support
  // cross-module action composition via import for simulation)
  // =========================================================================

  action init = all {
    tensions' = Map(),
    edges' = Set(),
    now' = 100,
  }

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
    val parentEdges = edges.filter(e => e.edge_type == Contains and e.to_id == id)
    if (parentEdges.size() == 1) {
      val parentId = parentEdges.fold("", (_, e) => e.from_id)
      val siblings = edges.filter(e => e.edge_type == Contains and e.from_id == parentId)
                          .map(e => e.to_id).exclude(Set(id))
      siblings.forall(sib => match tensions.get(sib).position {
        | Positioned(p) => p != pos
        | Unpositioned => true
      })
    } else true,
    tensions' = tensions.set(id, { position: Positioned(pos), ...tensions.get(id) }),
    edges' = edges, now' = now,
  }

  action tick: bool = all {
    now' = now + 100,
    tensions' = tensions, edges' = edges,
  }

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
      tick,
    }
  }

  // =========================================================================
  // Witnesses — negated goals, expect VIOLATION
  // =========================================================================

  // W1: Can create a tension?
  // Goal: tensions map becomes non-empty.
  val canCreateTension: bool = tensions.keys().size() == 0

  // W2: Can attach a child (create a contains-edge)?
  // Goal: at least one Contains edge exists.
  val canAttachChild: bool =
    edges.filter(e => e.edge_type == Contains).size() == 0

  // W3: Can resolve a tension?
  // Goal: at least one tension has status Resolved.
  val canResolve: bool =
    tensions.keys().forall(id => tensions.get(id).status != Resolved)

  // W4: Can release a tension?
  // Goal: at least one tension has status Released.
  val canRelease: bool =
    tensions.keys().forall(id => tensions.get(id).status != Released)

  // W5: Can reopen a resolved tension?
  // Goal: a tension transitions Resolved → Active.
  // (We check: exists a tension that was created, then there are >=2 tensions
  //  and at least one is Active — this is a weaker proxy.)

  // W6: Can set a deadline?
  // Goal: at least one tension has a horizon.
  val canSetDeadline: bool =
    tensions.keys().forall(id =>
      match tensions.get(id).horizon {
        | NoHorizon => true
        | SomeHorizon(_) => false
      }
    )

  // W7: Can position a sibling?
  // Goal: at least one tension has a position.
  val canPosition: bool =
    tensions.keys().forall(id =>
      match tensions.get(id).position {
        | Unpositioned => true
        | Positioned(_) => false
      }
    )

  // W8: Can time advance past a deadline (become overdue)?
  // Goal: urgency > 1.0 (now > deadline for some tension).
  val canBecomeOverdue: bool =
    tensions.keys().forall(id => {
      val t = tensions.get(id)
      match t.horizon {
        | NoHorizon => true
        | SomeHorizon(deadline) => now <= deadline
      }
    })

  // W9: Can create a containment violation?
  // Goal: child deadline > parent deadline.
  val canViolateContainment: bool = {
    val containsEdges = edges.filter(e => e.edge_type == Contains)
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
  }

  // W10: Can have multiple root tensions?
  // Goal: more than one root exists.
  val canHaveMultipleRoots: bool = {
    val allChildren = edges.filter(e => e.edge_type == Contains).map(e => e.to_id)
    val roots = tensions.keys().exclude(allChildren)
    roots.size() <= 1
  }

  // W11: Can close the gap? (desired == actual on some tension)
  val canCloseGap: bool =
    tensions.keys().forall(id => tensions.get(id).desired != tensions.get(id).actual)

}
