mod models;
mod database;
mod file_checker;
mod time_range;
mod config;
mod app;
mod ui;
mod event;
mod migrator;
mod logger;

use anyhow::{Context, Result};
use std::path::PathBuf;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use config::Config;
use database::Database;
use file_checker::FileChecker;
use app::{App, LogLevel, ConfirmAction};
use event::{EventHandler, AppEvent};
use migrator::{Migrator, MigrateOptions};
use logger::Logger;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let logger = Arc::new(Logger::new()?);
    println!("日志文件: {:?}", logger.get_log_path());
    
    let (stats, nt_data_dir) = initialize_app().await?;
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(stats, logger);
    let event_handler = EventHandler::new();
    let checker = FileChecker::new(nt_data_dir.clone());
    let migrator = Migrator::new(nt_data_dir.clone());

    let result = run_app(&mut terminal, &mut app, event_handler, &checker, &migrator).await;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("应用错误: {}", err);
    }

    Ok(())
}

async fn initialize_app() -> Result<(Vec<crate::models::GroupStats>, PathBuf)> {
    println!("\n正在初始化...");

    let config = Config::load()?;
    println!("✓ 配置加载成功");

    let qq_base_dir = config.get_qq_base_dir()?;
    if !qq_base_dir.exists() {
        anyhow::bail!("未找到 QQ 数据目录");
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
        anyhow::bail!("未找到 nt_data 目录: {:?}", nt_data_dir);
    }

    println!("✓ 找到数据目录");

    let files_db = config.get_files_db_path();
    let group_db = config.get_group_db_path();

    if !files_db.exists() {
        anyhow::bail!("未找到文件数据库: {:?}", files_db);
    }

    if !group_db.exists() {
        anyhow::bail!("未找到群组数据库: {:?}", group_db);
    }

    let db = Database::new(&files_db, &group_db)
        .context("打开数据库失败")?;
    println!("✓ 数据库打开成功");

    let group_files = db.group_files_by_peer()
        .context("读取文件信息失败")?;
    let groups = db.get_all_groups()
        .context("读取群组信息失败")?;

    println!("✓ 找到 {} 个群组，共 {} 个文件", 
        group_files.len(),
        group_files.values().map(|v| v.len()).sum::<usize>()
    );

    println!("正在分析文件（这可能需要一些时间）...");
    let checker = FileChecker::new(nt_data_dir.clone());
    let group_files_vec: Vec<_> = group_files.into_iter().collect();
    let stats = checker.generate_group_stats(group_files_vec, &groups).await?;
    println!("✓ 分析完成\n");

    Ok((stats, nt_data_dir))
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    event_handler: EventHandler,
    checker: &FileChecker,
    migrator: &Migrator,
) -> Result<()> {
    let mut pending_clean = false;
    let mut pending_migrate = false;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if pending_clean {
            pending_clean = false;
            execute_clean(app, checker).await?;
        }

        if pending_migrate {
            pending_migrate = false;
            execute_migrate(app, migrator).await?;
        }

        match event_handler.next()? {
            AppEvent::Key(key) => {
                event::handle_key_event(app, key);
                
                if !app.show_confirm_dialog {
                    if let Some(action) = app.confirm_action.take() {
                        match action {
                            ConfirmAction::Clean => pending_clean = true,
                            ConfirmAction::Migrate => pending_migrate = true,
                        }
                    }
                }
            }
            AppEvent::Tick => {}
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn execute_clean(app: &mut App, checker: &FileChecker) -> Result<()> {
    let selected_info: Vec<(usize, String, usize)> = app.selected_groups
        .iter()
        .enumerate()
        .filter_map(|(idx, &selected)| {
            if selected {
                app.stats.get(idx).map(|s| (idx, s.group_name.clone(), s.file_count))
            } else {
                None
            }
        })
        .collect();
    
    if selected_info.is_empty() {
        return Ok(());
    }

    app.add_log(LogLevel::Info, &format!("开始清理 {} 个群组", selected_info.len()));
    
    let total_files: usize = selected_info.iter().map(|(_, _, count)| count).sum();
    app.start_operation(total_files);

    let time_range = app.time_range;
    let mut current = 0;
    
    for (idx, group_name, file_count) in selected_info {
        app.add_log(LogLevel::Info, &format!("清理群组: {}", group_name));
        
        let stat = &app.stats[idx];
        match checker.delete_group_files(stat, Some(&time_range)).await {
            Ok((deleted, failed)) => {
                current += file_count;
                app.update_progress(current, &group_name);
                
                if failed > 0 {
                    app.add_log(
                        LogLevel::Warning,
                        &format!("{}: 成功 {} 个, 失败 {} 个", group_name, deleted, failed),
                    );
                } else {
                    app.add_log(
                        LogLevel::Success,
                        &format!("{}: 成功删除 {} 个文件", group_name, deleted),
                    );
                }
            }
            Err(e) => {
                app.add_log(
                    LogLevel::Error,
                    &format!("{}: 删除失败 - {}", group_name, e),
                );
            }
        }
    }

    app.finish_operation();
    app.add_log(LogLevel::Success, "清理操作完成");
    app.selected_groups = vec![false; app.stats.len()];

    Ok(())
}

async fn execute_migrate(app: &mut App, migrator: &Migrator) -> Result<()> {
    let selected_info: Vec<(usize, String, usize)> = app.selected_groups
        .iter()
        .enumerate()
        .filter_map(|(idx, &selected)| {
            if selected {
                app.stats.get(idx).map(|s| (idx, s.group_name.clone(), s.file_count))
            } else {
                None
            }
        })
        .collect();
    
    if selected_info.is_empty() {
        return Ok(());
    }

    app.add_log(LogLevel::Info, &format!("开始迁移 {} 个群组", selected_info.len()));
    
    let total_files: usize = selected_info.iter().map(|(_, _, count)| count).sum();
    app.start_operation(total_files);

    let options = MigrateOptions {
        target_dir: app.migrate_target_path.clone(),
        keep_structure: true,
        delete_after_migrate: false,
    };

    let mut current = 0;
    for (idx, group_name, file_count) in selected_info {
        app.add_log(LogLevel::Info, &format!("迁移群组: {}", group_name));
        
        let stat = &app.stats[idx];
        match migrator.migrate_group_files(stat, &options, None).await {
            Ok(result) => {
                current += file_count;
                app.update_progress(current, &group_name);
                
                if result.failed_files > 0 {
                    app.add_log(
                        LogLevel::Warning,
                        &format!(
                            "{}: 成功 {} 个, 失败 {} 个, 大小: {}",
                            group_name,
                            result.migrated_files,
                            result.failed_files,
                            crate::models::format_bytes(result.total_size)
                        ),
                    );
                } else {
                    app.add_log(
                        LogLevel::Success,
                        &format!(
                            "{}: 成功迁移 {} 个文件, 大小: {}",
                            group_name,
                            result.migrated_files,
                            crate::models::format_bytes(result.total_size)
                        ),
                    );
                }
            }
            Err(e) => {
                app.add_log(
                    LogLevel::Error,
                    &format!("{}: 迁移失败 - {}", group_name, e),
                );
            }
        }
    }

    app.finish_operation();
    app.add_log(LogLevel::Success, "迁移操作完成");
    app.selected_groups = vec![false; app.stats.len()];

    Ok(())
}
