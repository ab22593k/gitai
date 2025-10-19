use super::input_handler::{InputResult, handle_input};
use super::state::{Mode, TuiState};
use super::ui::draw_ui;
use crate::features::rebase::{RebaseAnalysis, RebaseService};
use anyhow::{Error, Result};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

use std::io;
use std::sync::Arc;
use std::time::Duration;

pub struct TuiRebase {
    pub state: TuiState,
    service: Arc<RebaseService>,
    analysis: RebaseAnalysis,
}

impl TuiRebase {
    pub fn new(analysis: RebaseAnalysis, service: Arc<RebaseService>) -> Self {
        let mut state = TuiState::new(vec![], String::new());
        state.mode = Mode::RebaseList;
        state.set_rebase_commits(analysis.commits.clone());

        Self {
            state,
            service,
            analysis,
        }
    }

    #[allow(clippy::unused_async)]
    pub async fn run(analysis: RebaseAnalysis, service: Arc<RebaseService>) -> Result<()> {
        let mut app = Self::new(analysis, service);

        app.run_app().map_err(Error::from)
    }

    pub fn run_app(&mut self) -> io::Result<()> {
        // Setup
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run main loop
        let result = self.main_loop(&mut terminal);

        // Cleanup
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Handle result
        match result {
            Ok(exit_status) => match exit_status {
                RebaseExitStatus::Completed => {
                    println!("Rebase completed successfully!");
                }
                RebaseExitStatus::Cancelled => {
                    println!("Rebase operation cancelled.");
                }
                RebaseExitStatus::Error(error_message) => {
                    eprintln!("An error occurred: {error_message}");
                }
            },
            Err(e) => {
                eprintln!("An unexpected error occurred: {e}");
                return Err(io::Error::other(e.to_string()));
            }
        }

        Ok(())
    }

    fn main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<RebaseExitStatus> {
        loop {
            // Redraw only if dirty
            if self.state.dirty {
                terminal.draw(|f| draw_ui(f, &mut self.state))?;
                self.state.dirty = false;
            }

            // Poll for input events
            if event::poll(Duration::from_millis(20))?
                && let Event::Key(key) = event::read()?
                && key.kind == crossterm::event::KeyEventKind::Press
            {
                match handle_input(self, key) {
                    InputResult::Exit => return Ok(RebaseExitStatus::Cancelled),
                    InputResult::Continue => self.state.dirty = true,
                    InputResult::Commit(_) => {
                        // This shouldn't happen in rebase mode, but handle gracefully
                        return Ok(RebaseExitStatus::Cancelled);
                    }
                }
            }
        }
    }

    pub fn perform_rebase(&self) -> Result<RebaseExitStatus, Error> {
        // Use the service to perform the rebase with the analysis
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            match self.service.perform_rebase_auto(self.analysis.clone()).await {
                Ok(result) => {
                    if result.success {
                        Ok(RebaseExitStatus::Completed)
                    } else {
                        Ok(RebaseExitStatus::Error("Rebase completed with conflicts".to_string()))
                    }
                }
                Err(e) => Ok(RebaseExitStatus::Error(e.to_string())),
            }
        })
    }
}

pub enum RebaseExitStatus {
    Completed,
    Cancelled,
    Error(String),
}

#[allow(clippy::unused_async)]
pub async fn run_tui_rebase(analysis: RebaseAnalysis, service: Arc<RebaseService>) -> Result<()> {
    TuiRebase::run(analysis, service).await
}
