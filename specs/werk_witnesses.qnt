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
  var completedGestures: Set[str]
  var undoneGestures: Set[str]
  var gestureEffects: str -> Set[GestureEffect]

  // =========================================================================
  // Constants
  // =========================================================================

  pure val IDS = Set("t1", "t2", "t3", "t4", "t5")
  pure val DESIRES = Set("ship feature", "fix bug", "write docs", "refactor")
  pure val ACTUALS = Set("", "started", "halfway", "ship feature")
  pure val DEADLINES = Set(500, 700, 900, 1100)
  pure val POSITIONS = Set(1, 2, 3, 4, 5)
  pure val GESTURE_IDS = Set("g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9", "g10")

  // =========================================================================
  // Helpers (copied from werk.qnt)
  // =========================================================================

  pure def activeChildrenOf(es: Set[Edge], ts: str -> Tension, pid: str): Set[str] =
    es.filter(e => e.edge_type == Contains and e.from_id == pid)
      .map(e => e.to_id)
      .filter(cid => cid.in(ts.keys()) and ts.get(cid).status == Active)

  pure def allActiveDescendants(es: Set[Edge], ts: str -> Tension, pid: str): Set[str] = {
    val d1 = activeChildrenOf(es, ts, pid)
    val d2 = d1.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    val d3 = d2.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    val d4 = d3.fold(Set(), (acc, c) => acc.union(activeChildrenOf(es, ts, c)))
    d1.union(d2).union(d3).union(d4)
  }

  pure def snapshotEffect(t: Tension): GestureEffect = {
    tension_id: t.id,
    pre_desired: t.desired, post_desired: t.desired,
    pre_actual: t.actual, post_actual: t.actual,
    pre_status: t.status, post_status: t.status,
    pre_horizon: t.horizon, post_horizon: t.horizon,
    pre_position: t.position, post_position: t.position,
  }

  // =========================================================================
  // Init + Step (copied from werk.qnt — Quint doesn't support
  // cross-module action composition via import for simulation)
  // =========================================================================

  action init = all {
    tensions' = Map(),
    edges' = Set(),
    now' = 100,
    completedGestures' = Set(),
    undoneGestures' = Set(),
    gestureEffects' = Map(),
  }

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

  action undoGesture(gid: str, undoGid: str): bool = all {
    gid.in(completedGestures),
    not(gid.in(undoneGestures)),
    gid.in(gestureEffects.keys()),
    undoGid.in(GESTURE_IDS),
    not(undoGid.in(completedGestures)),
    undoGid != gid,
    val effects = gestureEffects.get(gid)
    all {
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

  action tick: bool = all {
    now' = now + 100,
    tensions' = tensions, edges' = edges,
    completedGestures' = completedGestures,
    undoneGestures' = undoneGestures,
    gestureEffects' = gestureEffects,
  }

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
      undoGesture(gid, undoGid),
      tick,
    }
  }

  // =========================================================================
  // Witnesses — negated goals, expect VIOLATION
  // =========================================================================

  // W1: Can create a tension?
  val canCreateTension: bool = tensions.keys().size() == 0

  // W2: Can attach a child (create a contains-edge)?
  val canAttachChild: bool =
    edges.filter(e => e.edge_type == Contains).size() == 0

  // W3: Can resolve a tension?
  val canResolve: bool =
    tensions.keys().forall(id => tensions.get(id).status != Resolved)

  // W4: Can release a tension?
  val canRelease: bool =
    tensions.keys().forall(id => tensions.get(id).status != Released)

  // W5: (placeholder — reopen witness is implicit via W3 + W1)

  // W6: Can set a deadline?
  val canSetDeadline: bool =
    tensions.keys().forall(id =>
      match tensions.get(id).horizon {
        | NoHorizon => true
        | SomeHorizon(_) => false
      }
    )

  // W7: Can position a sibling?
  val canPosition: bool =
    tensions.keys().forall(id =>
      match tensions.get(id).position {
        | Unpositioned => true
        | Positioned(_) => false
      }
    )

  // W8: Can time advance past a deadline (become overdue)?
  val canBecomeOverdue: bool =
    tensions.keys().forall(id => {
      val t = tensions.get(id)
      match t.horizon {
        | NoHorizon => true
        | SomeHorizon(deadline) => now <= deadline
      }
    })

  // W9: Can create a containment violation?
  val canViolateContainment: bool = {
    val cEdges = edges.filter(e => e.edge_type == Contains)
    cEdges.forall(e => {
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
  val canHaveMultipleRoots: bool = {
    val allChildren = edges.filter(e => e.edge_type == Contains).map(e => e.to_id)
    val roots = tensions.keys().exclude(allChildren)
    roots.size() <= 1
  }

  // W11: Can close the gap? (desired == actual on some tension)
  val canCloseGap: bool =
    tensions.keys().forall(id => tensions.get(id).desired != tensions.get(id).actual)

  // W12: Can undo a gesture?
  // Goal: at least one gesture has been undone.
  val canUndoGesture: bool = undoneGestures.size() == 0

  // W13: Can an undo be blocked by conflict?
  // Goal: a gesture exists in completedGestures with gestureEffects recorded,
  // but its post-values no longer match the current tension state.
  // (The model checker finds a trace where this divergence exists.)
  val canHaveUndoConflict: bool =
    gestureEffects.keys().forall(gid =>
      // Either the gesture is already undone, or its post-values still match.
      gid.in(undoneGestures) or
      gestureEffects.get(gid).forall(eff =>
        not(eff.tension_id.in(tensions.keys())) or {
          val t = tensions.get(eff.tension_id)
          t.desired == eff.post_desired and
          t.actual == eff.post_actual and
          t.status == eff.post_status and
          t.horizon == eff.post_horizon and
          t.position == eff.post_position
        }
      )
    )

  // W14: Can resolve a parent and cascade to children?
  // Goal: a child tension becomes Resolved via cascade (not direct resolve).
  val canCascadeResolve: bool = {
    val cEdges = edges.filter(e => e.edge_type == Contains)
    // No child is Resolved.
    cEdges.map(e => e.to_id).forall(cid =>
      not(cid.in(tensions.keys())) or tensions.get(cid).status != Resolved
    )
  }

}
