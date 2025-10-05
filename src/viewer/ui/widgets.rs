//! Reusable UI widgets

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};

use crate::viewer::state::AppState;

/// Render CPU usage chart
pub fn render_cpu_chart(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if history.is_empty() {
            return;
        }

        // Extract CPU data points
        let data: Vec<(f64, f64)> = history
            .iter()
            .enumerate()
            .map(|(i, point)| (i as f64, point.metrics.cpus.average_usage as f64))
            .collect();

        // Calculate bounds
        let max_cpu = data
            .iter()
            .map(|(_, cpu)| *cpu)
            .fold(0.0, f64::max)
            .max(100.0);

        let datasets = vec![Dataset::default()
            .name("CPU %")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data)];

        let x_max = data.len().max(10) as f64;

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CPU Usage (%)"),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        "0".to_string(),
                        format!("{:.0}", max_cpu / 2.0),
                        format!("{:.0}", max_cpu),
                    ])
                    .bounds([0.0, max_cpu]),
            );

        frame.render_widget(chart, area);
    }
}

/// Render temperature chart
pub fn render_temp_chart(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if history.is_empty() {
            return;
        }

        // Extract temperature data points
        let data: Vec<(f64, f64)> = history
            .iter()
            .enumerate()
            .filter_map(|(i, point)| {
                point
                    .metrics
                    .components
                    .average_temperature
                    .map(|temp| (i as f64, temp as f64))
            })
            .collect();

        if data.is_empty() {
            return;
        }

        // Calculate bounds
        let max_temp = data
            .iter()
            .map(|(_, temp)| *temp)
            .fold(0.0, f64::max)
            .max(100.0);

        let datasets = vec![Dataset::default()
            .name("Temp °C")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&data)];

        let x_max = data.len().max(10) as f64;

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Temperature (°C)"),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        "0".to_string(),
                        format!("{:.0}", max_temp / 2.0),
                        format!("{:.0}", max_temp),
                    ])
                    .bounds([0.0, max_temp]),
            );

        frame.render_widget(chart, area);
    }
}
