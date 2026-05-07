# Expression library evaluation (research artifact)

Conducted during prep, 2026-05-07. Decision recorded as D7 in
`library/sigil-engine-decisions.md`. Pick: **rhai = "1"** in
expression-only mode.

## Candidates evaluated

### evalexpr (https://crates.io/crates/evalexpr)
- Latest: 13.1.0 (2025-11-26). Active.
- License: **AGPL-3.0** (changed from MIT in v12+).
- **Disqualified.** AGPL is incompatible with werk's MIT/Apache stack.

### cel-interpreter (https://crates.io/crates/cel-interpreter)
- Latest: 0.10.0 (2025-07-23). Active.
- License: Apache-2.0.
- Smaller (189 KB source), great for sandboxed/safe-for-untrusted-input
  use cases. Excellent type system.
- **Caveat:** stdlib lacks `sqrt`, `log`, `abs`, `floor`, `ceil`, `min`,
  `max` — would need ~10 custom functions registered. Minor friction.
- Strong fallback if Rhai turns out heavy.

### rhai (https://crates.io/crates/rhai) — **PRIMARY**
- Latest: 1.24.0 (2026-01-19). Very active.
- License: MIT OR Apache-2.0 (dual).
- `Engine::compile_expression` enforces expression-only at parse time
  (no statements, no `let`, no loops, no closures).
- Built-ins cover everything we need: `sqrt`, `ln`, `log`, `abs`,
  `floor`, `ceil`, `min`, `max`, `%`.
- Excellent error messages (line/col).
- Sandboxed via `set_max_operations` and `set_max_expr_depths`.
- **Cost:** ~2 MB source, ~7 transitive deps, ~1m compile time.

### Disqualified:
- meval-rs (2018, abandoned, f64 only — no string support).
- fasteval (2019, abandoned, f64 only).
- mexprp (2019, abandoned, numeric only).

## Integration sketch

```rust
use rhai::{Engine, OptimizationLevel, Scope};

pub struct ChannelEvaluator { engine: Engine }

impl ChannelEvaluator {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_optimization_level(OptimizationLevel::Simple);
        engine.set_max_expr_depths(32, 32);
        engine.set_max_operations(10_000);
        Self { engine }
    }

    pub fn compile(&self, src: &str) -> Result<rhai::AST, rhai::ParseError> {
        self.engine.compile_expression(src)
    }

    pub fn eval_f64(&self, ast: &rhai::AST, attrs: &Attributes)
        -> Result<f64, Box<rhai::EvalAltResult>>
    {
        let mut scope = Scope::new();
        for (k, v) in &attrs.numeric { scope.push_constant(k.as_str(), *v); }
        for (k, v) in &attrs.text    { scope.push_constant(k.as_str(), v.clone()); }
        for (k, v) in &attrs.bool    { scope.push_constant(k.as_str(), *v); }
        self.engine.eval_ast_with_scope::<f64>(&mut scope, ast)
    }
}
```

## Cargo dep declaration

Recommended trim:
```toml
rhai = { version = "1", default-features = false, features = ["std", "sync"] }
```

Disable by default: `metadata`, `serde`, `decimal`, `f32`, `internals`.
