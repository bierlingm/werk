//! Output formatting for werk-cli.
//!
//! Handles:
//! - Human-readable output (colored when TTY)
//! - JSON output (machine-readable)
//! - Color control (NO_COLOR env var, --no-color flag)

use owo_colors::OwoColorize;
use serde::Serialize;
use std::io::{self, IsTerminal};

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable output (colored when TTY).
    Human,
    /// Machine-readable JSON output.
    Json,
}

/// Output configuration.
#[derive(Debug, Clone)]
pub struct Output {
    /// Output format (Human or Json).
    format: OutputFormat,
    /// Whether colors should be used.
    use_color: bool,
    /// Whether stdout is a TTY.
    is_tty: bool,
}

impl Output {
    /// Create a new output configuration.
    ///
    /// Args:
    /// - `json`: If true, use JSON output format.
    /// - `no_color`: If true, disable colors (overrides TTY detection).
    pub fn new(json: bool, no_color: bool) -> Self {
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Human
        };

        // Color is enabled when:
        // 1. NO_COLOR env var is not set (owo-colors handles this)
        // 2. --no-color flag is not set
        // 3. stdout is a TTY (not piped)
        let is_tty = io::stdout().is_terminal();
        let use_color = !no_color && is_tty && std::env::var("NO_COLOR").is_err();

        Self {
            format,
            use_color,
            is_tty,
        }
    }

    /// Get the output format.
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Check if colors are enabled.
    pub fn use_color(&self) -> bool {
        self.use_color
    }

    /// Check if stdout is a TTY.
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Check if output should be JSON.
    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }

    /// Print a value to stdout.
    ///
    /// For JSON format, serializes to JSON.
    /// For human format, uses Display trait.
    pub fn print<T: Serialize + std::fmt::Display>(&self, value: &T) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
                println!("{}", json);
            }
            OutputFormat::Human => {
                println!("{}", value);
            }
        }
        Ok(())
    }

    /// Print JSON output directly (without serialization wrapper).
    pub fn print_json(&self, json: &str) -> io::Result<()> {
        println!("{}", json);
        Ok(())
    }

    /// Print a success message.
    pub fn success(&self, message: &str) -> io::Result<()> {
        if self.is_json() {
            let output = serde_json::json!({
                "success": true,
                "message": message
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else if self.use_color {
            println!("{} {}", "✓".green(), message);
        } else {
            println!("✓ {}", message);
        }
        Ok(())
    }

    /// Print an error message to stderr.
    pub fn error(&self, message: &str) -> io::Result<()> {
        if self.use_color {
            eprintln!("{} {}", "error:".red().bold(), message);
        } else {
            eprintln!("error: {}", message);
        }
        Ok(())
    }

    /// Print a JSON error response to stdout.
    /// Used when --json flag is set and an error occurs.
    pub fn error_json(&self, code: &str, message: &str) -> io::Result<()> {
        let output = serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        Ok(())
    }

    /// Print an info message to stdout.
    pub fn info(&self, message: &str) -> io::Result<()> {
        if self.is_json() {
            let output = serde_json::json!({
                "info": message
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else if self.use_color {
            println!("{} {}", "i".blue(), message);
        } else {
            println!("i {}", message);
        }
        Ok(())
    }

    /// Print a styled string (only when colors are enabled).
    pub fn styled(&self, text: &str, style: ColorStyle) -> String {
        if !self.use_color {
            return text.to_string();
        }
        match style {
            ColorStyle::Success => text.green().to_string(),
            ColorStyle::Error => text.red().to_string(),
            ColorStyle::Warning => text.yellow().to_string(),
            ColorStyle::Info => text.blue().to_string(),
            ColorStyle::Muted => text.bright_black().to_string(),
            ColorStyle::Highlight => text.bold().to_string(),
            ColorStyle::Id => text.cyan().to_string(),
            ColorStyle::Active => text.green().to_string(),
            ColorStyle::Resolved => text.blue().to_string(),
            ColorStyle::Released => text.bright_black().to_string(),
        }
    }
}

/// Color styles for output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorStyle {
    /// Success/positive style (green).
    Success,
    /// Error style (red).
    Error,
    /// Warning style (yellow).
    Warning,
    /// Info style (blue).
    Info,
    /// Muted/secondary style (gray).
    Muted,
    /// Highlight/emphasis style (bold).
    Highlight,
    /// ID/identifier style (cyan).
    Id,
    /// Active status style (green).
    Active,
    /// Resolved status style (blue).
    Resolved,
    /// Released status style (gray).
    Released,
}

impl Default for Output {
    fn default() -> Self {
        Self::new(false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_new_human() {
        let output = Output::new(false, false);
        assert_eq!(output.format(), OutputFormat::Human);
    }

    #[test]
    fn test_output_new_json() {
        let output = Output::new(true, false);
        assert_eq!(output.format(), OutputFormat::Json);
        assert!(output.is_json());
    }

    #[test]
    fn test_output_no_color_flag() {
        let output = Output::new(false, true);
        assert!(!output.use_color());
    }

    #[test]
    fn test_output_is_json() {
        let human = Output::new(false, false);
        let json = Output::new(true, false);
        assert!(!human.is_json());
        assert!(json.is_json());
    }

    #[test]
    fn test_styled_no_color() {
        let output = Output::new(false, true);
        let text = output.styled("test", ColorStyle::Success);
        assert_eq!(text, "test"); // No ANSI codes when no_color
    }

    #[test]
    fn test_default_is_human() {
        let output = Output::default();
        assert_eq!(output.format(), OutputFormat::Human);
    }
}
