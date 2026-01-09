use anyhow::{Result, Context};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;

pub struct Logger {
    log_file: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let log_dir = Self::get_log_directory()?;
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("创建日志目录失败: {:?}", log_dir))?;

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

    fn get_log_directory() -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let log_dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("logs");
            return Ok(log_dir);
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir()
                .context("无法获取用户主目录")?;
            Ok(home.join("Library").join("Logs").join("qqcleaner"))
        }

        #[cfg(not(target_os = "macos"))]
        {
            let cache_dir = dirs::cache_dir()
                .context("无法获取缓存目录")?;
            Ok(cache_dir.join("qqcleaner").join("logs"))
        }
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

