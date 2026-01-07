use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Utc};
use std::path::PathBuf;
use tokio::fs;

use crate::models::{FileInfo, GroupStats};

pub struct Migrator {
    qq_data_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MigrateOptions {
    pub target_dir: PathBuf,
    pub keep_structure: bool,  // 保留原始目录结构
    pub delete_after_migrate: bool,  // 迁移后删除原文件
}

impl Default for MigrateOptions {
    fn default() -> Self {
        Self {
            target_dir: PathBuf::from("./backup"),
            keep_structure: true,
            delete_after_migrate: false,
        }
    }
}

#[derive(Debug)]
pub struct MigrateResult {
    pub migrated_files: usize,
    pub failed_files: usize,
    pub total_size: u64,
}

impl Migrator {
    pub fn new(qq_data_dir: PathBuf) -> Self {
        Self { qq_data_dir }
    }

    fn get_thumb_filenames(filename: &str) -> Vec<String> {
        if let Some(dot_pos) = filename.rfind('.') {
            let name_without_ext = &filename[..dot_pos];
            let ext = &filename[dot_pos..];
            vec![
                format!("{}_0{}", name_without_ext, ext),
                format!("{}_720{}", name_without_ext, ext),
            ]
        } else {
            vec![
                format!("{}_0", filename),
                format!("{}_720", filename),
            ]
        }
    }

    async fn get_file_paths(&self, file: &FileInfo) -> Vec<(PathBuf, PathBuf)> {
        let mut paths = Vec::new();
        
        if file.file_name.is_empty() {
            return paths;
        }

        let datetime = DateTime::<Utc>::from_timestamp(file.msg_time, 0)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        
        let time_dir = format!("{}-{:02}", datetime.year(), datetime.month());
        let base_dir = self.qq_data_dir.join(&time_dir);

        // Original file
        let ori_path = base_dir.join("Ori").join(&file.file_name);
        if ori_path.exists() {
            paths.push((ori_path, PathBuf::from("Ori").join(&file.file_name)));
        }

        // Thumbnail files
        let thumb_filenames = Self::get_thumb_filenames(&file.file_name);
        for thumb_name in thumb_filenames {
            let thumb_path = base_dir.join("Thumb").join(&thumb_name);
            if thumb_path.exists() {
                paths.push((thumb_path, PathBuf::from("Thumb").join(&thumb_name)));
            }
        }

        paths
    }

    pub async fn migrate_group_files(
        &self,
        stats: &GroupStats,
        options: &MigrateOptions,
        progress_callback: Option<Box<dyn Fn(usize, &str) + Send>>,
    ) -> Result<MigrateResult> {
        let mut result = MigrateResult {
            migrated_files: 0,
            failed_files: 0,
            total_size: 0,
        };

        // 创建群组目标目录
        let group_dir = if options.keep_structure {
            options.target_dir.join(format!("{}_{}", stats.group_name, stats.group_id))
        } else {
            options.target_dir.clone()
        };

        fs::create_dir_all(&group_dir).await
            .context("创建目标目录失败")?;

        for (idx, file) in stats.files.iter().enumerate() {
            if let Some(ref callback) = progress_callback {
                callback(idx + 1, &file.file_name);
            }

            if file.actual_size.is_none() {
                continue;
            }

            let file_paths = self.get_file_paths(file).await;
            
            for (src_path, rel_path) in file_paths {
                let dst_path = if options.keep_structure {
                    // 保留时间和 Ori/Thumb 结构
                    let datetime = DateTime::<Utc>::from_timestamp(file.msg_time, 0)
                        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
                    let time_dir = format!("{}-{:02}", datetime.year(), datetime.month());
                    group_dir.join(time_dir).join(rel_path)
                } else {
                    // 扁平化存储
                    group_dir.join(src_path.file_name().unwrap())
                };

                // 创建父目录
                if let Some(parent) = dst_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent).await {
                        eprintln!("创建目录失败: {:?}, 错误: {}", parent, e);
                        result.failed_files += 1;
                        continue;
                    }
                }

                // 复制文件
                match fs::copy(&src_path, &dst_path).await {
                    Ok(size) => {
                        result.total_size += size;
                        result.migrated_files += 1;

                        // 如果设置了删除原文件
                        if options.delete_after_migrate {
                            let _ = fs::remove_file(&src_path).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("复制文件失败: {:?} -> {:?}, 错误: {}", src_path, dst_path, e);
                        result.failed_files += 1;
                    }
                }
            }
        }

        Ok(result)
    }
}

