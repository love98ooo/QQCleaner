use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
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
        let current_dir_config = env::current_dir().ok().map(|p| p.join("config.toml"));

        let user_config = dirs::config_dir().map(|p| p.join("qqcleaner").join("config.toml"));

        let config_path = [current_dir_config, user_config]
            .into_iter()
            .flatten()
            .find(|p| p.exists());

        if let Some(path) = config_path {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("无法读取配置文件: {:?}", path))?;
            let config: Config = toml::from_str(&content).context("配置文件格式错误")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    fn default() -> Self {
        Config {
            paths: PathsConfig {
                qq_data_base:
                    "Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ"
                        .to_string(),
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
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        Ok(home_dir.join(&self.paths.qq_data_base))
    }

    pub fn get_db_dir(&self) -> PathBuf {
        #[cfg(debug_assertions)]
        {
            // Debug 模式：使用当前工作目录
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&self.database.db_dir)
        }

        #[cfg(not(debug_assertions))]
        {
            // Release 模式：使用平台规范目录
            self.get_platform_data_dir().unwrap_or_else(|_| {
                // 降级到当前目录
                env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(&self.database.db_dir)
            })
        }
    }

    /// 获取符合平台规范的数据目录
    #[cfg(not(debug_assertions))]
    fn get_platform_data_dir(&self) -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .context("无法获取平台数据目录")?
            .join("qqcleaner")
            .join(&self.database.db_dir);

        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)
                .with_context(|| format!("创建数据目录失败: {:?}", data_dir))?;
        }

        Ok(data_dir)
    }

    pub fn get_files_db_path_in(&self, dir: &PathBuf) -> PathBuf {
        dir.join(&self.database.files_db_name)
    }

    pub fn get_group_db_path_in(&self, dir: &PathBuf) -> PathBuf {
        dir.join(&self.database.group_db_name)
    }
}
