//! Mori chủ động (người dùng cho phép 2026-11-13 — ADR-0013, thay quy tắc "không tự bắt chuyện"):
//! chào theo giờ khi mở máy (gọi tên, nhắc điều đã nhớ) và nhắc nghỉ khi dùng máy liên tục lâu.
//! Tắt bằng setting `proactive = 0`. Bong bóng nói bâng quơ theo hoạt cảnh do UI avatar tự làm (không gọi model).
use crate::{avatar, context, emotion, prompt, speech, AppState, GenerationSlot};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};

/// Nhắc nghỉ sau ngần này phút dùng máy liên tục (rảnh ≥ 5 phút thì đếm lại).
const BREAK_AFTER_MIN: u32 = 90;
const IDLE_RESET_MS: u32 = 5 * 60 * 1000;
/// Không chào lại nếu lần chào trước chưa quá 4 giờ (mở/đóng app nhiều lần).
const GREET_GAP_SECS: u64 = 4 * 3600;

static GREETED: AtomicBool = AtomicBool::new(false);

pub fn enabled(st: &AppState) -> bool {
    st.db.setting("proactive").ok().flatten().as_deref() != Some("0") // mặc định bật (người dùng yêu cầu)
}

/// Giờ máy (không thêm crate thời gian).
#[cfg(windows)]
pub fn clock() -> prompt::Clock {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime chỉ ghi vào SYSTEMTIME ta cấp.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    prompt::Clock {
        hour: t.wHour,
        minute: t.wMinute,
        weekday: t.wDayOfWeek,
        day: t.wDay,
        month: t.wMonth,
        year: t.wYear,
    }
}

/// Người dùng đã không chạm phím/chuột bao lâu (ms) — chỉ là một con số, không biết phím nào.
#[cfg(windows)]
fn idle_ms() -> u32 {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut lii = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: cbSize đúng kích thước struct.
    if unsafe { GetLastInputInfo(&mut lii) } == 0 {
        return 0;
    }
    // SAFETY: không tham số.
    unsafe { GetTickCount() }.wrapping_sub(lii.dwTime)
}

pub fn part_of_day(hour: u16) -> &'static str {
    match hour {
        5..=7 => "sáng sớm",
        8..=10 => "buổi sáng",
        11..=12 => "buổi trưa",
        13..=17 => "buổi chiều",
        18..=21 => "buổi tối",
        _ => "đêm khuya",
    }
}

/// Mori nói một câu ngắn (bong bóng + tiếng nếu bật "Mori nói"), chỉ khi không có lượt nào đang chạy.
fn say(app: &AppHandle, text: &str) {
    let st = app.state::<AppState>();
    let busy = st.generation.lock().unwrap().is_some() || st.player.is_playing();
    let _ = app.emit_to(avatar::LABEL, "avatar-say", text);
    let _ = app.emit_to(avatar::LABEL, "avatar-emotion", emotion::of(text));
    if !busy && st.voice_out() {
        let turn = st.tts.new_turn();
        st.player.stop(turn);
        st.tts.speak(turn, &speech::clean(text), &st.tts_voice());
    }
}

/// Chào khi model vừa sẵn sàng lần đầu trong phiên (và lần chào trước đã cách ≥ 4 giờ).
pub fn greet(app: AppHandle) {
    if GREETED.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let last: u64 = st
            .db
            .setting("last_greet")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if !enabled(&st) || now.saturating_sub(last) < GREET_GAP_SECS {
            return;
        }
        let Ok(Some(conv)) = st.db.latest_conversation() else {
            return;
        };
        let Ok(slot) = GenerationSlot::acquire(&st.generation) else {
            return; // người dùng đã bắt đầu nói chuyện
        };
        let c = clock();
        let (h, m) = (c.hour, c.minute);
        let Ok((system, mut history)) = context(&st.system, &st.db, &conv, "") else {
            return;
        };
        // Ghi chú của app (không lưu DB, không phải lời người dùng).
        history.push(crate::db::Message {
            id: String::new(),
            role: "user".into(),
            content: format!(
                "[Ghi chú của ứng dụng, không phải lời người dùng: người dùng vừa mở máy lúc {h}:{m:02} ({}). \
                 Hãy chủ động chào 1–2 câu ngắn, ấm áp, tự nhiên; gọi tên nếu biết; có thể hỏi thăm một điều \
                 trong phần Ghi nhớ. Không nhắc tới ghi chú này.]",
                part_of_day(h)
            ),
            created_at: String::new(),
        });
        let result = st
            .llm
            .chat(&prompt::build(&system, &history), &slot.stop, |_| {})
            .await;
        drop(slot);
        let text = match result {
            Ok((t, false)) if !t.trim().is_empty() => t.trim().to_string(),
            Ok(_) => return, // bị ngắt: người dùng nói trước
            Err(e) => {
                warn!(error = %e, "chào lỗi");
                return;
            }
        };
        let _ = st.db.set_setting("last_greet", &now.to_string());
        match st.db.add_message(&conv, "assistant", &text) {
            Ok(msg) => {
                let _ = app.emit_to("main", "proactive-message", (conv.clone(), msg));
            }
            Err(e) => warn!(error = %e, "không lưu lời chào"),
        }
        info!(chars = text.chars().count(), "Mori chào");
        say(&app, &text);
        crate::prefill_next(&st.llm, &st.system, &st.db, &conv);
    });
}

/// Nhắc nghỉ: đếm phút dùng máy liên tục (rảnh ≥ 5 phút thì về 0).
pub fn start_break_reminder(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut active_min = 0u32;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            active_min = if idle_ms() >= IDLE_RESET_MS {
                0
            } else {
                active_min + 1
            };
            if active_min >= BREAK_AFTER_MIN && enabled(&app.state::<AppState>()) {
                active_min = 0;
                info!("nhắc nghỉ");
                say(
                    &app,
                    "Bạn ngồi máy hơn tiếng rưỡi rồi đó. Đứng dậy uống nước, vươn vai một chút nha, Mori đợi! 😊",
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::part_of_day;

    #[test]
    fn parts_of_day() {
        assert_eq!(part_of_day(6), "sáng sớm");
        assert_eq!(part_of_day(12), "buổi trưa");
        assert_eq!(part_of_day(20), "buổi tối");
        assert_eq!(part_of_day(2), "đêm khuya");
    }
}
