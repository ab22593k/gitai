use crate::tui::spinner::SpinnerState;

use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    terminal::{Clear, ClearType},
};
use std::{
    io::{self, Write},
    time::Duration,
};
use tokio::time;

pub async fn run_with_spinner<F, T>(
    mut spinner: SpinnerState,
    operation: F,
) -> Result<T, anyhow::Error>
where
    F: AsyncFnOnce() -> Result<T, anyhow::Error>,
{
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let spinner_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        loop {
            tokio::select! {
                _ = rx.recv() => break,
                () = time::sleep(Duration::from_millis(100)) => {
                    let (frame, message, _color, _width) = spinner.tick();
                    if let Err(e) = write!(stdout, "\r{frame} {message}") {
                        log::debug!("Spinner write failed: {e}");
                    }
                    if let Err(e) = stdout.flush() {
                        log::debug!("Spinner flush failed: {e}");
                    }
                }
            }
        }
        if let Err(e) = stdout.execute(Clear(ClearType::CurrentLine)) {
            log::debug!("Spinner clear failed: {e}");
        }
        if let Err(e) = write!(stdout, "\r") {
            log::debug!("Spinner write failed: {e}");
        }
        if let Err(e) = stdout.flush() {
            log::debug!("Spinner flush failed: {e}");
        }
    });

    let result = operation().await;

    if tx.send(()).await.is_err() {
        log::debug!("Spinner stop signal channel closed");
    }
    spinner_handle.await?;

    result
}
