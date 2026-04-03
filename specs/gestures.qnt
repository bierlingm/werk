// gestures.qnt — Gesture semantics: atomicity, coherence, and composition.
//
// A gesture is the unit of meaningful action. It groups one or more mutations
// into a single intentional act. Specifies:
//   - Gesture boundaries (begin/end lifecycle)
//   - Split and merge as compound gestures with provenance

module gestures {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var tensions: str -> Tension
  var edges: Set[Edge]
  var mutations: List[Mutation]
  var activeGesture: str
  var completedGestures: Set[str]

  // =========================================================================
  // Gesture Lifecycle
  // =========================================================================

  action beginGesture(gestureId: str): bool = all {
    activeGesture == "",
    gestureId != "",
    not(gestureId.in(completedGestures)),
    activeGesture' = gestureId,
    completedGestures' = completedGestures,
    tensions' = tensions,
    edges' = edges,
    mutations' = mutations,
  }

  action endGesture: bool = all {
    activeGesture != "",
    completedGestures' = completedGestures.union(Set(activeGesture)),
    activeGesture' = "",
    tensions' = tensions,
    edges' = edges,
    mutations' = mutations,
  }

  action mutate(tensionId: str, fieldName: str, oldVal: str, newVal: str, ts: int): bool = all {
    tensionId.in(tensions.keys()),
    mutations' = mutations.append({
      tension_id: tensionId,
      timestamp: ts,
      field: fieldName,
      old_value: oldVal,
      new_value: newVal,
      gesture_id: activeGesture,
    }),
    tensions' = tensions,
    edges' = edges,
    activeGesture' = activeGesture,
    completedGestures' = completedGestures,
  }

  // =========================================================================
  // Compound Gestures
  // =========================================================================

  // Split: one tension becomes two.
  action split(sourceId: str, newId1: str, newId2: str,
               desire1: str, desire2: str, ts: int): bool = all {
    sourceId.in(tensions.keys()),
    tensions.get(sourceId).status == Active,
    not(newId1.in(tensions.keys())),
    not(newId2.in(tensions.keys())),
    newId1 != newId2,
    activeGesture != "",
    val source = tensions.get(sourceId)
    val t1: Tension = {
      id: newId1, desired: desire1, actual: source.actual,
      status: Active, horizon: source.horizon,
      position: Unpositioned, created_at: ts,
    }
    val t2: Tension = {
      id: newId2, desired: desire2, actual: source.actual,
      status: Active, horizon: source.horizon,
      position: Unpositioned, created_at: ts,
    }
    all {
      tensions' = tensions
        .set(sourceId, { status: Resolved, ...source })
        .put(newId1, t1)
        .put(newId2, t2),
      edges' = edges.union(Set(
        { from_id: sourceId, to_id: newId1, edge_type: SplitFrom },
        { from_id: sourceId, to_id: newId2, edge_type: SplitFrom }
      )),
      mutations' = mutations,
      activeGesture' = activeGesture,
      completedGestures' = completedGestures,
    }
  }

  // Merge: one tension absorbs another.
  action mergeInto(absorbedId: str, absorberId: str): bool = all {
    absorbedId.in(tensions.keys()),
    absorberId.in(tensions.keys()),
    tensions.get(absorbedId).status == Active,
    tensions.get(absorberId).status == Active,
    absorbedId != absorberId,
    activeGesture != "",
    tensions' = tensions.set(absorbedId, {
      status: Resolved, ...tensions.get(absorbedId)
    }),
    edges' = edges.union(Set({
      from_id: absorbedId,
      to_id: absorberId,
      edge_type: MergedInto,
    })),
    mutations' = mutations,
    activeGesture' = activeGesture,
    completedGestures' = completedGestures,
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  // INV-G1: A completed gesture is never reactivated.
  val gestureIdsUnique: bool =
    activeGesture == "" or not(activeGesture.in(completedGestures))

  // INV-G2: Split provenance edges reference existing tensions.
  val splitProvenanceValid: bool =
    edges.filter(e => e.edge_type == SplitFrom).forall(e =>
      e.from_id.in(tensions.keys()) and e.to_id.in(tensions.keys())
    )

  // INV-G3: Merge source is always resolved.
  val mergeProvenanceValid: bool =
    edges.filter(e => e.edge_type == MergedInto).forall(e =>
      e.from_id.in(tensions.keys()) and e.to_id.in(tensions.keys()) and
      tensions.get(e.from_id).status == Resolved
    )

}
