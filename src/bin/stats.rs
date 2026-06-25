#![windows_subsystem = "windows"]

//! Statistics window — shows 7-day drinking history as a refined bar chart.
//!
//! Launched from the tray menu as a separate process.

use iced::border::Radius;
use iced::mouse;
use iced::theme::Palette;
use iced::widget::canvas::{self, Canvas, Fill, Frame, Geometry, Path, Program};
use iced::widget::column;
use iced::widget::{container, text};
use iced::{
    alignment, application, window, window::icon as window_icon, Color, Element, Font, Length,
    Pixels, Point, Renderer, Size, Task, Theme,
};

use drink_water_rs2::icon;
use drink_water_rs2::stats::{DayRecord, DrinkStats};

fn main() -> iced::Result {
    // Bring to front on macOS
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(|| {
            for _ in 0..16 {
                std::thread::sleep(std::time::Duration::from_millis(40));
                let brought_front = std::process::Command::new("osascript")
                    .args([
                        "-e",
                        "tell application \"System Events\" to set frontmost of (first process whose name is \"drink-water-stats\") to true",
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

    application("喝水提醒 — 统计", update, view)
        .theme(|_| water_theme())
        .window(window::Settings {
            size: Size::new(420.0, 340.0),
            icon: make_window_icon(),
            resizable: false,
            ..window::Settings::default()
        })
        .default_font(default_font())
        .centered()
        .run()
}

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

fn make_window_icon() -> Option<iced::window::Icon> {
    let (rgba, w, h) = icon::create_water_drop_rgba();
    window_icon::from_rgba(rgba, w, h).ok()
}

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
    history: Vec<DayRecord>,
    total: u64,
    avg: f64,
    goal: u64,
}

// ── Messages ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {}

// ── Update ─────────────────────────────────────────────────────────────────

fn update(_state: &mut State, _message: Message) -> Task<Message> {
    Task::none()
}

// ── View ──────────────────────────────────────────────────────────────────

fn view(state: &State) -> Element<'_, Message> {
    let avg_str = format!("{:.1}", state.avg);
    let summary = if state.total == 0 {
        "本周还没喝水记录".to_string()
    } else {
        format!(
            "这周共喝 {} 杯 · 日均 {} 杯 · 目标 {} 杯",
            state.total, avg_str, state.goal
        )
    };

    let body = column![
        text(summary)
            .size(13)
            .color(Color::from_rgb8(0x5C, 0x6B, 0x77)),
        chart_section(&state.history, state.goal),
    ]
    .spacing(44);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([24.0, 24.0])
        .into()
}

fn chart_section<'a>(records: &'a [DayRecord], goal: u64) -> Canvas<WeekChart<'a>, Message> {
    let max_count = records
        .iter()
        .map(|r| r.count)
        .max()
        .unwrap_or(0)
        .max(goal)
        .max(1);
    Canvas::new(WeekChart {
        records,
        max_count,
        goal,
    })
    .width(Length::Fill)
    .height(Length::Fixed(200.0))
}

// ── Chart ─────────────────────────────────────────────────────────────────

struct WeekChart<'a> {
    records: &'a [DayRecord],
    max_count: u64,
    goal: u64,
}

impl<'a, Message> Program<Message> for WeekChart<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.palette();

        let bar_count = self.records.len();
        if bar_count == 0 {
            return vec![frame.into_geometry()];
        }

        // ── Layout ─────────────────────────────────────────────────────────
        let top = 20.0;
        let bottom = 26.0;
        let pad = 2.0;
        let chart_w = bounds.width - pad * 2.0;
        let chart_h = bounds.height - top - bottom;
        let max = self.max_count.max(1);

        // ── Bars ───────────────────────────────────────────────────────────
        let spacing = chart_w / bar_count as f32;
        let bar_w = (spacing * 0.52).min(28.0);
        let gap = (spacing - bar_w) / 2.0;
        let corner_r = 4.0;

        // Bar background color
        let bg_color = Color::from_rgba8(0x21, 0x96, 0xD3, 0.15);

        for (i, record) in self.records.iter().enumerate() {
            let bar_h = (record.count as f32 / max as f32) * chart_h;
            let x = pad + spacing * i as f32 + gap;
            let y = top + chart_h - bar_h;
            let bar_rect = Size::new(bar_w, bar_h.max(1.0));

            // Determine color
            let bar_color = if self.goal > 0 && record.count >= self.goal {
                palette.success
            } else {
                palette.primary
            };

            // Draw bar background (subtle pill shape behind the bar)
            if bar_h > 0.0 {
                let bg_path = Path::new(|b| {
                    b.rounded_rectangle(
                        Point::new(x, top),
                        Size::new(bar_w, chart_h),
                        Radius::from(corner_r),
                    );
                });
                frame.fill(&bg_path, Fill::from(bg_color));
            }

            // Draw the actual bar with rounded top (and bottom if full)
            let path = Path::new(|b| {
                b.rounded_rectangle(Point::new(x, y), bar_rect, Radius::from(corner_r));
            });
            frame.fill(&path, Fill::from(bar_color));

            // Day label
            let day_label = weekday_short(record.date.format("%w").to_string().as_str());
            frame.fill_text(canvas::Text {
                content: day_label.to_string(),
                position: Point::new(x + bar_w / 2.0, top + chart_h + 8.0),
                color: Color::from_rgb8(0x8C, 0x9A, 0xA8),
                size: Pixels(11.0),
                font: default_font(),
                horizontal_alignment: alignment::Horizontal::Center,
                ..canvas::Text::default()
            });

            // Count above bar
            if record.count > 0 {
                frame.fill_text(canvas::Text {
                    content: record.count.to_string(),
                    position: Point::new(x + bar_w / 2.0, y - 6.0),
                    color: palette.text,
                    size: Pixels(12.0),
                    font: default_font(),
                    horizontal_alignment: alignment::Horizontal::Center,
                    ..canvas::Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

fn weekday_short(w: &str) -> &'static str {
    match w {
        "0" => "日",
        "1" => "一",
        "2" => "二",
        "3" => "三",
        "4" => "四",
        "5" => "五",
        "6" => "六",
        _ => "?",
    }
}

impl State {
    fn load() -> Self {
        let stats = DrinkStats::load();
        let history = stats.week_history();
        let total: u64 = history.iter().map(|r| r.count).sum();
        let avg = total as f64 / 7.0;
        let cfg = drink_water_rs2::config::Config::load();
        Self {
            history,
            total,
            avg,
            goal: cfg.daily_goal_cups,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::load()
    }
}
