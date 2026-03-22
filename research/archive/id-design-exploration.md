# Tension ID Design Exploration

**Date:** 2026-03-01  
**Tension:** 01KJNKBFHHSSVM0BC9ZSAX3DMD  
**Status:** Active

## Problem Statement

Current werk uses ULIDs for tension IDs. The tree view displays abbreviated 8-character prefixes (`01KJNAA8`). When multiple tensions are created close together in time, they share the same timestamp prefix, leading to display collisions.

Example collision from `werk tree`:
```
├── [G]Active 01KJNAA8 → Have a concrete ordeal preparation arc (physica...)
│   └── [G]Active 01KJNAA8 → Have a finished personal coat of arms (front + ...)
```

The full ULIDs (26 characters, 16 bytes of entropy) are unique, but the 8-char display abbreviation is not guaranteed unique for tensions created within ~5 seconds of each other.

---

## Design Goals

1. **Retrievability** — Users can quickly find/specify tensions via fuzzy search
2. **Visual Identity** — Each tension has a sigil-like visual representation
3. **Namespace Isolation** — Workspaces don't collide
4. **Provenance** — Identity-aware IDs (git-based or explicit)
5. **Memorability** (optional but helpful) — Human-friendly references

---

## Inventive Approaches from the Wild

### 1. Proquints — Human-Pronounceable IDs
- **Pattern:** CVCVC syllables (consonant-vowel-consonant-vowel-consonant)
- **Example:** `lusab-babad` = 127.0.0.1
- **Use:** Maps 16-bit chunks to pronounceable "words"
- **Source:** Daniel Shawcross Wilkerson (2009), IETF draft

### 2. Urbit Sigils — Generative Visual Identity
- **Pattern:** 2×2 tile grids constructed from phonemes
- **Capacity:** 4.2 billion unique IDs → each gets a distinct sigil
- **Method:** Syllables (`fal-lyn-bal-fus`) map to tile combinations
- **Influences:** Japanese kamon, East Asian seals, maritime signal flags
- **Key Insight:** Hand-curated phoneme library + algorithmic composition

### 3. Semantic Hashing — Content-Nearby Addressing
- **Pattern:** Similar content → similar addresses
- **Mechanism:** Deep autoencoder → binary bottleneck → semantic address space
- **Benefit:** "Find documents like this" by address locality
- **Source:** Ruslan Salakhutdinov & Geoffrey Hinton (2007)

### 4. Chaos Magic Sigils — Vowel-Stripping Tradition
- **Pattern:** Desire phrase → vowel strip → dedupe consonants → glyph
- **Example:** "I WILL BECOME WEALTHY" → "WLBCMWLTHY" → unique sigil
- **Source:** Austin Osman Spare method (early 1900s)
- **Key:** Content-derived condensation for symbolic focus

### 5. ZFS/CAS — Content-Addressable Storage
- **Pattern:** Hash(content) = address
- **Property:** Same data, same ID anywhere in the world
- **Example:** IPFS CIDs with multihash + codec prefix
- **Benefit:** Self-certifying identifiers

---

## Hybrid Approaches Considered

| Approach | Structure | Collision Resistance |
|----------|-----------|---------------------|
| Timestamp+Content | `01KJ` + `hash(desired+actual)` | High (time + entropy) |
| Sigil IDs | `WLBC` + runic composite | Medium (condensed content) |
| Proquint ULID | `lusab-babad-gusid` | High (pronounceable entropy) |
| Semantic ID | Neural embedding → binary | Near-perfect |

---

## The Hyper Geometry

Multi-dimensional addressing where any subset of dimensions resolves to the tension:

```
IDENTITY × NAMESPACE × TIME × CONTENT → ID → SIGIL
   │           │         │       │
   │           │         │       └── "ordeal preparation arc"
   │           │         └────────── 2025-03-01T14:32:18.472Z
   │           └────────────────────── ~/werk/people/catton-nicholas
   └────────────────────────────────── moritz@werk.or / ~ridlur-figbud
```

Query paths:
- `~moritz/*` → all my tensions (identity)
- `*/catton-nicholas/*` → this workspace (namespace)
- `orde pr` → fuzzy on content (text)
- `ᚱᛞᛚᛈ` → visual sigil lookup (glyph)

---

## Sigil Construction Algorithm (Proposed)

```
"Have a concrete ordeal preparation arc"
→ strip vowels: "HV  C NCR T  RDL PRPR T N RC"
→ dedupe cons: "HVNCRT RDLPR TNRC"
→ take first 4: "HVNC"
→ runic mapping: ᚺᚢᛜᚲ (or ᚱᛞᛚᛈ for "ordeal prep")
```

**Design Decision:** Same text = same sigil (deterministic, chaos-magic style). The sigil represents the *type* of work, not the instance. Two tensions with identical desires share the sigil base, distinguished by full path/ID.

---

## Fuzzy Search Strategy

### Current (No TUI)
The beloved CLI pattern:
```bash
# External fzf integration
werk show $(werk list --ids | fzf)

# Shell completion
werk show orde<TAB>  # completes to matching tension ID
```

### Future (With TUI)
Ratatui-powered command palette as in `sc` (structural coherence):
- Live filtering on desired/actual text
- Structural weight ranking (recency, root priority)
- Rich labels with badges and minibar

---

## Identity Integration

**Git-based onboarding:**
1. Detect `git config user.email` / `user.name`
2. Offer to create identity if none detected
3. Identity becomes root of all IDs

**Extended ID formats:**
```
01KJNAA8-moritz-2025       # extended ULID
~moritz/01KJNAA8           # hierarchical
~moritz/catton-nicholas/ordeal/01KJNAA8  # full path
```

---

## Open Questions

1. **ID permanence:** If desire text changes, does sigil change? (Probably not — identity should persist)
2. **Collision handling:** Dynamic prefix extension vs fixed longer prefix
3. **CLI vs TUI:** Fuzzy search via shell completion (now) vs built-in palette (future)
4. **Namespace depth:** Flat vs hierarchical workspace structure

---

## References

- [Proquints: Readable, Spellable, and Pronounceable Identifiers](https://datatracker.ietf.org/doc/html/draft-rayner-proquint-00)
- [Creating Sigils (Urbit)](https://urbit.org/blog/creating-sigils)
- [Semantic Hashing (Salakhutdinov & Hinton)](http://www.cs.toronto.edu/~fritz/absps/sh.pdf)
- [IPFS Content Identifiers](https://docs.ipfs.tech/concepts/content-addressing/)
- Austin Osman Spare, *The Book of Pleasure* (self-love, the psychology of ecstasy)
