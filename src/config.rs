use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct PathsConfig {
    pub qq_data_base: String,
    pub nt_qq_prefix: String,
    pub nt_data_subpath: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub db_dir: String,
    pub files_db_name: String,
    pub group_db_name: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("无法读取配置文件")?;
            let config: Config = toml::from_str(&content)
                .context("配置文件格式错误")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    fn default() -> Self {
        Config {
            paths: PathsConfig {
                qq_data_base: "Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ".to_string(),
                nt_qq_prefix: "nt_qq_".to_string(),
                nt_data_subpath: "nt_data/Pic".to_string(),
            },
            database: DatabaseConfig {
                db_dir: "nt_db".to_string(),
                files_db_name: "files_in_chat.clean.db".to_string(),
                group_db_name: "group_info.clean.db".to_string(),
            },
        }
    }

    pub fn get_qq_base_dir(&self) -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .context("无法获取用户主目录")?;
        Ok(home_dir.join(&self.paths.qq_data_base))
    }

    pub fn get_db_dir(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&self.database.db_dir)
    }

    pub fn get_files_db_path(&self) -> PathBuf {
        self.get_db_dir().join(&self.database.files_db_name)
    }

    pub fn get_group_db_path(&self) -> PathBuf {
        self.get_db_dir().join(&self.database.group_db_name)
    }
}
