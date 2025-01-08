use std::{io::Write, sync::Once};

use env_logger::{Builder, Env};
use log::LevelFilter;

use super::super::config::LogLevel;

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
        let env = Env::default();
        let mut builder = Builder::from_env(env);
        builder
            .format(|buf, record| {
                if record.level() == log::Level::Error {
                    writeln!(
                        buf,
                        "[{}] {} {}:{} - {}",
                        record.level(),
                        record.target(),
                        record.file().unwrap_or("unknown"),
                        record.line().unwrap_or(0),
                        record.args()
                    )
                } else {
                    writeln!(
                        buf,
                        "[{}] {} - {}",
                        record.level(),
                        record.target(),
                        record.args()
                    )
                }
            })
            .filter(None, level.into())
            .try_init()
            .expect("LOG INITIALIZATION FAILED");
    });
}

pub fn set_log_level(level: LogLevel) {
    log::set_max_level(level.into())
}
