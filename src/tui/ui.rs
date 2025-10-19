use super::state::{Mode, TuiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// Enhanced theme with richer color palette
const ACCENT_COLOR: Color = Color::Cyan;
const ACCENT_COLOR_DIM: Color = Color::Blue;
const BORDER_COLOR: Color = Color::DarkGray;
const BORDER_COLOR_ACTIVE: Color = Color::Cyan;
const SEPARATOR_COLOR: Color = Color::DarkGray;
const TITLE_COLOR: Color = Color::Cyan;
const HELP_HEADER_COLOR: Color = Color::Magenta;
const SUCCESS_COLOR: Color = Color::Green;
const WARNING_COLOR: Color = Color::Yellow;

/// Main UI rendering entry point
pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let mut constraints = vec![];

    if state.nav_bar_visible {
        constraints.push(Constraint::Length(2));
    }
    constraints.push(Constraint::Min(16)); // Main content area
    if state.instructions_visible {
        constraints.push(Constraint::Length(4));
    }
    constraints.push(Constraint::Length(1)); // Status bar

    Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(f.area())
        .to_vec()
}

/// Renders all visible UI sections
fn render_sections(f: &mut Frame, state: &mut TuiState, chunks: &[Rect]) {
    let mut chunk_index = 0;

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

fn draw_nav_bar(f: &mut Frame, state: &TuiState, area: Rect) {
    let nav_items: Vec<(&str, &str)> = match state.mode {
        _ => vec![
            ("↔", "Navigate"),
            ("E", "Message"),
            ("I", "Instructions"),
            ("R", "Regenerate"),
            ("⏎", "Commit"),
        ],
    };

    let nav_spans = nav_items
        .iter()
        .enumerate()
        .flat_map(|(i, (key, desc))| {
            let mut spans = vec![
                Span::styled(
                    format!(" {}", *key),
                    Style::default()
                        .fg(ACCENT_COLOR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {}", desc), Style::default().fg(Color::Gray)),
            ];

            if i < nav_items.len() - 1 {
                spans.push(Span::styled("  │  ", Style::default().fg(SEPARATOR_COLOR)));
            }

            spans
        })
        .collect::<Vec<_>>();

    let nav_bar = Paragraph::new(Line::from(nav_spans))
        .alignment(ratatui::layout::Alignment::Left)
        .style(Style::default());

    f.render_widget(nav_bar, area);
}

fn draw_commit_message(f: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.mode {
        Mode::Help => draw_help(f, state, area),
        _ => {
            let is_editing = state.mode == Mode::EditingMessage;
            let border_color = if is_editing {
                BORDER_COLOR_ACTIVE
            } else {
                BORDER_COLOR
            };

            let title = format!(
                "{} Commit Message ({}/{}) {}",
                if is_editing { "✎" } else { "✦" },
                state.current_index + 1,
                state.messages.len(),
                if is_editing { "[EDITING]" } else { "" }
            );

            let message_block = Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(TITLE_COLOR)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));

            if is_editing {
                state.message_textarea.set_block(message_block);
                state.message_textarea.set_style(Style::default());
                f.render_widget(&state.message_textarea, area);
            } else {
                render_commit_message_content(f, state, message_block, area);
            }
        }
    }
}

fn render_commit_message_content(f: &mut Frame, state: &TuiState, block: Block, area: Rect) {
    let current_message = &state.messages[state.current_index];

    // Enhanced formatting with visual separation
    let title_line = Line::from(vec![Span::styled(
        &current_message.title,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]);

    let separator = Line::from(vec![Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(BORDER_COLOR),
    )]);

    let body_text = Line::from(vec![Span::styled(
        &current_message.message,
        Style::default().fg(Color::Gray),
    )]);

    let message = Paragraph::new(vec![title_line, separator, Line::from(""), body_text])
        .block(block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let is_editing = state.mode == Mode::EditingInstructions;
    let border_color = if is_editing {
        BORDER_COLOR_ACTIVE
    } else {
        BORDER_COLOR
    };

    let title = format!(
        "{} Custom Instructions {}",
        if is_editing { "✎" } else { "✧" },
        if is_editing { "[EDITING]" } else { "" }
    );

    let instructions_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(TITLE_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if is_editing {
        state.instructions_textarea.set_block(instructions_block);
        state.instructions_textarea.set_style(Style::default());
        f.render_widget(&state.instructions_textarea, area);
    } else {
        let instructions = Paragraph::new(state.custom_instructions.clone())
            .block(instructions_block)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });
        f.render_widget(instructions, area);
    }
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner, content, color, width) = get_status_components(state);
    let status_line = create_centered_status_line(f, spinner, content, color, width);

    let status_widget =
        Paragraph::new(vec![status_line]).alignment(ratatui::layout::Alignment::Left);

    f.render_widget(Clear, area);
    f.render_widget(status_widget, area);
}

fn get_status_components(state: &mut TuiState) -> (String, String, Color, usize) {
    if let Some(spinner) = &mut state.spinner {
        spinner.tick()
    } else {
        // Enhanced status color based on content
        let color = if state.status.contains("Error") || state.status.contains("Failed") {
            Color::Red
        } else if state.status.contains("Success") || state.status.contains("Complete") {
            SUCCESS_COLOR
        } else if state.status.contains("Warning") {
            WARNING_COLOR
        } else {
            Color::Reset
        };

        (
            "  ".to_string(),
            state.status.clone(),
            color,
            state.status.width() + 2,
        )
    }
}

fn create_centered_status_line(
    f: &Frame,
    spinner: String,
    content: String,
    color: Color,
    content_width: usize,
) -> Line<'static> {
    #[allow(clippy::as_conversions)]
    let terminal_width = f.area().width as usize;

    let (left_padding, right_padding) = calculate_padding(terminal_width, content_width);

    Line::from(vec![
        Span::raw(" ".repeat(left_padding)),
        Span::styled(spinner, Style::default()),
        Span::styled(content, Style::default().fg(color)),
        Span::raw(" ".repeat(right_padding)),
    ])
}

fn draw_help(f: &mut Frame, _state: &mut TuiState, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "✨ Commit Message TUI Help",
            Style::default()
                .fg(HELP_HEADER_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "━━ Navigation ━━",
            Style::default()
                .fg(ACCENT_COLOR_DIM)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ←→ / hl", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "         Navigate between messages",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ↑↓ / jk", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "         Navigate within message",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "━━ Actions ━━",
            Style::default()
                .fg(ACCENT_COLOR_DIM)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(SUCCESS_COLOR)),
            Span::styled(
                "            Commit with selected message",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  E", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "                 Edit current message",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  I", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "                 Edit custom instructions",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  R", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "                 Regenerate messages",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "                 Toggle navigation bar",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "━━ Other ━━",
            Style::default()
                .fg(ACCENT_COLOR_DIM)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(Color::Red)),
            Span::styled(
                "               Exit without committing",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(ACCENT_COLOR)),
            Span::styled(
                "                 Show this help",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to close help",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

    let help_block = Block::default()
        .title(Span::styled(
            "❓ Help",
            Style::default()
                .fg(HELP_HEADER_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR));

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(help_paragraph, area);
}

fn calculate_padding(terminal_width: usize, content_width: usize) -> (usize, usize) {
    if content_width >= terminal_width {
        (0, 0)
    } else {
        let left = (terminal_width - content_width) / 2;
        let right = terminal_width - content_width - left;
        (left, right)
    }
}
