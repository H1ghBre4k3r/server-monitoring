//! Servers tab UI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::viewer::state::AppState;

use super::widgets::{render_cpu_chart, render_temp_chart};

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
                style = style
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
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
                Constraint::Length(5),  // Server info
                Constraint::Percentage(50), // CPU chart
                Constraint::Percentage(50), // Temperature chart
            ])
            .split(area);

        render_server_info(frame, chunks[0], server, state);
        render_cpu_chart(frame, chunks[1], &server.server_id, state);
        render_temp_chart(frame, chunks[2], &server.server_id, state);
    } else {
        let message = Paragraph::new("No servers configured")
            .block(Block::default().borders(Borders::ALL).title("Server Metrics"))
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(message, area);
    }
}

/// Render server info panel
fn render_server_info(
    frame: &mut Frame,
    area: Rect,
    server: &crate::viewer::state::ServerInfo,
    state: &AppState,
) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Server: ", Style::default().fg(Color::Cyan)),
            Span::raw(&server.display_name),
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::Cyan)),
            Span::raw(&server.server_id),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &server.health_status,
                Style::default().fg(match server.health_status.as_str() {
                    "up" => Color::Green,
                    "stale" => Color::Yellow,
                    "unknown" => Color::Gray,
                    _ => Color::Red,
                }),
            ),
        ]),
    ];

    if let Some(last_seen) = &server.last_seen {
        lines.push(Line::from(vec![
            Span::styled("Last Seen: ", Style::default().fg(Color::Cyan)),
            Span::raw(last_seen),
        ]));
    }

    // Add current metrics if available
    if let Some(history) = state.get_metrics_history(&server.server_id) {
        if let Some(latest) = history.back() {
            lines.push(Line::from(vec![
                Span::styled("CPU: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.1}%", latest.metrics.cpus.average_usage)),
            ]));

            if let Some(temp) = latest.metrics.components.average_temperature {
                lines.push(Line::from(vec![
                    Span::styled("Temp: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{:.1}°C", temp)),
                ]));
            }

            let mem_percent = (latest.metrics.memory.used as f64 / latest.metrics.memory.total as f64) * 100.0;
            lines.push(Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.1}%", mem_percent)),
            ]));
        }
    }

    let info = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Server Info"));

    frame.render_widget(info, area);
}
