use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn next(&self) -> Result<AppEvent> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => Ok(AppEvent::Key(key)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}

pub fn handle_key_event(app: &mut crate::app::App, key: KeyEvent) {
    use crate::app::{AppTab, ConfirmAction};
    
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
        return;
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('h') => {
                app.toggle_help();
            }
            _ => {}
        }
        return;
    }

    if app.show_confirm_dialog {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.show_confirm_dialog = false;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.hide_confirm();
            }
            _ => {}
        }
        return;
    }

    if app.show_filter_dialog {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.filter_prev_item();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.filter_next_item();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                app.toggle_filter_option();
            }
            KeyCode::Char('a') => {
                app.apply_filter_dialog();
            }
            KeyCode::Char('c') | KeyCode::Esc => {
                app.cancel_filter_dialog();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.quit();
        }
        KeyCode::Char('?') | KeyCode::Char('h') => {
            app.toggle_help();
        }
        KeyCode::Tab => {
            app.next_tab();
        }
        KeyCode::BackTab => {
            app.prev_tab();
        }
        KeyCode::Char('1') => app.current_tab = AppTab::Home,
        KeyCode::Char('2') => app.current_tab = AppTab::Analysis,
        KeyCode::Char('3') => app.current_tab = AppTab::Clean,
        KeyCode::Char('4') => app.current_tab = AppTab::Migrate,
        KeyCode::Char('5') => app.current_tab = AppTab::Logs,
        _ => {}
    }

    match app.current_tab {
        AppTab::Home | AppTab::Clean | AppTab::Migrate => {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    app.next_item();
                    return;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.prev_item();
                    return;
                }
                KeyCode::Char(' ') => {
                    app.toggle_selected_group();
                    return;
                }
                KeyCode::Char('a') => {
                    app.select_all_filtered();
                    return;
                }
                KeyCode::Char('A') => {
                    app.deselect_all();
                    return;
                }
                _ => {}
            }
        }
        _ => {}
    }

    match app.current_tab {
        AppTab::Home | AppTab::Clean | AppTab::Migrate => {
            match key.code {
                KeyCode::Char('s') => {
                    app.sort_by = match app.sort_by {
                        crate::app::SortBy::Size => crate::app::SortBy::FileCount,
                        crate::app::SortBy::FileCount => crate::app::SortBy::Name,
                        crate::app::SortBy::Name => crate::app::SortBy::Size,
                    };
                    app.apply_sort();
                    app.add_log(crate::app::LogLevel::Info, &format!("排序方式: {:?}", app.sort_by));
                }
                KeyCode::Char('f') => {
                    app.open_filter_dialog();
                }
                _ => {}
            }
        }
        _ => {}
    }

    if app.current_tab == AppTab::Clean {
        match key.code {
            KeyCode::Char('t') => {
                app.time_range = match app.time_range {
                    crate::time_range::TimeRange::All => crate::time_range::TimeRange::DaysAgo(7),
                    crate::time_range::TimeRange::DaysAgo(7) => crate::time_range::TimeRange::DaysAgo(30),
                    crate::time_range::TimeRange::DaysAgo(30) => crate::time_range::TimeRange::DaysAgo(90),
                    crate::time_range::TimeRange::DaysAgo(90) => crate::time_range::TimeRange::DaysAgo(180),
                    crate::time_range::TimeRange::DaysAgo(_) => crate::time_range::TimeRange::All,
                };
                app.add_log(crate::app::LogLevel::Info, &format!("时间范围: {}", app.time_range.description()));
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if app.selected_count() > 0 {
                    app.show_confirm(ConfirmAction::Clean);
                } else {
                    app.add_log(crate::app::LogLevel::Warning, "请先选择要清理的群组");
                }
            }
            _ => {}
        }
    }

    if app.current_tab == AppTab::Migrate {
        match key.code {
            KeyCode::Char('m') | KeyCode::Enter => {
                if app.selected_count() > 0 {
                    app.show_confirm(ConfirmAction::Migrate);
                } else {
                    app.add_log(crate::app::LogLevel::Warning, "请先选择要迁移的群组");
                }
            }
            KeyCode::Char('p') => {
                app.next_migrate_path();
                app.add_log(crate::app::LogLevel::Info, &format!("迁移路径: {}", app.migrate_target_path.display()));
            }
            KeyCode::Left => {
                app.prev_migrate_path();
                app.add_log(crate::app::LogLevel::Info, &format!("迁移路径: {}", app.migrate_target_path.display()));
            }
            KeyCode::Right => {
                app.next_migrate_path();
                app.add_log(crate::app::LogLevel::Info, &format!("迁移路径: {}", app.migrate_target_path.display()));
            }
            _ => {}
        }
    }

    if app.current_tab == AppTab::Logs {
        match key.code {
            KeyCode::Char('c') => {
                app.logs.clear();
                app.add_log(crate::app::LogLevel::Info, "日志已清空");
            }
            _ => {}
        }
    }
}

