// types.qnt — Core domain types for the werk specification.
//
// These mirror the Rust types in sd-core but at the specification level:
// we model what matters for invariants, not serialization details.

module types {

  // --- Tension Status ---
  type TensionStatus = Active | Resolved | Released

  // --- Horizon ---
  // Temporal aim. We model as optional end-timestamp (seconds since epoch).
  type Horizon = NoHorizon | SomeHorizon(int)

  // --- Edge Types ---
  type EdgeType = Contains | SplitFrom | MergedInto

  // --- Position ---
  type Position = Unpositioned | Positioned(int)

  // --- Tension ---
  type Tension = {
    id: str,
    desired: str,
    actual: str,
    status: TensionStatus,
    horizon: Horizon,
    position: Position,
    created_at: int,
  }

  // --- Edge ---
  type Edge = {
    from_id: str,
    to_id: str,
    edge_type: EdgeType,
  }

  // --- Mutation ---
  type Mutation = {
    tension_id: str,
    timestamp: int,
    field: str,
    old_value: str,
    new_value: str,
    gesture_id: str,
  }

  // --- Helpers ---
  pure def hasPosition(p: Position): bool = match p {
    | Positioned(_) => true
    | Unpositioned => false
  }

  pure def positionVal(p: Position): int = match p {
    | Positioned(v) => v
    | Unpositioned => -1
  }

}
