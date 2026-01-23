use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use crate::app::{App, AppTab};
use crate::models::format_bytes;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    match app.current_tab {
        AppTab::Analysis => render_analysis(f, app, chunks[1]),
        AppTab::Clean => render_clean(f, app, chunks[1]),
        AppTab::Migrate => render_migrate(f, app, chunks[1]),
    }

    render_status(f, app, chunks[2]);

    if app.show_help {
        render_help_dialog(f);
    }

    if app.show_confirm_dialog {
        render_confirm_dialog(f, app);
    }

    if app.show_filter_dialog {
        render_filter_dialog(f, app);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<&str> = AppTab::titles();
    let current_idx = app.current_tab as usize;

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" QQCleaner "),
        )
        .select(current_idx)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let status_text = if app.progress.is_running {
        format!(
            "进行中: {}/{} | 当前: {} | [q]退出 [?]帮助",
            app.progress.current, app.progress.total, app.progress.current_file
        )
    } else {
        format!(
            "群组: {} | 已选: {} | 总大小: {} | [q]退出 [?]帮助 [Tab]切换",
            app.filtered_stats.len(),
            app.selected_count(),
            format_bytes(app.selected_total_size())
        )
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}

fn render_analysis(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(area);

    render_top_groups(f, app, chunks[0]);
    render_time_distribution(f, app, chunks[1]);
    render_statistics_summary(f, app, chunks[2]);
}

fn render_clean(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_group_list(f, app, chunks[0], "选择要清理的群组");
    render_clean_options(f, app, chunks[1]);
}

fn render_migrate(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_group_list(f, app, chunks[0], "选择要迁移的群组");
    render_migrate_options(f, app, chunks[1]);
}

fn render_group_list(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let visible_height = (area.height as usize).saturating_sub(2);
    let total_items = app.filtered_stats.len();

    let scroll_offset = if total_items == 0 {
        0
    } else if app.selected_index < visible_height / 2 {
        0
    } else if app.selected_index >= total_items.saturating_sub(visible_height / 2) {
        total_items.saturating_sub(visible_height)
    } else {
        app.selected_index.saturating_sub(visible_height / 2)
    };

    let visible_end = (scroll_offset + visible_height).min(total_items);

    let rows: Vec<Row> = app
        .filtered_stats
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(list_idx, &stat_idx)| {
            let stat = &app.stats[stat_idx];
            let is_selected = app.selected_groups.get(stat_idx).copied().unwrap_or(false);
            let is_current = list_idx == app.selected_index;

            let checkbox = if is_selected { "[x]" } else { "[ ]" };
            let group_display = if stat.group_name != format!("群 {}", stat.group_id) {
                format!("{} ({})", stat.group_name, stat.group_id)
            } else {
                stat.group_id.clone()
            };

            let checkbox_style = if is_selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let name_style = if is_current {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let count_style = if is_current {
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let row_style = if is_current {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let size_in_range = app.group_size_in_range(stat);
            let exist_count_in_range = app.group_exist_count_in_range(stat);
            let file_count_in_range = app.group_file_count_in_range(stat);

            Row::new(vec![
                Cell::from(checkbox).style(checkbox_style),
                Cell::from(group_display).style(name_style),
                Cell::from(format_bytes(size_in_range)).style(Style::default().fg(Color::Cyan)),
                Cell::from(format!(
                    "({}/{})",
                    exist_count_in_range, file_count_in_range
                ))
                .style(count_style),
            ])
            .style(row_style)
        })
        .collect();

    let scroll_indicator = if total_items > visible_height {
        format!(" ({}-{}/{}) ", scroll_offset + 1, visible_end, total_items)
    } else {
        String::new()
    };

    let sort_text = match app.sort_by {
        crate::app::SortBy::Size => "大小",
        crate::app::SortBy::FileCount => "文件数",
        crate::app::SortBy::Name => "名称",
    };

    let help_text = format!(
        " {}{} [排序:{}] [?]帮助",
        title, scroll_indicator, sort_text
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),  // checkbox
            Constraint::Min(10),    // 群名（自适应剩余空间）
            Constraint::Length(10), // 大小
            Constraint::Length(12), // 文件数
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(help_text))
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_clean_options(f: &mut Frame, app: &App, area: Rect) {
    let selected_count = app.selected_count();
    let deletable_size = app.selected_deletable_size();
    let total_size = app.selected_total_size();

    let text = vec![
        Line::from(vec![Span::styled(
            "清理选项",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "已选择群组: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(selected_count.to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "文件总大小: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format_bytes(total_size), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("预计释放: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format_bytes(deletable_size),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("时间范围: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                app.time_range.description(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from("─".repeat(35)),
        Line::from(""),
        Line::from(vec![
            Span::styled("[t] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换时间范围"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[d] ", Style::default().fg(Color::Red)),
            Span::raw("开始清理"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" 清理配置 "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_migrate_options(f: &mut Frame, app: &App, area: Rect) {
    let selected_count = app.selected_count();
    let selected_size = app.selected_total_size();
    let deletable_size = app.selected_deletable_size();

    // 显示当前路径索引和总数
    let path_indicator = format!(
        "({}/{})",
        app.migrate_path_index + 1,
        app.migrate_presets.len()
    );

    // 截断过长的路径
    let path_display = app.migrate_target_path.display().to_string();
    let max_path_len = 50;
    let truncated_path = if path_display.len() > max_path_len {
        format!(
            "...{}",
            &path_display[path_display.len() - max_path_len + 3..]
        )
    } else {
        path_display
    };

    let text = vec![
        Line::from(vec![Span::styled(
            "迁移选项",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "已选择群组: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(selected_count.to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "文件总大小: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format_bytes(selected_size),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "范围内大小: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format_bytes(deletable_size),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("时间范围: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                app.time_range.description(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("目标路径: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(path_indicator, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(truncated_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from("─".repeat(35)),
        Line::from(""),
        Line::from(vec![
            Span::styled("[t] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换时间范围"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[←→/p] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换路径"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[m] ", Style::default().fg(Color::Green)),
            Span::raw("开始迁移"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" 迁移配置 "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_top_groups(f: &mut Frame, app: &App, area: Rect) {
    let headers = ["群组名称", "文件数(范围内)", "占用空间(范围内)"];
    let header_cells = headers.iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .stats
        .iter()
        .take(10)
        .map(|stat| {
            let group_display = truncate(&stat.group_name, 30);
            let exist_in_range = app.group_exist_count_in_range(stat);
            let file_count_in_range = app.group_file_count_in_range(stat);
            let size_in_range = app.group_size_in_range(stat);
            Row::new(vec![
                Cell::from(group_display),
                Cell::from(format!("{}/{}", exist_in_range, file_count_in_range)),
                Cell::from(format_bytes(size_in_range)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Top 10 群组 "),
    );

    f.render_widget(table, area);
}

fn render_time_distribution(f: &mut Frame, app: &App, area: Rect) {
    use chrono::{DateTime, Datelike, Utc};
    use std::collections::HashMap;

    let mut month_stats: HashMap<String, u64> = HashMap::new();

    for stat in &app.stats {
        for file in &stat.files {
            if !app.time_range.should_delete(file.msg_time) {
                continue;
            }
            if let Some(datetime) = DateTime::<Utc>::from_timestamp(file.msg_time, 0) {
                let month_key = format!("{}-{:02}", datetime.year(), datetime.month());
                let size = file.actual_size.unwrap_or(0);
                *month_stats.entry(month_key).or_insert(0) += size;
            }
        }
    }

    let mut month_vec: Vec<_> = month_stats.into_iter().collect();
    month_vec.sort_by(|a, b| a.0.cmp(&b.0));

    let bar_width = 8u16;
    let bar_gap = 1u16;
    let available_width = area.width.saturating_sub(2);
    let max_bars = (available_width / (bar_width + bar_gap)).max(1) as usize;

    let skip_count = month_vec.len().saturating_sub(max_bars);
    let month_vec: Vec<_> = month_vec.into_iter().skip(skip_count).collect();

    let data: Vec<(&str, u64)> = month_vec
        .iter()
        .map(|(month, size)| (month.as_str(), *size / 1024 / 1024))
        .collect();

    let title = format!(" 文件大小时间分布 (MB) - {} ", app.time_range.description());

    if !data.is_empty() {
        let chart = BarChart::default()
            .block(Block::default().borders(Borders::ALL).title(title))
            .data(&data)
            .bar_width(bar_width)
            .bar_gap(bar_gap)
            .bar_style(Style::default().fg(Color::Cyan))
            .value_style(Style::default().fg(Color::White));

        f.render_widget(chart, area);
    } else {
        let paragraph = Paragraph::new("暂无数据")
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

fn render_statistics_summary(f: &mut Frame, app: &App, area: Rect) {
    let total_groups = app.stats.len();
    let total_files: usize = app.stats.iter().map(|s| s.file_count).sum();
    let total_size: u64 = app.stats.iter().map(|s| s.total_size).sum();
    let total_exist: usize = app.stats.iter().map(|s| s.exist_count).sum();
    let total_missing: usize = app.stats.iter().map(|s| s.missing_count).sum();

    let range_files: usize = app
        .stats
        .iter()
        .map(|s| app.group_file_count_in_range(s))
        .sum();
    let range_size: u64 = app.stats.iter().map(|s| app.group_size_in_range(s)).sum();
    let range_exist: usize = app
        .stats
        .iter()
        .map(|s| app.group_exist_count_in_range(s))
        .sum();

    let text = vec![
        Line::from(vec![
            Span::styled(
                "统计摘要",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("[t] 时间范围: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.time_range.description(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("总群组数: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(total_groups.to_string(), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("总文件数: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(total_files.to_string(), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("总大小: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format_bytes(total_size), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "范围内文件: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}/{}", range_exist, range_files),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "范围内大小: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format_bytes(range_size),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("存在文件: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(total_exist.to_string(), Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled(
                "缺失(已清理)文件: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(total_missing.to_string(), Style::default().fg(Color::Red)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" 总体统计 "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_help_dialog(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());

    let text = vec![
        Line::from(vec![Span::styled(
            "快捷键说明",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "全局操作:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  [q] ", Style::default().fg(Color::Cyan)),
            Span::raw("退出程序"),
        ]),
        Line::from(vec![
            Span::styled("  [?/h] ", Style::default().fg(Color::Cyan)),
            Span::raw("显示/隐藏帮助"),
        ]),
        Line::from(vec![
            Span::styled("  [Tab] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换标签页"),
        ]),
        Line::from(vec![
            Span::styled("  [1-3] ", Style::default().fg(Color::Cyan)),
            Span::raw("快速跳转到对应标签页"),
        ]),
        Line::from(vec![
            Span::styled("  [t] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换时间范围"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "群组操作:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  [↑↓/jk] ", Style::default().fg(Color::Cyan)),
            Span::raw("上下移动"),
        ]),
        Line::from(vec![
            Span::styled("  [Space] ", Style::default().fg(Color::Cyan)),
            Span::raw("选择/取消选择当前群组"),
        ]),
        Line::from(vec![
            Span::styled("  [a] ", Style::default().fg(Color::Green)),
            Span::raw("全选当前过滤的群组"),
        ]),
        Line::from(vec![
            Span::styled("  [A] ", Style::default().fg(Color::Red)),
            Span::raw("取消所有选择"),
        ]),
        Line::from(vec![
            Span::styled("  [s] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换排序方式"),
        ]),
        Line::from(vec![
            Span::styled("  [f] ", Style::default().fg(Color::Cyan)),
            Span::raw("打开过滤器（隐藏空群组、不活跃群组）"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "清理操作:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  [d] ", Style::default().fg(Color::Red)),
            Span::raw("执行清理操作"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "迁移操作:",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  [←→/p] ", Style::default().fg(Color::Cyan)),
            Span::raw("切换迁移路径"),
        ]),
        Line::from(vec![
            Span::styled("  [m] ", Style::default().fg(Color::Green)),
            Span::raw("执行迁移操作（确认时可选择是否保留原文件）"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 帮助 (按 ? 或 ESC 关闭) ")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_confirm_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 40, f.area());

    let action_name = match app.confirm_action {
        Some(crate::app::ConfirmAction::Clean) => "清理",
        Some(crate::app::ConfirmAction::Migrate) => "迁移",
        None => "操作",
    };

    let selected_count = app.selected_count();
    let selected_size = format_bytes(app.selected_total_size());

    let is_migrate = matches!(app.confirm_action, Some(crate::app::ConfirmAction::Migrate));

    let mut text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("确认{}操作？", action_name),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
    ];

    if is_migrate {
        text.push(Line::from(vec![Span::raw(format!(
            "将迁移 {} 个群组",
            selected_count
        ))]));
        text.push(Line::from(vec![Span::raw(format!(
            "迁移大小: {}",
            selected_size
        ))]));
    } else {
        text.push(Line::from(vec![Span::raw(format!(
            "将影响 {} 个群组",
            selected_count
        ))]));
        text.push(Line::from(vec![Span::raw(format!(
            "总计: {}",
            selected_size
        ))]));
    }

    text.push(Line::from(""));

    if is_migrate {
        // 显示目标路径
        let path_display = app.migrate_target_path.display().to_string();
        let max_path_len = 50;
        let truncated_path = if path_display.len() > max_path_len {
            format!(
                "...{}",
                &path_display[path_display.len() - max_path_len + 3..]
            )
        } else {
            path_display
        };
        let path_indicator = format!(
            "({}/{})",
            app.migrate_path_index + 1,
            app.migrate_presets.len()
        );

        text.push(Line::from(vec![
            Span::styled("目标路径: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(path_indicator, Style::default().fg(Color::DarkGray)),
        ]));
        text.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(truncated_path, Style::default().fg(Color::Cyan)),
        ]));
        text.push(Line::from(vec![Span::styled(
            "  (←→/p 切换)",
            Style::default().fg(Color::DarkGray),
        )]));
        text.push(Line::from(""));

        let checkbox = if app.temp_migrate_keep_original {
            "[x]"
        } else {
            "[ ]"
        };
        let checkbox_style = if app.temp_migrate_keep_original {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        };

        text.push(Line::from(vec![
            Span::styled(checkbox, checkbox_style),
            Span::raw(" "),
            Span::styled("保留原文件", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" (空格切换)", Style::default().fg(Color::DarkGray)),
        ]));
        text.push(Line::from(""));
    } else {
        text.push(Line::from(vec![Span::styled(
            "此操作不可恢复！",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        text.push(Line::from(""));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("[Y] ", Style::default().fg(Color::Green)),
        Span::raw("确认  "),
        Span::styled("[N/ESC] ", Style::default().fg(Color::Red)),
        Span::raw("取消"),
    ]));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 确认操作 ")
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_filter_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 50, f.area());
    let inner_width = area.width.saturating_sub(4) as usize;

    let mut text = vec![
        Line::from(vec![Span::styled(
            "过滤器设置",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("使用 ", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓/jk", Style::default().fg(Color::Cyan)),
            Span::styled(" 导航, ", Style::default().fg(Color::DarkGray)),
            Span::styled("空格/回车", Style::default().fg(Color::Cyan)),
            Span::styled(" 切换", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from("─".repeat(inner_width)),
        Line::from(""),
    ];

    let cursor_0 = if app.filter_cursor == 0 { "► " } else { "  " };
    let checkbox_0 = if app.temp_filter.hide_empty {
        "[x]"
    } else {
        "[ ]"
    };
    text.push(Line::from(vec![
        Span::styled(cursor_0, Style::default().fg(Color::Yellow)),
        Span::styled(
            checkbox_0,
            if app.temp_filter.hide_empty {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::raw(" "),
        Span::styled(
            "隐藏无图片群组 (exist_count = 0)",
            if app.filter_cursor == 0 {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
    ]));
    text.push(Line::from(""));

    let cursor_1 = if app.filter_cursor == 1 { "► " } else { "  " };
    let (activity_checkbox, activity_text) = match app.temp_filter.activity {
        crate::app::ActivityFilter::All => ("[ ]", "活跃度过滤: 关闭"),
        crate::app::ActivityFilter::Active(days) => (
            "[x]",
            match days {
                7 => "活跃度过滤: 活跃(7天内)",
                30 => "活跃度过滤: 活跃(30天内)",
                90 => "活跃度过滤: 活跃(90天内)",
                _ => "活跃度过滤: 活跃",
            },
        ),
        crate::app::ActivityFilter::Inactive(days) => (
            "[x]",
            match days {
                7 => "活跃度过滤: 不活跃(7天前)",
                30 => "活跃度过滤: 不活跃(30天前)",
                90 => "活跃度过滤: 不活跃(90天前)",
                _ => "活跃度过滤: 不活跃",
            },
        ),
    };
    text.push(Line::from(vec![
        Span::styled(cursor_1, Style::default().fg(Color::Yellow)),
        Span::styled(
            activity_checkbox,
            if !matches!(app.temp_filter.activity, crate::app::ActivityFilter::All) {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::raw(" "),
        Span::styled(
            activity_text,
            if app.filter_cursor == 1 {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        ),
    ]));
    text.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(
            "(关闭 → 活跃7/30/90天 → 不活跃7/30/90天 → 关闭)",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    text.push(Line::from(""));

    text.push(Line::from("─".repeat(inner_width)));
    text.push(Line::from(""));

    let now = chrono::Utc::now().timestamp();
    let would_filter = app
        .stats
        .iter()
        .filter(|stat| {
            if app.temp_filter.hide_empty && stat.exist_count == 0 {
                return false;
            }
            match app.temp_filter.activity {
                crate::app::ActivityFilter::All => {}
                crate::app::ActivityFilter::Active(days) => {
                    let cutoff = now - (days * 86400);
                    let latest_time = stat.files.iter().map(|f| f.msg_time).max().unwrap_or(0);
                    if latest_time < cutoff {
                        return false;
                    }
                }
                crate::app::ActivityFilter::Inactive(days) => {
                    let cutoff = now - (days * 86400);
                    let latest_time = stat.files.iter().map(|f| f.msg_time).max().unwrap_or(0);
                    if latest_time >= cutoff {
                        return false;
                    }
                }
            }
            true
        })
        .count();

    text.push(Line::from(vec![
        Span::styled("预览: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("将显示 {} / {} 个群组", would_filter, app.stats.len()),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    text.push(Line::from(""));
    text.push(Line::from(""));

    text.push(Line::from(vec![
        Span::styled(
            "[a] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("应用 (Apply)   "),
        Span::styled("[c/ESC] ", Style::default().fg(Color::Red)),
        Span::raw("取消 (Cancel)"),
    ]));

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 过滤器设置 ")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max_len - 3).collect();
        result.push_str("...");
        result
    }
}
