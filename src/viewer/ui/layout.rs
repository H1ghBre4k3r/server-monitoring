//! Main dashboard layout

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};

use crate::viewer::state::{AppState, Tab};

use super::{alerts, servers, services};

/// Render the main dashboard UI
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_content(frame, chunks[1], state);
    render_footer(frame, chunks[2], state);
}

/// Render header with tabs
fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let titles = vec![
        Tab::Servers.title(),
        Tab::Services.title(),
        Tab::Alerts.title(),
    ];

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Guardia Monitor"),
        )
        .select(match state.current_tab {
            Tab::Servers => 0,
            Tab::Services => 1,
            Tab::Alerts => 2,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

/// Render main content area based on selected tab
fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.current_tab {
        Tab::Servers => servers::render(frame, area, state),
        Tab::Services => services::render(frame, area, state),
        Tab::Alerts => alerts::render(frame, area, state),
    }
}

/// Render footer with status and keybindings
fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut footer_text = vec![
        Span::raw("Tab: "),
        Span::styled("←/→", Style::default().fg(Color::Yellow)),
        Span::raw(" | Items: "),
        Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
        Span::raw(" | Pause: "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" | Refresh: "),
        Span::styled("R", Style::default().fg(Color::Yellow)),
        Span::raw(" | Quit: "),
        Span::styled("Q", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
    ];

    // Connection status
    if state.connected {
        footer_text.push(Span::styled(
            "● Connected",
            Style::default().fg(Color::Green),
        ));
    } else {
        footer_text.push(Span::styled(
            "○ Disconnected",
            Style::default().fg(Color::Red),
        ));
    }

    // Paused indicator
    if state.paused {
        footer_text.push(Span::raw(" | "));
        footer_text.push(Span::styled(
            "⏸ PAUSED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Error message
    if let Some(error) = &state.error_message {
        footer_text.push(Span::raw(" | "));
        footer_text.push(Span::styled(
            format!("Error: {}", error),
            Style::default().fg(Color::Red),
        ));
    }

    let footer =
        Paragraph::new(Line::from(footer_text)).block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}
