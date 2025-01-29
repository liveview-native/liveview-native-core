use std::sync::Once;

use log::LevelFilter;

use crate::client::LogLevel;

static INIT_LOG: Once = Once::new();

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
        }
    }
}

pub fn init_log(level: LogLevel) {
    INIT_LOG.call_once(|| {
        platform::init_log(level);
    });
}

pub fn set_log_level(level: LogLevel) {
    log::set_max_level(level.into())
}

#[cfg(all(target_os = "android", not(test)))]
mod platform {
    use super::*;

    pub fn init_log(level: LogLevel) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(level.into())
                .with_tag("LiveViewNative")
                .format(|f, record| {
                    if record.level() == log::Level::Error {
                        writeln!(
                            f,
                            "[{}] {} {}:{} - {}",
                            record.level(),
                            record.target(),
                            record.file().unwrap_or("unknown"),
                            record
                                .line()
                                .map(|line| line.to_string())
                                .as_deref()
                                .unwrap_or("unknown"),
                            record.args()
                        )
                    } else {
                        writeln!(
                            f,
                            "[{}] {} - {}",
                            record.level(),
                            record.target(),
                            record.args()
                        )
                    }
                }),
        );
    }
}

#[cfg(all(target_vendor = "apple", not(test)))]
mod platform {
    use super::*;

    pub fn init_log(level: LogLevel) {
        if let Err(e) = oslog::OsLogger::new("com.liveview.core.lib")
            .level_filter(level.into())
            // For some reason uniffi really loves printing every fn call, for a dom, that sucks
            .category_level_filter("liveview_native_core::dom::node", LevelFilter::Warn)
            .category_level_filter("liveview_native_core::dom::ffi", LevelFilter::Warn)
            .init()
        {
            eprintln!("{e}");
        }
    }
}

#[cfg(any(test, not(any(target_os = "android", target_vendor = "apple"))))]
mod platform {
    use std::io::Write;

    use env_logger::{Builder, Env};

    use super::*;

    pub fn init_log(level: LogLevel) {
        let env = Env::default();
        let mut builder = Builder::from_env(env);
        let _ = builder
            .is_test(cfg!(test))
            .format(|formatter, record| {
                if record.level() == log::Level::Error {
                    writeln!(
                        formatter,
                        "[{}] {} {}:{} - {}",
                        record.level(),
                        record.target(),
                        record.file().unwrap_or("unknown"),
                        record
                            .line()
                            .map(|line| line.to_string())
                            .as_deref()
                            .unwrap_or("unknown"),
                        record.args()
                    )
                } else {
                    writeln!(
                        formatter,
                        "[{}] {} - {}",
                        record.level(),
                        record.target(),
                        record.args()
                    )
                }
            })
            .filter(None, level.into())
            .filter(Some("liveview_native_core::dom::node"), LevelFilter::Warn)
            .filter(Some("liveview_native_core::dom::ffi"), LevelFilter::Warn)
            .try_init();
    }
}
