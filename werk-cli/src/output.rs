//! Output formatting for werk-cli.
//!
//! Handles:
//! - Human-readable plain text output
//! - JSON output (machine-readable)

use serde::Serialize;
use std::io;

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable plain text output.
    Human,
    /// Machine-readable JSON output.
    Json,
}

/// Output configuration.
#[derive(Debug, Clone)]
pub struct Output {
    /// Output format (Human or Json).
    format: OutputFormat,
}

impl Output {
    /// Create a new output configuration.
    ///
    /// Args:
    /// - `json`: If true, use JSON output format.
    pub fn new(json: bool) -> Self {
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Human
        };

        Self { format }
    }

    /// Get the output format.
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Check if output should be JSON.
    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }

    /// Check if output should be a structured format (JSON).
    pub fn is_structured(&self) -> bool {
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

    /// Serialize a value to JSON and print it.
    ///
    /// Returns an error string suitable for WerkError::IoError wrapping.
    pub fn print_structured<T: Serialize>(&self, value: &T) -> Result<(), String> {
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| format!("failed to serialize JSON: {}", e))?;
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
            self.print_structured(&output).map_err(io::Error::other)?;
        } else {
            println!("✓ {}", message);
        }
        Ok(())
    }

    /// Print an error message to stderr.
    pub fn error(&self, message: &str) -> io::Result<()> {
        eprintln!("error: {}", message);
        Ok(())
    }

    /// Print a structured error response to stdout.
    /// Used when --json flag is set and an error occurs.
    pub fn error_json(&self, code: &str, message: &str) -> io::Result<()> {
        let output = serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        });
        self.print_structured(&output).map_err(io::Error::other)?;
        Ok(())
    }

    /// Print an info message to stdout.
    pub fn info(&self, message: &str) -> io::Result<()> {
        if self.is_json() {
            let output = serde_json::json!({
                "info": message
            });
            self.print_structured(&output).map_err(io::Error::other)?;
        } else {
            println!("i {}", message);
        }
        Ok(())
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_new_human() {
        let output = Output::new(false);
        assert_eq!(output.format(), OutputFormat::Human);
    }

    #[test]
    fn test_output_new_json() {
        let output = Output::new(true);
        assert_eq!(output.format(), OutputFormat::Json);
        assert!(output.is_json());
    }

    #[test]
    fn test_output_is_json() {
        let human = Output::new(false);
        let json = Output::new(true);
        assert!(!human.is_json());
        assert!(json.is_json());
    }

    #[test]
    fn test_output_is_structured() {
        let human = Output::new(false);
        let json = Output::new(true);
        assert!(!human.is_structured());
        assert!(json.is_structured());
    }

    #[test]
    fn test_default_is_human() {
        let output = Output::default();
        assert_eq!(output.format(), OutputFormat::Human);
    }
}
