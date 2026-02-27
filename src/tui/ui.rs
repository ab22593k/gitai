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
    widgets::{Block, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// Spacing Tokens (in character units)
const SPACING_SM: u16 = 3; // Small spacing (Expressive)

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
    // Fill the background with the base color
    f.render_widget(Block::default().bg(background_base()), f.area());

    let chunks = create_layout(f, state);
    render_sections(f, state, &chunks);
}

/// Creates dynamic layout based on visible sections
fn create_layout(f: &Frame, state: &TuiState) -> Vec<Rect> {
    let num_messages = state.messages().len();
    let has_tabs = num_messages > 1;
    let mut constraints = vec![];

    // Header (Compact)
    constraints.push(Constraint::Length(1));

    // Secondary Nav / Tabs
    if has_tabs {
        constraints.push(Constraint::Length(1));
    } else {
        constraints.push(Constraint::Length(0));
    }

    // Main Content (Card-like)
    constraints.push(Constraint::Min(10));

    if state.is_instructions_visible() {
        constraints.push(Constraint::Length(6)); // Instructions area
    }

    // Status / Help hint
    constraints.push(Constraint::Length(1));

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // Add horizontal padding for content areas (not header/footer)
    let mut final_chunks = vec![];
    for (i, chunk) in vertical_layout.iter().enumerate() {
        // Only add padding to main content and instructions (index 2 and 3 usually)
        if i == 2 || (i == 3 && state.is_instructions_visible()) {
            let horizontal_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(SPACING_SM),
                    Constraint::Min(1),
                    Constraint::Length(SPACING_SM),
                ])
                .split(*chunk);
            final_chunks.push(horizontal_layout[1]);
        } else {
            final_chunks.push(*chunk);
        }
    }

    final_chunks
}

/// Renders all visible UI sections
fn render_sections(f: &mut Frame, state: &mut TuiState, chunks: &[Rect]) {
    let mut chunk_index = 0;

    // 0: Header
    draw_header(f, state, chunks[chunk_index]);
    chunk_index += 1;

    // 1: Tabs/Nav
    if state.messages().len() > 1 {
        draw_tabs(f, state, chunks[chunk_index]);
    }
    chunk_index += 1;

    // 2: Main Workspace
    draw_main_content(f, state, chunks[chunk_index]);
    chunk_index += 1;

    // 3: Instructions (if visible)
    if state.is_instructions_visible() {
        draw_instructions(f, state, chunks[chunk_index]);
        chunk_index += 1;
    }

    // 4: Status
    draw_status(f, state, chunks[chunk_index]);
}

fn draw_header(f: &mut Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(1)])
        .split(area);

    // Brand / Logo with a clean background bar
    let brand_text = Line::from(vec![
        Span::styled(
            " 󰊢 GITAI ",
            Style::default()
                .bg(brand_color())
                .fg(text_on_accent())
                .add_modifier(font_weight_bold()),
        ),
        Span::styled(
            format!(" v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().bg(background_overlay()).fg(subtle_color()),
        ),
    ]);
    f.render_widget(
        Paragraph::new(brand_text).bg(background_overlay()),
        chunks[0],
    );

    // Navigation hints with expressive inline highlighting
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
            ("I", "Instructions"),
            ("C", "Context"),
            ("R", "Regenerate"),
            ("ENTER", "Commit"),
            ("?", "Help"),
        ],
    };

    let mut nav_spans = Vec::new();
    for (key, desc) in nav_items {
        let desc_lower = desc.to_lowercase();
        let key_lower = key.to_lowercase();

        // Expressive: if the key is a single char in the word, highlight it inline
        if let Some(idx) = desc_lower.find(&key_lower).filter(|_| key.len() == 1) {
            let (prefix, rest) = desc.split_at(idx);
            let (k, suffix) = rest.split_at(1);

            nav_spans.push(Span::raw(" "));
            if !prefix.is_empty() {
                nav_spans.push(Span::styled(prefix, Style::default().fg(subtle_color())));
            }
            nav_spans.push(Span::styled(
                k,
                Style::default()
                    .fg(component_focus())
                    .add_modifier(font_weight_bold()),
            ));
            if !suffix.is_empty() {
                nav_spans.push(Span::styled(suffix, Style::default().fg(subtle_color())));
            }
            nav_spans.push(Span::raw("  "));
            continue;
        }

        // Fallback for special keys (ENTER, TAB, etc.): expressive pill style
        nav_spans.push(Span::styled(
            format!(" {key} "),
            Style::default()
                .bg(background_overlay())
                .fg(component_focus())
                .add_modifier(font_weight_bold()),
        ));
        nav_spans.push(Span::styled(
            format!(" {desc}  "),
            Style::default().fg(subtle_color()),
        ));
    }

    let nav = Paragraph::new(Line::from(nav_spans))
        .alignment(ratatui::layout::Alignment::Right)
        .bg(background_overlay());

    f.render_widget(nav, chunks[1]);
}

fn draw_tabs(f: &mut Frame, state: &TuiState, area: Rect) {
    let mut tabs = Vec::new();
    for i in 0..state.messages().len() {
        let is_selected = i == state.current_index();
        if is_selected {
            tabs.push(Span::styled(
                format!("  󰄬 Message {}  ", i + 1),
                Style::default()
                    .bg(background_base())
                    .fg(accent_color())
                    .add_modifier(font_weight_bold()),
            ));
        } else {
            tabs.push(Span::styled(
                format!("    Message {}  ", i + 1),
                Style::default().fg(subtle_color()),
            ));
        }

        if i < state.messages().len() - 1 {
            tabs.push(Span::styled(" ", Style::default()));
        }
    }

    let p = Paragraph::new(Line::from(tabs))
        .alignment(ratatui::layout::Alignment::Center)
        .bg(background_surface());

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

    // Use a subtle card effect (different background)
    let card_bg = if is_editing {
        background_surface()
    } else {
        background_base()
    };

    let block = Block::default()
        .bg(card_bg)
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));

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

    // Expressive Title
    let title_text = Line::from(vec![
        Span::styled(" 󰜘 ", Style::default().fg(accent_color())),
        Span::styled(
            "COMMIT MESSAGE ",
            Style::default()
                .fg(subtle_color())
                .add_modifier(font_weight_bold()),
        ),
        Span::raw(" "),
        Span::styled(
            &current_message.title,
            Style::default()
                .fg(brand_color())
                .add_modifier(font_weight_bold()),
        ),
    ]);

    let mut content = vec![
        title_text,
        Line::from(vec![Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(background_overlay()),
        )]),
        Line::from(""), // Spacing
    ];

    for line in current_message.message.lines() {
        content.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(background_overlay())),
            Span::styled(line, Style::default().fg(text_color())),
        ]));
    }

    let message = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(message, area);
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let is_editing = state.mode() == Mode::EditingInstructions;

    let card_bg = if is_editing {
        background_surface()
    } else {
        background_base()
    };

    let mut block = Block::default()
        .bg(card_bg)
        .padding(ratatui::widgets::Padding::new(2, 2, 0, 0));

    if is_editing {
        // Expressive: Add a clear border and title when editing
        block = block
            .border_style(Style::default().fg(secondary_accent_color()))
            .borders(ratatui::widgets::Borders::ALL)
            .title(Span::styled(
                " EDITING INSTRUCTIONS ",
                Style::default()
                    .fg(secondary_accent_color())
                    .add_modifier(font_weight_bold()),
            ));

        state.instructions_textarea_mut().set_block(block);
        state
            .instructions_textarea_mut()
            .set_cursor_style(Style::default().bg(component_focus()).fg(text_on_accent()));
        // Ensure text is visible
        state
            .instructions_textarea_mut()
            .set_style(Style::default().fg(text_color()));

        f.render_widget(state.instructions_textarea(), area);
    } else {
        let title = Line::from(vec![
            Span::styled("󰋖 ", Style::default().fg(secondary_accent_color())),
            Span::styled(
                "CUSTOM INSTRUCTIONS",
                Style::default().fg(secondary_accent_color()),
            ),
        ]);

        let mut content = vec![title];

        if state.custom_instructions().trim().is_empty() {
            content.push(Line::from(Span::styled(
                " No instructions provided. Press 'I' to add some...",
                Style::default()
                    .fg(subtle_color())
                    .add_modifier(font_weight_italic()),
            )));
        } else {
            content.extend(
                state
                    .custom_instructions()
                    .lines()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(text_color())))),
            );
        }

        let p = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
    }
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let (spinner, content, _color, _) = get_status_components(state);

    let (bg, fg, label) = if state.status().contains("Error") || state.status().contains("Failed") {
        (error_color(), text_on_accent(), " 󰅚 ERROR ")
    } else if state.status().contains("Success") || state.status().contains("Complete") {
        (success_color(), text_on_accent(), " 󰄬 SUCCESS ")
    } else if state.status().contains("Warning") {
        (warning_color(), Color::Black, " 󱈸 WARNING ")
    } else {
        (background_overlay(), text_color(), " 󰋖 STATUS ")
    };

    let status_line = Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .bg(bg)
                .fg(fg)
                .add_modifier(font_weight_bold()),
        ),
        Span::styled(
            format!("  {spinner}  "),
            Style::default().bg(background_surface()).fg(accent_color()),
        ),
        Span::styled(format!(" {content} "), Style::default().fg(subtle_color())),
    ]);

    let status_widget = Paragraph::new(status_line)
        .alignment(ratatui::layout::Alignment::Left)
        .bg(background_base());

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

fn draw_shadow(f: &mut Frame, area: Rect) {
    let shadow_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width,
        height: area.height,
    };
    f.render_widget(Block::default().bg(Color::Rgb(17, 17, 27)), shadow_area); // Crust-like shadow
}

fn draw_help(f: &mut Frame, _state: &mut TuiState, area: Rect) {
    let popup_area = centered_rect(area, 60, 70);
    draw_shadow(f, popup_area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .bg(background_surface())
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));

    let help_content = vec![
        Line::from(vec![
            Span::styled(" 󰌌 ", Style::default().fg(accent_color())),
            Span::styled(
                "KEYBOARD SHORTCUTS ",
                Style::default()
                    .fg(accent_color())
                    .add_modifier(font_weight_bold()),
            ),
        ]),
        Line::from(vec![Span::styled(
            "━".repeat(popup_area.width.saturating_sub(4) as usize),
            Style::default().fg(background_overlay()),
        )]),
        Line::from(""),
        // Navigation Section
        Line::from(vec![
            Span::styled(
                "󰁕 Navigation ",
                Style::default().fg(secondary_accent_color()),
            ),
            Span::styled("━".repeat(10), Style::default().fg(background_overlay())),
        ]),
        Line::from(vec![
            Span::styled("  ← / →     ", Style::default().fg(component_focus())),
            Span::styled(
                "Cycle through generated messages",
                Style::default().fg(text_color()),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / ↓     ", Style::default().fg(component_focus())),
            Span::styled("Scroll content", Style::default().fg(text_color())),
        ]),
        Line::from(""),
        // Editing Section
        Line::from(vec![
            Span::styled("󰏫 Editing ", Style::default().fg(secondary_accent_color())),
            Span::styled("━".repeat(10), Style::default().fg(background_overlay())),
        ]),
        Line::from(vec![
            Span::styled("  e         ", Style::default().fg(component_focus())),
            Span::styled("Edit current message", Style::default().fg(text_color())),
        ]),
        Line::from(vec![
            Span::styled("  i         ", Style::default().fg(component_focus())),
            Span::styled("Edit instructions", Style::default().fg(text_color())),
        ]),
        Line::from(vec![
            Span::styled("  TAB       ", Style::default().fg(component_focus())),
            Span::styled("AI Completion", Style::default().fg(text_color())),
        ]),
        Line::from(""),
        // Actions Section
        Line::from(vec![
            Span::styled("󰀘 Actions ", Style::default().fg(secondary_accent_color())),
            Span::styled("━".repeat(10), Style::default().fg(background_overlay())),
        ]),
        Line::from(vec![
            Span::styled("  c         ", Style::default().fg(component_focus())),
            Span::styled("Manage context", Style::default().fg(text_color())),
        ]),
        Line::from(vec![
            Span::styled("  r         ", Style::default().fg(component_focus())),
            Span::styled("Regenerate", Style::default().fg(text_color())),
        ]),
        Line::from(vec![
            Span::styled("  ENTER     ", Style::default().fg(success_color())),
            Span::styled("Commit changes", Style::default().fg(text_color())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ESC / q   ", Style::default().fg(error_color())),
            Span::styled("Close / Exit", Style::default().fg(text_color())),
        ]),
    ];

    let p = Paragraph::new(help_content)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(p, popup_area);
}

fn draw_completion(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let popup_area = centered_rect(area, 40, 30);
    draw_shadow(f, popup_area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .bg(background_surface())
        .padding(ratatui::widgets::Padding::new(1, 1, 1, 1));

    let mut list_items = vec![
        Line::from(vec![
            Span::styled(" 󰋖 ", Style::default().fg(accent_color())),
            Span::styled(
                "SUGGESTIONS ",
                Style::default()
                    .fg(accent_color())
                    .add_modifier(font_weight_bold()),
            ),
        ]),
        Line::from(""),
    ];

    for (i, suggestion) in state.completion_suggestions().iter().enumerate() {
        let is_selected = i == state.completion_index();
        let style = if is_selected {
            selection_style().add_modifier(font_weight_bold())
        } else {
            Style::default().fg(text_color())
        };

        let prefix = if is_selected { " 󰁕 " } else { "   " };
        list_items.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(suggestion, style),
        ]));
    }

    let p = Paragraph::new(list_items)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(p, popup_area);
}

fn draw_context_selection(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_selection_list(f, state, chunks[0]);
    draw_preview(f, state, chunks[1]);
}

fn draw_selection_list(f: &mut Frame, state: &mut TuiState, area: Rect) {
    let block = Block::default()
        .bg(background_base())
        .padding(ratatui::widgets::Padding::new(1, 1, 1, 1));

    let mut list_items = Vec::new();

    if let Some(context) = state.context() {
        // Files Section
        list_items.push(Line::from(vec![
            Span::styled(
                " 󰈔 FILES ",
                Style::default()
                    .fg(accent_color())
                    .add_modifier(font_weight_bold()),
            ),
            Span::styled("━".repeat(10), Style::default().fg(background_overlay())),
        ]));

        for (i, file) in context.staged_files.iter().enumerate() {
            let is_selected = state.selected_files().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Files
                && state.context_selection_index() == i;

            let (checkbox, color) = if is_selected {
                ("󰄲 ", success_color())
            } else {
                ("󰄱 ", subtle_color())
            };

            let style = if is_current {
                selection_style()
            } else {
                Style::default().fg(text_color())
            };

            list_items.push(Line::from(vec![
                Span::styled(if is_current { " 󰁕 " } else { "   " }, style),
                Span::styled(checkbox, Style::default().fg(color)),
                Span::styled(&file.path, style),
            ]));
        }

        list_items.push(Line::from(""));

        // Commits Section
        list_items.push(Line::from(vec![
            Span::styled(
                " 󰜘 HISTORY ",
                Style::default()
                    .fg(secondary_accent_color())
                    .add_modifier(font_weight_bold()),
            ),
            Span::styled("━".repeat(10), Style::default().fg(background_overlay())),
        ]));

        for (i, commit) in context.recent_commits.iter().enumerate() {
            let file_index = context.staged_files.len() + i;
            let is_selected = state.selected_commits().get(i).copied().unwrap_or(true);
            let is_current = state.context_selection_category()
                == super::state::ContextSelectionCategory::Commits
                && state.context_selection_index() == file_index;

            let (checkbox, color) = if is_selected {
                ("󰄲 ", success_color())
            } else {
                ("󰄱 ", subtle_color())
            };

            let style = if is_current {
                selection_style()
            } else {
                Style::default().fg(text_color())
            };

            list_items.push(Line::from(vec![
                Span::styled(if is_current { " 󰁕 " } else { "   " }, style),
                Span::styled(checkbox, Style::default().fg(color)),
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
        .bg(background_surface())
        .padding(ratatui::widgets::Padding::new(2, 2, 1, 1));

    if let Some(context) = state.context() {
        if state.context_selection_category() == super::state::ContextSelectionCategory::Files {
            if let Some(file) = context.staged_files.get(state.context_selection_index()) {
                let mut lines = Vec::new();
                lines.push(Line::from(vec![
                    Span::styled("󰈔 ", Style::default().fg(accent_color())),
                    Span::styled(
                        &file.path,
                        Style::default().add_modifier(font_weight_bold()),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(subtle_color())),
                    Span::raw(format!("{}", file.change_type)),
                ]));
                lines.push(Line::from(vec![Span::styled(
                    "━".repeat(area.width.saturating_sub(4) as usize),
                    Style::default().fg(background_overlay()),
                )]));

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

                f.render_widget(
                    Paragraph::new(lines)
                        .block(block)
                        .wrap(Wrap { trim: false }),
                    area,
                );
                return;
            }
        } else {
            let commit_index = state
                .context_selection_index()
                .saturating_sub(context.staged_files.len());
            if let Some(commit) = context.recent_commits.get(commit_index) {
                let lines = vec![
                    Line::from(vec![
                        Span::styled("󰜘 ", Style::default().fg(secondary_accent_color())),
                        Span::styled(
                            &commit.hash,
                            Style::default().add_modifier(font_weight_bold()),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Date: ", Style::default().fg(subtle_color())),
                        Span::raw(&commit.timestamp),
                    ]),
                    Line::from(vec![Span::styled(
                        "━".repeat(area.width.saturating_sub(4) as usize),
                        Style::default().fg(background_overlay()),
                    )]),
                    Line::from(Span::styled(
                        &commit.message,
                        Style::default().fg(text_color()),
                    )),
                ];
                f.render_widget(
                    Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
                    area,
                );
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
