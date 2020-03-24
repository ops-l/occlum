use super::process;
/// Log infrastructure.
///
/// There are five APIs for producing log messages:
/// 1. `error!`
/// 2. `warn!`
/// 3. `info!`
/// 4. `debug!`
/// 5. `trace!`
/// which corresponds to five different log levels.
///
/// Safety. Sensitive, internal info may be leaked though log messages. To prevent
/// this from happening, the current solution is to turn off the log entirely
/// when initializing the log infrastructure, if the enclave is in release mode.
///
/// Note. Do not use log as a way to display critical info to users as log may be
/// turned off (even the error messages). For such messages, use `println!` or
/// `eprintln!` directly.
use log::*;
use std::cell::Cell;

pub use log::{max_level, LevelFilter};

/// Initialize the log infrastructure with the given log level.
pub fn init(level: LevelFilter) {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).expect("logger cannot be set twice");
    log::set_max_level(level);
}

/// Notify the logger that a new round starts.
///
/// Log messages generated in a thread are organized in _rounds_. Each round
/// is a group of related log messages. For examples, all log messages generated
/// during the execution of a single system call may belong to the same round.
pub fn next_round(desc: Option<&'static str>) {
    ROUND_COUNT.with(|cell| {
        cell.set(cell.get() + 1);
    });
    ROUND_DESC.with(|cell| {
        cell.set(desc);
    });
}

/// Set the description of the current round
pub fn set_round_desc(desc: Option<&'static str>) {
    ROUND_DESC.with(|cell| {
        cell.set(desc);
    });
}

fn round_count() -> u64 {
    ROUND_COUNT.with(|cell| cell.get())
}

fn round_desc() -> Option<&'static str> {
    ROUND_DESC.with(|cell| cell.get())
}

thread_local! {
    static ROUND_COUNT : Cell<u64> = Default::default();
    static ROUND_DESC : Cell<Option<&'static str>> = Default::default();
}

/// A simple logger that adds thread and round info to log messages.
struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Parts of message
            let level = record.level();
            let tid = process::get_current_tid();
            let rounds = round_count();
            let desc = round_desc();
            // Message (null-terminated)
            let message = if let Some(desc) = desc {
                format!(
                    "[{:>5}][T{}][#{}][{:·>8}] {}\0",
                    level,
                    tid,
                    rounds,
                    desc,
                    record.args()
                )
            } else {
                format!("[{:>5}][T{}][#{}] {}\0", level, tid, rounds, record.args())
            };
            // Print the message
            unsafe {
                occlum_ocall_print_log(level as u32, message.as_ptr());
            }
        }
    }
    fn flush(&self) {
        unsafe {
            occlum_ocall_flush_log();
        }
    }
}

extern "C" {
    fn occlum_ocall_print_log(level: u32, msg: *const u8);
    fn occlum_ocall_flush_log();
}
