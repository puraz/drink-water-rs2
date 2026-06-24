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
    log::info!("🚀 Starting Drink Water Reminder");

    let cfg = config::Config::load();
    let interval_secs = cfg.interval_minutes * 60;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // ── Tray Icon ──────────────────────────────────────────────────────────
    let (rgba, w, h) = drink_water_rs2::icon::create_water_drop_rgba();
    let tray_icon = tray_icon::Icon::from_rgba(rgba, w, h).expect("valid icon RGBA");

    // ── Menu Items ─────────────────────────────────────────────────────────
    let id_drink = MenuId::new("drink_now");
    let id_snooze = MenuId::new("snooze");
    let id_settings = MenuId::new("settings");
    let id_quit = MenuId::new("quit");

    let menu = {
        let m = Menu::new();
        let mk = |id: &MenuId, text: &str| -> MenuItemBuilder {
            MenuItemBuilder::new().id(id.clone()).text(text).enabled(true)
        };

        m.append(&mk(&id_drink, "💧  Drink Now").build()).ok();
        m.append(&mk(&id_snooze, "⏰  Snooze").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_settings, "⚙️  Settings…").build()).ok();
        m.append(&PredefinedMenuItem::separator()).ok();
        m.append(&mk(&id_quit, "❌  Quit").build()).ok();

        #[cfg(target_os = "macos")]
        m.init_for_nsapp();

        m
    };

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("喝水提醒")
        .with_icon(tray_icon)
        .build()
        .expect("tray icon should build");

    // ── Menu Event Handler ─────────────────────────────────────────────────
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(Box::new(move |event: MenuEvent| {
        let target = if event.id() == &id_drink {
            log::debug!("Menu: Drink Now");
            Some(UserEvent::DrinkNow)
        } else if event.id() == &id_snooze {
            log::debug!("Menu: Snooze");
            Some(UserEvent::Snooze)
        } else if event.id() == &id_settings {
            log::debug!("Menu: Settings");
            Some(UserEvent::Settings)
        } else if event.id() == &id_quit {
            log::debug!("Menu: Quit");
            Some(UserEvent::Quit)
        } else {
            log::debug!("Menu: unknown {:?}", event.id());
            None
        };
        if let Some(ue) = target {
            let _ = proxy.send_event(ue);
        }
    })));

    // ── Reminder Thread ────────────────────────────────────────────────────
    let proxy_rem = event_loop.create_proxy();
    let reminder = Reminder::start(proxy_rem, interval_secs, cfg.start_hour, cfg.end_hour);

    // ── Stats ──────────────────────────────────────────────────────────────
    let mut stats = DrinkStats::load_today();
    if stats.today_count() > 0 {
        log::info!("Today's drinks so far: {}", stats.today_count());
    }

    // ── Event Loop ─────────────────────────────────────────────────────────
    event_loop.run(move |event, window_target, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(StartCause::Init) => {
                #[cfg(target_os = "macos")]
                window_target.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
                log::info!("Event loop ready — tray icon should be visible");
            }
            Event::MainEventsCleared => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::TimeToDrink => {
                    log::info!("💧 Reminder triggered");
                }
                UserEvent::DrinkNow => {
                    log::info!("💧 Drink Now");
                    reminder.reset();
                    let total = stats.record_drink();
                    let amount = config::Config::load().water_amount_ml;
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
                    log::info!("⏰ Snoozing {} min", snooze);
                    reminder.snooze(snooze);
                }
                UserEvent::Settings => {
                    log::info!("⚙️  Opening settings…");
                    open_settings();
                }
                UserEvent::Quit => {
                    log::info!("👋 Quitting");
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
            show_error("Could not open settings");
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
        show_error("Settings binary not found. Build: cargo build --bin drink-water-settings");
        return;
    }

    match std::process::Command::new(&path).spawn() {
        Ok(child) => log::info!("Settings PID {}", child.id()),
        Err(e) => show_error(&format!("Failed to open settings: {e}")),
    }
}

fn show_error(msg: &str) {
    reminder::notify_error(msg);
}
