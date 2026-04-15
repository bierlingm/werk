//! Editor integration for werk-cli.
//!
//! Opens $EDITOR with content in a temp file, reads back the edited content.
//! Used by `werk reality` and `werk desire` when no value is provided.

use crate::error::{Result, WerkError};
use std::env;
use std::fs;
use std::process::Command;

/// Open an editor with the given content and return the edited content.
///
/// The content is written to a temporary file, the editor is opened,
/// and the edited content is read back. If the content changed, the
/// new content is returned. If unchanged or the editor exited without
/// saving, returns None.
///
/// Uses $EDITOR environment variable, falls back to "vi" on Unix or "notepad" on Windows.
pub fn edit_content(original: &str) -> Result<Option<String>> {
    // Create a temp directory and file
    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join(format!("werk_edit_{}.txt", ulid::Ulid::new()));

    // Write original content to the file
    fs::write(&file_path, original)
        .map_err(|e| WerkError::IoError(format!("failed to write temp file: {}", e)))?;

    // Editor resolution: config `editor.command` > $EDITOR > platform default.
    let editor = crate::commands::config_default_string(
        "editor.command",
        &env::var("EDITOR").unwrap_or_else(|_| default_editor()),
    );

    let exit_status = Command::new(&editor)
        .arg(&file_path)
        .status()
        .map_err(|e| WerkError::IoError(format!("failed to open editor '{}': {}", editor, e)))?;

    // Check if editor exited successfully
    if !exit_status.success() {
        // Clean up temp file
        let _ = fs::remove_file(&file_path);
        return Err(WerkError::IoError(format!(
            "editor '{}' exited with non-zero status: {}",
            editor, exit_status
        )));
    }

    // Read back the edited content
    let edited = fs::read_to_string(&file_path)
        .map_err(|e| WerkError::IoError(format!("failed to read temp file: {}", e)))?;

    // Clean up temp file
    let _ = fs::remove_file(&file_path);

    // Check if content changed
    if edited == original {
        Ok(None)
    } else {
        // Trim trailing newline if present (editors often add one)
        let trimmed = if edited.ends_with('\n') {
            edited.trim_end_matches('\n').to_string()
        } else {
            edited
        };

        // If still the same after trimming, no change
        if trimmed == original {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    }
}

/// Get the default editor for this platform.
fn default_editor() -> String {
    if cfg!(windows) {
        "notepad".to_string()
    } else {
        "vi".to_string()
    }
}

/// Open an editor with the given content for a specific field.
///
/// Includes a comment header indicating what field is being edited.
pub fn edit_field(field_name: &str, original: &str) -> Result<Option<String>> {
    // Prepend a comment header to help the user understand what they're editing
    let content_with_header = format!(
        "# Editing {} for tension\n# Lines starting with # are ignored\n{}",
        field_name, original
    );

    let result = edit_content(&content_with_header)?;

    // Strip the header comments from the result
    match result {
        Some(edited) => {
            // Remove comment lines and the header
            let cleaned: String = edited
                .lines()
                .filter(|line| !line.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");

            // Remove leading/trailing whitespace
            let trimmed = cleaned.trim().to_string();

            if trimmed.is_empty() || trimmed == original {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_editor_unix() {
        // On non-Windows, should be vi
        if !cfg!(windows) {
            assert_eq!(default_editor(), "vi");
        }
    }

    #[test]
    fn test_default_editor_windows() {
        // On Windows, should be notepad
        if cfg!(windows) {
            assert_eq!(default_editor(), "notepad");
        }
    }

}
