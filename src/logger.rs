use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};
use parking_lot::Mutex;
use std::fs::OpenOptions;
use std::io::Write;

struct GitLogger;

static LOGGER: GitLogger = GitLogger;
static LOGGING_ENABLED: std::sync::LazyLock<Mutex<bool>> =
    std::sync::LazyLock::new(|| Mutex::new(false));
static LOG_FILE: std::sync::LazyLock<Mutex<Option<std::fs::File>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));
static LOG_TO_STDOUT: std::sync::LazyLock<Mutex<bool>> =
    std::sync::LazyLock::new(|| Mutex::new(false));

impl log::Log for GitLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        *LOGGING_ENABLED.lock() && metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let message = format!("{} {} - {}\n", timestamp, record.level(), record.args());

            if let Some(file) = LOG_FILE.lock().as_mut() {
                let _ = file.write_all(message.as_bytes());
                let _ = file.flush();
            }

            if *LOG_TO_STDOUT.lock() {
                print!("{message}");
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug))
}

pub fn enable_logging() {
    let mut logging_enabled = LOGGING_ENABLED.lock();
    *logging_enabled = true;
}

pub fn disable_logging() {
    let mut logging_enabled = LOGGING_ENABLED.lock();
    *logging_enabled = false;
}

pub fn set_log_file(file_path: &str) -> std::io::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    let mut log_file = LOG_FILE.lock();
    *log_file = Some(file);
    Ok(())
}

pub fn set_log_to_stdout(enabled: bool) {
    let mut log_to_stdout = LOG_TO_STDOUT.lock();
    *log_to_stdout = enabled;
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log::error!($($arg)*)
    };
}
