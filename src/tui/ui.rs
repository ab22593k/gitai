use super::state::{Mode, TuiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

const _APP_VERSION: &str = env!("CARGO_PKG_VERSION");

// Theme constants for consistent styling
const ACCENT_COLOR: Color = Color::Cyan;
const BORDER_COLOR: Color = Color::DarkGray;
const SEPARATOR_COLOR: Color = Color::DarkGray;

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
<<<<<<< HEAD
        Mode::RebaseList => vec![
            ("↑↓", "Navigate"),
            ("⏎", "Edit"),
            ("␣", "Action"),
            ("Esc", "Exit"),
        ],
        Mode::RebaseEdit => vec![
            ("Esc", "Save"),
        ],
||||||| parent of a9d2184 (Remove rebase functionality)
        Mode::RebaseList => vec![
            ("↑↓", "Navigate"),
            ("⏎", "Edit"),
            ("␣", "Action"),
            ("Esc", "Exit"),
        ],
        Mode::RebaseEdit => vec![("Esc", "Save")],
=======
>>>>>>> a9d2184 (Remove rebase functionality)
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
                    (*key).to_string(),
                    Style::default()
                        .fg(ACCENT_COLOR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(":{desc}"), Style::default()),
            ];

            if i < nav_items.len() - 1 {
                spans.push(Span::styled(" • ", Style::default().fg(SEPARATOR_COLOR)));
            }

            spans
        })
        .collect::<Vec<_>>();

    let nav_bar = Paragraph::new(Line::from(nav_spans)).alignment(ratatui::layout::Alignment::Left);
    f.render_widget(nav_bar, area);
}

fn draw_commit_message(f: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.mode {
        Mode::Help => draw_help(f, state, area),
        _ => {
            let title = format!(
                "✦ Commit Message ({}/{})",
                state.current_index + 1,
                state.messages.len()
            );

            let message_block = Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(ACCENT_COLOR)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_COLOR));

            if state.mode == Mode::EditingMessage {
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
    let message_content = format!("{}\n{}", current_message.title, current_message.message);

    let message = Paragraph::new(message_content)
        .block(block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let instructions_block = Block::default()
        .title(Span::styled(
            "✧ Custom Instructions",
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR));

    if state.mode == Mode::EditingInstructions {
        state.instructions_textarea.set_block(instructions_block);
        state.instructions_textarea.set_style(Style::default());
        f.render_widget(&state.instructions_textarea, area);
    } else {
        let instructions = Paragraph::new(state.custom_instructions.clone())
            .block(instructions_block)
            .style(Style::default())
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
        (
            "  ".to_string(),
            state.status.clone(),
            Color::Reset,
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
        Line::from(vec![
            Span::styled("✨ Commit Message TUI Help", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  ←→ / hl         Navigate between messages"),
        Line::from("  ↑↓ / jk         Navigate within message"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Actions:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  Enter            Commit with selected message"),
        Line::from("  E                Edit current message"),
        Line::from("  I                Edit custom instructions"),
        Line::from("  R                Regenerate messages"),
        Line::from("  ?                Toggle navigation bar"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Other:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  Esc              Exit without committing"),
        Line::from("  ?                Show this help"),
        Line::from(""),
        Line::from("Press any key to close help"),
    ];

    let help_block = Block::default()
        .title(Span::styled(
            "❓ Help",
            Style::default()
                .fg(ACCENT_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR));

<<<<<<< HEAD
    let mut lines = vec![];
    for (i, commit) in state.rebase_commits.iter().enumerate() {
        let is_selected = i == state.rebase_current_index;
        let prefix = if is_selected { "▶ " } else { "  " };
        let action = format!("{:6}", commit.suggested_action.to_string());
        let hash = &commit.hash[..8];
        let message = commit.message.lines().next().unwrap_or("");

        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(action, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(hash, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(message, style),
        ]));
    }

    let list = Paragraph::new(lines)
        .block(list_block)
||||||| parent of a9d2184 (Remove rebase functionality)
    let mut lines = vec![];
    for (i, commit) in state.rebase_commits.iter().enumerate() {
        let is_selected = i == state.rebase_current_index;
        let prefix = if is_selected { "▶ " } else { "  " };
        let action = format!("{:6}", commit.suggested_action.to_string());
        let hash = format!("{:8}", &commit.hash);
        let message = commit.message.lines().next().unwrap_or("");

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(action, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(hash, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(message, style),
        ]));
    }

    let list = Paragraph::new(lines)
        .block(list_block)
=======
    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
>>>>>>> a9d2184 (Remove rebase functionality)
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
