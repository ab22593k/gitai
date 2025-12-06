use super::spinner::SpinnerState;
use crate::core::context::CommitContext;
use crate::features::commit::types::{GeneratedMessage, format_commit_message};

use tui_textarea::TextArea;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Normal,
    EditingMessage,
    EditingInstructions,
    Generating,
    Help,
    Completing,
    ContextSelection,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContextSelectionCategory {
    Files,
    Commits,
}

pub struct TuiState {
    messages: Vec<GeneratedMessage>,
    current_index: usize,
    custom_instructions: String,
    status: String,
    mode: Mode,
    message_textarea: TextArea<'static>,
    instructions_textarea: TextArea<'static>,
    spinner: Option<SpinnerState>,
    dirty: bool,
    last_spinner_update: std::time::Instant,
    instructions_visible: bool,
    nav_bar_visible: bool,
    completion_suggestions: Vec<String>,
    completion_index: usize,
    pending_completion_prefix: Option<String>,
    // Context selection fields
    context: Option<CommitContext>,
    selected_files: Vec<bool>,      // Which staged files are selected
    selected_commits: Vec<bool>,    // Which recent commits are selected
    context_selection_index: usize, // Current selection index in context selection UI
    context_selection_category: ContextSelectionCategory, // Files or commits
}

impl TuiState {
    pub fn new(initial_messages: Vec<GeneratedMessage>, custom_instructions: String) -> Self {
        let mut message_textarea = TextArea::default();
        let messages = if initial_messages.is_empty() {
            vec![GeneratedMessage {
                title: String::new(),
                message: String::new(),
            }]
        } else {
            initial_messages
        };
        if let Some(first_message) = messages.first() {
            message_textarea.insert_str(format_commit_message(first_message));
        }

        let mut instructions_textarea = TextArea::default();
        instructions_textarea.insert_str(&custom_instructions);

        Self {
            messages,
            current_index: 0,
            custom_instructions,
            status: "Press '?': help | 'Esc': exit".to_string(),
            mode: Mode::Normal,
            message_textarea,
            instructions_textarea,
            spinner: None,
            dirty: true,
            last_spinner_update: std::time::Instant::now(),
            instructions_visible: false,
            nav_bar_visible: true,
            completion_suggestions: Vec::new(),
            completion_index: 0,
            pending_completion_prefix: None,
            // Context selection fields
            context: None,
            selected_files: Vec::new(),
            selected_commits: Vec::new(),
            context_selection_index: 0,
            context_selection_category: ContextSelectionCategory::Files,
        }
    }

    // -- Getters --

    pub fn messages(&self) -> &[GeneratedMessage] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<GeneratedMessage> {
        &mut self.messages
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn current_message(&self) -> &GeneratedMessage {
        &self.messages[self.current_index]
    }

    pub fn custom_instructions(&self) -> &str {
        &self.custom_instructions
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn message_textarea(&self) -> &TextArea<'static> {
        &self.message_textarea
    }

    pub fn message_textarea_mut(&mut self) -> &mut TextArea<'static> {
        &mut self.message_textarea
    }

    pub fn instructions_textarea(&self) -> &TextArea<'static> {
        &self.instructions_textarea
    }

    pub fn instructions_textarea_mut(&mut self) -> &mut TextArea<'static> {
        &mut self.instructions_textarea
    }

    pub fn spinner(&self) -> Option<&SpinnerState> {
        self.spinner.as_ref()
    }

    pub fn spinner_mut(&mut self) -> Option<&mut SpinnerState> {
        self.spinner.as_mut()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn last_spinner_update(&self) -> std::time::Instant {
        self.last_spinner_update
    }

    pub fn is_instructions_visible(&self) -> bool {
        self.instructions_visible
    }

    pub fn is_nav_bar_visible(&self) -> bool {
        self.nav_bar_visible
    }

    pub fn completion_suggestions(&self) -> &[String] {
        &self.completion_suggestions
    }

    pub fn completion_index(&self) -> usize {
        self.completion_index
    }

    pub fn pending_completion_prefix(&self) -> Option<&String> {
        self.pending_completion_prefix.as_ref()
    }

    pub fn context(&self) -> Option<&CommitContext> {
        self.context.as_ref()
    }

    pub fn selected_files(&self) -> &[bool] {
        &self.selected_files
    }

    pub fn selected_commits(&self) -> &[bool] {
        &self.selected_commits
    }

    pub fn context_selection_index(&self) -> usize {
        self.context_selection_index
    }

    pub fn context_selection_category(&self) -> ContextSelectionCategory {
        self.context_selection_category
    }

    // -- Setters and Modifiers --

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.dirty = true;
    }

    pub fn set_status<S: Into<String>>(&mut self, status: S) {
        self.status = status.into();
        self.spinner = None;
        self.dirty = true;
    }

    pub fn set_custom_instructions(&mut self, instructions: String) {
        self.custom_instructions = instructions;
        self.dirty = true;
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub fn set_spinner(&mut self, spinner: Option<SpinnerState>) {
        self.spinner = spinner;
        self.dirty = true;
    }

    pub fn set_last_spinner_update(&mut self, time: std::time::Instant) {
        self.last_spinner_update = time;
    }

    pub fn set_messages(&mut self, messages: Vec<GeneratedMessage>) {
        self.messages = messages;
        self.dirty = true;
    }

    pub fn set_current_index(&mut self, index: usize) {
        if index < self.messages.len() {
            self.current_index = index;
            self.update_message_textarea();
            self.dirty = true;
        }
    }

    pub fn add_message(&mut self, message: GeneratedMessage) {
        self.messages.push(message);
        self.current_index = self.messages.len() - 1;
        self.update_message_textarea();
        self.dirty = true;
    }

    pub fn set_completion_suggestions(&mut self, suggestions: Vec<String>) {
        self.completion_suggestions = suggestions;
        self.completion_index = 0;
        self.dirty = true;
    }

    pub fn set_pending_completion_prefix(&mut self, prefix: Option<String>) {
        self.pending_completion_prefix = prefix;
    }

    pub fn update_message_textarea(&mut self) {
        let current_message = &self.messages[self.current_index];
        let message_content = format!(
            "{}\n\n{}",
            current_message.title,
            current_message.message.trim()
        );

        let mut new_textarea = TextArea::default();
        new_textarea.insert_str(&message_content);
        self.message_textarea = new_textarea;
        self.dirty = true;
    }

    pub fn toggle_instructions_visibility(&mut self) {
        self.instructions_visible = !self.instructions_visible;
        self.dirty = true;
    }

    pub fn next_message(&mut self) {
        if self.current_index + 1 < self.messages.len() {
            self.current_index += 1;
            self.update_message_textarea();
            self.dirty = true;
        }
    }

    pub fn previous_message(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.update_message_textarea();
            self.dirty = true;
        }
    }

    pub fn next_completion(&mut self) {
        if self.completion_index + 1 < self.completion_suggestions.len() {
            self.completion_index += 1;
            self.dirty = true;
        }
    }

    pub fn previous_completion(&mut self) {
        if self.completion_index > 0 {
            self.completion_index -= 1;
            self.dirty = true;
        }
    }

    pub fn update_current_message_from_textarea(&mut self) {
        let content = self.message_textarea.lines().join("\n");
        let parts: Vec<&str> = content.splitn(2, "\n\n").collect();
        if parts.len() == 2 {
            self.messages[self.current_index].title = parts[0].trim().to_string();
            self.messages[self.current_index].message = parts[1].trim().to_string();
        } else {
            self.messages[self.current_index].title = content.trim().to_string();
            self.messages[self.current_index].message = String::new();
        }
        self.dirty = true;
    }

    pub fn update_instructions_from_textarea(&mut self) {
        self.custom_instructions = self.instructions_textarea.lines().join("\n");
        self.dirty = true;
    }

    /// Initialize context for selection
    pub fn initialize_context(&mut self, context: CommitContext) {
        self.context = Some(context);
        // Initialize all files and commits as selected by default
        if let Some(ctx) = &self.context {
            self.selected_files = vec![true; ctx.staged_files.len()];
            self.selected_commits = vec![true; ctx.recent_commits.len()];
        }
        self.context_selection_index = 0;
        self.context_selection_category = ContextSelectionCategory::Files;
        self.dirty = true;
    }

    /// Toggle selection of current item
    pub fn toggle_current_selection(&mut self) {
        if let Some(ctx) = &self.context {
            match self.context_selection_category {
                ContextSelectionCategory::Files => {
                    if self.context_selection_index < self.selected_files.len() {
                        self.selected_files[self.context_selection_index] =
                            !self.selected_files[self.context_selection_index];
                    }
                }
                ContextSelectionCategory::Commits => {
                    let commit_index = self
                        .context_selection_index
                        .saturating_sub(ctx.staged_files.len());
                    if commit_index < self.selected_commits.len() {
                        self.selected_commits[commit_index] = !self.selected_commits[commit_index];
                    }
                }
            }
        }
        self.dirty = true;
    }

    /// Switch to next category or wrap around
    pub fn next_category(&mut self) {
        if let Some(ctx) = &self.context {
            match self.context_selection_category {
                ContextSelectionCategory::Files => {
                    if !ctx.recent_commits.is_empty() {
                        self.context_selection_category = ContextSelectionCategory::Commits;
                        self.context_selection_index = ctx.staged_files.len();
                    }
                }
                ContextSelectionCategory::Commits => {
                    if !ctx.staged_files.is_empty() {
                        self.context_selection_category = ContextSelectionCategory::Files;
                        self.context_selection_index = 0;
                    }
                }
            }
        }
        self.dirty = true;
    }

    /// Move selection up
    pub fn move_selection_up(&mut self) {
        if let Some(ctx) = &self.context
            && self.context_selection_index > 0
        {
            self.context_selection_index -= 1;
            // Check if we need to switch categories
            if self.context_selection_category == ContextSelectionCategory::Commits
                && self.context_selection_index < ctx.staged_files.len()
            {
                self.context_selection_category = ContextSelectionCategory::Files;
            }
        }
        self.dirty = true;
    }

    /// Move selection down
    pub fn move_selection_down(&mut self) {
        if let Some(ctx) = &self.context {
            let total_items = ctx.staged_files.len() + ctx.recent_commits.len();
            if self.context_selection_index < total_items.saturating_sub(1) {
                self.context_selection_index += 1;
                // Check if we need to switch categories
                if self.context_selection_category == ContextSelectionCategory::Files
                    && self.context_selection_index >= ctx.staged_files.len()
                {
                    self.context_selection_category = ContextSelectionCategory::Commits;
                }
            }
        }
        self.dirty = true;
    }

    /// Get filtered context based on selections
    pub fn get_filtered_context(&self) -> Option<CommitContext> {
        self.context.as_ref().map(|ctx| {
            let mut filtered = ctx.clone();
            // Filter staged files
            filtered.staged_files = ctx
                .staged_files
                .iter()
                .enumerate()
                .filter(|(i, _)| self.selected_files.get(*i).copied().unwrap_or(true))
                .map(|(_, file)| file.clone())
                .collect();
            // Filter recent commits
            filtered.recent_commits = ctx
                .recent_commits
                .iter()
                .enumerate()
                .filter(|(i, _)| self.selected_commits.get(*i).copied().unwrap_or(true))
                .map(|(_, commit)| commit.clone())
                .collect();
            filtered
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context::{ChangeType, RecentCommit, StagedFile};

    #[test]
    fn test_get_filtered_context_filters_files_and_commits() {
        // Create a mock context
        let context = CommitContext {
            branch: "main".to_string(),
            recent_commits: vec![
                RecentCommit {
                    hash: "abc123".to_string(),
                    message: "First commit".to_string(),
                    timestamp: "1234567890".to_string(),
                },
                RecentCommit {
                    hash: "def456".to_string(),
                    message: "Second commit".to_string(),
                    timestamp: "1234567891".to_string(),
                },
            ],
            staged_files: vec![
                StagedFile {
                    path: "file1.txt".to_string(),
                    change_type: ChangeType::Modified,
                    diff: "+ change".to_string(),
                    content: None,
                    content_excluded: false,
                },
                StagedFile {
                    path: "file2.txt".to_string(),
                    change_type: ChangeType::Added,
                    diff: "+ new file".to_string(),
                    content: None,
                    content_excluded: false,
                },
            ],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            author_history: vec![],
        };

        let mut state = TuiState::new(vec![], "test".to_string());
        state.initialize_context(context);

        // Initially all should be selected
        assert_eq!(state.selected_files.len(), 2);
        assert_eq!(state.selected_commits.len(), 2);
        assert!(state.selected_files.iter().all(|&x| x));
        assert!(state.selected_commits.iter().all(|&x| x));

        // Deselect first file and first commit
        state.selected_files[0] = false;
        state.selected_commits[0] = false;

        let filtered = state
            .get_filtered_context()
            .expect("Context should be available");

        // Should have 1 file and 1 commit
        assert_eq!(filtered.staged_files.len(), 1);
        assert_eq!(filtered.recent_commits.len(), 1);
        assert_eq!(filtered.staged_files[0].path, "file2.txt");
        assert_eq!(filtered.recent_commits[0].hash, "def456");
    }

    #[test]
    fn test_get_filtered_context_returns_none_when_no_context() {
        let state = TuiState::new(vec![], "test".to_string());
        assert!(state.get_filtered_context().is_none());
    }

    #[test]
    fn test_toggle_current_selection_files() {
        let context = CommitContext {
            branch: "main".to_string(),
            recent_commits: vec![],
            staged_files: vec![StagedFile {
                path: "file1.txt".to_string(),
                change_type: ChangeType::Modified,
                diff: "+ change".to_string(),
                content: None,
                content_excluded: false,
            }],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            author_history: vec![],
        };

        let mut state = TuiState::new(vec![], "test".to_string());
        state.initialize_context(context);
        state.context_selection_category = ContextSelectionCategory::Files;
        state.context_selection_index = 0;

        // Initially selected
        assert!(state.selected_files[0]);

        // Toggle to deselect
        state.toggle_current_selection();
        assert!(!state.selected_files[0]);

        // Toggle back to select
        state.toggle_current_selection();
        assert!(state.selected_files[0]);
    }

    #[test]
    fn test_toggle_current_selection_commits() {
        let context = CommitContext {
            branch: "main".to_string(),
            recent_commits: vec![RecentCommit {
                hash: "abc123".to_string(),
                message: "First commit".to_string(),
                timestamp: "1234567890".to_string(),
            }],
            staged_files: vec![],
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            author_history: vec![],
        };

        let mut state = TuiState::new(vec![], "test".to_string());
        state.initialize_context(context);
        state.context_selection_category = ContextSelectionCategory::Commits;
        state.context_selection_index = 0; // This should point to the first commit

        // Initially selected
        assert!(state.selected_commits[0]);

        // Toggle to deselect
        state.toggle_current_selection();
        assert!(!state.selected_commits[0]);
    }
}
