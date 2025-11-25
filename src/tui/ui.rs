#![allow(clippy::as_conversions)]

//! TUI Design System
//!
//! This module implements a design token-based UI system with:
//! - Color palette for consistent theming
//! - Spacing tokens for standardized layout
//! - Typography tokens for text styling

use super::state::{Mode, TuiState};
use super::theme::get_theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SynStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use unicode_width::UnicodeWidthStr;

// Spacing Tokens (in character units)
const SPACING_XS: u16 = 1; // Smallest spacing
const SPACING_SM: u16 = 2; // Small spacing
const SPACING_MD: u16 = 4; // Medium, default spacing
const SPACING_LG: u16 = 8; // Large spacing

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

// Helper functions to get theme colors
fn accent_color() -> Color {
    get_theme().accent
}
fn accent_color_active() -> Color {
    get_theme().accent_active
}
fn text_color() -> Color {
    get_theme().text_default
}
fn subtle_color() -> Color {
    get_theme().text_dimmed
}
fn border_color() -> Color {
    get_theme().border
}
fn border_color_active() -> Color {
    get_theme().border_active
}
fn success_color() -> Color {
    get_theme().state_success
}
fn warning_color() -> Color {
    get_theme().state_warning
}
fn error_color() -> Color {
    get_theme().state_error
}
fn brand_primary_color() -> Color {
    get_theme().brand_primary
}
fn font_weight_bold() -> Modifier {
    get_theme().font_weight_bold
}

/// Main UI rendering entry point
pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let num_messages = state.messages().len();
    let has_tabs = num_messages > 1;
    let mut constraints = vec![];

    // Top padding
    constraints.push(Constraint::Length(SPACING_XS));

    if state.is_nav_bar_visible() {
        constraints.push(Constraint::Length(SPACING_MD + SPACING_XS)); // Nav bar with some breathing room
    }

    if has_tabs {
        constraints.push(Constraint::Length(3)); // Tabs bar
    }
    constraints.push(Constraint::Min(10)); // Main content area

    if state.is_instructions_visible() {
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

    if state.is_nav_bar_visible() {
        draw_nav_bar(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_commit_message(f, state, chunks[chunk_index]);
    chunk_index += 1;

    if state.is_instructions_visible() {
        draw_instructions(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_status(f, state, chunks[chunk_index]);
}

fn draw_nav_bar(f: &mut Frame, state: &TuiState, area: Rect) {
    let nav_items: Vec<(&str, &str)> = match state.mode() {
        Mode::Completing => vec![("⇥/⇤", "Navigate"), ("⏎", "Accept"), ("Esc", "Cancel")],
        Mode::ContextSelection => vec![
            ("⬆/⬇", "Navigate"),
            ("␣", "Toggle"),
            ("⇥", "Category"),
            ("⏎", "Confirm"),
            ("Esc", "Cancel"),
        ],
        Mode::EditingMessage => vec![("Tab", "Complete"), ("Esc", "Finish")],
        Mode::EditingInstructions => vec![("Esc", "Finish")],
        Mode::Help => vec![("Any Key", "Close")],
        _ => vec![
            ("⬅️/➡️", "Navigate msgs"),
            ("✏️", "Edit msg"),
            ("📝", "Edit instr"),
            ("📁", "Context"),
            ("🔄", "Regenerate"),
            ("⏎", "Commit"),
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
                        .bg(accent_color_active())
                        .add_modifier(font_weight_bold()),
                ),
                Span::styled(format!(" {desc} "), Style::default().fg(text_color())),
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
    match state.mode() {
        Mode::Help => draw_help(f, state, area),
        Mode::Completing => draw_completion(f, state, area),
        Mode::ContextSelection => draw_context_selection(f, state, area),
        _ => {
            let is_editing = state.mode() == Mode::EditingMessage;
            let border_color = if is_editing {
                border_color_active()
            } else {
                border_color()
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
                            accent_color_active()
                        } else {
                            accent_color()
                        })
                        .add_modifier(font_weight_bold()),
                ))
                .title_alignment(ratatui::layout::Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color));

            if is_editing {
                state.message_textarea_mut().set_block(message_block);
                state
                    .message_textarea_mut()
                    .set_style(Style::default().fg(text_color()));
                state
                    .message_textarea_mut()
                    .set_cursor_style(Style::default().bg(accent_color_active()).fg(Color::Black));
                f.render_widget(state.message_textarea(), area);
            } else {
                render_commit_message_content(f, state, message_block, area);
            }
        }
    }
}

fn render_commit_message_content(f: &mut Frame, state: &TuiState, block: Block, area: Rect) {
    let current_message = state.current_message();

    // Title
    let title_text = Line::from(vec![Span::styled(
        &current_message.title,
        Style::default()
            .fg(accent_color_active())
            .add_modifier(font_weight_bold()),
    )]);

    let mut content = vec![
        Line::from(""), // Padding
        title_text,
        Line::from(""), // Spacing
        Line::from(vec![Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(subtle_color()),
        )]),
        Line::from(""), // Spacing
    ];

    // Split body by newlines to handle them correctly
    for line in current_message.message.lines() {
        content.push(Line::from(vec![Span::styled(
            line,
            Style::default().fg(text_color()),
        )]));
    }

    let message = Paragraph::new(content)
        .block(block)
        .style(Style::default())
        .wrap(Wrap { trim: false }); // Don't trim to preserve formatting

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let is_editing = state.mode() == Mode::EditingInstructions;
    let border_color = if is_editing {
        border_color_active()
    } else {
        border_color()
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
                    accent_color_active()
                } else {
                    accent_color()
                })
                .add_modifier(font_weight_bold()),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if is_editing {
        state
            .instructions_textarea_mut()
            .set_block(instructions_block);
        state
            .instructions_textarea_mut()
            .set_style(Style::default().fg(text_color()));
        state
            .instructions_textarea_mut()
            .set_cursor_style(Style::default().bg(accent_color_active()).fg(Color::Black));
        f.render_widget(state.instructions_textarea(), area);
    } else {
        let instructions = Paragraph::new(state.custom_instructions())
            .block(instructions_block)
            .style(Style::default().fg(subtle_color()))
            .wrap(Wrap { trim: true });
        f.render_widget(instructions, area);
    }
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner, content, color, _) = get_status_components(state);

    let status_line = Line::from(vec![
        Span::styled(spinner, Style::default().fg(accent_color_active())),
        Span::raw(" "),
        Span::styled(content, Style::default().fg(color)),
    ]);

    let status_widget = Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(status_widget, area);
}

fn get_status_components(state: &mut TuiState) -> (String, String, Color, usize) {
    if let Some(spinner) = state.spinner_mut() {
        spinner.tick()
    } else {
        let color = if state.status().contains("Error") || state.status().contains("Failed") {
            error_color()
        } else if state.status().contains("Success") || state.status().contains("Complete") {
            success_color()
        } else if state.status().contains("Warning") {
            warning_color()
        } else {
            subtle_color()
        };

        (
            String::new(),
            state.status().to_string(),
            color,
            state.status().width(),
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
                .fg(brand_primary_color())
                .add_modifier(font_weight_bold()),
        )]),
        Line::from(vec![Span::raw("  ←/→        Next/Prev Message")]),
        Line::from(vec![Span::raw("  ↑/↓        Scroll Content")]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(brand_primary_color())
                .add_modifier(font_weight_bold()),
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
        .border_style(Style::default().fg(brand_primary_color()));

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
        state.completion_index() + 1,
        state.completion_suggestions().len()
    );

    let completion_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(accent_color_active())
                .add_modifier(font_weight_bold()),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent_color_active()));

    let mut completion_lines = Vec::new();

    // Add instructions at the top
    completion_lines.push(Line::from(vec![Span::styled(
        "Use Tab/Shift+Tab to navigate, Enter to accept, Esc to cancel",
        Style::default().fg(subtle_color()),
    )]));
    completion_lines.push(Line::from(""));

    for (i, suggestion) in state.completion_suggestions().iter().enumerate() {
        let (marker, style) = if i == state.completion_index() {
            (
                ">",
                Style::default()
                    .fg(Color::Black)
                    .bg(accent_color_active())
                    .add_modifier(font_weight_bold()),
            )
        } else {
            (" ", Style::default().fg(text_color()))
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

fn draw_context_selection(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_selection_list(f, state, chunks[0]);
    draw_preview(f, state, chunks[1]);
}

#[allow(clippy::too_many_lines)]
fn draw_selection_list(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let title = " Context Selection ";

    let context_block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(accent_color_active())
                .add_modifier(font_weight_bold()),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent_color_active()));

    let mut context_lines = Vec::new();

    // Instructions
    context_lines.push(Line::from(vec![Span::styled(
        "Select context to include:",
        Style::default().fg(subtle_color()),
    )]));
    context_lines.push(Line::from(vec![Span::styled(
        "Space: toggle | Tab: category | Enter: confirm",
        Style::default().fg(subtle_color()),
    )]));
    context_lines.push(Line::from(""));

    if let Some(context) = state.context() {
        // Files section
        context_lines.push(Line::from(vec![Span::styled(
            "Files:",
            Style::default()
                .fg(
                    if state.context_selection_category()
                        == super::state::ContextSelectionCategory::Files
                    {
                        accent_color_active()
                    } else {
                        text_color()
                    },
                )
                .add_modifier(font_weight_bold()),
        )]));

        for (i, file) in context.staged_files.iter().enumerate() {
            let is_selected = state.selected_files().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Files
                && state.context_selection_index() == i;

            let marker = if is_current { ">" } else { " " };
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            let style = if is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(accent_color_active())
                    .add_modifier(font_weight_bold())
            } else {
                Style::default().fg(text_color())
            };

            context_lines.push(Line::from(vec![
                Span::styled(format!("{marker} {checkbox}"), style),
                Span::raw(" "),
                Span::styled(&file.path, style),
                Span::raw(" "),
                Span::styled(
                    format!("({})", file.change_type),
                    Style::default().fg(subtle_color()),
                ),
            ]));
        }

        context_lines.push(Line::from(""));

        // Commits section
        context_lines.push(Line::from(vec![Span::styled(
            "Recent Commits:",
            Style::default()
                .fg(
                    if state.context_selection_category()
                        == super::state::ContextSelectionCategory::Commits
                    {
                        accent_color_active()
                    } else {
                        text_color()
                    },
                )
                .add_modifier(font_weight_bold()),
        )]));

        for (i, commit) in context.recent_commits.iter().enumerate() {
            let file_index = context.staged_files.len() + i;
            let is_selected = state.selected_commits().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Commits
                && state.context_selection_index() == file_index;

            let marker = if is_current { ">" } else { " " };
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            let style = if is_current {
                Style::default()
                    .fg(Color::Black)
                    .bg(accent_color_active())
                    .add_modifier(font_weight_bold())
            } else {
                Style::default().fg(text_color())
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
                Span::styled(&commit.hash[..7], Style::default().fg(subtle_color())),
                Span::raw(" "),
                Span::styled(short_message, style),
            ]));
        }
    } else {
        context_lines.push(Line::from(vec![Span::styled(
            "No context available",
            Style::default().fg(warning_color()),
        )]));
    }

    let context_paragraph = Paragraph::new(context_lines)
        .block(context_block)
        .style(Style::default())
        .wrap(Wrap { trim: false });

    f.render_widget(context_paragraph, area);
}

fn draw_preview(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let block = Block::default()
        .title(" Preview ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color()));

    if let Some(context) = state.context() {
        if state.context_selection_category() == super::state::ContextSelectionCategory::Files {
            if let Some(file) = context.staged_files.get(state.context_selection_index()) {
                let diff = &file.diff;
                let syntax = SYNTAX_SET
                    .find_syntax_by_extension("diff")
                    .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
                let theme = &THEME_SET.themes["base16-ocean.dark"]; // Use a default dark theme for now

                let mut highlighter = HighlightLines::new(syntax, theme);
                let mut lines = Vec::new();

                for line in LinesWithEndings::from(diff) {
                    // Apply custom color logic for diff markers
                    if line.starts_with('+') {
                        lines.push(Line::from(vec![Span::styled(
                            line.trim_end_matches(['\n', '\r']),
                            Style::default().fg(success_color()),
                        )]));
                    } else if line.starts_with('-') {
                        lines.push(Line::from(vec![Span::styled(
                            line.trim_end_matches(['\n', '\r']),
                            Style::default().fg(error_color()),
                        )]));
                    } else {
                        // Use syntect for other lines (e.g. metadata, or just highlight as plain/diff)
                        let ranges: Vec<(SynStyle, &str)> = highlighter
                            .highlight_line(line, &SYNTAX_SET)
                            .unwrap_or_default();
                        let spans: Vec<Span> = ranges
                            .into_iter()
                            .map(|(style, text)| {
                                let fg = Color::Rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                );
                                Span::styled(
                                    text.trim_end_matches(['\n', '\r']),
                                    Style::default().fg(fg),
                                )
                            })
                            .collect();
                        lines.push(Line::from(spans));
                    }
                }

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(Wrap { trim: false });
                f.render_widget(p, area);
                return;
            }
        } else if state.context_selection_category()
            == super::state::ContextSelectionCategory::Commits
        {
            let commit_index = state
                .context_selection_index()
                .saturating_sub(context.staged_files.len());
            if let Some(commit) = context.recent_commits.get(commit_index) {
                let lines = vec![
                    Line::from(vec![
                        Span::styled("Commit: ", Style::default().fg(subtle_color())),
                        Span::raw(&commit.hash),
                    ]),
                    Line::from(vec![
                        Span::styled("Author: ", Style::default().fg(subtle_color())),
                        Span::raw(&commit.author),
                    ]),
                    Line::from(vec![
                        Span::styled("Date:   ", Style::default().fg(subtle_color())),
                        Span::raw(&commit.timestamp),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        &commit.message,
                        Style::default().fg(text_color()),
                    )),
                ];

                let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
                f.render_widget(p, area);
                return;
            }
        }
    }

    // Fallback if nothing selected or context missing
    f.render_widget(Paragraph::new("No selection").block(block), area);
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
