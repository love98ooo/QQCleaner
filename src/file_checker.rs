use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use std::path::PathBuf;
use tokio::fs;
use tokio::task::JoinSet;

use crate::models::{FileInfo, GroupInfo, GroupStats};

pub struct FileChecker {
    qq_data_dir: PathBuf,
}

impl FileChecker {
    pub fn new(qq_data_dir: PathBuf) -> Self {
        FileChecker { qq_data_dir }
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

    pub async fn check_files_exist_with_size(&self, files: &[FileInfo]) -> Result<Vec<FileInfo>> {
        let mut join_set = JoinSet::new();

        for file in files {
            let file_clone = file.clone();
            let filename = file.file_name.clone();
            let qq_data_dir = self.qq_data_dir.clone();
            let msg_time = file.msg_time;

            join_set.spawn(async move {
                let mut file_info = file_clone;

                if filename.is_empty() {
                    file_info.actual_size = None;
                    return file_info;
                }

                let datetime = DateTime::<Utc>::from_timestamp(msg_time, 0)
                    .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

                let time_dir = format!("{}-{:02}", datetime.year(), datetime.month());
                let base_dir = qq_data_dir.join(&time_dir);

                let mut total_size = 0u64;

                let ori_path = base_dir.join("Ori").join(&filename);
                if let Ok(metadata) = fs::metadata(&ori_path).await {
                    total_size += metadata.len();
                }

                let thumb_filenames = Self::get_thumb_filenames(&filename);
                for thumb_name in thumb_filenames {
                    let thumb_path = base_dir.join("Thumb").join(&thumb_name);
                    if let Ok(metadata) = fs::metadata(&thumb_path).await {
                        total_size += metadata.len();
                    }
                }

                file_info.actual_size = if total_size > 0 {
                    Some(total_size)
                } else {
                    None
                };

                file_info
            });
        }

        let mut updated_files = Vec::new();
        while let Some(result) = join_set.join_next().await {
            if let Ok(file_info) = result {
                updated_files.push(file_info);
            }
        }

        Ok(updated_files)
    }

    pub async fn generate_group_stats(
        &self,
        group_files: Vec<(String, Vec<FileInfo>)>,
        groups: &std::collections::HashMap<String, GroupInfo>,
    ) -> Result<Vec<GroupStats>> {
        let mut stats_list = Vec::new();

        for (group_id, files) in group_files {
            let updated_files = self.check_files_exist_with_size(&files).await?;

            let exist_count = updated_files.iter().filter(|f| f.actual_size.is_some()).count();
            let missing_count = updated_files.len() - exist_count;
            let total_size: u64 = updated_files.iter()
                .filter_map(|f| f.actual_size)
                .sum();

            let group_name = groups.get(&group_id)
                .map(|g| g.group_name.clone())
                .unwrap_or_else(|| format!("群 {}", group_id));

            stats_list.push(GroupStats {
                group_id,
                group_name,
                total_size,
                file_count: updated_files.len(),
                exist_count,
                missing_count,
                files: updated_files,
            });
        }

        stats_list.sort_by(|a, b| b.total_size.cmp(&a.total_size));

        Ok(stats_list)
    }

    pub async fn delete_group_files(
        &self,
        stats: &GroupStats,
        time_range: Option<&crate::time_range::TimeRange>,
    ) -> Result<(usize, usize)> {
        let mut join_set = JoinSet::new();

        for file in &stats.files {
            let filename = file.file_name.clone();
            let qq_data_dir = self.qq_data_dir.clone();
            let msg_time = file.msg_time;
            let time_range = time_range.cloned();

            join_set.spawn(async move {
                let mut deleted = 0;
                let failed = 0;

                if filename.is_empty() {
                    return (deleted, failed);
                }

                if let Some(ref range) = time_range {
                    if !range.should_delete(msg_time) {
                        return (deleted, failed);
                    }
                }

                let datetime = DateTime::<Utc>::from_timestamp(msg_time, 0)
                    .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

                let time_dir = format!("{}-{:02}", datetime.year(), datetime.month());
                let base_dir = qq_data_dir.join(&time_dir);

                let ori_path = base_dir.join("Ori").join(&filename);
                match fs::remove_file(&ori_path).await {
                    Ok(_) => deleted += 1,
                    Err(_) => {}
                }

                let thumb_filenames = Self::get_thumb_filenames(&filename);
                for thumb_name in thumb_filenames {
                    let thumb_path = base_dir.join("Thumb").join(&thumb_name);
                    match fs::remove_file(&thumb_path).await {
                        Ok(_) => deleted += 1,
                        Err(_) => {}
                    }
                }

                (deleted, failed)
            });
        }

        let mut total_deleted = 0;
        let mut total_failed = 0;

        while let Some(result) = join_set.join_next().await {
            if let Ok((deleted, failed)) = result {
                total_deleted += deleted;
                total_failed += failed;
            }
        }

        Ok((total_deleted, total_failed))
    }
}
