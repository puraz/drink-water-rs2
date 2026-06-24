use chrono::Local;
use chrono::Timelike;
#[cfg(not(target_os = "macos"))]
use notify_rust::Notification;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::UserEvent;
use tao::event_loop::EventLoopProxy;

// ── Commands ──────────────────────────────────────────────────────────────

pub enum ReminderCommand {
    Reset,
    Snooze(u64),
    ChangeInterval(u64),
    Quit,
}

// ── Reminder Handle ───────────────────────────────────────────────────────

pub struct Reminder {
    pub sender: mpsc::Sender<ReminderCommand>,
}

impl Reminder {
    pub fn start(
        proxy: EventLoopProxy<UserEvent>,
        interval_secs: u64,
        start_hour: u8,
        end_hour: u8,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut interval = Duration::from_secs(interval_secs);

        thread::spawn(move || {
            log::info!(
                "提醒已启动：间隔={interval_secs}秒，时段={start_hour}:00–{end_hour}:00"
            );

            loop {
                let now = Local::now();
                let hour = now.hour() as u8;

                if hour >= start_hour && hour < end_hour {
                    match rx.recv_timeout(interval) {
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            log::info!("💧 该喝水了！");
                            show_notification("💧 该喝水了！", "保持水分充足，身体才能发挥最佳状态。");
                            let _ = proxy.send_event(UserEvent::TimeToDrink);
                        }
                        Ok(ReminderCommand::Reset) => {
                            log::info!("↻ 计时器已重置");
                            continue;
                        }
                        Ok(ReminderCommand::Snooze(minutes)) => {
                            log::info!("⏰ 暂停 {minutes} 分钟");
                            show_snooze_notification(minutes);
                            thread::sleep(Duration::from_secs(minutes * 60));
                            show_notification("💧 该喝水了！", "保持水分充足，身体才能发挥最佳状态。");
                            let _ = proxy.send_event(UserEvent::TimeToDrink);
                        }
                        Ok(ReminderCommand::ChangeInterval(secs)) => {
                            log::info!("⏱ 提醒间隔已更新为 {} 秒", secs);
                            interval = Duration::from_secs(secs);
                            continue;
                        }
                        Ok(ReminderCommand::Quit)
                        | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                } else {
                    match rx.recv_timeout(Duration::from_secs(60)) {
                        Ok(ReminderCommand::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                        _ => {}
                    }
                }
            }

            log::info!("提醒线程退出");
        });

        Reminder { sender: tx }
    }

    pub fn reset(&self) {
        let _ = self.sender.send(ReminderCommand::Reset);
    }

    pub fn snooze(&self, minutes: u64) {
        let _ = self.sender.send(ReminderCommand::Snooze(minutes));
    }

    pub fn change_interval(&self, interval_secs: u64) {
        let _ = self.sender.send(ReminderCommand::ChangeInterval(interval_secs));
    }

    pub fn quit(&self) {
        let _ = self.sender.send(ReminderCommand::Quit);
    }
}

// ── Notifications ─────────────────────────────────────────────────────────
//
// On macOS we use `osascript` to show notifications because notify-rust
// may not work correctly when the app runs as a LSUIElement background
// process (no dock icon).  osascript-based notifications always work
// regardless of the app's activation policy.

#[cfg(target_os = "macos")]
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn osascript_notify(summary: &str, body: &str) {
    let escaped_body = escape_applescript(body);
    let escaped_summary = escape_applescript(summary);
    let result = std::process::Command::new("osascript")
        .args(&[
            "-e",
            &format!(
                r#"display notification "{}" with title "{}" sound name "Ping""#,
                escaped_body, escaped_summary,
            ),
        ])
        .output();

    match result {
        Ok(out) => {
            if out.status.success() {
                log::info!("通过 osascript 发送通知");
            } else {
                log::error!(
                    "osascript 失败: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
        Err(e) => log::error!("osascript 错误: {e}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn osascript_notify(_summary: &str, _body: &str) {}

/// Send a notification using the best available method for the platform.
///
/// On macOS we go straight to `osascript`: when the app runs from a bundle,
/// `notify-rust` reports success (`Ok`) even though the OS silently drops the
/// notification for an unsigned / unauthorized bundle — so the old "try
/// notify-rust, fall back to osascript on error" path never reached the
/// fallback and nothing showed. osascript works regardless of signing or the
/// LSUIElement activation policy.
#[cfg(target_os = "macos")]
fn send_notification(summary: &str, body: &str, _timeout_ms: u32) {
    osascript_notify(summary, body);
}

#[cfg(not(target_os = "macos"))]
fn send_notification(summary: &str, body: &str, timeout_ms: u32) {
    let mut n = Notification::new();
    n.summary(summary);
    n.body(body);
    n.appname("喝水提醒");
    n.timeout(notify_rust::Timeout::Milliseconds(timeout_ms));
    #[cfg(target_os = "linux")]
    n.sound_name("message-new-instant");

    match n.show() {
        Ok(_) => log::info!("通过 notify-rust 发送通知"),
        Err(e) => log::error!("notify-rust 失败: {e}"),
    }
}

/// Show the "time to drink" notification.
fn show_notification(summary: &str, body: &str) {
    send_notification(summary, body, 12000);
}

fn show_snooze_notification(minutes: u64) {
    let body = format!("{minutes} 分钟后再次提醒。");
    send_notification("⏰ 已暂停", &body, 5000);
}

/// Show a generic informational notification
pub fn notify(body: &str) {
    send_notification("💧 喝水提醒", body, 5000);
}

/// Show an error notification
pub fn notify_error(body: &str) {
    send_notification("⚠️ 喝水提醒", body, 8000);
}
