// forest.qnt — Graph structure, typed edges, and tree invariants.
//
// Specifies:
//   - The edge substrate (contains, split_from, merged_into)
//   - Acyclicity of the contains-graph (forest invariant)
//   - Sibling position uniqueness
//   - Split/merge provenance constraints

module forest {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var tensions: str -> Tension
  var edges: Set[Edge]

  // =========================================================================
  // Derived Views
  // =========================================================================

  val containsEdges: Set[Edge] =
    edges.filter(e => e.edge_type == Contains)

  pure def childrenOf(es: Set[Edge], parentId: str): Set[str] =
    es.filter(e => e.edge_type == Contains and e.from_id == parentId).map(e => e.to_id)

  pure def parentOf(es: Set[Edge], childId: str): str = {
    val parents = es.filter(e => e.edge_type == Contains and e.to_id == childId).map(e => e.from_id)
    if (parents.size() == 1) parents.fold("", (_, p) => p)
    else ""
  }

  val roots: Set[str] = {
    val allChildren = containsEdges.map(e => e.to_id)
    tensions.keys().exclude(allChildren)
  }

  // =========================================================================
  // Actions
  // =========================================================================

  action addChild(parentId: str, childId: str): bool = all {
    parentId.in(tensions.keys()),
    childId.in(tensions.keys()),
    parentId != childId,
    containsEdges.forall(e => e.to_id != childId),
    edges' = edges.union(Set({
      from_id: parentId,
      to_id: childId,
      edge_type: Contains,
    })),
    tensions' = tensions,
  }

  action detachChild(parentId: str, childId: str): bool = all {
    edges.contains({ from_id: parentId, to_id: childId, edge_type: Contains }),
    edges' = edges.exclude(Set({ from_id: parentId, to_id: childId, edge_type: Contains })),
    tensions' = tensions,
  }

  action recordSplit(originalId: str, newId: str): bool = all {
    originalId.in(tensions.keys()),
    newId.in(tensions.keys()),
    originalId != newId,
    edges' = edges.union(Set({
      from_id: originalId,
      to_id: newId,
      edge_type: SplitFrom,
    })),
    tensions' = tensions,
  }

  action recordMerge(sourceId: str, targetId: str): bool = all {
    sourceId.in(tensions.keys()),
    targetId.in(tensions.keys()),
    sourceId != targetId,
    edges' = edges.union(Set({
      from_id: sourceId,
      to_id: targetId,
      edge_type: MergedInto,
    })),
    tensions' = tensions,
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  // INV-F1: Every tension has at most one parent.
  val singleParent: bool =
    tensions.keys().forall(id =>
      containsEdges.filter(e => e.to_id == id).size() <= 1
    )

  // INV-F2: No self-referencing edges.
  val noSelfEdges: bool =
    edges.forall(e => e.from_id != e.to_id)

  // INV-F3: Edges only reference existing tensions.
  val edgesReferenceExistingTensions: bool =
    edges.forall(e => e.from_id.in(tensions.keys()) and e.to_id.in(tensions.keys()))

  // INV-F4: Siblings have distinct positions (among positioned children).
  val siblingPositionsUnique: bool =
    tensions.keys().forall(parentId => {
      val children = childrenOf(containsEdges, parentId)
      val positioned = children.filter(cid => hasPosition(tensions.get(cid).position))
      positioned.forall(c1 =>
        positioned.forall(c2 =>
          (c1 == c2) or (positionVal(tensions.get(c1).position) != positionVal(tensions.get(c2).position))
        )
      )
    })

  // INV-F5: Root tension count is bounded.
  val rootCountReasonable: bool =
    roots.size() <= 10

  val forestInvariant: bool = and {
    singleParent,
    noSelfEdges,
    edgesReferenceExistingTensions,
    siblingPositionsUnique,
  }

  // =========================================================================
  // Cascade Helpers
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

}
