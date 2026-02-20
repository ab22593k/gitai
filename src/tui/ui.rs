#![allow(clippy::as_conversions)]

//! TUI Design System
//!
//! This module implements a design token-based UI system with:
//! - Color palette for consistent theming
//! - Spacing tokens for standardized layout
//! - Typography tokens for text styling

use super::state::{Mode, TuiState};
use super::theme::get_theme;
use ratatui::prelude::Stylize;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// Spacing Tokens (in character units)
const SPACING_SM: u16 = 2; // Small spacing

// Helper functions to get theme colors
fn brand_color() -> Color {
    get_theme().brand_primary
}
fn accent_color() -> Color {
    get_theme().accent
}
fn secondary_accent_color() -> Color {
    get_theme().secondary_accent
}
fn text_color() -> Color {
    get_theme().text_default
}
fn subtle_color() -> Color {
    get_theme().text_dimmed
}
fn text_on_accent() -> Color {
    get_theme().text_on_accent
}
fn background_base() -> Color {
    get_theme().background_base
}
fn background_surface() -> Color {
    get_theme().background_surface
}
fn background_overlay() -> Color {
    get_theme().background_overlay
}
fn component_focus() -> Color {
    get_theme().component_focus
}
fn selection_style() -> Style {
    Style::default()
        .bg(get_theme().selection_bg)
        .fg(get_theme().selection_fg)
}
fn border_color() -> Color {
    get_theme().border
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
fn info_color() -> Color {
    get_theme().state_info
}
fn font_weight_bold() -> Modifier {
    get_theme().font_weight_bold
}
fn font_weight_italic() -> Modifier {
    get_theme().font_weight_italic
}

/// Main UI rendering entry point
pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let base_block = Block::default().bg(background_base());
    f.render_widget(base_block, f.area());

    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let num_messages = state.messages().len();
    let has_tabs = num_messages > 1;
    let mut constraints = vec![];

    // Header / Nav
    constraints.push(Constraint::Length(3));

    if has_tabs {
        constraints.push(Constraint::Length(3)); // Tabs bar
    }

    // Main Content
    constraints.push(Constraint::Min(10));

    if state.is_instructions_visible() {
        constraints.push(Constraint::Length(8)); // Instructions area
    }

    // Status / Help hint
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
                Constraint::Length(SPACING_SM),
                Constraint::Min(1),
                Constraint::Length(SPACING_SM),
            ])
            .split(*chunk);
        final_chunks.push(horizontal_layout[1]);
    }

    final_chunks
}

/// Renders all visible UI sections
fn render_sections(f: &mut Frame, state: &mut TuiState, chunks: &[Rect]) {
    let mut chunk_index = 0;

    // Combined Header & Nav
    draw_header_and_nav(f, state, chunks[chunk_index]);
    chunk_index += 1;

    // Tabs if needed
    if state.messages().len() > 1 {
        draw_tabs(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_main_content(f, state, chunks[chunk_index]);
    chunk_index += 1;

    if state.is_instructions_visible() {
        draw_instructions(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    draw_status(f, state, chunks[chunk_index]);
}

fn draw_header_and_nav(f: &mut Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(1)])
        .split(area);

    // Brand / Logo
    let brand = Paragraph::new(Line::from(vec![
        Span::styled(
            " GITAI ",
            Style::default()
                .bg(brand_color())
                .fg(text_on_accent())
                .add_modifier(font_weight_bold()),
        ),
        Span::styled(" v0.1.3", Style::default().fg(subtle_color())),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(border_color())),
    );

    f.render_widget(brand, chunks[0]);

    // Navigation
    let nav_items: Vec<(&str, &str)> = match state.mode() {
        Mode::Completing => vec![
            ("TAB", "Next"),
            ("S-TAB", "Prev"),
            ("ENTER", "Accept"),
            ("ESC", "Back"),
        ],
        Mode::ContextSelection => vec![
            ("SPACE", "Toggle"),
            ("TAB", "Category"),
            ("ENTER", "Confirm"),
            ("ESC", "Cancel"),
        ],
        Mode::EditingMessage => vec![("TAB", "Complete"), ("ESC", "Save")],
        Mode::EditingInstructions => vec![("ESC", "Save")],
        Mode::Help => vec![("ANY", "Close")],
        _ => vec![
            ("E", "Edit"),
            ("I", "Instr"),
            ("C", "Context"),
            ("R", "Regen"),
            ("ENTER", "Commit"),
            ("?", "Help"),
        ],
    };

    let mut nav_spans = Vec::new();
    for (i, (key, desc)) in nav_items.iter().enumerate() {
        nav_spans.push(Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(component_focus())
                .add_modifier(font_weight_bold()),
        ));
        nav_spans.push(Span::styled(
            desc.to_string(),
            Style::default().fg(text_color()),
        ));
        if i < nav_items.len() - 1 {
            nav_spans.push(Span::styled(" │", Style::default().fg(border_color())));
        }
    }

    let nav = Paragraph::new(Line::from(nav_spans))
        .alignment(ratatui::layout::Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(border_color())),
        );

    f.render_widget(nav, chunks[1]);
}

fn draw_tabs(f: &mut Frame, state: &TuiState, area: Rect) {
    let mut tabs = Vec::new();
    for i in 0..state.messages().len() {
        let is_selected = i == state.current_index();
        let style = if is_selected {
            Style::default()
                .fg(accent_color())
                .add_modifier(font_weight_bold())
        } else {
            Style::default().fg(subtle_color())
        };

        let label = format!(" Message {} ", i + 1);
        tabs.push(Span::styled(label, style));

        if is_selected {
            tabs.push(Span::styled("● ", Style::default().fg(accent_color())));
        } else {
            tabs.push(Span::raw("  "));
        }
    }

    let p = Paragraph::new(Line::from(tabs))
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(border_color())),
        );

    f.render_widget(p, area);
}

fn draw_main_content(f: &mut Frame, state: &mut TuiState, area: Rect) {
    match state.mode() {
        Mode::Help => draw_help(f, state, area),
        Mode::Completing => draw_completion(f, state, area),
        Mode::ContextSelection => draw_context_selection(f, state, area),
        _ => draw_commit_editor(f, state, area),
    }
}

fn draw_commit_editor(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let is_editing = state.mode() == Mode::EditingMessage;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if is_editing {
            component_focus()
        } else {
            border_color()
        }))
        .title(Span::styled(
            " COMMIT MESSAGE ",
            Style::default().add_modifier(font_weight_bold()),
        ));

    if is_editing {
        state.message_textarea_mut().set_block(block);
        state
            .message_textarea_mut()
            .set_cursor_style(Style::default().bg(component_focus()).fg(text_on_accent()));
        f.render_widget(state.message_textarea(), area);
    } else {
        render_commit_message_content(f, state, block, area);
    }
}

fn render_commit_message_content(f: &mut Frame, state: &TuiState, block: Block, area: Rect) {
    let current_message = state.current_message();

    // Title
    let title_text = Line::from(vec![Span::styled(
        &current_message.title,
        Style::default()
            .fg(accent_color())
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if is_editing {
            component_focus()
        } else {
            border_color()
        }))
        .title(Span::styled(
            " CUSTOM INSTRUCTIONS ",
            Style::default().add_modifier(font_weight_bold()),
        ));

    if is_editing {
        state.instructions_textarea_mut().set_block(block);
        state
            .instructions_textarea_mut()
            .set_cursor_style(Style::default().bg(component_focus()).fg(text_on_accent()));
        f.render_widget(state.instructions_textarea(), area);
    } else {
        let content = if state.custom_instructions().trim().is_empty() {
            vec![Line::from(Span::styled(
                " No instructions provided. Press 'i' to add some...",
                Style::default()
                    .fg(subtle_color())
                    .add_modifier(font_weight_italic()),
            ))]
        } else {
            state
                .custom_instructions()
                .lines()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(text_color()))))
                .collect()
        };

        let p = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
    }
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner, content, _color, _) = get_status_components(state);

    let (bg, fg) = if state.status().contains("Error") || state.status().contains("Failed") {
        (error_color(), text_on_accent())
    } else if state.status().contains("Success") || state.status().contains("Complete") {
        (success_color(), text_on_accent())
    } else if state.status().contains("Warning") {
        (warning_color(), Color::Black)
    } else {
        (background_overlay(), text_color())
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {spinner} "),
            Style::default()
                .bg(bg)
                .fg(fg)
                .add_modifier(font_weight_bold()),
        ),
        Span::styled(format!(" {content} "), Style::default().bg(bg).fg(fg)),
    ]);

    let status_widget = Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Left);

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

fn draw_help(f: &mut Frame, _state: &mut TuiState, area: Rect) {
    let area = centered_rect(area, 60, 60);
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            " Keyboard Shortcuts ",
            Style::default().add_modifier(font_weight_bold()),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(brand_color()));

    let help_content = vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(accent_color())
                .add_modifier(font_weight_bold()),
        )]),
        Line::from(vec![
            Span::styled("  ←/→       ", Style::default().fg(component_focus())),
            Span::raw("Cycle through generated messages"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓       ", Style::default().fg(component_focus())),
            Span::raw("Scroll commit message content"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Editing",
            Style::default()
                .fg(accent_color())
                .add_modifier(font_weight_bold()),
        )]),
        Line::from(vec![
            Span::styled("  e         ", Style::default().fg(component_focus())),
            Span::raw("Edit current commit message"),
        ]),
        Line::from(vec![
            Span::styled("  i         ", Style::default().fg(component_focus())),
            Span::raw("Edit generation instructions"),
        ]),
        Line::from(vec![
            Span::styled("  TAB       ", Style::default().fg(component_focus())),
            Span::raw("Trigger AI completion (while editing)"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(accent_color())
                .add_modifier(font_weight_bold()),
        )]),
        Line::from(vec![
            Span::styled("  c         ", Style::default().fg(component_focus())),
            Span::raw("Manage commit context (files/history)"),
        ]),
        Line::from(vec![
            Span::styled("  r         ", Style::default().fg(component_focus())),
            Span::raw("Regenerate with current instructions"),
        ]),
        Line::from(vec![
            Span::styled("  ENTER     ", Style::default().fg(component_focus())),
            Span::raw("Accept and perform commit"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ESC/q     ", Style::default().fg(error_color())),
            Span::raw("Exit application"),
        ]),
    ];

    let p = Paragraph::new(help_content)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(p, area);
}

fn draw_completion(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let area = centered_rect(area, 50, 40);
    f.render_widget(Clear, area);

    let title = format!(
        " Suggestions ({}/{}) ",
        state.completion_index() + 1,
        state.completion_suggestions().len()
    );
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().add_modifier(font_weight_bold()),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent_color()));

    let mut list_items = Vec::new();
    for (i, suggestion) in state.completion_suggestions().iter().enumerate() {
        let is_selected = i == state.completion_index();
        let style = if is_selected {
            selection_style().add_modifier(font_weight_bold())
        } else {
            Style::default().fg(text_color())
        };

        let prefix = if is_selected { " > " } else { "   " };
        list_items.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(suggestion, style),
        ]));
    }

    let p = Paragraph::new(list_items)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn draw_context_selection(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_selection_list(f, state, chunks[0]);
    draw_preview(f, state, chunks[1]);
}

fn draw_selection_list(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " CONTEXT SELECTION ",
            Style::default().add_modifier(font_weight_bold()),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent_color()));

    let mut list_items = Vec::new();

    if let Some(context) = state.context() {
        // Files Section
        list_items.push(Line::from(vec![Span::styled(
            " FILES ",
            Style::default()
                .bg(accent_color())
                .fg(text_on_accent())
                .add_modifier(font_weight_bold()),
        )]));

        for (i, file) in context.staged_files.iter().enumerate() {
            let is_selected = state.selected_files().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Files
                && state.context_selection_index() == i;

            let style = if is_current {
                selection_style()
            } else {
                Style::default().fg(text_color())
            };
            let checkbox = if is_selected { "󰄲 " } else { "󰄱 " }; // Using nerd font symbols (fallback to [x] if no font)
            let checkbox = if checkbox.width() == 1 {
                if is_selected { "[x] " } else { "[ ] " }
            } else {
                checkbox
            };

            list_items.push(Line::from(vec![
                Span::styled(if is_current { " > " } else { "   " }, style),
                Span::styled(checkbox, style),
                Span::styled(&file.path, style),
            ]));
        }

        list_items.push(Line::from(""));

        // Commits Section
        list_items.push(Line::from(vec![Span::styled(
            " HISTORY ",
            Style::default()
                .bg(secondary_accent_color())
                .fg(text_on_accent())
                .add_modifier(font_weight_bold()),
        )]));

        for (i, commit) in context.recent_commits.iter().enumerate() {
            let file_index = context.staged_files.len() + i;
            let is_selected = state.selected_commits().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Commits
                && state.context_selection_index() == file_index;

            let style = if is_current {
                selection_style()
            } else {
                Style::default().fg(text_color())
            };
            let checkbox = if is_selected { "󰄲 " } else { "󰄱 " };
            let checkbox = if checkbox.width() == 1 {
                if is_selected { "[x] " } else { "[ ] " }
            } else {
                checkbox
            };

            list_items.push(Line::from(vec![
                Span::styled(if is_current { " > " } else { "   " }, style),
                Span::styled(checkbox, style),
                Span::styled(&commit.hash[..7], Style::default().fg(subtle_color())),
                Span::raw(" "),
                Span::styled(commit.message.lines().next().unwrap_or(""), style),
            ]));
        }
    }

    let p = Paragraph::new(list_items)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn draw_preview(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " PREVIEW ",
            Style::default().add_modifier(font_weight_bold()),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color()))
        .bg(background_surface());

    if let Some(context) = state.context() {
        if state.context_selection_category() == super::state::ContextSelectionCategory::Files {
            if let Some(file) = context.staged_files.get(state.context_selection_index()) {
                let mut lines = Vec::new();
                lines.push(Line::from(vec![
                    Span::styled(" File: ", Style::default().fg(subtle_color())),
                    Span::styled(
                        &file.path,
                        Style::default().add_modifier(font_weight_bold()),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" Type: ", Style::default().fg(subtle_color())),
                    Span::raw(format!("{}", file.change_type)),
                ]));
                lines.push(Line::from(""));

                // Diff content with basic highlighting
                for line in file.diff.lines().take(100) {
                    let style = if line.starts_with('+') {
                        Style::default().fg(success_color())
                    } else if line.starts_with('-') {
                        Style::default().fg(error_color())
                    } else if line.starts_with('@') {
                        Style::default().fg(info_color())
                    } else {
                        Style::default().fg(text_color())
                    };
                    lines.push(Line::from(Span::styled(line, style)));
                }

                let p = Paragraph::new(lines)
                    .block(block)
                    .wrap(Wrap { trim: false });
                f.render_widget(p, area);
                return;
            }
        } else {
            let commit_index = state
                .context_selection_index()
                .saturating_sub(context.staged_files.len());
            if let Some(commit) = context.recent_commits.get(commit_index) {
                let lines = vec![
                    Line::from(vec![
                        Span::styled("Commit: ", Style::default().fg(subtle_color())),
                        Span::styled(
                            &commit.hash,
                            Style::default().add_modifier(font_weight_bold()),
                        ),
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

    f.render_widget(Paragraph::new(" No selection ").block(block), area);
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
