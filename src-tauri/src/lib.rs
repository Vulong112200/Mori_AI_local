mod audio;
mod db;
mod llm;
mod player;
mod prompt;
mod samples;
mod speech;
mod stt;
mod tts;

use db::{Db, Message};
use llm::{Llm, Status};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{ipc::Channel, Emitter, Manager, State};
use tokio::sync::Notify;
use tracing::{info, warn};

/// ADR-0008: 8B mặc định, 4B cho "chế độ nhanh".
const DEFAULT_MODEL: &str = "Qwen3-8B-Q4_K_M.gguf";
const FAST_MODEL: &str = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
const LLAMA_BUILD: &str = "b12284-vulkan";
/// ADR-0007 (runtime) + ADR-0011 (model): whisper.cpp tự build Vulkan.
const WHISPER_BUILD: &str = "v1.9.5-vulkan";
/// ADR-0011: large-v3 đầy đủ (WER giọng thật 20.6 % so với turbo 28.8 %).
const STT_MODEL: &str = "ggml-large-v3-q5_0.bin";
/// ADR-0005: ZeroTTS int8 trong venv Python 3.12.
const TTS_MODEL_DIR: &str = "zerotts-int8";
const DEFAULT_VOICE: &str = "maichi";
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
    recorder: Arc<audio::Recorder>,
    stt: Arc<stt::Stt>,
    tts: tts::Tts,
    player: Arc<player::Player>,
    tts_voices_dir: PathBuf,
}

impl AppState {
    /// Ngắt lời (SPEC §7.3): dừng sinh + bỏ mọi câu chưa đọc + dừng phát.
    fn interrupt(&self) {
        if let Some(n) = self.generation.lock().unwrap().as_ref() {
            n.notify_one();
        }
        let turn = self.tts.new_turn();
        self.player.stop(turn);
    }

    fn voice_out(&self) -> bool {
        self.db.setting("voice_out").ok().flatten().as_deref() != Some("0") // mặc định bật
    }

    fn tts_voice(&self) -> String {
        self.db
            .setting("tts_voice")
            .ok()
            .flatten()
            .filter(|v| self.tts_voices().contains(v))
            .unwrap_or_else(|| DEFAULT_VOICE.into())
    }

    /// Giọng có sẵn = thư mục con của `voices/` (whitelist cho setting, không nhận tên tùy ý).
    fn tts_voices(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.tts_voices_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        v.sort();
        v
    }
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
    /// id micro đã chọn; `None` = mặc định của Windows.
    mic: Option<String>,
    voice_out: bool,
    tts_voice: String,
    tts_voices: Vec<String>,
}

#[tauri::command]
async fn init(state: State<'_, AppState>) -> Result<InitState, String> {
    // Trang vừa (tải lại) → UI không biết micro nào đang mở: tắt mọi lượt thu còn sót.
    // Ngoài main thread: stop có thể chờ một lượt start đang mở thiết bị (tới 5 s).
    let rec = state.recorder.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || rec.stop()).await;
    let conversation_id = match state.db.latest_conversation().map_err(err)? {
        Some(id) => id,
        None => state.db.create_conversation().map_err(err)?,
    };
    Ok(InitState {
        messages: state.db.messages(&conversation_id).map_err(err)?,
        conversation_id,
        status: state.llm.status(),
        fast_mode: state.db.setting("fast_mode").map_err(err)?.as_deref() == Some("1"),
        mic: mic_setting(&state)?,
        voice_out: state.voice_out(),
        tts_voice: state.tts_voice(),
        tts_voices: state.tts_voices(),
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

fn mic_setting(state: &AppState) -> Result<Option<String>, String> {
    Ok(state
        .db
        .setting("mic")
        .map_err(err)?
        .filter(|s| !s.is_empty()))
}

#[tauri::command]
async fn list_mics() -> Vec<audio::Mic> {
    tauri::async_runtime::spawn_blocking(audio::list)
        .await
        .unwrap_or_default()
}

#[tauri::command]
fn set_mic(state: State<'_, AppState>, id: Option<String>) -> Result<(), String> {
    let id = id.unwrap_or_default();
    if id.len() > 1024 {
        return Err("id micro không hợp lệ".into());
    }
    state.db.set_setting("mic", &id).map_err(err)
}

/// Push-to-talk bắt đầu: bật micro, đồng thời nạp whisper-server trong lúc người dùng nói.
#[tauri::command]
async fn start_listening(state: State<'_, AppState>) -> Result<(), String> {
    state.interrupt(); // nói chen vào khi Mori đang nói/sinh = ngắt lời
    let (rec, mic, t) = (state.recorder.clone(), mic_setting(&state)?, Instant::now());
    tauri::async_runtime::spawn_blocking(move || rec.start(mic))
        .await
        .map_err(err)??;
    info!(mic_ready_ms = t.elapsed().as_millis() as u64, "micro bật");
    let stt = state.stt.clone();
    tauri::async_runtime::spawn(async move { stt.warm().await });
    Ok(())
}

/// Thả phím: tắt micro → nhận dạng → trả transcript cho UI sửa trước khi gửi. Audio bị drop ở đây.
#[tauri::command]
async fn stop_listening(state: State<'_, AppState>) -> Result<String, String> {
    let t = Instant::now();
    let rec = state.recorder.clone();
    let pcm = tauri::async_runtime::spawn_blocking(move || rec.stop())
        .await
        .map_err(err)??;
    let secs = pcm.len() as f32 / audio::RATE as f32;
    info!(
        mic_stop_ms = t.elapsed().as_millis() as u64,
        secs, "micro tắt"
    );
    // ponytail: ngưỡng năng lượng cố định chặn Whisper "bịa" chữ khi im lặng; micro quá nhỏ bị bỏ sót → hạ ngưỡng.
    if secs < 0.3 || audio::rms(&pcm) < 0.003 {
        return Ok(String::new());
    }
    let wav = audio::wav(&pcm);
    drop(pcm);
    let t = Instant::now();
    let text = state.stt.transcribe(wav).await?;
    info!(
        stt_ms = t.elapsed().as_millis() as u64,
        chars = text.chars().count(),
        "transcript"
    );
    Ok(text)
}

#[tauri::command]
async fn cancel_listening(state: State<'_, AppState>) -> Result<(), String> {
    let rec = state.recorder.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || rec.stop()).await;
    Ok(())
}

const DEV_ONLY: &str = "chỉ có ở bản dev";

/// Bộ câu thu mẫu cho gate Phase 2 (chỉ bản dev).
#[tauri::command]
fn list_samples() -> Result<Vec<samples::Sample>, String> {
    if !cfg!(debug_assertions) {
        return Err(DEV_ONLY.into());
    }
    Ok(samples::list())
}

#[derive(Serialize)]
struct SavedSample {
    secs: f32,
    /// Mori nghe được gì — để người dùng thấy câu đọc lệch/nghe sai mà thu lại.
    heard: String,
}

/// Tắt micro và lưu WAV vào tests/fixtures/audio/<file> (chỉ bản dev, `file` phải có trong manifest).
#[tauri::command]
async fn save_sample(state: State<'_, AppState>, file: String) -> Result<SavedSample, String> {
    // Tắt micro trước mọi kiểm tra → micro luôn đóng, kể cả khi bị gọi nhầm ở bản release.
    let rec = state.recorder.clone();
    let pcm = tauri::async_runtime::spawn_blocking(move || rec.stop())
        .await
        .map_err(err)??;
    if !cfg!(debug_assertions) {
        return Err(DEV_ONLY.into());
    }
    let path = samples::path_for(&file).ok_or("file không có trong manifest")?;
    let secs = pcm.len() as f32 / audio::RATE as f32;
    if secs < 0.3 || audio::rms(&pcm) < 0.003 {
        return Err("Mình chưa nghe thấy gì — đọc lại câu này nhé.".into());
    }
    std::fs::create_dir_all(samples::dir()).map_err(err)?;
    let wav = audio::wav(&pcm);
    std::fs::write(&path, &wav).map_err(err)?;
    info!(file, secs, "lưu mẫu giọng");
    let heard = state
        .stt
        .transcribe(wav)
        .await
        .unwrap_or_else(|e| format!("({e})"));
    Ok(SavedSample { secs, heard })
}

/// Nút Dừng / Ngắt: dừng sinh và dừng nói.
#[tauri::command]
fn stop_generation(state: State<'_, AppState>) {
    state.interrupt();
}

#[tauri::command]
fn set_voice_out(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .db
        .set_setting("voice_out", if enabled { "1" } else { "0" })
        .map_err(err)?;
    if enabled {
        state.tts.warm();
    } else {
        state.interrupt();
    }
    Ok(())
}

#[tauri::command]
fn set_tts_voice(state: State<'_, AppState>, voice: String) -> Result<(), String> {
    if !state.tts_voices().contains(&voice) {
        return Err("giọng không có".into());
    }
    state.db.set_setting("tts_voice", &voice).map_err(err)
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
    let turn = state.tts.new_turn();
    state.player.stop(turn);
    let (voice_out, voice) = (state.voice_out(), state.tts_voice());
    let mut splitter = speech::Splitter::default();
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
                if voice_out {
                    for s in splitter.push(d) {
                        state.tts.speak(turn, &s, &voice);
                    }
                }
            },
        )
        .await;
    let (reply, stopped) = result?;
    if voice_out && !stopped {
        for s in splitter.finish() {
            state.tts.speak(turn, &s, &voice);
        }
    }

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

/// ADR-0009: dump lỗi của WER có thể chứa hội thoại trong RAM → loại trừ theo tên exe (HKCU, không cần admin).
#[cfg(windows)]
fn exclude_from_wer() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::ErrorReporting::WerAddExcludedApplication;
    let me = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_owned()));
    for name in me
        .iter()
        .map(|n| n.as_os_str())
        .chain(["llama-server.exe", "whisper-server.exe"].map(std::ffi::OsStr::new))
    {
        let wide: Vec<u16> = name.encode_wide().chain([0]).collect();
        // SAFETY: `wide` là chuỗi UTF-16 kết thúc bằng 0, sống hết lời gọi.
        let hr = unsafe { WerAddExcludedApplication(wide.as_ptr(), 0) };
        if hr < 0 {
            warn!(exe = ?name, hr, "không loại trừ được khỏi WER");
        }
    }
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
    #[cfg(windows)]
    exclude_from_wer();

    tauri::Builder::default()
        .setup(move |app| {
            // WebView2 có Crashpad riêng (không tắt được qua cờ trình duyệt, đã thử): xóa dump cũ lúc khởi động.
            // ponytail: chỉ xóa dump local; việc upload phụ thuộc cài đặt diagnostic data của Windows (ADR-0009).
            if let Ok(dir) = app.path().app_local_data_dir() {
                let crashpad = dir.join("EBWebView").join("Crashpad");
                for sub in ["reports", "attachments"] {
                    let _ = std::fs::remove_dir_all(crashpad.join(sub));
                }
            }
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
            let stt = Arc::new(stt::Stt::new(
                data.join("runtime")
                    .join("whisper.cpp")
                    .join(WHISPER_BUILD)
                    .join("whisper-server.exe"),
                data.join("models").join("stt").join(STT_MODEL),
            ));
            let reaper = stt.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    reaper.reap_idle().await;
                }
            });
            let player = Arc::new(player::Player::new());
            let (sink_player, status_app) = (player.clone(), app.handle().clone());
            let tts_dir = data.join("models").join("tts");
            let tts = tts::Tts::new(
                data.join("runtime")
                    .join("python")
                    .join("tts-venv")
                    .join("Scripts")
                    .join("python.exe"),
                tts_dir.join(TTS_MODEL_DIR),
                tts_dir.join("hf"),
                Arc::new(move |turn, _id, _seq, pcm, rate| sink_player.push(turn, &pcm, rate)),
                Arc::new(move |s| {
                    let _ = status_app.emit("tts-status", s);
                }),
            );
            // Mức âm lượng ~30 Hz khi đang nói (lip-sync Phase 5) + báo bắt đầu/hết nói cho UI.
            let (level_player, level_app) = (player.clone(), app.handle().clone());
            tauri::async_runtime::spawn(async move {
                let mut was = false;
                loop {
                    tokio::time::sleep(Duration::from_millis(33)).await;
                    let now = level_player.is_playing();
                    if now {
                        let _ = level_app.emit("tts-level", level_player.level());
                    }
                    if now != was {
                        let _ = level_app.emit("tts-playing", now);
                        was = now;
                    }
                }
            });
            let (supervisor, handle) = (llm.clone(), app.handle().clone());
            tauri::async_runtime::spawn(async move { supervisor.supervise(handle).await });
            app.manage(AppState {
                db,
                llm,
                system: prompt::system_prompt(&personality(&data)),
                generation: Mutex::new(None),
                recorder: Arc::default(),
                stt,
                tts,
                player,
                tts_voices_dir: tts_dir.join(TTS_MODEL_DIR).join("voices"),
            });
            let state = app.state::<AppState>();
            if state.voice_out() {
                state.tts.warm(); // nạp giọng sẵn để câu trả lời đầu không phải chờ
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init,
            new_conversation,
            set_fast_mode,
            stop_generation,
            send_message,
            list_mics,
            set_mic,
            start_listening,
            stop_listening,
            cancel_listening,
            list_samples,
            save_sample,
            set_voice_out,
            set_tts_voice
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
