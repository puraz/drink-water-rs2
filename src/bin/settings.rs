//! Settings window for Drink Water Reminder — built with Iced.
//!
//! Runs as a separate binary so its event loop (winit) doesn't conflict
//! with the tray app's event loop (tao).

use iced::widget::{
    button, column, container, row, text, text_input,
};
use iced::{application, window, window::icon as window_icon, Color, Element, Font, Length, Task};

use drink_water_rs2::config::Config;

fn main() -> iced::Result {
    // macOS: bring window to front after launch
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::process::Command::new("osascript")
                .args(&["-e",
                    "tell application \"System Events\" to set frontmost of every process whose name is \"drink-water-settings\" to true"
                ])
                .spawn()
                .ok();
        });
    }

    application("喝水提醒 — 设置", update, view)
        .window(window::Settings {
            size: iced::Size::new(360.0, 460.0),
            icon: make_window_icon(),
            ..window::Settings::default()
        })
        .default_font(default_font())
        .centered()
        .run()
}

/// Create a window icon from the water-drop RGBA data
fn make_window_icon() -> Option<iced::window::Icon> {
    let (rgba, w, h) = drink_water_rs2::icon::create_water_drop_rgba();
    window_icon::from_rgba(rgba, w, h).ok()
}

/// Choose a system font that supports Chinese characters
fn default_font() -> Font {
    if cfg!(target_os = "macos") {
        Font::with_name("PingFang SC")
    } else if cfg!(target_os = "windows") {
        Font::with_name("Microsoft YaHei")
    } else if cfg!(target_os = "linux") {
        Font::with_name("Noto Sans CJK SC")
    } else {
        Font::default()
    }
}

// ── State ─────────────────────────────────────────────────────────────────

struct State {
    config: Config,
    interval_str: String,
    snooze_str: String,
    water_amount_str: String,
    start_hour_str: String,
    end_hour_str: String,
    error_message: Option<String>,
}

// ── Messages ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    IntervalChanged(String),
    SnoozeChanged(String),
    WaterAmountChanged(String),
    StartHourChanged(String),
    EndHourChanged(String),
    Save,
    Cancel,
}

// ── Update ─────────────────────────────────────────────────────────────────

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::IntervalChanged(v) => {
            state.interval_str = v;
            state.error_message = None;
            Task::none()
        }
        Message::SnoozeChanged(v) => {
            state.snooze_str = v;
            state.error_message = None;
            Task::none()
        }
        Message::WaterAmountChanged(v) => {
            state.water_amount_str = v;
            state.error_message = None;
            Task::none()
        }
        Message::StartHourChanged(v) => {
            state.start_hour_str = v;
            state.error_message = None;
            Task::none()
        }
        Message::EndHourChanged(v) => {
            state.end_hour_str = v;
            state.error_message = None;
            Task::none()
        }
        Message::Save => match try_save(state) {
            Ok(()) => {
                log::info!("设置已保存");
                iced::exit()
            }
            Err(e) => {
                state.error_message = Some(e);
                Task::none()
            }
        },
        Message::Cancel => {
            log::info!("设置已取消");
            iced::exit()
        }
    }
}

// ── View ───────────────────────────────────────────────────────────────────

fn view<'a>(state: &'a State) -> Element<'a, Message> {
    let form = column![
        text("提醒间隔（分钟）").size(13),
        text_input("30", &state.interval_str).on_input(Message::IntervalChanged).padding(6),
        text("稍后提醒（分钟）").size(13),
        text_input("5", &state.snooze_str).on_input(Message::SnoozeChanged).padding(6),
        text("每次喝水量（ml）").size(13),
        text_input("250", &state.water_amount_str).on_input(Message::WaterAmountChanged).padding(6),
        text("开始提醒时间").size(13),
        text_input("9", &state.start_hour_str).on_input(Message::StartHourChanged).padding(6),
        text("结束提醒时间").size(13),
        text_input("22", &state.end_hour_str).on_input(Message::EndHourChanged).padding(6),
    ]
    .spacing(4)
    .padding(8);

    let maybe_error: Element<Message> = if let Some(err) = &state.error_message {
        text(err).color(Color::from_rgb(0.8, 0.2, 0.2)).into()
    } else {
        text("").into()
    };

    let buttons = row![
        button("取消").on_press(Message::Cancel).padding(6),
        button("保存").on_press(Message::Save).padding(6),
    ]
    .spacing(8);

    let content = column![form, maybe_error, buttons]
        .spacing(6)
        .align_x(iced::Alignment::Center)
        .max_width(400);

    container(content)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding(8)
        .into()
}

impl State {
    fn load() -> Self {
        let cfg = Config::load();
        Self {
            interval_str: cfg.interval_minutes.to_string(),
            snooze_str: cfg.snooze_minutes.to_string(),
            water_amount_str: cfg.water_amount_ml.to_string(),
            start_hour_str: cfg.start_hour.to_string(),
            end_hour_str: cfg.end_hour.to_string(),
            config: cfg,
            error_message: None,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::load()
    }
}

// ── Validation & Save ─────────────────────────────────────────────────────

fn try_save(state: &mut State) -> Result<(), String> {
    let interval: u64 = state
        .interval_str
        .parse()
        .map_err(|_| "提醒间隔必须是正数".to_string())?;
    let snooze: u64 = state
        .snooze_str
        .parse()
        .map_err(|_| "稍后提醒时长必须是正数".to_string())?;
    let amount: u64 = state
        .water_amount_str
        .parse()
        .map_err(|_| "喝水量必须是正数".to_string())?;
    let start: u8 = state
        .start_hour_str
        .parse()
        .map_err(|_| "开始时间必须是数字 (0-23)".to_string())?;
    let end: u8 = state
        .end_hour_str
        .parse()
        .map_err(|_| "结束时间必须是数字 (0-23)".to_string())?;

    if interval < 1 {
        return Err("提醒间隔至少 1 分钟".into());
    }
    if snooze < 1 {
        return Err("稍后提醒至少 1 分钟".into());
    }
    if amount < 1 || amount > 5000 {
        return Err("喝水量必须在 1-5000 ml 之间".into());
    }
    if start > 23 {
        return Err("开始时间必须在 0-23 之间".into());
    }
    if end > 23 {
        return Err("结束时间必须在 0-23 之间".into());
    }
    if start >= end {
        return Err("开始时间必须早于结束时间".into());
    }

    state.config.interval_minutes = interval;
    state.config.snooze_minutes = snooze;
    state.config.water_amount_ml = amount;
    state.config.start_hour = start;
    state.config.end_hour = end;
    state.config.save();

    Ok(())
}
