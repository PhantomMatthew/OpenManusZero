//! Chart visualization tool for creating charts and graphs
//!
//! Provides capabilities for:
//! - Bar charts
//! - Line charts
//! - Pie charts
//! - Scatter plots
//! - Histogram

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chart types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Histogram,
    Area,
    Box,
    Heatmap,
}

impl Default for ChartType {
    fn default() -> Self {
        Self::Bar
    }
}

/// Chart data series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    /// Series name
    pub name: String,
    /// Data values
    pub values: Vec<f64>,
    /// Optional labels for each value
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

/// Chart configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    /// Chart title
    pub title: String,
    /// X-axis label
    #[serde(default)]
    pub x_label: Option<String>,
    /// Y-axis label
    #[serde(default)]
    pub y_label: Option<String>,
    /// Chart type
    #[serde(default)]
    pub chart_type: ChartType,
    /// Data series
    pub series: Vec<DataSeries>,
    /// X-axis categories (for bar/line charts)
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    /// Width in characters (for text output)
    #[serde(default = "default_width")]
    pub width: usize,
    /// Height in characters (for text output)
    #[serde(default = "default_height")]
    pub height: usize,
}

fn default_width() -> usize { 60 }
fn default_height() -> usize { 20 }

/// Chart visualization tool
pub struct ChartTool {
    default_width: usize,
    default_height: usize,
}

impl ChartTool {
    /// Create a new chart tool
    pub fn new() -> Self {
        Self {
            default_width: 60,
            default_height: 20,
        }
    }

    /// Generate a text-based chart
    fn generate_chart(&self, config: &ChartConfig) -> String {
        match config.chart_type {
            ChartType::Bar => self.generate_bar_chart(config),
            ChartType::Line => self.generate_line_chart(config),
            ChartType::Pie => self.generate_pie_chart(config),
            _ => format!("Chart type {:?} not yet supported in text mode", config.chart_type),
        }
    }

    /// Generate a bar chart
    fn generate_bar_chart(&self, config: &ChartConfig) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", config.title));
        output.push_str(&"=".repeat(config.title.len()));
        output.push_str("\n\n");

        if config.series.is_empty() {
            output.push_str("No data to display\n");
            return output;
        }

        let width = config.width.min(self.default_width);
        let series = &config.series[0];
        let max_val = series.values.iter().cloned().fold(0.0_f64, f64::max);

        if max_val == 0.0 {
            output.push_str("All values are zero\n");
            return output;
        }

        let categories = config.categories.as_ref();
        let bar_width = (width / series.values.len().max(1)).saturating_sub(1);

        for (i, &value) in series.values.iter().enumerate() {
            let label = categories
                .and_then(|c| c.get(i).cloned())
                .unwrap_or_else(|| format!("Item {}", i + 1));

            let normalized = (value / max_val).min(1.0);
            let filled = (normalized * bar_width as f64) as usize;
            let filled = filled.min(bar_width);

            output.push_str(&format!("{:>12} |", label));
            output.push_str(&"█".repeat(filled));
            output.push_str(&"░".repeat(bar_width - filled));
            output.push_str(&format!(" {:.2}\n", value));
        }

        if let Some(ref x_label) = config.x_label {
            output.push_str(&format!("\nX: {}\n", x_label));
        }
        if let Some(ref y_label) = config.y_label {
            output.push_str(&format!("Y: {}\n", y_label));
        }

        output
    }

    /// Generate a line chart
    fn generate_line_chart(&self, config: &ChartConfig) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", config.title));
        output.push_str(&"=".repeat(config.title.len()));
        output.push_str("\n\n");

        if config.series.is_empty() {
            output.push_str("No data to display\n");
            return output;
        }

        let height = config.height.min(self.default_height);
        let width = config.width.min(self.default_width);
        let series = &config.series[0];

        if series.values.len() < 2 {
            output.push_str("Need at least 2 data points for line chart\n");
            return output;
        }

        let min_val = series.values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = series.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;

        if range == 0.0 {
            output.push_str("All values are the same\n");
            return output;
        }

        // Create a grid
        let mut grid = vec![vec![' '; width]; height];

        for (i, &value) in series.values.iter().enumerate() {
            let x = if series.values.len() > 1 {
                (i * (width - 1)) / (series.values.len() - 1)
            } else {
                width / 2
            };

            let normalized = (value - min_val) / range;
            let y = (height - 1) - (normalized * (height - 1) as f64) as usize;
            let y = y.min(height - 1);

            grid[y][x] = '●';
        }

        // Draw Y-axis labels
        for (row, grid_row) in grid.iter().enumerate().rev() {
            let y_val = min_val + (row as f64 / (height - 1) as f64) * range;
            output.push_str(&format!("{:8.2} |", y_val));
            output.push_str(&grid_row.iter().collect::<String>());
            output.push('\n');
        }

        // Draw X-axis
        output.push_str(&format!("{:8}  ", ""));
        output.push_str(&"─".repeat(width));
        output.push('\n');

        if let Some(ref x_label) = config.x_label {
            output.push_str(&format!("{:8}  {}\n", "", x_label));
        }
        if let Some(ref y_label) = config.y_label {
            output.push_str(&format!("Y: {}\n", y_label));
        }

        output
    }

    /// Generate a pie chart (simplified text version)
    fn generate_pie_chart(&self, config: &ChartConfig) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", config.title));
        output.push_str(&"=".repeat(config.title.len()));
        output.push_str("\n\n");

        if config.series.is_empty() {
            output.push_str("No data to display\n");
            return output;
        }

        let series = &config.series[0];
        let total: f64 = series.values.iter().sum();

        if total == 0.0 {
            output.push_str("All values are zero\n");
            return output;
        }

        let categories = config.categories.as_ref();

        output.push_str("Distribution:\n\n");

        for (i, &value) in series.values.iter().enumerate() {
            let label = categories
                .and_then(|c| c.get(i).cloned())
                .unwrap_or_else(|| format!("Item {}", i + 1));

            let percentage = (value / total) * 100.0;
            let filled = (percentage / 5.0) as usize; // Each block = 5%

            output.push_str(&format!("{:>12} ", label));
            output.push_str(&"█".repeat(filled.min(20)));
            output.push_str(&format!(" {:.1}%\n", percentage));
        }

        output
    }
}

impl Default for ChartTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ChartTool {
    fn name(&self) -> &'static str {
        "chart"
    }

    fn description(&self) -> &'static str {
        "Chart visualization tool for creating bar, line, pie, and other chart types"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "title".to_string(),
                    ToolParameter {
                        name: "title".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Chart title".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "chart_type".to_string(),
                    ToolParameter {
                        name: "chart_type".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Type of chart: bar, line, pie, scatter, histogram".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!("bar")),
                        enum_values: Some(vec![
                            "bar".to_string(),
                            "line".to_string(),
                            "pie".to_string(),
                            "scatter".to_string(),
                            "histogram".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "series".to_string(),
                    ToolParameter {
                        name: "series".to_string(),
                        param_type: "array".to_string(),
                        description: Some("Data series for the chart".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "categories".to_string(),
                    ToolParameter {
                        name: "categories".to_string(),
                        param_type: "array".to_string(),
                        description: Some("Labels for data points".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "x_label".to_string(),
                    ToolParameter {
                        name: "x_label".to_string(),
                        param_type: "string".to_string(),
                        description: Some("X-axis label".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "y_label".to_string(),
                    ToolParameter {
                        name: "y_label".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Y-axis label".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "width".to_string(),
                    ToolParameter {
                        name: "width".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Chart width in characters (default: 60)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(60)),
                        enum_values: None,
                    },
                );
                props.insert(
                    "height".to_string(),
                    ToolParameter {
                        name: "height".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Chart height in characters (default: 20)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(20)),
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["title".to_string(), "series".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let config: ChartConfig = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid chart config: {}", e)))?;

        let chart = self.generate_chart(&config);
        Ok(ToolResult::success(chart))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_tool_new() {
        let tool = ChartTool::new();
        assert_eq!(tool.name(), "chart");
    }

    #[test]
    fn test_bar_chart() {
        let tool = ChartTool::new();
        let config = ChartConfig {
            title: "Test Bar Chart".to_string(),
            chart_type: ChartType::Bar,
            series: vec![DataSeries {
                name: "Series 1".to_string(),
                values: vec![10.0, 20.0, 30.0, 40.0],
                labels: None,
            }],
            categories: Some(vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()]),
            x_label: Some("Category".to_string()),
            y_label: Some("Value".to_string()),
            width: 40,
            height: 10,
        };

        let chart = tool.generate_chart(&config);
        assert!(chart.contains("Test Bar Chart"));
        assert!(chart.contains("A"));
        assert!(chart.contains("D"));
    }

    #[test]
    fn test_line_chart() {
        let tool = ChartTool::new();
        let config = ChartConfig {
            title: "Test Line Chart".to_string(),
            chart_type: ChartType::Line,
            series: vec![DataSeries {
                name: "Series 1".to_string(),
                values: vec![1.0, 2.0, 3.0, 4.0, 5.0],
                labels: None,
            }],
            categories: None,
            x_label: None,
            y_label: None,
            width: 30,
            height: 10,
        };

        let chart = tool.generate_chart(&config);
        assert!(chart.contains("Test Line Chart"));
    }

    #[test]
    fn test_pie_chart() {
        let tool = ChartTool::new();
        let config = ChartConfig {
            title: "Test Pie Chart".to_string(),
            chart_type: ChartType::Pie,
            series: vec![DataSeries {
                name: "Series 1".to_string(),
                values: vec![25.0, 25.0, 50.0],
                labels: None,
            }],
            categories: Some(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
            x_label: None,
            y_label: None,
            width: 40,
            height: 10,
        };

        let chart = tool.generate_chart(&config);
        assert!(chart.contains("Test Pie Chart"));
        assert!(chart.contains("50.0%"));
    }
}
