//! The app's log: kept in `calliope.log` in the app's data directory (the
//! main way to look into problems on a device), and echoed to stderr.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

/// A log over this size is started afresh when the app starts.
const MAX_BYTES: u64 = 1024 * 1024;

struct FileLogger {
    file: Mutex<Option<File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let line = format!(
            "{secs} {} {}: {}\n",
            record.level(),
            record.target(),
            record.args()
        );
        eprint!("{line}");
        if let Some(file) = self.file.lock().unwrap().as_mut() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Some(file) = self.file.lock().unwrap().as_mut() {
            let _ = file.flush();
        }
    }
}

pub(crate) fn path() -> Option<PathBuf> {
    crate::paths::data_dir().map(|dir| dir.join("calliope.log"))
}

/// Logs to the file from now on; does nothing when the process already has
/// a logger (a platform's own, or a second call).
pub(crate) fn init() {
    let file = path().and_then(|path| {
        fs::create_dir_all(path.parent()?).ok()?;
        let too_big = fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES);
        OpenOptions::new()
            .create(true)
            .append(!too_big)
            .write(true)
            .truncate(too_big)
            .open(path)
            .ok()
    });
    let logger = FileLogger {
        file: Mutex::new(file),
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}
