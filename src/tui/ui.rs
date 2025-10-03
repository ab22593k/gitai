use super::state::{EmojiMode, Mode, TuiState, UserInfoFocus};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn draw_ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Title
                Constraint::Length(2), // Navigation bar
                Constraint::Length(2), // User info
                Constraint::Min(5),    // Commit message
                Constraint::Length(8), // Instructions
                Constraint::Length(3), // Emoji and Preset
                Constraint::Length(1), // Status
            ]
            .as_ref(),
        )
        .split(f.area());

    let theme = state.theme.clone();
    draw_title(f, chunks[0], &theme);
    draw_nav_bar(f, chunks[1], &theme);
    draw_user_info(f, state, chunks[2], &theme);
    draw_commit_message(f, state, chunks[3], &theme);
    draw_instructions(f, state, chunks[4], &theme);
    draw_emoji_preset(f, state, chunks[5], &theme);
    draw_status(f, state, chunks[6], &theme);

    if state.mode == Mode::SelectingEmoji {
        draw_emoji_popup(f, state, &theme);
    } else if state.mode == Mode::SelectingPreset {
        draw_preset_popup(f, state, &theme);
    } else if state.mode == Mode::SelectingTheme {
        draw_theme_popup(f, state, &theme);
    } else if state.mode == Mode::EditingUserInfo {
        draw_user_info_popup(f, state, &theme);
    } else if state.mode == Mode::Help {
        draw_help_popup(f, &theme);
    }
}

fn draw_title(f: &mut Frame, area: Rect, theme: &super::theme::Theme) {
    let gradient_colors = &theme.title_gradient;

    // Define the title with emojis and text
    let title_text = format!("Pilot v{APP_VERSION} - Cosmic Commit");
    let prefix_emoji = "✨🔮 ";
    let suffix_emoji = " 🔮✨";

    let text_len = title_text.chars().count();
    let mid_point = text_len / 2;

    // Apply gradient to the first half of the title text
    let first_half: Vec<Span> = title_text
        .chars()
        .take(mid_point)
        .enumerate()
        .map(|(i, c)| {
            let color_index = i * (gradient_colors.len() - 1) / (mid_point - 1);
            Span::styled(
                c.to_string(),
                Style::default()
                    .fg(gradient_colors[color_index])
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    // Apply gradient to the second half of the title text (reverse gradient)
    let second_half: Vec<Span> = title_text
        .chars()
        .skip(mid_point)
        .enumerate()
        .map(|(i, c)| {
            let color_index = i * (gradient_colors.len() - 1) / (text_len - mid_point - 1);
            Span::styled(
                c.to_string(),
                Style::default()
                    .fg(gradient_colors[gradient_colors.len() - 1 - color_index])
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    // Combine prefix emoji, gradient text, and suffix emoji
    let mut title_line = vec![Span::styled(
        prefix_emoji,
        Style::default().fg(theme.starlight),
    )];
    title_line.extend(first_half);
    title_line.extend(second_half);
    title_line.push(Span::styled(
        suffix_emoji,
        Style::default().fg(theme.starlight),
    ));

    // Create a paragraph with the title text, center-aligned
    let title_widget = Paragraph::new(Line::from(title_line)).alignment(Alignment::Center);

    f.render_widget(title_widget, area);
}

fn draw_nav_bar(f: &mut Frame, area: Rect, theme: &super::theme::Theme) {
    let nav_items = vec![
        ("←→", "Navigate", theme.celestial_blue),
        ("E", "Message", theme.solar_yellow),
        ("I", "Instructions", theme.aurora_green),
        ("G", "Emoji", theme.plasma_cyan),
        ("P", "Preset", theme.comet_orange),
        ("U", "User Info", theme.galaxy_pink),
        ("T", "Theme", theme.nebula_purple),
        ("?", "Help", theme.starlight),
        ("R", "Regenerate", theme.meteor_red),
        ("⏎", "Commit", theme.starlight),
    ];
    let nav_spans: Vec<Span> = nav_items
        .into_iter()
        .flat_map(|(key, desc, color)| {
            vec![
                Span::styled(
                    key.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(": {desc} "),
                    Style::default().fg(theme.nebula_purple),
                ),
            ]
        })
        .collect();
    let nav_bar =
        Paragraph::new(Line::from(nav_spans)).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(nav_bar, area);
}

fn draw_user_info(f: &mut Frame, state: &TuiState, area: Rect, theme: &super::theme::Theme) {
    let user_info = Paragraph::new(Line::from(vec![
        Span::styled(
            "👤 ",
            Style::default()
                .fg(theme.plasma_cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &state.user_name,
            Style::default()
                .fg(theme.aurora_green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            "✉️ ",
            Style::default()
                .fg(theme.plasma_cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &state.user_email,
            Style::default()
                .fg(theme.aurora_green)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(Style::default())
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(user_info, area);
}

fn draw_commit_message(
    f: &mut Frame,
    state: &mut TuiState,
    area: Rect,
    theme: &super::theme::Theme,
) {
    let message_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.celestial_blue))
        .title(Span::styled(
            format!(
                "✦ Commit Message ({}/{})",
                state.current_index + 1,
                state.messages.len()
            ),
            Style::default()
                .fg(theme.galaxy_pink)
                .add_modifier(Modifier::BOLD),
        ));

    if state.mode == Mode::EditingMessage {
        state.message_textarea.set_block(message_block);
        state
            .message_textarea
            .set_style(Style::default().fg(theme.solar_yellow));
        f.render_widget(&state.message_textarea, area);
    } else {
        let current_message = &state.messages[state.current_index];
        let emoji_prefix = state
            .get_current_emoji()
            .map_or(String::new(), |e| format!("{e} "));
        let message_content = format!(
            "{}{}\n\n{}",
            emoji_prefix, current_message.title, current_message.message
        );
        let message = Paragraph::new(message_content)
            .block(message_block)
            .style(Style::default().fg(theme.solar_yellow))
            .wrap(Wrap { trim: true });
        f.render_widget(message, area);
    }
}

fn draw_instructions(f: &mut Frame, state: &mut TuiState, area: Rect, theme: &super::theme::Theme) {
    let instructions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.celestial_blue))
        .title(Span::styled(
            "✧ Custom Instructions",
            Style::default()
                .fg(theme.galaxy_pink)
                .add_modifier(Modifier::BOLD),
        ));

    if state.mode == Mode::EditingInstructions {
        state.instructions_textarea.set_block(instructions_block);
        state
            .instructions_textarea
            .set_style(Style::default().fg(theme.plasma_cyan));
        f.render_widget(&state.instructions_textarea, area);
    } else {
        let instructions = Paragraph::new(state.custom_instructions.clone())
            .block(instructions_block)
            .style(Style::default().fg(theme.plasma_cyan))
            .wrap(Wrap { trim: true });
        f.render_widget(instructions, area);
    }
}

fn draw_emoji_preset(f: &mut Frame, state: &TuiState, area: Rect, theme: &super::theme::Theme) {
    let preset_with_emoji = state.get_selected_preset_name_with_emoji();
    let emoji_display = match &state.emoji_mode {
        EmojiMode::None => "None".to_string(),
        EmojiMode::Auto => "Auto".to_string(),
        EmojiMode::Custom(emoji) => emoji.clone(),
    };
    let emoji_preset = Paragraph::new(Line::from(vec![
        Span::styled(
            "Emoji: ",
            Style::default()
                .fg(theme.nebula_purple)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            emoji_display,
            Style::default()
                .fg(theme.solar_yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  "),
        Span::styled(
            "Preset: ",
            Style::default()
                .fg(theme.nebula_purple)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            preset_with_emoji,
            Style::default()
                .fg(theme.comet_orange)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  "),
        Span::styled(
            "Theme: ",
            Style::default()
                .fg(theme.nebula_purple)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &state.theme.name,
            Style::default()
                .fg(theme.plasma_cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(Style::default())
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(emoji_preset, area);
}

pub fn draw_status(f: &mut Frame, state: &mut TuiState, area: Rect, theme: &super::theme::Theme) {
    let (spinner_with_space, status_content, color, content_width) =
        if let Some(spinner) = &mut state.spinner {
            spinner.tick()
        } else {
            (
                "  ".to_string(),
                state.status.clone(),
                theme.aurora_green,
                state.status.width() + 2,
            )
        };

    #[allow(clippy::as_conversions)]
    let terminal_width = f.area().width as usize;

    // Ensure we don't overflow when calculating padding
    let left_padding = if content_width >= terminal_width {
        0
    } else {
        (terminal_width - content_width) / 2
    };
    let right_padding = if content_width >= terminal_width {
        0
    } else {
        terminal_width - content_width - left_padding
    };

    let status_line = Line::from(vec![
        Span::raw(" ".repeat(left_padding)),
        Span::styled(spinner_with_space, Style::default().fg(theme.plasma_cyan)),
        Span::styled(status_content, Style::default().fg(color)),
        Span::raw(" ".repeat(right_padding)),
    ]);

    let status_widget =
        Paragraph::new(vec![status_line]).alignment(ratatui::layout::Alignment::Left);
    f.render_widget(Clear, area); // Clear the entire status line
    f.render_widget(status_widget, area);
}

fn draw_emoji_popup(f: &mut Frame, state: &mut TuiState, theme: &super::theme::Theme) {
    let popup_block = Block::default()
        .title(Span::styled(
            "✨ Select Emoji",
            Style::default()
                .fg(theme.solar_yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.nebula_purple));

    let area = f.area();
    let popup_area = Rect::new(
        area.x + 10,
        area.y + 5,
        area.width.saturating_sub(20).min(60),
        area.height.saturating_sub(10).min(20),
    );

    let items: Vec<ListItem> = state
        .emoji_list
        .iter()
        .map(|(emoji, description)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{emoji} "), Style::default().fg(theme.solar_yellow)),
                Span::styled(description, Style::default().fg(theme.plasma_cyan)),
            ]))
        })
        .collect();

    let list = List::new(items).block(popup_block).highlight_style(
        Style::default()
            .bg(theme.celestial_blue)
            .fg(theme.starlight)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut state.emoji_list_state);
}

fn draw_preset_popup(f: &mut Frame, state: &mut TuiState, theme: &super::theme::Theme) {
    let popup_block = Block::default()
        .title(Span::styled(
            "🌟 Select Preset",
            Style::default()
                .fg(theme.comet_orange)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.nebula_purple));

    let area = f.area();
    let popup_area = Rect::new(
        area.x + 5,
        area.y + 5,
        area.width.saturating_sub(10).min(70),
        area.height.saturating_sub(10).min(20),
    );

    let items: Vec<ListItem> = state
        .preset_list
        .iter()
        .map(|(_, emoji, name, description)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{emoji} {name} "),
                    Style::default().fg(theme.comet_orange),
                ),
                Span::styled(description, Style::default().fg(theme.plasma_cyan)),
            ]))
        })
        .collect();

    let list = List::new(items).block(popup_block).highlight_style(
        Style::default()
            .bg(theme.celestial_blue)
            .fg(theme.starlight)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut state.preset_list_state);
}

fn draw_user_info_popup(f: &mut Frame, state: &mut TuiState, theme: &super::theme::Theme) {
    let popup_block = Block::default()
        .title(Span::styled(
            "Edit User Info",
            Style::default()
                .fg(theme.solar_yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.nebula_purple));

    let area = f.area();
    let popup_area = Rect::new(
        area.x + 10,
        area.y + 5,
        area.width.saturating_sub(20).min(60),
        area.height.saturating_sub(10).min(10),
    );

    let popup_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Name
                Constraint::Length(3), // Email
            ]
            .as_ref(),
        )
        .split(popup_area);

    state.user_name_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if state.user_info_focus == UserInfoFocus::Name {
                    theme.solar_yellow
                } else {
                    theme.celestial_blue
                }),
            )
            .title("Name"),
    );

    state.user_email_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default().fg(if state.user_info_focus == UserInfoFocus::Email {
                    theme.solar_yellow
                } else {
                    theme.celestial_blue
                }),
            )
            .title("Email"),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(popup_block, popup_area);
    f.render_widget(&state.user_name_textarea, popup_chunks[0]);
    f.render_widget(&state.user_email_textarea, popup_chunks[1]);
}

fn draw_theme_popup(f: &mut Frame, state: &mut TuiState, theme: &super::theme::Theme) {
    let popup_block = Block::default()
        .title(Span::styled(
            "🎨 Select Theme",
            Style::default()
                .fg(theme.comet_orange)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.nebula_purple));

    let area = f.area();
    let popup_area = Rect::new(
        area.x + 5,
        area.y + 5,
        area.width.saturating_sub(10).min(40),
        area.height.saturating_sub(10).min(15),
    );

    let items: Vec<ListItem> = state
        .theme_list
        .iter()
        .map(|t| {
            ListItem::new(Span::styled(
                &t.name,
                Style::default().fg(theme.plasma_cyan),
            ))
        })
        .collect();

    let list = List::new(items).block(popup_block).highlight_style(
        Style::default()
            .bg(theme.celestial_blue)
            .fg(theme.starlight)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut state.theme_list_state);
}

fn draw_help_popup(f: &mut Frame, theme: &super::theme::Theme) {
    let popup_block = Block::default()
        .title(Span::styled(
            "❓ Help",
            Style::default()
                .fg(theme.solar_yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.nebula_purple));

    let area = f.area();
    let popup_area = Rect::new(
        area.x + 2,
        area.y + 2,
        area.width.saturating_sub(4).min(70),
        area.height.saturating_sub(4).min(25),
    );

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default()
                .fg(theme.aurora_green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ←→", Style::default().fg(theme.celestial_blue)),
            Span::raw(" Navigate between commit messages"),
        ]),
        Line::from(vec![
            Span::styled("  E", Style::default().fg(theme.solar_yellow)),
            Span::raw(" Edit current commit message"),
        ]),
        Line::from(vec![
            Span::styled("  I", Style::default().fg(theme.plasma_cyan)),
            Span::raw(" Edit custom instructions"),
        ]),
        Line::from(vec![
            Span::styled("  G", Style::default().fg(theme.plasma_cyan)),
            Span::raw(" Select emoji mode"),
        ]),
        Line::from(vec![
            Span::styled("  P", Style::default().fg(theme.comet_orange)),
            Span::raw(" Select preset"),
        ]),
        Line::from(vec![
            Span::styled("  T", Style::default().fg(theme.nebula_purple)),
            Span::raw(" Change theme"),
        ]),
        Line::from(vec![
            Span::styled("  U", Style::default().fg(theme.galaxy_pink)),
            Span::raw(" Edit user info"),
        ]),
        Line::from(vec![
            Span::styled("  R", Style::default().fg(theme.meteor_red)),
            Span::raw(" Regenerate commit message"),
        ]),
        Line::from(vec![
            Span::styled("  ⏎", Style::default().fg(theme.starlight)),
            Span::raw(" Commit with current message"),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(theme.meteor_red)),
            Span::raw(" Exit or cancel current operation"),
        ]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(theme.starlight)),
            Span::raw(" Show this help"),
        ]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(popup_block)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}
