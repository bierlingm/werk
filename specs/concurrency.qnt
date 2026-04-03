// concurrency.qnt — Concurrent writer model for fsqlite MVCC.
//
// Werk uses fsqlite with concurrent_mode=ON. Multiple processes can
// write simultaneously; conflicts are detected at the page level and
// retried with exponential backoff.
//
// Specifies:
//   - Two concurrent writers interleaving mutations
//   - Safety: per-tension mutation history is totally ordered
//   - Liveness: writers eventually succeed (bounded retries)

module concurrency {

  import types.* from "./types"

  // =========================================================================
  // State
  // =========================================================================

  var committedMutations: List[Mutation]
  var writerA: List[Mutation]
  var writerB: List[Mutation]

  type WriterStatus = Idle | Writing | CommittingNow | Retrying(int)
  var statusA: WriterStatus
  var statusB: WriterStatus
  var clock: int

  // =========================================================================
  // Helpers
  // =========================================================================

  pure def tensionIds(muts: List[Mutation]): Set[str] =
    muts.foldl(Set(), (acc, m) => acc.union(Set(m.tension_id)))

  // =========================================================================
  // Actions
  // =========================================================================

  action writerABegin(tensionId: str, fieldName: str, oldVal: str, newVal: str): bool = all {
    statusA == Idle,
    statusA' = Writing,
    writerA' = writerA.append({
      tension_id: tensionId, timestamp: clock,
      field: fieldName, old_value: oldVal, new_value: newVal,
      gesture_id: "",
    }),
    clock' = clock + 1,
    writerB' = writerB, statusB' = statusB,
    committedMutations' = committedMutations,
  }

  action writerACommit: bool = all {
    statusA == Writing,
    or {
      statusB != CommittingNow,
      tensionIds(writerA).intersect(tensionIds(writerB)) == Set(),
    },
    committedMutations' = committedMutations.concat(writerA),
    writerA' = List(),
    statusA' = Idle,
    writerB' = writerB, statusB' = statusB, clock' = clock,
  }

  action writerAConflict: bool = all {
    statusA == Writing,
    statusB == CommittingNow,
    tensionIds(writerA).intersect(tensionIds(writerB)) != Set(),
    statusA' = Retrying(1),
    writerA' = writerA, writerB' = writerB, statusB' = statusB,
    committedMutations' = committedMutations, clock' = clock,
  }

  // Guard: is writer A in a retryable state?
  pure def isRetryingA(s: WriterStatus): bool = match s {
    | Retrying(n) => n < 10
    | _ => false
  }

  action writerARetry: bool = all {
    isRetryingA(statusA),
    statusA' = Writing,
    writerA' = writerA, writerB' = writerB, statusB' = statusB,
    committedMutations' = committedMutations, clock' = clock,
  }

  action writerBBegin(tensionId: str, fieldName: str, oldVal: str, newVal: str): bool = all {
    statusB == Idle,
    statusB' = Writing,
    writerB' = writerB.append({
      tension_id: tensionId, timestamp: clock,
      field: fieldName, old_value: oldVal, new_value: newVal,
      gesture_id: "",
    }),
    clock' = clock + 1,
    writerA' = writerA, statusA' = statusA,
    committedMutations' = committedMutations,
  }

  action writerBCommit: bool = all {
    statusB == Writing,
    or {
      statusA != CommittingNow,
      tensionIds(writerA).intersect(tensionIds(writerB)) == Set(),
    },
    committedMutations' = committedMutations.concat(writerB),
    writerB' = List(),
    statusB' = Idle,
    writerA' = writerA, statusA' = statusA, clock' = clock,
  }

  // =========================================================================
  // Invariants
  // =========================================================================

  // INV-C1: Per-tension mutation history is totally ordered by timestamp.
  val perTensionTotalOrder: bool = {
    committedMutations.indices().forall(i =>
      committedMutations.indices().forall(j =>
        if (i < j and
            committedMutations[i].tension_id == committedMutations[j].tension_id) {
          committedMutations[i].timestamp < committedMutations[j].timestamp
        } else true
      )
    )
  }

  // INV-C2: Retries are bounded.
  val retriesBounded: bool = and {
    match statusA {
      | Retrying(n) => n <= 10
      | _ => true
    },
    match statusB {
      | Retrying(n) => n <= 10
      | _ => true
    },
  }

}
