# Quint Language Constraints

**CRITICAL**: These are fundamental limitations of the Quint language. Violating these constraints will result in compilation errors that cannot be worked around.

## 1. String Manipulation is NOT Supported

Quint treats strings as **opaque values** for comparison only.

- **NOT ALLOWED**: concatenation, interpolation, indexing, methods, conversion
- **ALLOWED**: string literals, comparison (`==`), as map keys or set elements

Use sum types or records instead of string manipulation.

## 2. Nested Pattern Matching is NOT Supported

Match one level at a time. Use sequential `match` statements or intermediate bindings.

```quint
// YES: match outer, then inner
match msg
  | Request(inner) => match inner
    | Prepare(n, v) => ...
```

## 3. Destructuring is NOT Supported

Use explicit field access (`.field`, `._1`, `._2`) or `match` expressions.

## 4. Reserved Words to Avoid as Identifiers

`temporal`, `field`, `to` (built-in operator), `and`, `or`, `iff`, `implies`, `leadsTo`, `match`

## 5. Map Operations

- `m.keys().contains(k)` — NOT `m.has(k)` (doesn't exist)
- `m.put(k, v)` — insert or update (use for new keys)
- `m.set(k, v)` — update existing key only
- Record spread goes at the END: `{ field: val, ...record }`
- Empty map: `Map()`

## 6. Cross-File Imports

```quint
import types.* from "./types"   // WITH quotes, WITHOUT .qnt extension
```

## 7. Module Names

Module names cannot be reserved words. Use `timeCalculus` not `temporal`.

## 8. Action Effect Consistency

All branches of a `match` in an action must have the same effect (same set of state variable assignments). If one branch updates state, all must — or extract the guard to a `pure def`.

## 9. No Loops, No Mutable Variables, No Early Returns

Use recursion or set comprehensions. Functions have a single expression body.
