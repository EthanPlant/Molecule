use log::{Level, Metadata, Record};

static LOGGER: MoleculeLogger = MoleculeLogger;

struct MoleculeLogger;

impl log::Log for MoleculeLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            use crate::drivers::uart::*;

            let file = record.file().unwrap_or("unknown");
            let file = file.strip_prefix("kernel/src").unwrap_or(file);

            let line = record.line().unwrap_or(0);

            let level = record.level();

            macro generic_log($($arg:tt)*) {
                serial_print!("{}", format_args!($($arg)*))
            }

            match level {
                Level::Error => generic_log!("\x1b[31m[ERROR]"),
                Level::Warn => generic_log!("\x1b[33m[WARN]"),
                Level::Info => generic_log!("\x1b[32m[INFO]"),
                Level::Debug => generic_log!("\x1b[34m[DEBUG]"),
                Level::Trace => generic_log!("\x1b[37m[TRACE]"),
            };
            generic_log!("\x1b[0m {}:{} - {}\n", file, line, record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap();
}
