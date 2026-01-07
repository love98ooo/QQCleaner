mod models;
mod database;
mod file_checker;
mod cli;
mod config;

use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;

use config::Config;
use database::Database;
use file_checker::FileChecker;
use cli::CliInterface;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "QQ 群组文件清理工具".to_string());
    println!("{}\n", "=".repeat(40));

    CliInterface::display_progress("正在加载配置...");
    let config = Config::load()?;
    CliInterface::display_success("配置加载成功");

    CliInterface::display_progress("正在查找 QQ 数据目录...");
    let qq_base_dir = config.get_qq_base_dir()?;

    if !qq_base_dir.exists() {
        CliInterface::display_error("未找到 QQ 数据目录");
        return Ok(());
    }

    let mut nt_qq_dir: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&qq_base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with(&config.paths.nt_qq_prefix) {
                        nt_qq_dir = Some(path);
                        break;
                    }
                }
            }
        }
    }

    let nt_qq_dir = nt_qq_dir.context("未找到 nt_qq_* 目录")?;
    let nt_data_dir = nt_qq_dir.join(&config.paths.nt_data_subpath);

    if !nt_data_dir.exists() {
        CliInterface::display_error(&format!("未找到 nt_data 目录: {:?}", nt_data_dir));
        return Ok(());
    }

    CliInterface::display_success(&format!("找到数据目录: {:?}", nt_data_dir));

    CliInterface::display_progress("正在打开数据库...");
    let files_db = config.get_files_db_path();
    let group_db = config.get_group_db_path();

    if !files_db.exists() {
        CliInterface::display_error(&format!("未找到文件数据库: {:?}", files_db));
        return Ok(());
    }

    if !group_db.exists() {
        CliInterface::display_error(&format!("未找到群组数据库: {:?}", group_db));
        return Ok(());
    }

    let db = Database::new(&files_db, &group_db)
        .context("打开数据库失败")?;

    CliInterface::display_progress("正在读取文件和群组信息...");
    let group_files = db.group_files_by_peer()
        .context("读取文件信息失败")?;
    let groups = db.get_all_groups()
        .context("读取群组信息失败")?;

    CliInterface::display_success(&format!("找到 {} 个群组，共 {} 个文件",
        group_files.len(),
        group_files.values().map(|v| v.len()).sum::<usize>()
    ));

    CliInterface::display_progress("正在检查文件并生成统计信息（这可能需要一些时间）...");
    let checker = FileChecker::new(nt_data_dir.clone());
    let group_files_vec: Vec<_> = group_files.into_iter().collect();
    let stats = checker.generate_group_stats(group_files_vec, &groups).await?;

    let selections = CliInterface::select_groups_to_delete(&stats)?;

    if selections.is_empty() {
        println!("\n未选择任何群组，退出。");
        return Ok(());
    }

    let selected_groups: Vec<_> = selections.iter()
        .map(|&idx| &stats[idx])
        .collect();

    let time_range = CliInterface::select_time_range(&stats, &selections)?;
    println!("\n{} {}", "已选择时间范围:".cyan(), time_range.description().yellow().bold());

    if !CliInterface::confirm_deletion(&selected_groups)? {
        println!("\n取消删除操作。");
        return Ok(());
    }

    println!("\n{}", "正在删除文件...".to_string());
    for &idx in &selections {
        let stat = &stats[idx];
        let (deleted, failed) = checker.delete_group_files(stat, Some(&time_range)).await?;
        CliInterface::display_deletion_result(&stat.group_name, deleted, failed);
    }

    println!();
    CliInterface::display_success("删除操作完成！");

    Ok(())
}
