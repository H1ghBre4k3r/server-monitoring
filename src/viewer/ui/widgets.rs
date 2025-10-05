//! Reusable UI widgets

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
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

/// Render memory usage gauge
pub fn render_memory_gauge(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if let Some(latest) = history.back() {
            let memory = &latest.metrics.memory;

            // Calculate percentages
            let mem_percent = (memory.used as f64 / memory.total as f64) * 100.0;
            let swap_percent = if memory.total_swap > 0 {
                (memory.used_swap as f64 / memory.total_swap as f64) * 100.0
            } else {
                0.0
            };

            // Format values in GB
            let mem_used_gb = memory.used as f64 / 1_000_000_000.0;
            let mem_total_gb = memory.total as f64 / 1_000_000_000.0;
            let swap_used_gb = memory.used_swap as f64 / 1_000_000_000.0;
            let swap_total_gb = memory.total_swap as f64 / 1_000_000_000.0;

            // Color based on usage
            let mem_color = if mem_percent >= 85.0 {
                Color::Red
            } else if mem_percent >= 70.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            let swap_color = if swap_percent >= 85.0 {
                Color::Red
            } else if swap_percent >= 70.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            // Create progress bars
            let mem_bar_width = (mem_percent / 100.0 * 20.0) as usize;
            let mem_bar = "█".repeat(mem_bar_width) + &"░".repeat(20 - mem_bar_width);

            let swap_bar_width = (swap_percent / 100.0 * 20.0) as usize;
            let swap_bar = "█".repeat(swap_bar_width) + &"░".repeat(20 - swap_bar_width);

            let lines = vec![
                Line::from(vec![
                    Span::styled("RAM: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:.1}/{:.1} GB ", mem_used_gb, mem_total_gb)),
                    Span::styled(&mem_bar, Style::default().fg(mem_color)),
                    Span::raw(format!(" {:.1}%", mem_percent)),
                ]),
                Line::from(vec![
                    Span::styled("Swap: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:.1}/{:.1} GB ", swap_used_gb, swap_total_gb)),
                    Span::styled(&swap_bar, Style::default().fg(swap_color)),
                    Span::raw(format!(" {:.1}%", swap_percent)),
                ]),
            ];

            let gauge = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Memory Usage"),
            );

            frame.render_widget(gauge, area);
        }
    }
}
