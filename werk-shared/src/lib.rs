#![forbid(unsafe_code)]

// werk-shared: Shared types and logic for werk
//
// Extracted from werk-cli to enable reuse by TUI and other frontends.

pub mod aggregate;
pub mod batch_mutation;
pub mod cli_display;
pub mod config;
pub mod config_registry;
pub mod daemon_workspaces;
pub mod duration;
pub mod value_labels;
pub mod error;
pub mod flush;
pub mod hooks;
pub mod palette;
pub mod prefix;
pub mod registry;
pub mod util;
pub mod workspace;

pub use batch_mutation::BatchMutation;
pub use config::{AnalysisThresholds, Config, SignalThresholds};
pub use flush::{flush_to_file, content_equivalent as flush_content_equivalent, FlushOutcome, FLUSH_FILENAME};
pub use hooks::{GitHooks, HookBridge, HookBridgeHandle, HookEvent, HookFilter, HookLogEntry, HookRunner, ShippedHooks};
pub use error::{ErrorCode, Result, WerkError};
pub use palette::{
    Palette, PaletteChoice, PaletteContext, PaletteOption,
    apply_choice, apply_containment_choice, apply_sequencing_choice,
    containment_palette, detect_containment_palettes, detect_sequencing_palettes,
    sequencing_palette,
};
pub use prefix::PrefixResolver;
pub use util::{display_id, display_id_named, format_timestamp, relative_time, truncate};
pub use workspace::Workspace;
