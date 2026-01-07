use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

pub struct Logger {
    log_file: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let log_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs");
        fs::create_dir_all(&log_dir)?;

        let log_filename = format!("qqcleaner_{}.log", Local::now().format("%Y%m%d_%H%M%S"));
        let log_file = log_dir.join(log_filename);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_file)?;

        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file, "QQCleaner - 日志记录")?;
        writeln!(file, "启动时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "{}", "=".repeat(80))?;
        writeln!(file)?;

        Ok(Self { log_file })
    }

    pub fn log(&self, level: &str, message: &str) -> Result<()> {
        let timestamp = Local::now().format("%H:%M:%S");
        let log_line = format!("[{}] {:5} {}", timestamp, level, message);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.log_file)?;

        writeln!(file, "{}", log_line)?;
        file.flush()?;

        Ok(())
    }

    pub fn get_log_path(&self) -> &PathBuf {
        &self.log_file
    }
}

