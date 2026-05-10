#![forbid(unsafe_code)]

mod archive;
mod ctx;
mod engine;
mod error;
mod expr;
mod glyphs;
mod animation;
mod combinators;
mod ir;
mod logic;
mod registry;
mod scope;
mod sigil;
mod stages;
mod toml_schema;
#[cfg(feature = "hot-reload")]
mod hot_reload;

pub use archive::{CleanupReport, archive_path, cache_path, cleanup_cache, werk_state_revision};
pub use ctx::{Ctx, Diagnostics};
pub use engine::{CompiledLogic, Engine, derive_seed, scope_canonical};
pub use error::SigilError;
pub use expr::{CompiledExpr, ExprSource};
pub use glyphs::{AlchemicalFamily, GeomanticFamily, GlyphFamily, HandDrawnFamily};
pub use ir::{Ir, IrKind};
pub use logic::{Logic, LogicId, LogicVersion, Meta, Pipeline, SeedSpec, StageParams};
pub use registry::{AttributeName, ChannelName, GlyphFamilyName, Primitive};
pub use scope::{ResolvedScope, Scope, ScopeKind, ScopeSpec};
pub use sigil::{Sigil, SvgBytes};
pub use stages::*;
pub use toml_schema::{PresetSpec, compute_logic_hash, load_preset, load_preset_str};
pub use animation::{render_animation, AnimationAxis, AnimationOutput, AnimatedSigil, StageRef};
pub use combinators::{SheetLogic, CompositeLogic, CompositionRule};
#[cfg(feature = "hot-reload")]
pub use hot_reload::{HotReloadEvent, HotReloadWatcher, start_hot_reload};
