use chrono::Local;
use console::Style;
use std::fmt::Display;

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Trace, format!($($arg)*));
    }};
}

#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        if false {
            let _ = format_args!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Info, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Warn, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Error, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Debug, format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        $crate::cli::log::_log(crate::cli::log::Level::Success, format!($($arg)*));
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trace,
    Info,
    Warn,
    Error,
    Debug,
    Success,
}

pub fn _log(level: Level, message: impl Display) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    
    let term = console::Term::stdout();
    
    if term.features().colors_supported() {
        match level {
            #[cfg(feature = "trace")]
            Level::Trace => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().white().dim();
                println!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("TRACE"),
                    message
                );
            }
            Level::Error => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().red().bold();
                eprintln!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("ERROR"),
                    message
                );
            }
            Level::Warn => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().yellow().bold();
                println!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("WARN"),
                    message
                );
            }
            Level::Info => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().cyan().bold();
                println!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("INFO"),
                    message
                );
            }
            Level::Debug => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().magenta().bold();
                println!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("DEBUG"),
                    message
                );
            }
            Level::Success => {
                let timestamp_style = Style::new().dim();
                let level_style = Style::new().green().bold();
                println!("{} [{}] {}", 
                    timestamp_style.apply_to(timestamp.to_string()),
                    level_style.apply_to("SUCCESS"),
                    message
                );
            }
            #[cfg(not(feature = "trace"))]
            Level::Trace => {
                unreachable!();
            }
        }
    } else {
        match level {
            #[cfg(feature = "trace")]
            Level::Trace => {
                println!("{} [TRACE] {}", timestamp, message);
            }
            Level::Error => {
                eprintln!("{} [ERROR] {}", timestamp, message);
            }
            Level::Warn => {
                println!("{} [WARN] {}", timestamp, message);
            }
            Level::Info => {
                println!("{} [INFO] {}", timestamp, message);
            }
            Level::Debug => {
                println!("{} [DEBUG] {}", timestamp, message);
            }
            Level::Success => {
                println!("{} [SUCCESS] {}", timestamp, message);
            }
            #[cfg(not(feature = "trace"))]
            Level::Trace => {
                unreachable!();
            }
        }
    }
}