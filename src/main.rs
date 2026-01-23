mod app;
mod config;
mod database;
mod decryptor;
mod event;
mod file_checker;
mod logger;
mod migrator;
mod models;
mod time_range;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::path::PathBuf;

use app::{App, ConfirmAction, LogLevel};
use config::Config;
use database::Database;
use decryptor::Decryptor;
use event::{AppEvent, EventHandler};
use file_checker::FileChecker;
use logger::Logger;
use migrator::{MigrateOptions, Migrator};
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
                    if name
                        .to_string_lossy()
                        .starts_with(&config.paths.nt_qq_prefix)
                    {
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

    println!("正在检查数据库状态...");

    let local_db_dir = config.get_db_dir();

    // 显示当前使用的工作目录
    #[cfg(debug_assertions)]
    println!("数据库工作目录 (debug): {:?}", local_db_dir);

    #[cfg(not(debug_assertions))]
    println!("数据库工作目录 (release): {:?}", local_db_dir);

    println!("\n=== 重要提示 ===");
    println!("本程序不会自动访问或复制任何应用的数据。");
    println!("如需使用本程序，您需要：");
    println!("1. 手动复制数据库文件到工作目录");
    println!("2. 确认您拥有对这些数据的合法访问权");
    println!("3. 理解您正在访问的数据内容");
    println!("4. 自行承担数据访问的法律责任");
    println!("================\n");
    let local_files_db = config.get_files_db_path_in(&local_db_dir);
    let local_group_db = config.get_group_db_path_in(&local_db_dir);

    // 检查是否已有解密后的数据库
    let (files_db, group_db) = if local_files_db.exists() && local_group_db.exists() {
        println!("✓ 找到已解密的数据库: {:?}", local_db_dir);
        (local_files_db, local_group_db)
    } else {
        // 检查用户是否手动复制了原始数据库
        let source_files_db = local_db_dir.join("files_in_chat.db");
        let source_group_db = local_db_dir.join("group_info.db");

        if !source_files_db.exists() || !source_group_db.exists() {
            println!("⚠ 未找到数据库文件");
            println!("\n请按以下步骤操作：");
            println!("1. 手动复制以下文件到工作目录：");
            println!("   - files_in_chat.db");
            println!("   - group_info.db");

            // 获取 QQ 数据库源目录
            let nt_db_source_dir = nt_qq_dir.join("nt_db");

            println!("\n2. 源目录（从这里复制）：");
            println!(
                "   {:?}",
                nt_db_source_dir
                    .canonicalize()
                    .unwrap_or(nt_db_source_dir.clone())
            );

            println!("\n3. 目标目录（复制到这里）：");
            println!(
                "   {:?}",
                local_db_dir.canonicalize().unwrap_or(local_db_dir.clone())
            );

            if !local_db_dir.exists() {
                std::fs::create_dir_all(&local_db_dir)
                    .with_context(|| format!("创建数据库目录失败: {:?}", local_db_dir))?;
                println!("\n✓ 已创建目标目录");
            }

            // 打开两个文件管理器窗口
            println!("\n正在为您打开源目录和目标目录...");

            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(&nt_db_source_dir)
                    .spawn();
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = std::process::Command::new("open")
                    .arg(&local_db_dir)
                    .spawn();
            }

            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(&nt_db_source_dir)
                    .spawn();
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = std::process::Command::new("explorer")
                    .arg(&local_db_dir)
                    .spawn();
            }

            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(&nt_db_source_dir)
                    .spawn();
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = std::process::Command::new("xdg-open")
                    .arg(&local_db_dir)
                    .spawn();
            }

            println!("\n复制完成后，请重新运行本程序。");
            anyhow::bail!("等待用户手动复制数据库文件");
        }

        // 用户已复制文件，现在进行解密
        println!("✓ 找到数据库文件");

        match Decryptor::new() {
            Ok(decryptor) => {
                println!("✓ 找到密钥文件: {:?}", decryptor.get_key_path());
                println!("开始解密数据库...");

                let db_files = ["files_in_chat.db", "group_info.db"];
                decryptor
                    .decrypt_databases(&local_db_dir, &local_db_dir, &db_files)
                    .context("数据库解密失败")?;

                println!("✓ 数据库解密完成");

                (
                    config.get_files_db_path_in(&local_db_dir),
                    config.get_group_db_path_in(&local_db_dir),
                )
            }
            Err(e) => {
                println!("⚠ 未找到密钥文件");
                println!("  提示：请将 sqlcipher.key 放在项目根目录或 ~/.config/qqcleaner/ 目录");
                println!("  错误详情: {}", e);
                anyhow::bail!("无法解密数据库：缺少密钥文件");
            }
        }
    };

    if !files_db.exists() {
        anyhow::bail!("未找到文件数据库: {:?}", files_db);
    }

    if !group_db.exists() {
        anyhow::bail!("未找到群组数据库: {:?}", group_db);
    }

    let db = Database::new(&files_db, &group_db).context("打开数据库失败")?;
    println!("✓ 数据库打开成功");

    let group_files = db.group_files_by_peer().context("读取文件信息失败")?;
    let groups = db.get_all_groups().context("读取群组信息失败")?;

    println!(
        "✓ 找到 {} 个群组，共 {} 个文件",
        group_files.len(),
        group_files.values().map(|v| v.len()).sum::<usize>()
    );

    println!("正在分析文件（这可能需要一些时间）...");
    let checker = FileChecker::new(nt_data_dir.clone());
    let group_files_vec: Vec<_> = group_files.into_iter().collect();
    let stats = checker
        .generate_group_stats(group_files_vec, &groups)
        .await?;
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
            execute_migrate(app, migrator, checker).await?;
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
    let selected_info: Vec<(usize, String, usize)> = app
        .selected_groups
        .iter()
        .enumerate()
        .filter_map(|(idx, &selected)| {
            if selected {
                app.stats
                    .get(idx)
                    .map(|s| (idx, s.group_name.clone(), s.file_count))
            } else {
                None
            }
        })
        .collect();

    if selected_info.is_empty() {
        return Ok(());
    }

    app.add_log(
        LogLevel::Info,
        &format!("开始清理 {} 个群组", selected_info.len()),
    );

    let total_files: usize = selected_info.iter().map(|(_, _, count)| count).sum();
    app.start_operation(total_files);

    let time_range = app.time_range;
    let mut current = 0;
    let mut updated_indices = Vec::new();

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

                if deleted > 0 {
                    updated_indices.push(idx);
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

    if !updated_indices.is_empty() {
        app.add_log(LogLevel::Info, "正在更新统计信息...");
        for idx in updated_indices {
            if let Some(stat) = app.stats.get_mut(idx) {
                let group_name = stat.group_name.clone();
                if let Err(e) = checker.update_group_stats(stat).await {
                    app.add_log(
                        LogLevel::Warning,
                        &format!("更新群组 {} 统计信息失败: {}", group_name, e),
                    );
                }
            }
        }
        app.apply_sort();
    }

    app.finish_operation();
    app.add_log(LogLevel::Success, "清理操作完成");
    app.selected_groups = vec![false; app.stats.len()];

    Ok(())
}

async fn execute_migrate(app: &mut App, migrator: &Migrator, checker: &FileChecker) -> Result<()> {
    let selected_info: Vec<(usize, String, usize)> = app
        .selected_groups
        .iter()
        .enumerate()
        .filter_map(|(idx, &selected)| {
            if selected {
                app.stats
                    .get(idx)
                    .map(|s| (idx, s.group_name.clone(), s.file_count))
            } else {
                None
            }
        })
        .collect();

    if selected_info.is_empty() {
        return Ok(());
    }

    app.add_log(
        LogLevel::Info,
        &format!("开始迁移 {} 个群组", selected_info.len()),
    );

    let total_files: usize = selected_info.iter().map(|(_, _, count)| count).sum();
    app.start_operation(total_files);

    let options = MigrateOptions {
        target_dir: app.migrate_target_path.clone(),
        keep_structure: true,
        delete_after_migrate: !app.get_migrate_keep_original(),
    };

    let mut current = 0;
    let mut updated_indices = Vec::new();
    let should_update = !app.get_migrate_keep_original();

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

                if should_update && result.migrated_files > 0 {
                    updated_indices.push(idx);
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

    if !updated_indices.is_empty() {
        app.add_log(LogLevel::Info, "正在更新统计信息...");
        for idx in updated_indices {
            if let Some(stat) = app.stats.get_mut(idx) {
                let group_name = stat.group_name.clone();
                if let Err(e) = checker.update_group_stats(stat).await {
                    app.add_log(
                        LogLevel::Warning,
                        &format!("更新群组 {} 统计信息失败: {}", group_name, e),
                    );
                }
            }
        }
        app.apply_sort();
    }

    app.finish_operation();
    app.add_log(LogLevel::Success, "迁移操作完成");
    app.selected_groups = vec![false; app.stats.len()];

    Ok(())
}
