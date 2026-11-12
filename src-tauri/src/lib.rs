mod db;
mod llm;
mod prompt;

use db::{Db, Message};
use llm::{Llm, Status};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{ipc::Channel, Manager, State};
use tokio::sync::Notify;
use tracing::info;

/// ADR-0008: 8B mặc định, 4B cho "chế độ nhanh".
const DEFAULT_MODEL: &str = "Qwen3-8B-Q4_K_M.gguf";
const FAST_MODEL: &str = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
const LLAMA_BUILD: &str = "b12284-vulkan";
/// Giới hạn đầu vào ở ranh giới tin cậy (UI → Rust).
const MAX_INPUT_CHARS: usize = 8_000;
/// personality.md quá dài sẽ chiếm hết context.
const MAX_PERSONALITY_CHARS: usize = 8_000;

struct AppState {
    db: Db,
    llm: Arc<Llm>,
    system: String,
    /// Notify của lượt sinh đang chạy (để nút Dừng).
    generation: Mutex<Option<Arc<Notify>>>,
}

/// Data root (ADR-0002). Ghi đè bằng biến môi trường MORI_DATA_DIR.
fn data_dir() -> PathBuf {
    std::env::var_os("MORI_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"E:\Mori Data"))
}

/// personality.md: bản người dùng sửa ở `<data>/settings/` nếu có, không thì bản mặc định trong repo.
fn personality(data: &std::path::Path) -> String {
    std::fs::read_to_string(data.join("settings").join("personality.md"))
        .unwrap_or_else(|_| include_str!("../../config/personality.md").to_string())
        .chars()
        .take(MAX_PERSONALITY_CHARS)
        .collect()
}

fn model_for(fast: bool) -> &'static str {
    if fast {
        FAST_MODEL
    } else {
        DEFAULT_MODEL
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct InitState {
    conversation_id: String,
    messages: Vec<Message>,
    status: Status,
    fast_mode: bool,
}

#[tauri::command]
fn init(state: State<'_, AppState>) -> Result<InitState, String> {
    let conversation_id = match state.db.latest_conversation().map_err(err)? {
        Some(id) => id,
        None => state.db.create_conversation().map_err(err)?,
    };
    Ok(InitState {
        messages: state.db.messages(&conversation_id).map_err(err)?,
        conversation_id,
        status: state.llm.status(),
        fast_mode: state.db.setting("fast_mode").map_err(err)?.as_deref() == Some("1"),
    })
}

#[tauri::command]
fn new_conversation(state: State<'_, AppState>) -> Result<String, String> {
    state.db.create_conversation().map_err(err)
}

#[tauri::command]
fn set_fast_mode(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .db
        .set_setting("fast_mode", if enabled { "1" } else { "0" })
        .map_err(err)?;
    state.llm.set_model(model_for(enabled));
    Ok(())
}

#[tauri::command]
fn stop_generation(state: State<'_, AppState>) {
    if let Some(n) = state.generation.lock().unwrap().as_ref() {
        n.notify_one();
    }
}

/// Giữ "slot" sinh câu trả lời; tự trả khi ra khỏi scope (kể cả khi lỗi giữa chừng).
struct GenerationSlot<'a> {
    slot: &'a Mutex<Option<Arc<Notify>>>,
    stop: Arc<Notify>,
}

impl<'a> GenerationSlot<'a> {
    /// Chỉ một lượt sinh tại một thời điểm (UI chặn, nhưng IPC là ranh giới tin cậy).
    fn acquire(slot: &'a Mutex<Option<Arc<Notify>>>) -> Result<Self, String> {
        let mut g = slot.lock().unwrap();
        if g.is_some() {
            return Err("Mori đang trả lời, đợi chút nha".into());
        }
        let stop = Arc::new(Notify::new());
        *g = Some(stop.clone());
        Ok(Self { slot, stop })
    }
}

impl Drop for GenerationSlot<'_> {
    fn drop(&mut self) {
        let mut g = self.slot.lock().unwrap();
        if g.as_ref().is_some_and(|n| Arc::ptr_eq(n, &self.stop)) {
            g.take();
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ChatEvent {
    Delta {
        text: String,
    },
    Done {
        message: Option<Message>,
        stopped: bool,
    },
}

/// Lưu tin nhắn người dùng, stream câu trả lời qua `on_event`, lưu câu trả lời (kể cả khi bị dừng giữa chừng).
/// Trả về tin nhắn người dùng đã lưu.
#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    text: String,
    on_event: Channel<ChatEvent>,
) -> Result<Message, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Tin nhắn trống".into());
    }
    if text.chars().count() > MAX_INPUT_CHARS {
        return Err(format!("Tin nhắn quá dài (tối đa {MAX_INPUT_CHARS} ký tự)"));
    }
    if !matches!(state.llm.status(), Status::Ready { .. }) {
        return Err("Mori chưa sẵn sàng, đợi chút nha".into()); // không lưu tin nhắn khi không trả lời được
    }
    let generation = GenerationSlot::acquire(&state.generation)?;
    let user = state
        .db
        .add_message(&conversation_id, "user", text)
        .map_err(err)?;
    let history = state.db.messages(&conversation_id).map_err(err)?;
    info!(conversation = %conversation_id, chars = text.chars().count(), "send_message"); // không log nội dung

    let result = state
        .llm
        .chat(
            &prompt::build(&state.system, &history),
            &generation.stop,
            |d| {
                let _ = on_event.send(ChatEvent::Delta {
                    text: d.to_string(),
                });
            },
        )
        .await;
    let (reply, stopped) = result?;

    let reply = reply.trim();
    let message = if reply.is_empty() {
        None
    } else {
        Some(
            state
                .db
                .add_message(&conversation_id, "assistant", reply)
                .map_err(err)?,
        )
    };
    info!(conversation = %conversation_id, reply_chars = reply.chars().count(), stopped, "reply");
    let _ = on_event.send(ChatEvent::Done { message, stopped });
    Ok(user)
}

pub fn run() {
    let data = data_dir();
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("mori")
        .filename_suffix("log")
        .max_log_files(14)
        .build(data.join("logs"))
        .expect("không tạo được thư mục log");
    let (writer, _log_guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .init();
    info!(data = %data.display(), "Mori khởi động");

    tauri::Builder::default()
        .setup(move |app| {
            let db = Db::open(&data.join("database").join("mori.db"))?;
            let fast = db.setting("fast_mode")?.as_deref() == Some("1");
            let llm = Arc::new(Llm::new(
                data.join("runtime")
                    .join("llama.cpp")
                    .join(LLAMA_BUILD)
                    .join("llama-server.exe"),
                data.join("models").join("llm"),
                model_for(fast),
            ));
            let (supervisor, handle) = (llm.clone(), app.handle().clone());
            tauri::async_runtime::spawn(async move { supervisor.supervise(handle).await });
            app.manage(AppState {
                db,
                llm,
                system: prompt::system_prompt(&personality(&data)),
                generation: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init,
            new_conversation,
            set_fast_mode,
            stop_generation,
            send_message
        ])
        .build(tauri::generate_context!())
        .expect("không khởi tạo được Tauri")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<AppState>().llm.shutdown();
                info!("Mori thoát");
            }
        });
}
