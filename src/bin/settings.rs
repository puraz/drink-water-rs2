#![windows_subsystem = "windows"]

//! Settings window for Drink Water Reminder — built with Iced.
//!
//! Runs as a separate binary so its event loop (winit) doesn't conflict
//! with the tray app's event loop (tao).

use iced::theme::Palette;
use iced::widget::{column, container, row, text, text_input};
use iced::{
    application, window, window::icon as window_icon, Alignment, Background, Border, Color,
    Element, Event, Font, Length, Shadow, Subscription, Task, Theme, Vector,
};

use drink_water_rs2::config::Config;

fn main() -> iced::Result {
    // macOS: bring the window to the front as soon as the process is
    // registered with the window server. Poll instead of waiting a fixed
    // 500ms so the window surfaces as fast as the system allows (~100ms).
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(|| {
            for _ in 0..16 {
                std::thread::sleep(std::time::Duration::from_millis(40));
                let brought_front = std::process::Command::new("osascript")
                    .args([
                        "-e",
                        "tell application \"System Events\" to set frontmost of (first process whose name is \"drink-water-settings\") to true",
                    ])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                if brought_front {
                    break;
                }
            }
        });
    }

    application("喝水提醒 — 设置", update, view)
        .theme(|_| water_theme())
        .subscription(subscription)
        .window(window::Settings {
            size: iced::Size::new(380.0, 580.0),
            icon: make_window_icon(),
            exit_on_close_request: false,
            ..window::Settings::default()
        })
        .default_font(default_font())
        .centered()
        .run()
}

/// Listen for window focus / close events so we can auto-save.
fn subscription(_state: &State) -> Subscription<Message> {
    iced::event::listen_with(|event, _status, _id| match event {
        Event::Window(window::Event::Focused) => Some(Message::Focused),
        Event::Window(window::Event::Unfocused) => Some(Message::FocusLost),
        Event::Window(window::Event::CloseRequested) => Some(Message::CloseRequested),
        _ => None,
    })
}

/// A custom light theme with a water-blue accent.
fn water_theme() -> Theme {
    Theme::custom(
        "水蓝".to_string(),
        Palette {
            background: Color::from_rgb8(0xF2, 0xF7, 0xFB),
            text: Color::from_rgb8(0x1B, 0x2B, 0x34),
            primary: Color::from_rgb8(0x21, 0x96, 0xD3),
            success: Color::from_rgb8(0x3F, 0xB6, 0x6B),
            danger: Color::from_rgb8(0xE5, 0x4B, 0x4B),
        },
    )
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
    daily_goal_str: String,
    start_hour_str: String,
    end_hour_str: String,
    error_message: Option<String>,
    /// Whether the window has been focused at least once — guards against
    /// a spurious save/exit from an early Unfocused event during launch.
    has_been_focused: bool,
}

// ── Messages ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    IntervalChanged(String),
    SnoozeChanged(String),
    WaterAmountChanged(String),
    DailyGoalChanged(String),
    StartHourChanged(String),
    EndHourChanged(String),
    Focused,
    FocusLost,
    CloseRequested,
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
        Message::DailyGoalChanged(v) => {
            state.daily_goal_str = v;
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
        Message::Focused => {
            log::debug!("窗口获得焦点");
            state.has_been_focused = true;
            Task::none()
        }
        Message::FocusLost => {
            // Auto-save when the window loses focus. Persist valid input;
            // surface a banner (and keep the window) if it's invalid.
            if state.has_been_focused {
                match try_save(state) {
                    Ok(()) => {
                        state.error_message = None;
                        log::info!("失焦自动保存");
                    }
                    Err(e) => state.error_message = Some(e),
                }
            }
            Task::none()
        }
        Message::CloseRequested => {
            // Save a last time on close, then quit.
            let _ = try_save(state);
            log::info!("窗口关闭");
            iced::exit()
        }
    }
}

// ── View ───────────────────────────────────────────────────────────────────

fn view<'a>(state: &'a State) -> Element<'a, Message> {
    let form = column![
        field(
            "提醒间隔",
            "30",
            &state.interval_str,
            "分钟",
            Message::IntervalChanged
        ),
        field(
            "稍后提醒",
            "5",
            &state.snooze_str,
            "分钟",
            Message::SnoozeChanged
        ),
        field(
            "每次喝水量",
            "250",
            &state.water_amount_str,
            "ml",
            Message::WaterAmountChanged
        ),
        field(
            "每日目标",
            "8",
            &state.daily_goal_str,
            "杯",
            Message::DailyGoalChanged
        ),
        field(
            "开始提醒时间",
            "9",
            &state.start_hour_str,
            "点",
            Message::StartHourChanged
        ),
        field(
            "结束提醒时间",
            "22",
            &state.end_hour_str,
            "点",
            Message::EndHourChanged
        ),
    ]
    .spacing(14);

    let card = container(form)
        .padding(22)
        .width(Length::Fill)
        .style(card_style);

    // ── Assemble ────────────────────────────────────────────────────────────
    let mut content = column![card].spacing(14).max_width(440);
    if let Some(err) = &state.error_message {
        content = content.push(error_banner(err));
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(22)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

// ── Styling ────────────────────────────────────────────────────────────────

/// Muted grey used for secondary labels.
fn muted() -> Color {
    Color::from_rgb8(0x5C, 0x6B, 0x77)
}

/// A labeled text input with an optional trailing unit.
fn field<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    unit: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding(10)
        .size(15)
        .width(Length::Fill);

    let input_row: Element<Message> = if unit.is_empty() {
        input.into()
    } else {
        row![
            input,
            text(unit)
                .size(13)
                .color(muted())
                .width(Length::Fixed(32.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    };

    column![text(label).size(13).color(muted()), input_row]
        .spacing(7)
        .into()
}

/// A red banner shown when validation fails.
fn error_banner(message: &str) -> Element<'_, Message> {
    container(text(message).size(13).color(Color::WHITE))
        .padding([9.0, 14.0])
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xE5, 0x4B, 0x4B))),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 9.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// White rounded card with a soft blue shadow.
fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: Color::from_rgb8(0xDD, 0xE6, 0xED),
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0x14, 0x7C, 0xB8, 0.12),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 18.0,
        },
        ..container::Style::default()
    }
}

impl State {
    fn load() -> Self {
        let cfg = Config::load();
        Self {
            interval_str: cfg.interval_minutes.to_string(),
            snooze_str: cfg.snooze_minutes.to_string(),
            water_amount_str: cfg.water_amount_ml.to_string(),
            daily_goal_str: cfg.daily_goal_cups.to_string(),
            start_hour_str: cfg.start_hour.to_string(),
            end_hour_str: cfg.end_hour.to_string(),
            config: cfg,
            error_message: None,
            has_been_focused: false,
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
    let goal: u64 = state
        .daily_goal_str
        .parse()
        .map_err(|_| "每日目标必须是正数".to_string())?;
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
    if !(1..=5000).contains(&amount) {
        return Err("喝水量必须在 1-5000 ml 之间".into());
    }
    if !(1..=100).contains(&goal) {
        return Err("每日目标必须在 1-100 杯之间".into());
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
    state.config.daily_goal_cups = goal;
    state.config.start_hour = start;
    state.config.end_hour = end;
    state.config.save();

    Ok(())
}
