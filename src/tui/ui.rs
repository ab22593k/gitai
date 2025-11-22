#![allow(clippy::as_conversions)]

use super::state::{Mode, TuiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// Modern Color Palette

const TEXT_COLOR: Color = Color::White;
const SUBTLE_COLOR: Color = Color::DarkGray;

const ACCENT_COLOR: Color = Color::Blue; // A calmer blue
const ACCENT_COLOR_ACTIVE: Color = Color::Cyan; // Brighter for active elements

const BORDER_COLOR: Color = Color::DarkGray;
const BORDER_COLOR_ACTIVE: Color = Color::Cyan;

const SUCCESS_COLOR: Color = Color::Green;
const WARNING_COLOR: Color = Color::Yellow;
const ERROR_COLOR: Color = Color::Red;

/// Main UI rendering entry point
pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let mut constraints = vec![];

    // Top padding
    constraints.push(Constraint::Length(1));

    if state.nav_bar_visible {
        constraints.push(Constraint::Length(3)); // Nav bar with some breathing room
    }

    constraints.push(Constraint::Min(10)); // Main content area

    if state.instructions_visible {
        constraints.push(Constraint::Length(6)); // Instructions area
    }

    constraints.push(Constraint::Length(1)); // Status bar

    // Bottom padding
    constraints.push(Constraint::Length(1));

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // Add horizontal padding
    let mut final_chunks = vec![];
    for chunk in vertical_layout.iter() {
        let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(5),
                Constraint::Percentage(90),
                Constraint::Percentage(5),
            ])
            .split(*chunk);
        final_chunks.push(horizontal_layout[1]);
    }

    final_chunks
}

/// Renders all visible UI sections
fn render_sections(f: &mut Frame, state: &mut TuiState, chunks: &[Rect]) {
    // Note: chunks indices need to match create_layout logic
    // 0: Top padding (skip)
    let mut chunk_index = 1;

    if state.nav_bar_visible {
        draw_nav_bar(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_commit_message(f, state, chunks[chunk_index]);
    chunk_index += 1;

    if state.instructions_visible {
        draw_instructions(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_status(f, state, chunks[chunk_index]);
}

fn draw_nav_bar(f: &mut Frame, _state: &TuiState, area: Rect) {
    let nav_items: Vec<(&str, &str)> = vec![
        ("Tab/Shift+Tab", "Nav"),
        ("E", "Edit Msg"),
        ("I", "Edit Instr"),
        ("R", "Regen"),
        ("Enter", "Commit"),
        ("Esc", "Cancel"),
    ];

    let nav_spans = nav_items
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut spans = vec![
                Span::styled(
                    format!(" {key} "),
                    Style::default()
                        .fg(Color::Black)
                        .bg(ACCENT_COLOR_ACTIVE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {desc} "), Style::default().fg(TEXT_COLOR)),
            ];

            if i < nav_items.len() - 1 {
                spans.push(Span::styled("   ", Style::default()));
            }

            spans
        })
        .collect::<Vec<_>>();

    let nav_bar = Paragraph::new(Line::from(nav_spans))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(nav_bar, area);
}

fn draw_commit_message(f: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.mode {
        Mode::Help => draw_help(f, state, area),
        Mode::Completing => draw_completion(f, state, area),
        _ => {
            let is_editing = state.mode == Mode::EditingMessage;
            let border_color = if is_editing {
                BORDER_COLOR_ACTIVE
            } else {
                BORDER_COLOR
            };

            let title = if is_editing {
                " ✎ Edit Message "
            } else {
                " Commit Message "
            };

            let message_block = Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(if is_editing {
                            ACCENT_COLOR_ACTIVE
                        } else {
                            ACCENT_COLOR
                        })
                        .add_modifier(Modifier::BOLD),
                ))
                .title_alignment(ratatui::layout::Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color));

            if is_editing {
                state.message_textarea.set_block(message_block);
                state
                    .message_textarea
                    .set_style(Style::default().fg(TEXT_COLOR));
                state
                    .message_textarea
                    .set_cursor_style(Style::default().bg(ACCENT_COLOR_ACTIVE).fg(Color::Black));
                f.render_widget(&state.message_textarea, area);
            } else {
                render_commit_message_content(f, state, message_block, area);
            }
        }
    }
}

fn render_commit_message_content(f: &mut Frame, state: &TuiState, block: Block, area: Rect) {
    let current_message = &state.messages[state.current_index];

    // Title
    let title_text = Line::from(vec![Span::styled(
        &current_message.title,
        Style::default()
            .fg(ACCENT_COLOR_ACTIVE)
            .add_modifier(Modifier::BOLD),
    )]);

    let mut content = vec![
        Line::from(""), // Padding
        title_text,
        Line::from(""), // Spacing
        Line::from(vec![Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(SUBTLE_COLOR),
        )]),
        Line::from(""), // Spacing
    ];

    // Split body by newlines to handle them correctly
    for line in current_message.message.lines() {
        content.push(Line::from(vec![Span::styled(
            line,
            Style::default().fg(TEXT_COLOR),
        )]));
    }

    let message = Paragraph::new(content)
        .block(block)
        .style(Style::default())
        .wrap(Wrap { trim: false }); // Don't trim to preserve formatting

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let is_editing = state.mode == Mode::EditingInstructions;
    let border_color = if is_editing {
        BORDER_COLOR_ACTIVE
    } else {
        BORDER_COLOR
    };

    let title = if is_editing {
        " ✎ Edit Instructions "
    } else {
        " Custom Instructions "
    };

    let instructions_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(if is_editing {
                    ACCENT_COLOR_ACTIVE
                } else {
                    ACCENT_COLOR
                })
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if is_editing {
        state.instructions_textarea.set_block(instructions_block);
        state
            .instructions_textarea
            .set_style(Style::default().fg(TEXT_COLOR));
        state
            .instructions_textarea
            .set_cursor_style(Style::default().bg(ACCENT_COLOR_ACTIVE).fg(Color::Black));
        f.render_widget(&state.instructions_textarea, area);
    } else {
        let instructions = Paragraph::new(state.custom_instructions.clone())
            .block(instructions_block)
            .style(Style::default().fg(SUBTLE_COLOR))
            .wrap(Wrap { trim: true });
        f.render_widget(instructions, area);
    }
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner, content, color, _) = get_status_components(state);

    let status_line = Line::from(vec![
        Span::styled(spinner, Style::default().fg(ACCENT_COLOR_ACTIVE)),
        Span::raw(" "),
        Span::styled(content, Style::default().fg(color)),
    ]);

    let status_widget = Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(status_widget, area);
}

fn get_status_components(state: &mut TuiState) -> (String, String, Color, usize) {
    if let Some(spinner) = &mut state.spinner {
        spinner.tick()
    } else {
        let color = if state.status.contains("Error") || state.status.contains("Failed") {
            ERROR_COLOR
        } else if state.status.contains("Success") || state.status.contains("Complete") {
            SUCCESS_COLOR
        } else if state.status.contains("Warning") {
            WARNING_COLOR
        } else {
            SUBTLE_COLOR
        };

        (
            String::new(),
            state.status.clone(),
            color,
            state.status.width(),
        )
    }
}

#[allow(clippy::too_many_lines)]
fn draw_help(f: &mut Frame, _state: &mut TuiState, area: Rect) {
    // Simplified Help
    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("  ←/→        Next/Prev Message")]),
        Line::from(vec![Span::raw("  ↑/↓        Scroll Content")]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("  Enter      Commit")]),
        Line::from(vec![Span::raw("  E          Edit Message")]),
        Line::from(vec![Span::raw("  I          Edit Instructions")]),
        Line::from(vec![Span::raw("  R          Regenerate")]),
        Line::from(vec![Span::raw("  Esc        Cancel/Back")]),
    ];

    let help_block = Block::default()
        .title(" Help ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_COLOR));

    let area = centered_rect(area, 60, 50);

    f.render_widget(Clear, area); // Clear background

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(help_paragraph, area);
}

fn draw_completion(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let title = format!(
        " Suggestions ({}/{}) ",
        state.completion_index + 1,
        state.completion_suggestions.len()
    );

    let completion_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_COLOR_ACTIVE));

    let mut completion_lines = Vec::new();

    for (i, suggestion) in state.completion_suggestions.iter().enumerate() {
        let style = if i == state.completion_index {
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT_COLOR_ACTIVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT_COLOR)
        };
        // Pad the selection for better look
        completion_lines.push(Line::from(vec![Span::styled(
            format!(" {suggestion} "),
            style,
        )]));
    }

    // Position completion box near the cursor or centered for now
    // For simplicity in this modernization, let's center it but make it smaller
    let area = centered_rect(area, 50, 40);

    f.render_widget(Clear, area);

    let completion_paragraph = Paragraph::new(completion_lines)
        .block(completion_block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(completion_paragraph, area);
}

/// Helper to center a rect
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
