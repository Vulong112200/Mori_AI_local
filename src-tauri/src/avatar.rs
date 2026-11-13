//! Cửa sổ avatar Live2D (SPEC §9.4, Phase 5): chế độ hiển thị, kéo thả, xuyên chuột, nhớ vị trí,
//! trạng thái nghe/nghĩ/rảnh cho UI. Làm bằng lệnh Rust → không phải mở quyền `core:window:*` cho webview.
use crate::{data_dir, err, AppState};
use base64::Engine;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};
use tracing::{info, warn};

pub const LABEL: &str = "avatar";
/// Companion: chỉ nhân vật · Chat: nhân vật + khung chat · Focus: ẩn nhân vật, dịch vụ vẫn chạy.
pub const MODES: &[&str] = &["companion", "chat", "focus"];

/// Trạng thái cho avatar ("nói" do UI lấy từ `tts-playing`).
pub fn set_state(app: &AppHandle, s: &str) {
    let _ = app.emit_to(LABEL, "avatar-state", s);
}

pub fn mode(st: &AppState) -> String {
    st.db
        .setting("avatar_mode")
        .ok()
        .flatten()
        .filter(|m| MODES.contains(&m.as_str()))
        .unwrap_or_else(|| "chat".into())
}

pub fn click_through(st: &AppState) -> bool {
    st.db
        .setting("avatar_click_through")
        .ok()
        .flatten()
        .as_deref()
        == Some("1")
}

fn apply_mode(app: &AppHandle, mode: &str) {
    let (main, avatar) = (
        app.get_webview_window("main"),
        app.get_webview_window(LABEL),
    );
    let show = |w: &Option<WebviewWindow>, on: bool| {
        if let Some(w) = w {
            let _ = if on { w.show() } else { w.hide() };
        }
    };
    show(&avatar, mode != "focus");
    show(&main, mode != "companion");
    if mode == "companion" {
        if let Some(a) = &avatar {
            // Chỉ còn nhân vật để nhấp đúp mở lại chat → không được xuyên chuột (audit Phase 5 M1).
            let _ = a.set_ignore_cursor_events(false);
        }
    }
    if mode != "companion" {
        if let Some(m) = &main {
            let _ = m.set_focus();
        }
    }
}

/// Lúc khởi động: vị trí đã nhớ (nếu còn nằm trên một màn hình), xuyên chuột, chế độ; lưu vị trí khi kéo.
pub fn setup(app: &AppHandle) {
    let st = app.state::<AppState>();
    if let Some(w) = app.get_webview_window(LABEL) {
        let saved = st
            .db
            .setting("avatar_pos")
            .ok()
            .flatten()
            .and_then(|s| {
                let (x, y) = s.split_once(',')?;
                Some((x.parse::<i32>().ok()?, y.parse::<i32>().ok()?))
            })
            .filter(|&(x, y)| {
                w.available_monitors().unwrap_or_default().iter().any(|m| {
                    let (p, s) = (m.position(), m.size());
                    x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
                })
            });
        let pos = saved.or_else(|| {
            // Mặc định: góc dưới phải màn hình chính.
            let m = w.primary_monitor().ok()??;
            let (s, ws) = (m.size(), w.outer_size().ok()?);
            Some((
                m.position().x + s.width as i32 - ws.width as i32 - 40,
                m.position().y + s.height as i32 - ws.height as i32 - 80,
            ))
        });
        if let Some((x, y)) = pos {
            let _ = w.set_position(PhysicalPosition::new(x, y));
        }
        let _ = w.set_ignore_cursor_events(click_through(&st));
        // Kéo sinh hàng chục sự kiện/giây → chỉ lưu khi đứng yên 0.5 s.
        let (gen, handle) = (Arc::new(AtomicU64::new(0)), app.clone());
        let closer = app.clone();
        w.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                // Alt+F4 trên nhân vật: không để app chạy ẩn không cửa sổ (audit Phase 5 M2).
                api.prevent_close();
                let _ = closer
                    .state::<AppState>()
                    .db
                    .set_setting("avatar_mode", "focus");
                apply_mode(&closer, "focus");
                let _ = closer.emit_to("main", "avatar-mode", "focus");
            }
            if let WindowEvent::Moved(p) = e {
                let (n, h, (x, y)) = (
                    gen.fetch_add(1, Ordering::SeqCst) + 1,
                    handle.clone(),
                    (p.x, p.y),
                );
                let gen = gen.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if gen.load(Ordering::SeqCst) == n {
                        let _ = h
                            .state::<AppState>()
                            .db
                            .set_setting("avatar_pos", &format!("{x},{y}"));
                    }
                });
            }
        });
    }
    // Đóng khung chat = thoát app (cửa sổ avatar không giữ app sống ngầm).
    if let Some(main) = app.get_webview_window("main") {
        let h = app.clone();
        main.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { .. } = e {
                h.exit(0);
            }
        });
    }
    apply_mode(app, &mode(&st));
}

#[tauri::command]
pub fn set_avatar_mode(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    if !MODES.contains(&mode.as_str()) {
        return Err("chế độ không hợp lệ".into());
    }
    if mode == "companion" && click_through(&state) {
        // Xuyên chuột + chỉ nhân vật = không còn chỗ bấm để mở lại chat.
        set_avatar_click_through(app.clone(), state.clone(), false)?;
    }
    state.db.set_setting("avatar_mode", &mode).map_err(err)?;
    apply_mode(&app, &mode);
    Ok(())
}

#[tauri::command]
pub fn set_avatar_click_through(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    if enabled && mode(&state) == "companion" {
        return Err(
            "Chế độ \"Chỉ Mori\" cần bấm được vào nhân vật — chọn \"Mori + chat\" trước".into(),
        );
    }
    state
        .db
        .set_setting("avatar_click_through", if enabled { "1" } else { "0" })
        .map_err(err)?;
    if let Some(w) = app.get_webview_window(LABEL) {
        w.set_ignore_cursor_events(enabled).map_err(err)?;
    }
    Ok(())
}

/// Bắt đầu kéo cửa sổ avatar (gọi khi nhấn chuột trên nhân vật).
#[tauri::command]
pub fn avatar_drag(window: WebviewWindow) -> Result<(), String> {
    if window.label() != LABEL {
        return Err("chỉ dùng cho cửa sổ avatar".into());
    }
    window.start_dragging().map_err(err)
}

/// F2 trong cửa sổ avatar → chuyển cho cửa sổ chat (nơi có luồng nói/gửi), kể cả khi nó đang ẩn.
#[tauri::command]
pub fn avatar_ptt(app: AppHandle, down: bool) {
    let _ = app.emit_to("main", "avatar-ptt", down);
}

/// FPS / lỗi hiển thị từ cửa sổ avatar (đo gate Phase 5). Chỉ số và lỗi kỹ thuật, không có nội dung hội thoại.
#[tauri::command]
pub fn avatar_report(fps: Option<f32>, active: Option<bool>, error: Option<String>) {
    if let Some(fps) = fps {
        info!(avatar_fps = format!("{fps:.1}"), active, "avatar");
    }
    if let Some(e) = error {
        warn!(error = ?e.chars().take(300).collect::<String>(), "avatar lỗi"); // ?: escape \n
    }
}

/// Bản dev: lưu một khung hình của nhân vật (chỉ canvas WebGL, không chụp màn hình) để kiểm tra hiển thị.
/// `name` (chữ thường/số/`_`, ≤ 24 ký tự) → `avatar_snapshot_<name>.png` để chụp từng biểu cảm/hoạt cảnh (Phase 7).
#[tauri::command]
pub fn avatar_snapshot(png: String, name: Option<String>) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err(crate::DEV_ONLY.into());
    }
    let file = match name {
        None => "avatar_snapshot.png".to_string(),
        Some(n)
            if (1..=24).contains(&n.len())
                && n.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') =>
        {
            format!("avatar_snapshot_{n}.png")
        }
        Some(_) => return Err("tên ảnh không hợp lệ".into()),
    };
    let b64 = png
        .strip_prefix("data:image/png;base64,")
        .filter(|b| b.len() <= 16_000_000)
        .ok_or("ảnh không hợp lệ")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(err)?;
    let path = data_dir().join("logs").join(file);
    std::fs::write(&path, bytes).map_err(err)?;
    info!("đã lưu ảnh avatar");
    Ok(())
}
