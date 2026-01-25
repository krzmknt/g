use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn init() {
    let mut guard = LOG_FILE.lock().unwrap();
    if guard.is_none() {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("/tmp/g_debug.log")
        {
            *guard = Some(file);
        }
    }
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    // Simple timestamp: seconds.millis
    format!("{}.{:03}", secs % 100000, millis)
}

pub fn log(module: &str, level: &str, message: &str) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        let _ = writeln!(
            file,
            "[{}] [{}] [{}] {}",
            timestamp(),
            level,
            module,
            message
        );
        let _ = file.flush();
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::logger::log(module_path!(), "DEBUG", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::logger::log(module_path!(), "INFO", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::logger::log(module_path!(), "WARN", &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::logger::log(module_path!(), "ERROR", &format!($($arg)*))
    };
}
