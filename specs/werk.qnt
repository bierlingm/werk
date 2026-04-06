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

  // Gesture tracking for undo support.
  var completedGestures: Set[str]
  var undoneGestures: Set[str]
  var gestureEffects: str -> Set[GestureEffect]

  // =========================================================================
  // Constants (bounded model checking)
  // =========================================================================

  pure val IDS = Set("t1", "t2", "t3", "t4", "t5")
  pure val DESIRES = Set("ship feature", "fix bug", "write docs", "refactor")
  pure val ACTUALS = Set("", "started", "halfway", "ship feature")
  pure val DEADLINES = Set(500, 700, 900, 1100)
  pure val POSITIONS = Set(1, 2, 3, 4, 5)
  pure val GESTURE_IDS = Set("g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9", "g10")

  // =========================================================================
  // Helpers
  // =========================================================================

  // Direct active children of a parent.
  pure def activeChildrenOf(es: Set[Edge], ts: str -> Tension, pid: str): Set[str] =
    es.filter(e => e.edge_type == Contains and e.from_id == pid)
      .map(e => e.to_id)
      .filter(cid => cid.in(ts.keys()) and ts.get(cid).status == Active)

  // All active descendants reachable through chains of active nodes.
  // Bounded to 4 levels (max depth with 5 IDs).
  pure def allActiveDescendants(es: Set[Edge], ts: str -> Tension, pid: str): Set[str] = {
    val d1 = activeChildrenOf(es, ts, pid)
    val d2 = d1.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    val d3 = d2.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    val d4 = d3.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    d1.union(d2).union(d3).union(d4)
  }

  // Snapshot a tension into a GestureEffect with identical pre/post (no change).
  pure def snapshotEffect(t: Tension): GestureEffect = {
    tension_id: t.id,
    pre_desired: t.desired, post_desired: t.desired,
    pre_actual: t.actual, post_actual: t.actual,
    pre_status: t.status, post_status: t.status,
    pre_horizon: t.horizon, post_horizon: t.horizon,
    pre_position: t.position, post_position: t.position,
  }

  // =========================================================================
  // Init
  // =========================================================================

  action init = all {
    tensions' = Map(),
    edges' = Set(),
    now' = 100,
    completedGestures' = Set(),
    undoneGestures' = Set(),
    gestureEffects' = Map(),
  }

  // =========================================================================
  // Actions
  // =========================================================================

  // --- Creation (not undoable in v1 — no GestureEffect recorded) ---

  action createTension(id: str, desired: str, actual: str, gid: str): bool = all {
    id.in(IDS),
    not(id.in(tensions.keys())),
    desired != "",
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    tensions' = tensions.put(id, {
      id: id, desired: desired, actual: actual,
      status: Active, horizon: NoHorizon,
      position: Unpositioned, created_at: now,
    }),
    completedGestures' = completedGestures.union(Set(gid)),
    undoneGestures' = undoneGestures,
    gestureEffects' = gestureEffects,
    edges' = edges, now' = now,
  }

  action createChild(childId: str, parentId: str, desired: str, gid: str): bool = all {
    childId.in(IDS),
    not(childId.in(tensions.keys())),
    parentId.in(tensions.keys()),
    parentId != childId,
    desired != "",
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    tensions' = tensions.put(childId, {
      id: childId, desired: desired, actual: "",
      status: Active, horizon: NoHorizon,
      position: Unpositioned, created_at: now,
    }),
    edges' = edges.union(Set({ from_id: parentId, to_id: childId, edge_type: Contains })),
    completedGestures' = completedGestures.union(Set(gid)),
    undoneGestures' = undoneGestures,
    gestureEffects' = gestureEffects,
    now' = now,
  }

  // --- Status transitions (with child cascade on resolve/release) ---

  action resolveTension(id: str, gid: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val descendants = allActiveDescendants(edges, tensions, id)
    val allToResolve = Set(id).union(descendants)
    val effects = allToResolve.map(tid => {
      post_status: Resolved, ...snapshotEffect(tensions.get(tid))
    })
    all {
      tensions' = allToResolve.fold(tensions, (ts, tid) =>
        ts.set(tid, { status: Resolved, ...ts.get(tid) })
      ),
      gestureEffects' = gestureEffects.put(gid, effects),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  action releaseTension(id: str, gid: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val descendants = allActiveDescendants(edges, tensions, id)
    val allToRelease = Set(id).union(descendants)
    val effects = allToRelease.map(tid => {
      post_status: Released, ...snapshotEffect(tensions.get(tid))
    })
    all {
      tensions' = allToRelease.fold(tensions, (ts, tid) =>
        ts.set(tid, { status: Released, ...ts.get(tid) })
      ),
      gestureEffects' = gestureEffects.put(gid, effects),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  action reopenTension(id: str, gid: str): bool = all {
    id.in(tensions.keys()),
    or { tensions.get(id).status == Resolved, tensions.get(id).status == Released },
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val t = tensions.get(id)
    all {
      tensions' = tensions.set(id, { status: Active, ...t }),
      gestureEffects' = gestureEffects.put(gid, Set({
        post_status: Active, ...snapshotEffect(t)
      })),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  // --- Field updates ---

  action setDesired(id: str, newDesired: str, gid: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    newDesired != "",
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val t = tensions.get(id)
    all {
      tensions' = tensions.set(id, { desired: newDesired, ...t }),
      gestureEffects' = gestureEffects.put(gid, Set({
        post_desired: newDesired, ...snapshotEffect(t)
      })),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  action setActual(id: str, newActual: str, gid: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val t = tensions.get(id)
    all {
      tensions' = tensions.set(id, { actual: newActual, ...t }),
      gestureEffects' = gestureEffects.put(gid, Set({
        post_actual: newActual, ...snapshotEffect(t)
      })),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  action setHorizon(id: str, deadline: int, gid: str): bool = all {
    id.in(tensions.keys()),
    tensions.get(id).status == Active,
    deadline.in(DEADLINES),
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
    val t = tensions.get(id)
    all {
      tensions' = tensions.set(id, { horizon: SomeHorizon(deadline), ...t }),
      gestureEffects' = gestureEffects.put(gid, Set({
        post_horizon: SomeHorizon(deadline), ...snapshotEffect(t)
      })),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  action setPosition(id: str, pos: int, gid: str): bool = all {
    id.in(tensions.keys()),
    pos.in(POSITIONS),
    gid.in(GESTURE_IDS),
    not(gid.in(completedGestures)),
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
    val t = tensions.get(id)
    all {
      tensions' = tensions.set(id, { position: Positioned(pos), ...t }),
      gestureEffects' = gestureEffects.put(gid, Set({
        post_position: Positioned(pos), ...snapshotEffect(t)
      })),
      completedGestures' = completedGestures.union(Set(gid)),
      undoneGestures' = undoneGestures,
      edges' = edges, now' = now,
    }
  }

  // --- Structural (not undoable — edge changes, no GestureEffect) ---

  action moveTension(childId: str, newParentId: str, gid: str): bool = {
    val oldParentEdges = edges.filter(e => e.edge_type == Contains and e.to_id == childId)
    all {
      childId.in(tensions.keys()),
      newParentId.in(tensions.keys()),
      childId != newParentId,
      oldParentEdges.size() == 1,
      gid.in(GESTURE_IDS),
      not(gid.in(completedGestures)),
      val oldEdge = oldParentEdges.fold(
        { from_id: "", to_id: "", edge_type: Contains },
        (_, e) => e
      )
      all {
        edges' = edges.exclude(Set(oldEdge)).union(Set({ from_id: newParentId, to_id: childId, edge_type: Contains })),
        tensions' = tensions,
        completedGestures' = completedGestures.union(Set(gid)),
        undoneGestures' = undoneGestures,
        gestureEffects' = gestureEffects,
        now' = now,
      }
    }
  }

  action tick: bool = all {
    now' = now + 100,
    tensions' = tensions, edges' = edges,
    completedGestures' = completedGestures,
    undoneGestures' = undoneGestures,
    gestureEffects' = gestureEffects,
  }

  // --- Undo ---

  action undoGesture(gid: str, undoGid: str): bool = all {
    gid.in(completedGestures),
    not(gid.in(undoneGestures)),
    gid.in(gestureEffects.keys()),
    undoGid.in(GESTURE_IDS),
    not(undoGid.in(completedGestures)),
    undoGid != gid,
    val effects = gestureEffects.get(gid)
    all {
      // Conflict check: current field values must match post-values.
      effects.forall(eff =>
        eff.tension_id.in(tensions.keys()) and {
          val t = tensions.get(eff.tension_id)
          t.desired == eff.post_desired and
          t.actual == eff.post_actual and
          t.status == eff.post_status and
          t.horizon == eff.post_horizon and
          t.position == eff.post_position
        }
      ),
      // Apply reversals: restore pre-values for every affected tension.
      tensions' = effects.fold(tensions, (ts, eff) =>
        ts.set(eff.tension_id, {
          id: eff.tension_id,
          desired: eff.pre_desired,
          actual: eff.pre_actual,
          status: eff.pre_status,
          horizon: eff.pre_horizon,
          position: eff.pre_position,
          created_at: ts.get(eff.tension_id).created_at,
        })
      ),
      // Record the undo gesture with reversed effects.
      gestureEffects' = gestureEffects.put(undoGid, effects.map(eff => {
        tension_id: eff.tension_id,
        pre_desired: eff.post_desired, post_desired: eff.pre_desired,
        pre_actual: eff.post_actual, post_actual: eff.pre_actual,
        pre_status: eff.post_status, post_status: eff.pre_status,
        pre_horizon: eff.post_horizon, post_horizon: eff.pre_horizon,
        pre_position: eff.post_position, post_position: eff.pre_position,
      })),
      undoneGestures' = undoneGestures.union(Set(gid)),
      completedGestures' = completedGestures.union(Set(undoGid)),
      edges' = edges, now' = now,
    }
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
    nondet gid = GESTURE_IDS.oneOf()
    nondet undoGid = GESTURE_IDS.oneOf()
    any {
      createTension(id, desired, actual, gid),
      createChild(id, parentId, desired, gid),
      resolveTension(id, gid),
      releaseTension(id, gid),
      reopenTension(id, gid),
      setDesired(id, desired, gid),
      setActual(id, actual, gid),
      setHorizon(id, deadline, gid),
      setPosition(id, pos, gid),
      moveTension(id, parentId, gid),
      undoGesture(gid, undoGid),
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

  // Gesture: undone gestures are always a subset of completed gestures.
  val undoneSubsetOfCompleted: bool =
    undoneGestures.subseteq(completedGestures)

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
    undoneSubsetOfCompleted,
  }

  // Strong invariant: includes containment (detected but not prevented in Rust).
  val strongInvariant: bool = and {
    systemInvariant,
    noContainmentViolations,
  }

}
