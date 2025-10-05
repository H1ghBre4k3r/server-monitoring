//! Reusable UI widgets

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};

use crate::viewer::state::AppState;

/// Color palette for multiple lines (cores, components, etc.)
const LINE_COLORS: [Color; 16] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Blue,
    Color::Red,
    Color::LightCyan,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightRed,
    Color::Indexed(208), // Orange
    Color::Indexed(213), // Pink
    Color::Indexed(117), // Light Blue
    Color::White,
];

/// Render CPU usage chart with per-core breakdown
pub fn render_cpu_chart(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if history.is_empty() {
            return;
        }

        use chrono::Utc;

        // Calculate time window bounds
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(state.time_window_seconds as i64);

        // Determine number of cores from first metric with data
        let num_cores = history
            .iter()
            .find(|point| !point.metrics.cpus.cpus.is_empty())
            .map(|point| point.metrics.cpus.cpus.len())
            .unwrap_or(0);

        if num_cores == 0 {
            // Fallback to average if no per-core data
            render_cpu_chart_average(frame, area, server_id, state);
            return;
        }

        // Collect per-core data
        let mut core_data: Vec<Vec<(f64, f64)>> = vec![Vec::new(); num_cores];

        for point in history.iter().filter(|p| p.timestamp >= window_start) {
            let timestamp = point.timestamp.timestamp() as f64;
            
            for (core_idx, cpu) in point.metrics.cpus.cpus.iter().enumerate() {
                if core_idx < num_cores {
                    core_data[core_idx].push((timestamp, cpu.usage as f64));
                }
            }
        }

        // Check if we have any data
        if core_data.iter().all(|data| data.is_empty()) {
            return;
        }

        // Sort each core's data by timestamp
        for data in &mut core_data {
            data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Calculate bounds
        let max_cpu = core_data
            .iter()
            .flat_map(|data| data.iter().map(|(_, cpu)| *cpu))
            .fold(0.0, f64::max)
            .max(100.0);

        // Fixed X-axis bounds: [now - time_window, now]
        let x_min = window_start.timestamp() as f64;
        let x_max = now.timestamp() as f64;

        // Create time labels
        let start_time = window_start.format("%H:%M:%S").to_string();
        let mid_time = (window_start
            + chrono::Duration::seconds(state.time_window_seconds as i64 / 2))
        .format("%H:%M:%S")
        .to_string();
        let end_time = now.format("%H:%M:%S").to_string();

        // Create datasets for each core
        let mut datasets = Vec::new();
        for (core_idx, data) in core_data.iter().enumerate() {
            if !data.is_empty() {
                let color = LINE_COLORS[core_idx % LINE_COLORS.len()];
                datasets.push(
                    Dataset::default()
                        .name(format!("cpu{}", core_idx))
                        .marker(symbols::Marker::Braille)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(color))
                        .data(data),
                );
            }
        }

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("CPU Usage Per Core (%) - {} cores", num_cores)),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![start_time, mid_time, end_time])
                    .bounds([x_min, x_max]),
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

/// Fallback: Render average CPU usage chart (when no per-core data)
fn render_cpu_chart_average(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        use chrono::Utc;

        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(state.time_window_seconds as i64);

        let mut data: Vec<(f64, f64)> = history
            .iter()
            .filter(|point| point.timestamp >= window_start)
            .map(|point| {
                let timestamp = point.timestamp.timestamp() as f64;
                let cpu_usage = point.metrics.cpus.average_usage as f64;
                (timestamp, cpu_usage)
            })
            .collect();

        if data.is_empty() {
            return;
        }

        data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let max_cpu = 100.0;
        let x_min = window_start.timestamp() as f64;
        let x_max = now.timestamp() as f64;

        let start_time = window_start.format("%H:%M:%S").to_string();
        let mid_time = (window_start
            + chrono::Duration::seconds(state.time_window_seconds as i64 / 2))
        .format("%H:%M:%S")
        .to_string();
        let end_time = now.format("%H:%M:%S").to_string();

        let datasets = vec![
            Dataset::default()
                .name("CPU Avg")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&data),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CPU Usage (Average)"),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![start_time, mid_time, end_time])
                    .bounds([x_min, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec!["0".to_string(), "50".to_string(), "100".to_string()])
                    .bounds([0.0, max_cpu]),
            );

        frame.render_widget(chart, area);
    }
}

/// Render temperature chart with per-component breakdown
pub fn render_temp_chart(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if history.is_empty() {
            return;
        }

        use chrono::Utc;
        use std::collections::HashMap;

        // Calculate time window bounds
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(state.time_window_seconds as i64);

        // Collect all unique component names
        let mut component_names: Vec<String> = Vec::new();
        for point in history.iter() {
            for component in &point.metrics.components.components {
                if !component_names.contains(&component.name) {
                    component_names.push(component.name.clone());
                }
            }
        }

        if component_names.is_empty() {
            // Fallback to average if no per-component data
            render_temp_chart_average(frame, area, server_id, state);
            return;
        }

        // Collect per-component data
        let mut component_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
        for name in &component_names {
            component_data.insert(name.clone(), Vec::new());
        }

        for point in history.iter().filter(|p| p.timestamp >= window_start) {
            let timestamp = point.timestamp.timestamp() as f64;
            
            for component in &point.metrics.components.components {
                if let Some(temp) = component.temperature {
                    if let Some(data) = component_data.get_mut(&component.name) {
                        data.push((timestamp, temp as f64));
                    }
                }
            }
        }

        // Calculate max_temp before moving data
        let max_temp = component_data
            .values()
            .flat_map(|data| data.iter().map(|(_, temp)| *temp))
            .fold(0.0, f64::max)
            .max(50.0); // Minimum scale of 50°C

        // Create datasets by taking references
        let mut datasets = Vec::new();
        let mut sorted_data: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
        
        for (idx, name) in component_names.iter().enumerate() {
            if let Some(mut data) = component_data.remove(name) {
                if !data.is_empty() {
                    data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                    sorted_data.push((name.clone(), data));
                }
            }
        }

        for (idx, (name, data)) in sorted_data.iter().enumerate() {
            let color = LINE_COLORS[idx % LINE_COLORS.len()];
            datasets.push(
                Dataset::default()
                    .name(name.clone())
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(color))
                    .data(data),
            );
        }

        if datasets.is_empty() {
            return;
        }

        // Fixed X-axis bounds
        let x_min = window_start.timestamp() as f64;
        let x_max = now.timestamp() as f64;

        // Create time labels
        let start_time = window_start.format("%H:%M:%S").to_string();
        let mid_time = (window_start
            + chrono::Duration::seconds(state.time_window_seconds as i64 / 2))
        .format("%H:%M:%S")
        .to_string();
        let end_time = now.format("%H:%M:%S").to_string();

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Temperature Per Component (°C) - {} sensors", component_names.len())),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![start_time, mid_time, end_time])
                    .bounds([x_min, x_max]),
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

/// Fallback: Render average temperature chart (when no per-component data)
fn render_temp_chart_average(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        use chrono::Utc;

        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(state.time_window_seconds as i64);

        let mut data: Vec<(f64, f64)> = history
            .iter()
            .filter(|point| point.timestamp >= window_start)
            .filter_map(|point| {
                point.metrics.components.average_temperature.map(|temp| {
                    let timestamp = point.timestamp.timestamp() as f64;
                    (timestamp, temp as f64)
                })
            })
            .collect();

        if data.is_empty() {
            return;
        }

        data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let max_temp = data
            .iter()
            .map(|(_, temp)| *temp)
            .fold(0.0, f64::max)
            .max(50.0);

        let x_min = window_start.timestamp() as f64;
        let x_max = now.timestamp() as f64;

        let start_time = window_start.format("%H:%M:%S").to_string();
        let mid_time = (window_start
            + chrono::Duration::seconds(state.time_window_seconds as i64 / 2))
        .format("%H:%M:%S")
        .to_string();
        let end_time = now.format("%H:%M:%S").to_string();

        let datasets = vec![
            Dataset::default()
                .name("Temp Avg")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Red))
                .data(&data),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Temperature (Average)"),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![start_time, mid_time, end_time])
                    .bounds([x_min, x_max]),
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

/// Render memory usage history chart
pub fn render_memory_chart(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id) {
        if history.is_empty() {
            return;
        }

        use chrono::Utc;

        // Calculate time window bounds
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(state.time_window_seconds as i64);

        // Collect RAM and Swap usage percentages
        let mut ram_data: Vec<(f64, f64)> = Vec::new();
        let mut swap_data: Vec<(f64, f64)> = Vec::new();

        for point in history.iter().filter(|p| p.timestamp >= window_start) {
            let timestamp = point.timestamp.timestamp() as f64;
            let memory = &point.metrics.memory;

            // Calculate RAM percentage
            let ram_percent = (memory.used as f64 / memory.total as f64) * 100.0;
            ram_data.push((timestamp, ram_percent));

            // Calculate Swap percentage (if swap exists)
            if memory.total_swap > 0 {
                let swap_percent = (memory.used_swap as f64 / memory.total_swap as f64) * 100.0;
                swap_data.push((timestamp, swap_percent));
            }
        }

        if ram_data.is_empty() {
            return;
        }

        // Sort by timestamp
        ram_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        swap_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Fixed X-axis bounds
        let x_min = window_start.timestamp() as f64;
        let x_max = now.timestamp() as f64;

        // Create time labels
        let start_time = window_start.format("%H:%M:%S").to_string();
        let mid_time = (window_start
            + chrono::Duration::seconds(state.time_window_seconds as i64 / 2))
        .format("%H:%M:%S")
        .to_string();
        let end_time = now.format("%H:%M:%S").to_string();

        // Create datasets
        let mut datasets = vec![
            Dataset::default()
                .name("RAM")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&ram_data),
        ];

        if !swap_data.is_empty() {
            datasets.push(
                Dataset::default()
                    .name("Swap")
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::Red))
                    .data(&swap_data),
            );
        }

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Memory Usage Over Time (%)"),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![start_time, mid_time, end_time])
                    .bounds([x_min, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        "0".to_string(),
                        "50".to_string(),
                        "100".to_string(),
                    ])
                    .bounds([0.0, 100.0]),
            );

        frame.render_widget(chart, area);
    }
}

/// Render memory usage gauge (current snapshot)
pub fn render_memory_gauge(frame: &mut Frame, area: Rect, server_id: &str, state: &AppState) {
    if let Some(history) = state.get_metrics_history(server_id)
        && let Some(latest) = history.back()
    {
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

        let gauge = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Memory Usage"));

        frame.render_widget(gauge, area);
    }
}
