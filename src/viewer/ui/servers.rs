//! Servers tab UI

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::viewer::state::AppState;

use super::widgets::{
    render_cpu_chart, render_memory_chart, render_temp_chart,
};

/// Render servers tab
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Server list
            Constraint::Percentage(70), // Metrics display
        ])
        .split(area);

    render_server_list(frame, chunks[0], state);
    render_server_metrics(frame, chunks[1], state);
}

/// Render server list on the left
fn render_server_list(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .servers
        .iter()
        .enumerate()
        .map(|(i, server)| {
            let health_color = match server.health_status.as_str() {
                "up" => Color::Green,
                "stale" => Color::Yellow,
                "unknown" => Color::Gray,
                _ => Color::Red,
            };

            let content = Line::from(vec![
                Span::styled("● ", Style::default().fg(health_color)),
                Span::raw(&server.display_name),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", server.health_status),
                    Style::default().fg(health_color),
                ),
            ]);

            let mut style = Style::default();
            if i == state.selected_server {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Servers ({})", state.servers.len())),
    );

    frame.render_widget(list, area);
}

/// Render server metrics on the right
fn render_server_metrics(frame: &mut Frame, area: Rect, state: &AppState) {
    if let Some(server) = state.get_selected_server() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),      // Server info
                Constraint::Percentage(25), // Memory chart (new!)
                Constraint::Percentage(37), // CPU per-core chart (enhanced!)
                Constraint::Percentage(31), // Temperature per-component chart (enhanced!)
            ])
            .split(area);

        render_server_info(frame, chunks[0], server, state);
        render_memory_chart(frame, chunks[1], &server.server_id, state);
        render_cpu_chart(frame, chunks[2], &server.server_id, state);
        render_temp_chart(frame, chunks[3], &server.server_id, state);
    } else {
        let message = Paragraph::new("No servers configured")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Server Metrics"),
            )
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(message, area);
    }
}

/// Render server info panel
fn render_server_info(
    frame: &mut Frame,
    area: Rect,
    server: &crate::api::ServerInfo,
    state: &AppState,
) {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Server: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(&server.display_name),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", &server.health_status),
            Style::default()
                .fg(match server.health_status.as_str() {
                    "up" => Color::Green,
                    "stale" => Color::Yellow,
                    "unknown" => Color::Gray,
                    _ => Color::Red,
                })
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    // Add system information if available
    if let Some(history) = state.get_metrics_history(&server.server_id)
        && let Some(latest) = history.back()
    {
        let system = &latest.metrics.system;

        // Hostname and OS
        if let Some(hostname) = &system.host_name {
            lines.push(Line::from(vec![
                Span::styled("Host: ", Style::default().fg(Color::Cyan)),
                Span::raw(hostname),
            ]));
        }

        if let Some(os) = &system.os_version {
            lines.push(Line::from(vec![
                Span::styled("OS: ", Style::default().fg(Color::Cyan)),
                Span::raw(os),
                Span::raw("  "),
                Span::styled("Arch: ", Style::default().fg(Color::Cyan)),
                Span::raw(&latest.metrics.cpus.arch),
            ]));
        }

        // Quick metrics summary
        let mem_percent =
            (latest.metrics.memory.used as f64 / latest.metrics.memory.total as f64) * 100.0;
        let mut summary = vec![
            Span::styled("CPU: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.1}% ", latest.metrics.cpus.average_usage)),
        ];

        if let Some(temp) = latest.metrics.components.average_temperature {
            summary.push(Span::styled(" Temp: ", Style::default().fg(Color::Cyan)));
            summary.push(Span::raw(format!("{:.1}°C ", temp)));
        }

        summary.push(Span::styled(" Mem: ", Style::default().fg(Color::Cyan)));
        summary.push(Span::raw(format!("{:.1}%", mem_percent)));

        lines.push(Line::from(summary));
    }

    if let Some(last_seen) = &server.last_seen {
        lines.push(Line::from(vec![
            Span::styled("Last Update: ", Style::default().fg(Color::DarkGray)),
            Span::styled(last_seen, Style::default().fg(Color::DarkGray)),
        ]));
    }

    let info =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Server Info"));

    frame.render_widget(info, area);
}
