#![allow(clippy::as_conversions)]

//! TUI Design System
//!
//! This module implements a design token-based UI system with:
//! - Color palette for consistent theming
//! - Spacing tokens for standardized layout
//! - Typography tokens for text styling

use super::state::{Mode, TuiState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// Design Token Color Palette

// Brand colors
const COLOR_BRAND_PRIMARY: Color = Color::Blue; // Main accent color for interactive elements

// Text colors
const COLOR_TEXT_DEFAULT: Color = Color::White; // Standard text color
const COLOR_TEXT_DIMMED: Color = Color::DarkGray; // Muted text for secondary information

// Background colors
const COLOR_BACKGROUND_DEFAULT: Color = Color::Black; // Main background
const COLOR_BACKGROUND_ELEVATED: Color = Color::DarkGray; // For modals/cards

// State colors
const COLOR_STATE_SUCCESS: Color = Color::Green; // Success messages
const COLOR_STATE_ERROR: Color = Color::Red; // Error/warning messages
const COLOR_STATE_INFO: Color = Color::Blue; // Informational messages

// Derived colors for UI states
const ACCENT_COLOR: Color = COLOR_BRAND_PRIMARY;
const ACCENT_COLOR_ACTIVE: Color = Color::Cyan; // Brighter version for active elements
const TEXT_COLOR: Color = COLOR_TEXT_DEFAULT;
const SUBTLE_COLOR: Color = COLOR_TEXT_DIMMED;
const BORDER_COLOR: Color = COLOR_BACKGROUND_ELEVATED;
const BORDER_COLOR_ACTIVE: Color = ACCENT_COLOR_ACTIVE;
const SUCCESS_COLOR: Color = COLOR_STATE_SUCCESS;
const WARNING_COLOR: Color = Color::Yellow; // Warning (not in design tokens, keeping existing)
const ERROR_COLOR: Color = COLOR_STATE_ERROR;

// Spacing Tokens (in character units)
const SPACING_XS: u16 = 1; // Smallest spacing
const SPACING_SM: u16 = 2; // Small spacing
const SPACING_MD: u16 = 4; // Medium, default spacing
const SPACING_LG: u16 = 8; // Large spacing

// Typography Tokens (using ratatui modifiers)
const FONT_WEIGHT_REGULAR: Modifier = Modifier::empty();
const FONT_WEIGHT_BOLD: Modifier = Modifier::BOLD;

/// Main UI rendering entry point
pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let mut constraints = vec![];

    // Top padding
    constraints.push(Constraint::Length(SPACING_XS));

    if state.nav_bar_visible {
        constraints.push(Constraint::Length(SPACING_MD + SPACING_XS)); // Nav bar with some breathing room
    }

    constraints.push(Constraint::Min(10)); // Main content area

    if state.instructions_visible {
        constraints.push(Constraint::Length(SPACING_LG + SPACING_SM)); // Instructions area
    }

    constraints.push(Constraint::Length(SPACING_XS)); // Status bar

    // Bottom padding
    constraints.push(Constraint::Length(SPACING_XS));

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // Add horizontal padding using spacing tokens
    let mut final_chunks = vec![];
    for chunk in vertical_layout.iter() {
        let horizontal_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(SPACING_MD),
                Constraint::Min(1), // Take remaining space
                Constraint::Length(SPACING_MD),
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

fn draw_nav_bar(f: &mut Frame, state: &TuiState, area: Rect) {
    let nav_items: Vec<(&str, &str)> = match state.mode {
        Mode::Completing => vec![
            ("Tab/Shift+Tab", "Navigate"),
            ("Enter", "Accept"),
            ("Esc", "Cancel"),
        ],
        Mode::ContextSelection => vec![
            ("↑/↓", "Navigate"),
            ("Space", "Toggle"),
            ("Tab", "Category"),
            ("Enter", "Confirm"),
            ("Esc", "Cancel"),
        ],
        Mode::EditingMessage => vec![("Tab", "Complete"), ("Esc", "Finish")],
        Mode::EditingInstructions => vec![("Esc", "Finish")],
        Mode::Help => vec![("Any Key", "Close")],
        _ => vec![
            ("←/→", "Navigate"),
            ("E", "Edit Msg"),
            ("I", "Edit Instr"),
            ("C", "Context"),
            ("R", "Regen"),
            ("Enter", "Commit"),
            ("Esc", "Cancel"),
        ],
    };

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
                        .add_modifier(FONT_WEIGHT_BOLD),
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
        Mode::ContextSelection => draw_context_selection(f, state, area),
        _ => {
            let is_editing = state.mode == Mode::EditingMessage;
            let border_color = if is_editing {
                BORDER_COLOR_ACTIVE
            } else {
                BORDER_COLOR
            };

            let title = if is_editing {
                " Edit Message "
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
                        .add_modifier(FONT_WEIGHT_BOLD),
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
        " Edit Instructions "
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
                .fg(COLOR_BRAND_PRIMARY)
                .add_modifier(FONT_WEIGHT_BOLD),
        )]),
        Line::from(vec![Span::raw("  ←/→        Next/Prev Message")]),
        Line::from(vec![Span::raw("  ↑/↓        Scroll Content")]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(COLOR_BRAND_PRIMARY)
                .add_modifier(FONT_WEIGHT_BOLD),
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
        .border_style(Style::default().fg(COLOR_BRAND_PRIMARY));

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
        " Completion Suggestions ({}/{}) ",
        state.completion_index + 1,
        state.completion_suggestions.len()
    );

    let completion_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(ACCENT_COLOR_ACTIVE)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_COLOR_ACTIVE));

    let mut completion_lines = Vec::new();

    // Add instructions at the top
    completion_lines.push(Line::from(vec![Span::styled(
        "Use Tab/Shift+Tab to navigate, Enter to accept, Esc to cancel",
        Style::default().fg(SUBTLE_COLOR),
    )]));
    completion_lines.push(Line::from(""));

    for (i, suggestion) in state.completion_suggestions.iter().enumerate() {
        let (marker, style) = if i == state.completion_index {
            (
                ">",
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT_COLOR_ACTIVE)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (" ", Style::default().fg(TEXT_COLOR))
        };

        completion_lines.push(Line::from(vec![
            Span::styled(marker, style),
            Span::raw(" "),
            Span::styled(suggestion, style),
        ]));
    }

    // Position completion box - make it wider for better readability
    let area = centered_rect(area, 70, 50);

    f.render_widget(Clear, area);

    let completion_paragraph = Paragraph::new(completion_lines)
        .block(completion_block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(completion_paragraph, area);
}

#[allow(clippy::too_many_lines)]
fn draw_context_selection(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let title = " Context Selection ";

    let context_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(ACCENT_COLOR_ACTIVE)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_COLOR_ACTIVE));

    let mut context_lines = Vec::new();

    // Instructions
    context_lines.push(Line::from(vec![Span::styled(
        "Select context to include in commit message generation:",
        Style::default().fg(SUBTLE_COLOR),
    )]));
    context_lines.push(Line::from(vec![Span::styled(
        "Arrow keys: navigate | Space: toggle | Tab: switch category | Enter: confirm | Esc: cancel",
        Style::default().fg(SUBTLE_COLOR),
    )]));
    context_lines.push(Line::from(""));

    if let Some(context) = &state.context {
        // Files section
        context_lines.push(Line::from(vec![Span::styled(
            "Files:",
            Style::default()
                .fg(
                    if state.context_selection_category
                        == super::state::ContextSelectionCategory::Files
                    {
                        ACCENT_COLOR_ACTIVE
                    } else {
                        TEXT_COLOR
                    },
                )
                .add_modifier(Modifier::BOLD),
        )]));

        for (i, file) in context.staged_files.iter().enumerate() {
            let is_selected = state.selected_files.get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category
                == super::state::ContextSelectionCategory::Files
                && state.context_selection_index == i;

            let marker = if is_current { ">" } else { " " };
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            let style = if is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT_COLOR_ACTIVE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT_COLOR)
            };

            context_lines.push(Line::from(vec![
                Span::styled(format!("{marker} {checkbox}"), style),
                Span::raw(" "),
                Span::styled(&file.path, style),
                Span::raw(" "),
                Span::styled(
                    format!("({})", file.change_type),
                    Style::default().fg(SUBTLE_COLOR),
                ),
            ]));
        }

        context_lines.push(Line::from(""));

        // Commits section
        context_lines.push(Line::from(vec![Span::styled(
            "Recent Commits:",
            Style::default()
                .fg(
                    if state.context_selection_category
                        == super::state::ContextSelectionCategory::Commits
                    {
                        ACCENT_COLOR_ACTIVE
                    } else {
                        TEXT_COLOR
                    },
                )
                .add_modifier(Modifier::BOLD),
        )]));

        for (i, commit) in context.recent_commits.iter().enumerate() {
            let file_index = context.staged_files.len() + i;
            let is_selected = state.selected_commits.get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category
                == super::state::ContextSelectionCategory::Commits
                && state.context_selection_index == file_index;

            let marker = if is_current { ">" } else { " " };
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            let style = if is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT_COLOR_ACTIVE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT_COLOR)
            };

            // Truncate commit message for display
            let short_message = if commit.message.len() > 60 {
                format!("{}...", &commit.message[..57])
            } else {
                commit.message.clone()
            };

            context_lines.push(Line::from(vec![
                Span::styled(format!("{marker} {checkbox}"), style),
                Span::raw(" "),
                Span::styled(&commit.hash[..7], Style::default().fg(SUBTLE_COLOR)),
                Span::raw(" "),
                Span::styled(short_message, style),
            ]));
        }
    } else {
        context_lines.push(Line::from(vec![Span::styled(
            "No context available",
            Style::default().fg(WARNING_COLOR),
        )]));
    }

    let context_paragraph = Paragraph::new(context_lines)
        .block(context_block)
        .style(Style::default())
        .wrap(Wrap { trim: true });

    f.render_widget(context_paragraph, area);
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
