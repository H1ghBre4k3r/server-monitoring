//! Alerts tab UI

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::viewer::state::{AlertSeverity, AppState};

/// Render alerts tab
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.alerts.is_empty() {
        let message = Paragraph::new("No alerts yet")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Alerts Timeline"),
            )
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(message, area);
        return;
    }

    let items: Vec<ListItem> = state
        .alerts
        .iter()
        .rev() // Show newest first
        .enumerate()
        .map(|(i, alert)| {
            let severity_color = match alert.severity {
                AlertSeverity::Critical => Color::Red,
                AlertSeverity::Warning => Color::Yellow,
                AlertSeverity::Info => Color::Blue,
            };

            let severity_icon = match alert.severity {
                AlertSeverity::Critical => "⚠ ",
                AlertSeverity::Warning => "⚡",
                AlertSeverity::Info => "ℹ ",
            };

            let timestamp = alert.timestamp.format("%H:%M:%S");

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(severity_icon, Style::default().fg(severity_color)),
                Span::raw(" "),
                Span::styled(&alert.server_id, Style::default().fg(Color::Cyan)),
                Span::raw(" - "),
                Span::styled(
                    &alert.alert_type,
                    Style::default()
                        .fg(severity_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(": "),
                Span::raw(&alert.message),
            ]);

            let mut style = Style::default();
            if i == state.selected_alert {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Alerts Timeline ({} total)", state.alerts.len())),
    );

    frame.render_widget(list, area);
}
