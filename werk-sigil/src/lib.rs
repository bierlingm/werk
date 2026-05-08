#![forbid(unsafe_code)]

mod archive;
mod ctx;
mod engine;
mod error;
mod expr;
mod glyphs;
mod ir;
mod logic;
mod registry;
mod scope;
mod sigil;
mod stages;
mod toml_schema;

pub use archive::{CleanupReport, archive_path, cache_path};
pub use ctx::{Ctx, Diagnostics};
pub use engine::{CompiledLogic, Engine};
pub use error::SigilError;
pub use expr::{CompiledExpr, ExprSource};
pub use glyphs::{AlchemicalFamily, GeomanticFamily, GlyphFamily, HandDrawnFamily};
pub use ir::{Ir, IrKind};
pub use logic::{Logic, LogicId, LogicVersion, Pipeline};
pub use registry::{AttributeName, ChannelName, GlyphFamilyName, Primitive};
pub use scope::{ResolvedScope, Scope, ScopeKind, ScopeSpec};
pub use sigil::{Sigil, SvgBytes};
pub use stages::*;
pub use toml_schema::{PresetSpec, load_preset};
