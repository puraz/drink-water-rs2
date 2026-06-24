use std::path::PathBuf;
use std::time::Duration;

use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::event::{Event, StartCause};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItemBuilder, PredefinedMenuItem};
use tray_icon::TrayIconBuilder;

mod reminder;
mod stats;

use drink_water_rs2::config;

use reminder::Reminder;
use stats::DrinkStats;

#[derive(Clone, Debug)]
enum UserEvent {
    TimeToDrink,
    DrinkNow,
    Snooze,
    Settings,
    Quit,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    log::info!("🚀 启动喝水提醒");

    let cfg = config::Config::load();
    let interval_secs = cfg.interval_minutes * 60;
    let water_amount = cfg.water_amount_ml;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // ── Tray Icon ──────────────────────────────────────────────────────────
    let (rgba, w, h) = drink_water_rs2::icon::create_water_drop_rgba();
    let tray_icon = tray_icon::Icon::from_rgba(rgba, w, h).expect("图标 RGBA 数据无效");

    // ── Stats ──────────────────────────────────────────────────────────────
    let mut stats = DrinkStats::load_today();
    if stats.today_count() > 0 {
        log::info!("今天已喝 {} 杯", stats.today_count());
    }

    // ── Menu Items ─────────────────────────────────────────────────────────
    let id_drink = MenuId::new("drink_now");
    let id_snooze = MenuId::new("snooze");
    let id_settings = MenuId::new("settings");
    let id_quit = MenuId::new("quit");

    // Non-clickable header showing today's progress; refreshed on each drink.
    let status_item = MenuItemBuilder::new()
        .text(format_status(stats.today_count(), water_amount))
        .enabled(false)
        .build();

    let menu = {
        let m = Menu::new();
        let mk = |id: &MenuId, text: &str| -> MenuItemBuilder {
            MenuItemBuilder::new().id(id.clone()).text(text).enabled(true)
        };

        m.append(&status_item).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_drink, "💧  喝一杯水").build()).ok();
        m.append(&mk(&id_snooze, "⏰  稍后提醒").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_settings, "⚙️  设置…").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_quit, "❌  退出").build()).ok();

        #[cfg(target_os = "macos")]
        m.init_for_nsapp();

        m
    };

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("喝水提醒")
        .with_icon(tray_icon)
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
    let reminder = Reminder::start(proxy_rem, interval_secs, cfg.start_hour, cfg.end_hour);

    // ── Config File Watcher ────────────────────────────────────────────────
    // Track modification time so we can detect when the settings app saves
    // changes and update the reminder thread dynamically (no restart needed).
    let config_path = config::Config::path();
    let mut last_config_mtime = config_path.metadata().ok().and_then(|m| m.modified().ok());

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
                // Check config file for changes (throttled by the ~100ms poll rate).
                if let Ok(meta) = config_path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if last_config_mtime.map_or(true, |t| mtime != t) {
                            last_config_mtime = Some(mtime);
                            log::info!("📋 配置文件已变更，重新加载…");
                            let new_cfg = config::Config::load();
                            let new_interval_secs = new_cfg.interval_minutes * 60;
                            reminder.change_interval(new_interval_secs);
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
                    let amount = config::Config::load().water_amount_ml;
                    status_item.set_text(&format_status(total, amount));
                    let total_ml = total * amount;
                    let msg = if total == 1 {
                        format!("第 1 杯！{}ml 🎉", amount)
                    } else {
                        format!("{} 杯 — {}ml 💪", total, total_ml)
                    };
                    reminder::notify(&msg);
                }
                UserEvent::Snooze => {
                    let snooze = config::Config::load().snooze_minutes;
                    log::info!("⏰ 暂停 {} 分钟", snooze);
                    reminder.snooze(snooze);
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

// ── Settings ──────────────────────────────────────────────────────────────

fn open_settings() {
    let exe_dir = match std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        Some(d) => d,
        None => {
            show_error("无法打开设置");
            return;
        }
    };

    #[cfg(target_os = "windows")]
    let name = "drink-water-settings.exe";
    #[cfg(not(target_os = "windows"))]
    let name = "drink-water-settings";

    let mut path = exe_dir.join(name);
    if !path.exists() {
        if let Some(fb) = exe_dir.parent().map(|p| p.join(name)) {
            if fb.exists() {
                path = fb;
            }
        }
    }

    if !path.exists() {
        show_error("未找到设置程序，请先编译: cargo build --bin drink-water-settings");
        return;
    }

    match std::process::Command::new(&path).spawn() {
        Ok(child) => log::info!("设置程序 PID {}", child.id()),
        Err(e) => show_error(&format!("打开设置失败: {e}")),
    }
}

fn format_status(count: u64, ml: u64) -> String {
    if count == 0 {
        "今天还没喝水".to_string()
    } else {
        format!("今天 {} 杯 — {}ml", count, count * ml)
    }
}

fn show_error(msg: &str) {
    reminder::notify_error(msg);
}
