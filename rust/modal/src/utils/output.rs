//! Output formatting utilities for CLI commands.

use serde::Serialize;
use std::str::FromStr;

/// Output format for CLI commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Text,
}

impl OutputFormat {
    /// Check if this is JSON format.
    #[allow(dead_code)]
    pub fn is_json(&self) -> bool {
        matches!(self, OutputFormat::Json)
    }
}

impl FromStr for OutputFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        })
    }
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        s.parse().unwrap()
    }
}

impl From<String> for OutputFormat {
    fn from(s: String) -> Self {
        s.parse().unwrap()
    }
}

/// Format and print output based on the format type.
///
/// For JSON format, serializes the data as pretty-printed JSON.
/// For Text format, uses the provided text formatter function.
///
/// # Arguments
/// * `format` - The output format (Json or Text)
/// * `data` - The data to serialize (must implement Serialize)
/// * `text_formatter` - A function that produces the text output
#[allow(dead_code)]
pub fn format_output<T, F>(format: &OutputFormat, data: &T, text_formatter: F)
where
    T: Serialize,
    F: FnOnce() -> String,
{
    match format {
        OutputFormat::Json => {
            if let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{}", json);
            }
        }
        OutputFormat::Text => {
            println!("{}", text_formatter());
        }
    }
}

/// Helper macro for creating output with both JSON and text formats.
///
/// Usage:
/// ```ignore
/// output_result!(format, {
///     "field1": value1,
///     "field2": value2,
/// }, {
///     format!("Field 1: {}\nField 2: {}", value1, value2)
/// });
/// ```
#[macro_export]
macro_rules! output_result {
    ($format:expr, $json:tt, $text:expr) => {
        if $format.is_json() {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!($json)).unwrap_or_default());
        } else {
            println!("{}", $text);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("anything".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
    }

    #[test]
    fn test_is_json() {
        assert!(OutputFormat::Json.is_json());
        assert!(!OutputFormat::Text.is_json());
    }
}

