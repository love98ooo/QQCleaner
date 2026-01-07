use anyhow::Result;
use chrono::Utc;
use colored::*;
use dialoguer::theme::ColorfulTheme;
use inquire::formatter::MultiOptionFormatter;
use inquire::{MultiSelect as InquireMultiSelect, Select};

use crate::models::{format_bytes, GroupStats};

#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
    All,
    DaysAgo(i64),
}

impl TimeRange {
    pub fn should_delete(&self, timestamp: i64) -> bool {
        match self {
            TimeRange::All => true,
            TimeRange::DaysAgo(days) => {
                let now = Utc::now().timestamp();
                let cutoff = now - (days * 86400);
                timestamp < cutoff
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            TimeRange::All => "全部时间".to_string(),
            TimeRange::DaysAgo(days) => format!("{} 天前", days),
        }
    }

    pub fn display_text(&self) -> String {
        match self {
            TimeRange::All => "全部时间（删除所有文件）".to_string(),
            TimeRange::DaysAgo(days) => format!("{} 天前（保留最近 {} 天）", days, days),
        }
    }
}

pub struct CliInterface;

impl CliInterface {
    pub fn select_groups_to_delete(stats: &[GroupStats]) -> Result<Vec<usize>> {
        if stats.is_empty() {
            println!("{}", "没有找到任何群组文件。".yellow());
            return Ok(vec![]);
        }

        let options: Vec<(String, u64)> = stats
            .iter()
            .map(|stat| {
                let group_display = if stat.group_name != format!("群 {}", stat.group_id) {
                    format!("{} ({})", stat.group_name, stat.group_id)
                } else {
                    stat.group_id.clone()
                };
                let display = format!(
                    "{} - {} ({} 个文件, {} 存在)",
                    group_display,
                    stat.format_size(),
                    stat.file_count,
                    stat.exist_count
                );
                (display, stat.total_size)
            })
            .collect();

        let items: Vec<&str> = options.iter().map(|(s, _)| s.as_str()).collect();

        let formatter: MultiOptionFormatter<'_, &str> = &|selected| {
            let total_size: u64 = selected
                .iter()
                .map(|opt| options[opt.index].1)
                .sum();
            format!(
                "已选择 {} 个群组，总计: {}",
                selected.len(),
                format_bytes(total_size)
            )
        };

        println!();
        let selections = InquireMultiSelect::new(
            "请选择要删除文件的群组（使用空格选择，回车确认）:",
            items,
        )
        .with_formatter(formatter)
        .with_page_size(25)
        .prompt()?;

        // 获取选中项的索引
        let indices: Vec<usize> = selections
            .iter()
            .filter_map(|&selected| {
                options.iter().position(|(s, _)| s.as_str() == selected)
            })
            .collect();

        Ok(indices)
    }

    /// 确认删除操作
    pub fn confirm_deletion(groups: &[&GroupStats]) -> Result<bool> {
        if groups.is_empty() {
            return Ok(false);
        }

        println!("\n{}", "即将删除以下群组的文件:".bold().red());
        let mut total_size = 0u64;
        let mut total_files = 0;

        for stat in groups {
            println!(
                "  - {} ({}, {} 个文件)",
                stat.group_name.yellow(),
                stat.format_size().red(),
                stat.file_count
            );
            total_size += stat.total_size;
            total_files += stat.file_count;
        }

        println!();
        println!(
            "{} {} ({} 个文件)",
            "总计:".bold(),
            crate::models::format_bytes(total_size).red().bold(),
            total_files.to_string().yellow()
        );

        let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("确认删除？此操作不可恢复")
            .default(false)
            .interact()?;

        Ok(confirm)
    }

    /// 显示删除结果
    pub fn display_deletion_result(
        group_name: &str,
        deleted: usize,
        failed: usize,
    ) {
        if failed > 0 {
            println!(
                "  {} - 成功删除 {} 个文件，失败 {} 个",
                group_name.yellow(),
                deleted.to_string().green(),
                failed.to_string().red()
            );
        } else {
            println!(
                "  {} - 成功删除 {} 个文件",
                group_name.yellow(),
                deleted.to_string().green()
            );
        }
    }

    /// 显示错误信息
    pub fn display_error(message: &str) {
        println!("{} {}", "错误:".red().bold(), message);
    }

    /// 显示成功信息
    pub fn display_success(message: &str) {
        println!("{} {}", "✓".green().bold(), message);
    }

    /// 显示进度信息
    pub fn display_progress(message: &str) {
        println!("{} {}", "⟳".cyan(), message);
    }

    /// 选择时间范围，显示每个选项可释放的空间
    pub fn select_time_range(stats: &[GroupStats], selected_indices: &[usize]) -> Result<TimeRange> {
        // 定义时间范围选项
        let time_ranges = vec![
            TimeRange::All,
            TimeRange::DaysAgo(3),
            TimeRange::DaysAgo(7),
            TimeRange::DaysAgo(14),
            TimeRange::DaysAgo(30),
            TimeRange::DaysAgo(90),
            TimeRange::DaysAgo(180),
        ];

        // 计算每个时间范围的可释放空间
        let options_with_size: Vec<String> = time_ranges
            .iter()
            .map(|range| {
                let total_size = Self::calculate_deletable_size(stats, selected_indices, range);
                format!(
                    "{} - 可释放: {}",
                    range.display_text(),
                    format_bytes(total_size)
                )
            })
            .collect();

        println!();
        let selection = Select::new(
            "请选择要删除的文件时间范围:",
            options_with_size.iter().map(|s| s.as_str()).collect(),
        )
        .with_page_size(10)
        .prompt()?;

        // 根据选择返回对应的时间范围
        let index = options_with_size
            .iter()
            .position(|s| s == selection)
            .unwrap_or(0);

        Ok(time_ranges[index])
    }

    /// 计算指定时间范围内可删除的文件总大小
    fn calculate_deletable_size(
        stats: &[GroupStats],
        selected_indices: &[usize],
        time_range: &TimeRange,
    ) -> u64 {
        selected_indices
            .iter()
            .filter_map(|&idx| stats.get(idx))
            .flat_map(|stat| &stat.files)
            .filter(|file| time_range.should_delete(file.msg_time))
            .filter_map(|file| file.actual_size)  // 使用实际大小，忽略不存在的文件
            .sum()
    }
}
