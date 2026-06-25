#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItemBuilder, PredefinedMenuItem};
use tray_icon::TrayIconBuilder;

mod reminder;

use drink_water_rs2::config;
use drink_water_rs2::icon;
use drink_water_rs2::stats::DrinkStats;

use reminder::Reminder;

#[derive(Clone, Debug)]
enum UserEvent {
    TimeToDrink,
    DrinkNow,
    Snooze,
    ToggleDnd,
    Stats,
    Settings,
    Quit,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("🚀 启动喝水提醒");

    let cfg = config::Config::load();
    let interval_secs = cfg.interval_minutes * 60;
    let water_amount = cfg.water_amount_ml;
    let daily_goal = cfg.daily_goal_cups;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // ── DND State ───────────────────────────────────────────────────────────
    let dnd = Arc::new(AtomicBool::new(false));

    // ── Tray Icon (normal) ──────────────────────────────────────────────────
    let (rgba, w, h) = icon::create_water_drop_rgba();
    let tray_icon_normal = tray_icon::Icon::from_rgba(rgba, w, h).expect("图标 RGBA 数据无效");

    // ── Stats ──────────────────────────────────────────────────────────────
    let mut stats = DrinkStats::load();
    if stats.today_count() > 0 {
        log::info!("今天已喝 {} 杯", stats.today_count());
    }

    // ── Menu Items ─────────────────────────────────────────────────────────
    let id_drink = MenuId::new("drink_now");
    let id_snooze = MenuId::new("snooze");
    let id_dnd = MenuId::new("toggle_dnd");
    let id_stats = MenuId::new("stats");
    let id_settings = MenuId::new("settings");
    let id_quit = MenuId::new("quit");

    // Non-clickable header showing today's progress; refreshed on each drink.
    let status_item = MenuItemBuilder::new()
        .text(format_status(stats.today_count(), water_amount, daily_goal))
        .enabled(false)
        .build();

    let dnd_item = MenuItemBuilder::new()
        .id(id_dnd.clone())
        .text("🔇  勿扰模式")
        .enabled(true)
        .build();

    let menu = {
        let m = Menu::new();
        let mk = |id: &MenuId, text: &str| -> MenuItemBuilder {
            MenuItemBuilder::new()
                .id(id.clone())
                .text(text)
                .enabled(true)
        };

        m.append(&status_item).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_drink, "💧  喝一杯水").build()).ok();
        m.append(&mk(&id_snooze, "⏰  稍后提醒").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_stats, "📊  本周统计").build()).ok();
        m.append(&dnd_item).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_settings, "⚙️  设置…").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_quit, "❌  退出").build()).ok();

        #[cfg(target_os = "macos")]
        m.init_for_nsapp();

        m
    };

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("喝水提醒")
        .with_icon(tray_icon_normal)
        .build()
        .expect("托盘图标创建失败");

    // ── Menu Event Handler ─────────────────────────────────────────────────
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(Box::new(move |event: MenuEvent| {
        let target = if event.id() == &id_drink {
            log::debug!("菜单：喝水");
            Some(UserEvent::DrinkNow)
        } else if event.id() == &id_snooze {
            log::debug!("菜单：稍后提醒");
            Some(UserEvent::Snooze)
        } else if event.id() == &id_dnd {
            log::debug!("菜单：勿扰模式");
            Some(UserEvent::ToggleDnd)
        } else if event.id() == &id_stats {
            log::debug!("菜单：统计");
            Some(UserEvent::Stats)
        } else if event.id() == &id_settings {
            log::debug!("菜单：设置");
            Some(UserEvent::Settings)
        } else if event.id() == &id_quit {
            log::debug!("菜单：退出");
            Some(UserEvent::Quit)
        } else {
            log::debug!("菜单：未知 {:?}", event.id());
            None
        };
        if let Some(ue) = target {
            let _ = proxy.send_event(ue);
        }
    })));

    // ── Reminder Thread ────────────────────────────────────────────────────
    let proxy_rem = event_loop.create_proxy();
    let dnd_reminder = dnd.clone();
    let reminder = Reminder::start(
        proxy_rem,
        interval_secs,
        cfg.start_hour,
        cfg.end_hour,
        dnd_reminder,
    );

    // ── Config File Watcher ────────────────────────────────────────────────
    // Track modification time so we can detect when the settings app saves
    // changes and update the reminder thread dynamically (no restart needed).
    let config_path = config::Config::path();
    let mut last_config_mtime = config_path.metadata().ok().and_then(|m| m.modified().ok());

    // ── Date Watcher ────────────────────────────────────────────────────────
    // Track the current date so we can refresh the status item when the day
    // rolls over (otherwise the menu text would still show yesterday's count).
    let mut last_date = Local::now().date_naive();

    // ── Event Loop ─────────────────────────────────────────────────────────
    event_loop.run(move |event, window_target, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(StartCause::Init) => {
                #[cfg(target_os = "macos")]
                window_target.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
                log::info!("事件循环就绪 — 托盘图标已显示");
            }
            Event::MainEventsCleared => {
                // Check if the date has rolled over.
                let today = Local::now().date_naive();
                if today != last_date {
                    log::info!("📅 新的一天 ({})，重置统计数据", today);
                    last_date = today;
                    stats = DrinkStats::load();
                    let cfg = config::Config::load();
                    status_item.set_text(format_status(
                        stats.today_count(),
                        cfg.water_amount_ml,
                        cfg.daily_goal_cups,
                    ));
                }

                // Check config file for changes (throttled by the ~100ms poll rate).
                if let Ok(meta) = config_path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if last_config_mtime != Some(mtime) {
                            last_config_mtime = Some(mtime);
                            log::info!("📋 配置文件已变更，重新加载…");
                            let new_cfg = config::Config::load();
                            let new_interval_secs = new_cfg.interval_minutes * 60;
                            reminder.change_interval(new_interval_secs);
                            // Refresh status in case goal was changed
                            status_item.set_text(format_status(
                                stats.today_count(),
                                new_cfg.water_amount_ml,
                                new_cfg.daily_goal_cups,
                            ));
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::TimeToDrink => {
                    log::info!("💧 提醒触发");
                }
                UserEvent::DrinkNow => {
                    log::info!("💧 喝了一杯");
                    reminder.reset();
                    let total = stats.record_drink();
                    let cfg = config::Config::load();
                    let amount = cfg.water_amount_ml;
                    let goal = cfg.daily_goal_cups;
                    status_item.set_text(format_status(total, amount, goal));
                    let total_ml = total * amount;
                    let msg = if total == 1 {
                        format!("第 1 杯！{}ml 🎉", amount)
                    } else {
                        let goal_msg = if total >= goal {
                            " 🎯 今日目标已达成！".to_string()
                        } else {
                            format!("（目标 {goal} 杯）")
                        };
                        format!("{} 杯 — {}ml 💪{}", total, total_ml, goal_msg)
                    };
                    reminder::notify(&msg);
                }
                UserEvent::Snooze => {
                    let snooze = config::Config::load().snooze_minutes;
                    log::info!("⏰ 暂停 {} 分钟", snooze);
                    reminder.snooze(snooze);
                }
                UserEvent::Stats => {
                    log::info!("📊  打开统计…");
                    open_stats();
                }
                UserEvent::ToggleDnd => {
                    let was_dnd = dnd.fetch_xor(true, Ordering::SeqCst);
                    if was_dnd {
                        // Turning DND off → blue icon
                        dnd_item.set_text("🔇  勿扰模式");
                        let (rgba, w, h) = icon::create_water_drop_rgba();
                        if let Ok(new_icon) = tray_icon::Icon::from_rgba(rgba, w, h) {
                            let _ = tray.set_icon(Some(new_icon));
                        }
                        let _ = tray.set_tooltip(Some("喝水提醒"));
                        log::info!("🔊 勿扰模式已关闭");
                    } else {
                        // Turning DND on → gray icon
                        dnd_item.set_text("🔊  关闭勿扰");
                        let (rgba, w, h) = icon::create_gray_water_drop_rgba();
                        if let Ok(new_icon) = tray_icon::Icon::from_rgba(rgba, w, h) {
                            let _ = tray.set_icon(Some(new_icon));
                        }
                        let _ = tray.set_tooltip(Some("🔇 勿扰模式"));
                        log::info!("🔇 勿扰模式已开启");
                    }
                }
                UserEvent::Settings => {
                    log::info!("⚙️  打开设置…");
                    open_settings();
                }
                UserEvent::Quit => {
                    log::info!("👋 退出");
                    reminder.quit();
                    *control_flow = ControlFlow::Exit;
                }
            },
            _ => {}
        }
    });
}

// ── Sub-process Launcher ────────────────────────────────────────────────

fn spawn_binary(name: &str, error_label: &str) -> bool {
    let exe_dir = match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        Some(d) => d,
        None => {
            show_error(&format!("无法打开{error_label}"));
            return false;
        }
    };

    #[cfg(target_os = "windows")]
    let bin_name = format!("{name}.exe");
    #[cfg(not(target_os = "windows"))]
    let bin_name = name.to_string();

    let mut path = exe_dir.join(&bin_name);
    if !path.exists() {
        if let Some(fb) = exe_dir.parent().map(|p| p.join(&bin_name)) {
            if fb.exists() {
                path = fb;
            }
        }
    }

    if !path.exists() {
        show_error(&format!(
            "未找到{error_label}程序，请先编译: cargo build --bin {name}"
        ));
        return false;
    }

    match std::process::Command::new(&path).spawn() {
        Ok(child) => {
            log::info!("{error_label}程序 PID {}", child.id());
            true
        }
        Err(e) => {
            show_error(&format!("打开{error_label}失败: {e}"));
            false
        }
    }
}

fn open_settings() {
    spawn_binary("drink-water-settings", "设置");
}

fn open_stats() {
    spawn_binary("drink-water-stats", "统计");
}

fn format_status(count: u64, ml: u64, goal: u64) -> String {
    if count == 0 {
        format!("今天还没喝水（目标 {goal} 杯 🎯）")
    } else {
        let cup_emoji = if count >= goal { "🎯" } else { "☕️" };
        format!("今天 {count}/{goal} 杯 {cup_emoji} — {}ml", count * ml)
    }
}

fn show_error(msg: &str) {
    reminder::notify_error(msg);
}
