use crate::models::GroupStats;
use crate::time_range::TimeRange;
use crate::logger::Logger;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTab {
    Home,
    Analysis,
    Clean,
    Migrate,
    Logs,
}

impl AppTab {
    pub fn titles() -> Vec<&'static str> {
        vec!["首页", "分析", "清理", "迁移", "日志"]
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => AppTab::Home,
            1 => AppTab::Analysis,
            2 => AppTab::Clean,
            3 => AppTab::Migrate,
            4 => AppTab::Logs,
            _ => AppTab::Home,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortBy {
    Size,
    FileCount,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivityFilter {
    All,
    Active(i64),
    Inactive(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupFilter {
    pub min_size: u64,
    pub min_file_count: usize,
    pub hide_empty: bool,
    pub activity: ActivityFilter,
}

impl Default for GroupFilter {
    fn default() -> Self {
        Self {
            min_size: 0,
            min_file_count: 0,
            hide_empty: true,
            activity: ActivityFilter::All,
        }
    }
}


#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct OperationProgress {
    pub total: usize,
    pub current: usize,
    pub current_file: String,
    pub is_running: bool,
}

impl Default for OperationProgress {
    fn default() -> Self {
        Self {
            total: 0,
            current: 0,
            current_file: String::new(),
            is_running: false,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub current_tab: AppTab,
    pub stats: Vec<GroupStats>,
    pub filtered_stats: Vec<usize>,
    pub selected_index: usize,
    pub selected_groups: Vec<bool>,
    pub sort_by: SortBy,
    pub filter: GroupFilter,
    pub time_range: TimeRange,
    pub logs: Vec<LogEntry>,
    pub progress: OperationProgress,
    pub migrate_target_path: PathBuf,
    pub migrate_presets: Vec<PathBuf>,
    pub migrate_path_index: usize,
    pub show_help: bool,
    pub show_filter_dialog: bool,
    pub show_confirm_dialog: bool,
    pub confirm_action: Option<ConfirmAction>,
    pub temp_filter: GroupFilter,
    pub filter_cursor: usize,
    pub logger: Arc<Logger>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmAction {
    Clean,
    Migrate,
}

impl App {
    pub fn new(stats: Vec<GroupStats>, logger: Arc<Logger>) -> Self {
        let len = stats.len();
        let filtered_stats: Vec<usize> = (0..len).collect();
        let selected_groups = vec![false; len];

        let migrate_presets = if cfg!(debug_assertions) {
            vec![
                PathBuf::from("./migration"),
                dirs::document_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join("qqnt_migration"),
            ]
        } else {
            vec![
                dirs::document_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join("QQCleaner"),
                dirs::desktop_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join("QQCleaner"),
            ]
        };

        let mut app = Self {
            should_quit: false,
            current_tab: AppTab::Home,
            stats,
            filtered_stats,
            selected_index: 0,
            selected_groups,
            sort_by: SortBy::Size,
            filter: GroupFilter::default(),
            time_range: TimeRange::All,
            logs: Vec::new(),
            progress: OperationProgress::default(),
            migrate_target_path: migrate_presets[0].clone(),
            migrate_presets,
            migrate_path_index: 0,
            show_help: false,
            show_filter_dialog: false,
            show_confirm_dialog: false,
            confirm_action: None,
            temp_filter: GroupFilter::default(),
            filter_cursor: 0,
            logger,
        };

        app.apply_filter();
        app.add_log(LogLevel::Info, "应用启动成功");
        app
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn next_tab(&mut self) {
        let current_idx = self.current_tab as usize;
        let next_idx = (current_idx + 1) % AppTab::titles().len();
        self.current_tab = AppTab::from_index(next_idx);
    }

    pub fn prev_tab(&mut self) {
        let current_idx = self.current_tab as usize;
        let prev_idx = if current_idx == 0 {
            AppTab::titles().len() - 1
        } else {
            current_idx - 1
        };
        self.current_tab = AppTab::from_index(prev_idx);
    }

    pub fn next_item(&mut self) {
        if !self.filtered_stats.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_stats.len();
        }
    }

    pub fn prev_item(&mut self) {
        if !self.filtered_stats.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_stats.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn toggle_selected_group(&mut self) {
        if let Some(&actual_idx) = self.filtered_stats.get(self.selected_index) {
            if actual_idx < self.selected_groups.len() {
                self.selected_groups[actual_idx] = !self.selected_groups[actual_idx];
            }
        }
    }

    pub fn select_all_filtered(&mut self) {
        for &idx in &self.filtered_stats {
            if idx < self.selected_groups.len() {
                self.selected_groups[idx] = true;
            }
        }
        self.add_log(LogLevel::Info, &format!("已选择 {} 个群组", self.filtered_stats.len()));
    }

    pub fn deselect_all(&mut self) {
        self.selected_groups.fill(false);
        self.add_log(LogLevel::Info, "已取消所有选择");
    }

    pub fn apply_sort(&mut self) {
        match self.sort_by {
            SortBy::Size => {
                self.stats.sort_by(|a, b| b.total_size.cmp(&a.total_size));
            }
            SortBy::FileCount => {
                // 使用存在的文件数量排序，而非总文件数
                self.stats.sort_by(|a, b| b.exist_count.cmp(&a.exist_count));
            }
            SortBy::Name => {
                self.stats.sort_by(|a, b| a.group_name.cmp(&b.group_name));
            }
        }
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        let now = chrono::Utc::now().timestamp();

        self.filtered_stats = self.stats
            .iter()
            .enumerate()
            .filter(|(_, stat)| {
                if self.filter.hide_empty && stat.exist_count == 0 {
                    return false;
                }

                if stat.total_size < self.filter.min_size {
                    return false;
                }

                if stat.file_count < self.filter.min_file_count {
                    return false;
                }

                match self.filter.activity {
                    ActivityFilter::All => {}
                    ActivityFilter::Active(days) => {
                        let cutoff = now - (days * 86400);
                        let latest_time = stat.files.iter()
                            .map(|f| f.msg_time)
                            .max()
                            .unwrap_or(0);

                        if latest_time < cutoff {
                            return false;
                        }
                    }
                    ActivityFilter::Inactive(days) => {
                        let cutoff = now - (days * 86400);
                        let latest_time = stat.files.iter()
                            .map(|f| f.msg_time)
                            .max()
                            .unwrap_or(0);

                        if latest_time >= cutoff {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.selected_index >= self.filtered_stats.len() {
            self.selected_index = self.filtered_stats.len().saturating_sub(1);
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogEntry {
            timestamp: timestamp.clone(),
            level,
            message: message.to_string(),
        });

        let level_str = match level {
            LogLevel::Info => "INFO",
            LogLevel::Success => "OK",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERR",
        };
        let _ = self.logger.log(level_str, message);
    }

    pub fn start_operation(&mut self, total: usize) {
        self.progress = OperationProgress {
            total,
            current: 0,
            current_file: String::new(),
            is_running: true,
        };
    }

    pub fn update_progress(&mut self, current: usize, file: &str) {
        self.progress.current = current;
        self.progress.current_file = file.to_string();
    }

    pub fn finish_operation(&mut self) {
        self.progress.is_running = false;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn show_confirm(&mut self, action: ConfirmAction) {
        self.confirm_action = Some(action);
        self.show_confirm_dialog = true;
    }

    pub fn hide_confirm(&mut self) {
        self.show_confirm_dialog = false;
        self.confirm_action = None;
    }

    pub fn selected_count(&self) -> usize {
        self.selected_groups.iter().filter(|&&x| x).count()
    }

    pub fn selected_total_size(&self) -> u64 {
        self.selected_groups
            .iter()
            .enumerate()
            .filter_map(|(idx, &selected)| {
                if selected {
                    self.stats.get(idx).map(|s| s.total_size)
                } else {
                    None
                }
            })
            .sum()
    }

    /// 根据时间范围计算选中群组可删除的文件总大小
    pub fn selected_deletable_size(&self) -> u64 {
        self.selected_groups
            .iter()
            .enumerate()
            .filter_map(|(idx, &selected)| {
                if selected {
                    self.stats.get(idx)
                } else {
                    None
                }
            })
            .flat_map(|stat| &stat.files)
            .filter(|file| self.time_range.should_delete(file.msg_time))
            .filter_map(|file| file.actual_size)
            .sum()
    }

    pub fn next_migrate_path(&mut self) {
        self.migrate_path_index = (self.migrate_path_index + 1) % self.migrate_presets.len();
        self.migrate_target_path = self.migrate_presets[self.migrate_path_index].clone();
    }

    pub fn prev_migrate_path(&mut self) {
        if self.migrate_path_index == 0 {
            self.migrate_path_index = self.migrate_presets.len() - 1;
        } else {
            self.migrate_path_index -= 1;
        }
        self.migrate_target_path = self.migrate_presets[self.migrate_path_index].clone();
    }

    pub fn open_filter_dialog(&mut self) {
        self.temp_filter = self.filter.clone();
        self.filter_cursor = 0;
        self.show_filter_dialog = true;
    }

    pub fn apply_filter_dialog(&mut self) {
        self.filter = self.temp_filter.clone();
        self.apply_filter();
        self.show_filter_dialog = false;
        self.add_log(LogLevel::Info, "过滤器已应用");
    }

    pub fn cancel_filter_dialog(&mut self) {
        self.show_filter_dialog = false;
    }

    pub fn filter_next_item(&mut self) {
        self.filter_cursor = (self.filter_cursor + 1) % 4;
    }

    pub fn filter_prev_item(&mut self) {
        if self.filter_cursor == 0 {
            self.filter_cursor = 3;
        } else {
            self.filter_cursor -= 1;
        }
    }

    pub fn toggle_filter_option(&mut self) {
        match self.filter_cursor {
            0 => {
                self.temp_filter.hide_empty = !self.temp_filter.hide_empty;
            }
            1 => {
                self.temp_filter.activity = match self.temp_filter.activity {
                    ActivityFilter::All => ActivityFilter::Active(7),
                    ActivityFilter::Active(7) => ActivityFilter::Active(30),
                    ActivityFilter::Active(30) => ActivityFilter::Active(90),
                    ActivityFilter::Active(_) => ActivityFilter::Inactive(7),
                    ActivityFilter::Inactive(7) => ActivityFilter::Inactive(30),
                    ActivityFilter::Inactive(30) => ActivityFilter::Inactive(90),
                    ActivityFilter::Inactive(_) => ActivityFilter::All,
                };
            }
            _ => {}
        }
    }
}

