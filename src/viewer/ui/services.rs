//! Services tab UI

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::viewer::state::AppState;

/// Render services tab
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Service list
            Constraint::Length(8), // Service details
        ])
        .split(area);

    render_service_list(frame, chunks[0], state);
    render_service_details(frame, chunks[1], state);
}

/// Render service list
fn render_service_list(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.services.is_empty() {
        let message = Paragraph::new("No services configured")
            .block(Block::default().borders(Borders::ALL).title("Services"))
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(message, area);
        return;
    }

    let header = Row::new(vec!["Status", "Service Name", "URL", "Last Check"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = state
        .services
        .iter()
        .enumerate()
        .map(|(i, service)| {
            let health_color = match service.health_status.as_str() {
                "up" => Color::Green,
                "down" => Color::Red,
                "degraded" => Color::Yellow,
                "stale" => Color::Magenta,
                "unknown" => Color::Gray,
                _ => Color::White,
            };

            let mut style = Style::default();
            if i == state.selected_service {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }

            Row::new(vec![
                format!("● {}", service.health_status),
                service.name.clone(),
                service.url.clone(),
                service
                    .last_check
                    .clone()
                    .unwrap_or_else(|| "Never".to_string()),
            ])
            .style(style)
            .fg(health_color)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Services ({})", state.services.len())),
    );

    frame.render_widget(table, area);
}

/// Render service details panel
fn render_service_details(frame: &mut Frame, area: Rect, state: &AppState) {
    let service = state.services.get(state.selected_service);

    if let Some(service) = service {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Service: ", Style::default().fg(Color::Cyan)),
                Span::raw(&service.name),
            ]),
            Line::from(vec![
                Span::styled("URL: ", Style::default().fg(Color::Cyan)),
                Span::raw(&service.url),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    service.health_status.to_string(),
                    Style::default().fg(match service.health_status {
                        crate::api::types::ServiceHealthStatus::Up => Color::Green,
                        crate::api::types::ServiceHealthStatus::Down => Color::Red,
                        crate::api::types::ServiceHealthStatus::Degraded => Color::Yellow,
                        crate::api::types::ServiceHealthStatus::Stale => Color::Magenta,
                        crate::api::types::ServiceHealthStatus::Unknown => Color::Gray,
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Monitoring: ", Style::default().fg(Color::Cyan)),
                Span::raw(service.monitoring_status.to_string()),
            ]),
        ];

        if let Some(last_check) = &service.last_check {
            lines.push(Line::from(vec![
                Span::styled("Last Check: ", Style::default().fg(Color::Cyan)),
                Span::raw(last_check),
            ]));
        }

        if let Some(last_status) = &service.last_status {
            lines.push(Line::from(vec![
                Span::styled("Last Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(last_status.to_string()),
            ]));
        }

        let details = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Service Details"),
        );

        frame.render_widget(details, area);
    } else {
        let message = Paragraph::new("Select a service to view details")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Service Details"),
            )
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(message, area);
    }
}
