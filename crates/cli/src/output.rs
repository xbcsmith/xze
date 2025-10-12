//! CLI output formatting module

use crate::config::{CliConfig, OutputFormat};
use serde::Serialize;
use std::io::{self, Write};
use xze_core::Result;

/// Output formatter for CLI results
pub struct OutputFormatter {
    format: OutputFormat,
    use_colors: bool,
    writer: Box<dyn Write + Send>,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new(config: &CliConfig) -> Self {
        Self {
            format: config.default_output_format,
            use_colors: config.use_colors && crate::config::supports_color(),
            writer: Box::new(io::stdout()),
        }
    }

    /// Create a formatter with specific format
    pub fn with_format(format: OutputFormat, use_colors: bool) -> Self {
        Self {
            format,
            use_colors: use_colors && crate::config::supports_color(),
            writer: Box::new(io::stdout()),
        }
    }

    /// Create a formatter with custom writer
    pub fn with_writer<W: Write + Send + 'static>(
        format: OutputFormat,
        use_colors: bool,
        writer: W,
    ) -> Self {
        Self {
            format,
            use_colors: use_colors && crate::config::supports_color(),
            writer: Box::new(writer),
        }
    }

    /// Format and output a serializable value
    pub fn output<T: Serialize>(&mut self, value: &T) -> Result<()> {
        match self.format {
            OutputFormat::Json => self.output_json(value),
            OutputFormat::Yaml => self.output_yaml(value),
            OutputFormat::Pretty => self.output_pretty(value),
            OutputFormat::Compact => self.output_compact(value),
            OutputFormat::Table => self.output_table(value),
        }
    }

    /// Output JSON format
    fn output_json<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(value)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Output YAML format
    fn output_yaml<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let yaml = serde_yaml::to_string(value)?;
        writeln!(self.writer, "{}", yaml)?;
        Ok(())
    }

    /// Output pretty format (human-readable)
    fn output_pretty<T: Serialize>(&mut self, value: &T) -> Result<()> {
        // For pretty output, we'll use a custom formatter based on the type
        let json_value: serde_json::Value = serde_json::to_value(value)?;
        self.format_json_pretty(&json_value, 0)?;
        Ok(())
    }

    /// Output compact format
    fn output_compact<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Output table format
    fn output_table<T: Serialize>(&mut self, value: &T) -> Result<()> {
        // For table format, we'll convert to a simple key-value representation
        let json_value: serde_json::Value = serde_json::to_value(value)?;
        self.format_as_table(&json_value)?;
        Ok(())
    }

    /// Format JSON value in a pretty, human-readable way
    fn format_json_pretty(&mut self, value: &serde_json::Value, indent: usize) -> Result<()> {
        let indent_str = "  ".repeat(indent);

        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    match val {
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                            writeln!(self.writer, "{}{}:", indent_str, self.colorize_key(key))?;
                            self.format_json_pretty(val, indent + 1)?;
                        }
                        _ => {
                            writeln!(
                                self.writer,
                                "{}{}: {}",
                                indent_str,
                                self.colorize_key(key),
                                self.format_value(val)
                            )?;
                        }
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    writeln!(self.writer, "{}[{}]:", indent_str, i)?;
                    self.format_json_pretty(item, indent + 1)?;
                }
            }
            _ => {
                writeln!(self.writer, "{}{}", indent_str, self.format_value(value))?;
            }
        }
        Ok(())
    }

    /// Format as a simple table
    fn format_as_table(&mut self, value: &serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::Object(map) => {
                // Find the maximum key length for alignment
                let max_key_len = map.keys().map(|k| k.len()).max().unwrap_or(0);

                writeln!(
                    self.writer,
                    "┌{}┬{}┐",
                    "─".repeat(max_key_len + 2),
                    "─".repeat(40)
                )?;
                writeln!(
                    self.writer,
                    "│ {:<width$} │ Value                                    │",
                    "Key",
                    width = max_key_len
                )?;
                writeln!(
                    self.writer,
                    "├{}┼{}┤",
                    "─".repeat(max_key_len + 2),
                    "─".repeat(40)
                )?;

                for (key, val) in map {
                    let value_str = self.value_to_string(val);
                    let truncated_value = if value_str.len() > 38 {
                        format!("{}...", &value_str[..35])
                    } else {
                        value_str
                    };

                    writeln!(
                        self.writer,
                        "│ {:<width$} │ {:<38} │",
                        key,
                        truncated_value,
                        width = max_key_len
                    )?;
                }

                writeln!(
                    self.writer,
                    "└{}┴{}┘",
                    "─".repeat(max_key_len + 2),
                    "─".repeat(40)
                )?;
            }
            serde_json::Value::Array(arr) => {
                writeln!(self.writer, "┌───┬{}┐", "─".repeat(50))?;
                writeln!(
                    self.writer,
                    "│ # │ Value                                            │"
                )?;
                writeln!(self.writer, "├───┼{}┤", "─".repeat(50))?;

                for (i, item) in arr.iter().enumerate() {
                    let value_str = self.value_to_string(item);
                    let truncated_value = if value_str.len() > 48 {
                        format!("{}...", &value_str[..45])
                    } else {
                        value_str
                    };

                    writeln!(self.writer, "│{:>2} │ {:<48} │", i, truncated_value)?;
                }

                writeln!(self.writer, "└───┴{}┘", "─".repeat(50))?;
            }
            _ => {
                writeln!(self.writer, "{}", self.format_value(value))?;
            }
        }
        Ok(())
    }

    /// Convert a JSON value to a string representation
    fn value_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
            serde_json::Value::Object(obj) => format!("{{{}}} keys", obj.len()),
        }
    }

    /// Format a single value with appropriate styling
    fn format_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                if self.use_colors {
                    format!("\x1b[32m\"{}\"\x1b[0m", s) // Green for strings
                } else {
                    format!("\"{}\"", s)
                }
            }
            serde_json::Value::Number(n) => {
                if self.use_colors {
                    format!("\x1b[36m{}\x1b[0m", n) // Cyan for numbers
                } else {
                    n.to_string()
                }
            }
            serde_json::Value::Bool(b) => {
                if self.use_colors {
                    format!("\x1b[35m{}\x1b[0m", b) // Magenta for booleans
                } else {
                    b.to_string()
                }
            }
            serde_json::Value::Null => {
                if self.use_colors {
                    "\x1b[90mnull\x1b[0m".to_string() // Gray for null
                } else {
                    "null".to_string()
                }
            }
            serde_json::Value::Array(arr) => {
                format!("[{} items]", arr.len())
            }
            serde_json::Value::Object(obj) => {
                format!("{{{}}} keys", obj.len())
            }
        }
    }

    /// Colorize a key name
    fn colorize_key(&self, key: &str) -> String {
        if self.use_colors {
            format!("\x1b[34m{}\x1b[0m", key) // Blue for keys
        } else {
            key.to_string()
        }
    }

    /// Output a simple message
    pub fn message(&mut self, msg: &str) -> Result<()> {
        writeln!(self.writer, "{}", msg)?;
        Ok(())
    }

    /// Output a success message
    pub fn success(&mut self, msg: &str) -> Result<()> {
        if self.use_colors {
            writeln!(self.writer, "\x1b[32m✓\x1b[0m {}", msg)?;
        } else {
            writeln!(self.writer, "✓ {}", msg)?;
        }
        Ok(())
    }

    /// Output an error message
    pub fn error(&mut self, msg: &str) -> Result<()> {
        if self.use_colors {
            writeln!(self.writer, "\x1b[31m✗\x1b[0m {}", msg)?;
        } else {
            writeln!(self.writer, "✗ {}", msg)?;
        }
        Ok(())
    }

    /// Output a warning message
    pub fn warning(&mut self, msg: &str) -> Result<()> {
        if self.use_colors {
            writeln!(self.writer, "\x1b[33m⚠\x1b[0m {}", msg)?;
        } else {
            writeln!(self.writer, "⚠ {}", msg)?;
        }
        Ok(())
    }

    /// Output an info message
    pub fn info(&mut self, msg: &str) -> Result<()> {
        if self.use_colors {
            writeln!(self.writer, "\x1b[34mℹ\x1b[0m {}", msg)?;
        } else {
            writeln!(self.writer, "ℹ {}", msg)?;
        }
        Ok(())
    }

    /// Start a progress indicator
    pub fn progress(&mut self, msg: &str) -> Result<()> {
        if self.use_colors {
            write!(self.writer, "\x1b[36m⏳\x1b[0m {}...", msg)?;
        } else {
            write!(self.writer, "⏳ {}...", msg)?;
        }
        self.writer.flush()?;
        Ok(())
    }

    /// Finish a progress indicator
    pub fn progress_done(&mut self) -> Result<()> {
        if self.use_colors {
            writeln!(self.writer, " \x1b[32mdone\x1b[0m")?;
        } else {
            writeln!(self.writer, " done")?;
        }
        Ok(())
    }
}

/// Convenience function to create a formatter and output a value
pub fn output_with_format<T: Serialize>(
    value: &T,
    format: OutputFormat,
    use_colors: bool,
) -> Result<()> {
    let mut formatter = OutputFormatter::with_format(format, use_colors);
    formatter.output(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn test_json_output() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Json, false, Cursor::new(&mut output));

        let data = json!({"key": "value", "number": 42});
        formatter.output(&data).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("\"key\""));
        assert!(output_str.contains("\"value\""));
    }

    #[test]
    fn test_yaml_output() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Yaml, false, Cursor::new(&mut output));

        let data = json!({"key": "value", "number": 42});
        formatter.output(&data).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("key: value"));
    }

    #[test]
    fn test_message_output() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Pretty, false, Cursor::new(&mut output));

        formatter.success("Operation completed").unwrap();
        formatter.error("Something went wrong").unwrap();
        formatter.warning("Be careful").unwrap();
        formatter.info("For your information").unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("✓ Operation completed"));
        assert!(output_str.contains("✗ Something went wrong"));
        assert!(output_str.contains("⚠ Be careful"));
        assert!(output_str.contains("ℹ For your information"));
    }

    #[test]
    fn test_table_output() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Table, false, Cursor::new(&mut output));

        let data = json!({"name": "test", "version": "1.0", "active": true});
        formatter.output(&data).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("┌"));
        assert!(output_str.contains("│"));
        assert!(output_str.contains("└"));
        assert!(output_str.contains("name"));
        assert!(output_str.contains("test"));
    }

    #[test]
    fn test_colorization_disabled() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Pretty, false, Cursor::new(&mut output));

        formatter.success("test").unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // Should not contain ANSI escape codes
        assert!(!output_str.contains("\x1b["));
    }

    #[test]
    fn test_progress_output() {
        let mut output = Vec::new();
        let mut formatter =
            OutputFormatter::with_writer(OutputFormat::Pretty, false, Cursor::new(&mut output));

        formatter.progress("Processing").unwrap();
        formatter.progress_done().unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("⏳ Processing..."));
        assert!(output_str.contains("done"));
    }
}
