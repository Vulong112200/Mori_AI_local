//! Quản lý tiến trình llama-server (ADR-0001, ADR-0008) và stream câu trả lời.
//!
//! - Chỉ bind 127.0.0.1, cổng ngẫu nhiên, API key ngẫu nhiên (truyền qua env, không lộ trên command line),
//!   tắt web UI và /slots, `--offline` → trang web/tiến trình khác không gọi được model, worker không ra mạng.
//! - Xóa mọi biến môi trường `LLAMA_*` trước khi spawn: llama.cpp đọc chúng cho hầu hết cờ
//!   (tools chạy shell, log file, tải model...) → không để môi trường bật những thứ đó (audit MEDIUM-1).
//! - Windows: tiến trình con nằm trong Job Object KILL_ON_JOB_CLOSE → Mori chết vì bất kỳ lý do gì,
//!   kernel cũng tắt llama-server (audit MEDIUM-3).
//! - Chết bất thường → tự khởi động lại, backoff 1-2-4-8-16 s; quá MAX_RESTARTS → Error.
//! - Sau khi sẵn sàng gửi một request warm-up (Vulkan biên dịch shader ở lượt đầu, ~4 s).
use crate::prompt::ChatMessage;
use futures_util::StreamExt;
use serde::Serialize;
use std::{path::PathBuf, process::Stdio, sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter};
use tokio::{
    process::Command,
    sync::{watch, Notify},
};
use tracing::{info, warn};

const MAX_RESTARTS: u32 = 5;
const READY_TIMEOUT: Duration = Duration::from_secs(180);
/// Chặn vòng lặp sinh vô hạn (~1.5 phút ở 12 tok/s).
const MAX_REPLY_TOKENS: u32 = 1024;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Status {
    Starting { model: String },
    Ready { model: String },
    Restarting { model: String, attempt: u32 },
    Error { message: String },
    Stopped,
}

pub struct Llm {
    exe: PathBuf,
    models_dir: PathBuf,
    /// Model muốn chạy; `None` = dừng hẳn.
    desired: watch::Sender<Option<String>>,
    status: watch::Sender<Status>,
    /// (cổng, API key) khi đang Ready.
    endpoint: Mutex<Option<(u16, String)>>,
    http: reqwest::Client,
    #[cfg(windows)]
    job: Option<job::Job>,
}

impl Llm {
    pub fn new(exe: PathBuf, models_dir: PathBuf, model: &str) -> Self {
        Self {
            exe,
            models_dir,
            desired: watch::Sender::new(Some(model.to_string())),
            status: watch::Sender::new(Status::Starting {
                model: display(model),
            }),
            endpoint: Mutex::new(None),
            // Không đi qua proxy hệ thống: mọi request chỉ tới loopback.
            http: reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(2))
                .build()
                .expect("reqwest client"),
            #[cfg(windows)]
            job: job::Job::new(),
        }
    }

    pub fn status(&self) -> Status {
        self.status.borrow().clone()
    }

    pub fn set_model(&self, model: &str) {
        self.desired.send_replace(Some(model.to_string()));
    }

    /// Dừng llama-server và chờ tối đa 3 s (gọi lúc thoát app, từ thread không phải async).
    pub fn shutdown(&self) {
        self.desired.send_replace(None);
        for _ in 0..30 {
            if *self.status.borrow() == Status::Stopped {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        warn!("llama-server chưa dừng sau 3 s");
    }

    fn set(&self, app: &AppHandle, s: Status) {
        info!(status = ?s, "llm");
        self.status.send_replace(s.clone());
        let _ = app.emit("llm-status", s);
    }

    pub async fn supervise(&self, app: AppHandle) {
        let mut desired = self.desired.subscribe();
        let mut failures = 0u32;
        loop {
            let Some(model) = desired.borrow_and_update().clone() else {
                self.set(&app, Status::Stopped);
                return;
            };
            let name = display(&model);
            if failures == 0 {
                self.set(
                    &app,
                    Status::Starting {
                        model: name.clone(),
                    },
                );
            }
            let key = random_key();
            let spawned = free_port().and_then(|port| Ok((port, self.spawn(&model, port, &key)?)));
            let (port, mut child) = match spawned {
                Ok(v) => v,
                Err(e) => {
                    self.set(
                        &app,
                        Status::Error {
                            message: format!("không chạy được llama-server: {e}"),
                        },
                    );
                    if desired.changed().await.is_err() {
                        return;
                    }
                    continue;
                }
            };

            // Chờ sẵn sàng; đổi model giữa chừng → giết và làm lại.
            let ready = tokio::select! {
                ok = self.wait_ready(port, &key) => Some(ok),
                _ = child.wait() => Some(false),
                _ = desired.changed() => None,
            };
            let switched = match ready {
                None => true,
                Some(true) => {
                    failures = 0;
                    *self.endpoint.lock().unwrap() = Some((port, key));
                    self.set(
                        &app,
                        Status::Ready {
                            model: name.clone(),
                        },
                    );
                    tokio::select! {
                        status = child.wait() => {
                            warn!(?status, "llama-server thoát bất thường");
                            false
                        }
                        _ = desired.changed() => true,
                    }
                }
                Some(false) => false,
            };
            *self.endpoint.lock().unwrap() = None;
            let _ = child.kill().await;
            if switched {
                continue; // đổi model / dừng: không tính là lỗi
            }

            failures += 1;
            if failures > MAX_RESTARTS {
                self.set(
                    &app,
                    Status::Error {
                        message: format!("llama-server lỗi {MAX_RESTARTS} lần liên tiếp"),
                    },
                );
                if desired.changed().await.is_err() {
                    return;
                }
                failures = 0;
                continue;
            }
            self.set(
                &app,
                Status::Restarting {
                    model: name,
                    attempt: failures,
                },
            );
            let backoff = Duration::from_secs(1 << (failures - 1).min(4));
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = desired.changed() => {}
            }
        }
    }

    fn spawn(&self, model: &str, port: u16, key: &str) -> std::io::Result<tokio::process::Child> {
        let mut cmd = Command::new(&self.exe);
        for (k, _) in std::env::vars_os() {
            if k.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("LLAMA_")
            {
                cmd.env_remove(&k);
            }
        }
        cmd.env("LLAMA_API_KEY", key) // không đặt trên command line (đọc được qua Win32_Process)
            .arg("-m")
            .arg(self.models_dir.join(model))
            .args(["--host", "127.0.0.1", "--port", &port.to_string()])
            .args(["-c", "8192", "-ngl", "99", "--jinja", "--reasoning", "off"])
            .args(["--no-webui", "--no-slots", "--offline"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null()) // log của llama-server có thể chứa nội dung prompt → không ghi
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        info!(model, port, "spawn llama-server");
        let child = cmd.spawn()?;
        #[cfg(windows)]
        match (&self.job, child.raw_handle()) {
            (Some(job), Some(handle)) if job.assign(handle) => {}
            _ => warn!("không gắn được llama-server vào Job Object — có thể mồ côi nếu Mori crash"),
        }
        Ok(child)
    }

    async fn wait_ready(&self, port: u16, key: &str) -> bool {
        let base = format!("http://127.0.0.1:{port}");
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
        loop {
            let ok = self
                .http
                .get(format!("{base}/health"))
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success());
            if ok {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let warmup = serde_json::json!({"messages": [{"role": "user", "content": "Xin chào"}], "max_tokens": 1});
        self.http
            .post(format!("{base}/v1/chat/completions"))
            .bearer_auth(key)
            .timeout(Duration::from_secs(60))
            .json(&warmup)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
    }

    /// Stream câu trả lời. Trả `(toàn văn, bị_dừng)`. `stop` được notify khi người dùng bấm Dừng.
    pub async fn chat(
        &self,
        messages: &[ChatMessage<'_>],
        stop: &Notify,
        mut on_delta: impl FnMut(&str),
    ) -> Result<(String, bool), String> {
        let (port, key) = self
            .endpoint
            .lock()
            .unwrap()
            .clone()
            .ok_or("Mori chưa sẵn sàng")?;
        let body = serde_json::json!({
            "messages": messages, "stream": true, "cache_prompt": true, "max_tokens": MAX_REPLY_TOKENS,
        });
        let request = self
            .http
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .bearer_auth(key)
            .json(&body)
            .send();
        let resp = tokio::select! {
            r = request => r.and_then(|r| r.error_for_status()).map_err(|e| format!("lỗi gọi model: {e}"))?,
            _ = stop.notified() => return Ok((String::new(), true)),
        };

        let mut stream = resp.bytes_stream();
        let (mut buf, mut text) = (Vec::new(), String::new());
        loop {
            let chunk = tokio::select! {
                c = stream.next() => c,
                _ = stop.notified() => return Ok((text, true)), // drop stream → llama-server dừng sinh
            };
            let Some(chunk) = chunk else {
                return Ok((text, false));
            };
            let chunk = match chunk {
                Ok(c) => c,
                // Đã có một phần câu trả lời → giữ lại như khi bấm Dừng, không bỏ phí.
                Err(e) if !text.is_empty() => {
                    warn!(error = %e, "mất kết nối model giữa chừng");
                    return Ok((text, true));
                }
                Err(e) => return Err(format!("mất kết nối model: {e}")),
            };
            buf.extend_from_slice(&chunk);
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                match parse_sse_line(&String::from_utf8_lossy(&line)) {
                    Some(Sse::Delta(d)) => {
                        text.push_str(&d);
                        on_delta(&d);
                    }
                    Some(Sse::Done) => return Ok((text, false)),
                    None => {}
                }
            }
        }
    }
}

#[cfg(windows)]
pub(crate) mod job {
    use windows_sys::Win32::{Foundation::HANDLE, System::JobObjects::*};

    /// Handle Job Object, giữ suốt đời tiến trình Mori. Khi Mori chết, OS đóng handle → giết mọi tiến trình trong job.
    pub struct Job(HANDLE);
    // HANDLE là con trỏ thô; Job Object an toàn khi dùng từ nhiều thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> Option<Self> {
            // SAFETY: gọi Win32 với tham số hợp lệ; struct limit được khởi tạo zero rồi đặt cờ.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                // DIE_ON_UNHANDLED_EXCEPTION: tiến trình con crash thì chết ngay, không gọi WER
                // (dump có thể chứa hội thoại) — áp cho mọi worker kể cả python.exe của TTS.
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
                let ok = SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of_val(&info) as u32,
                );
                (ok != 0).then_some(Job(h))
            }
        }

        pub fn assign(&self, process: std::os::windows::io::RawHandle) -> bool {
            // SAFETY: `process` là handle hợp lệ của tiến trình con vừa spawn.
            unsafe { AssignProcessToJobObject(self.0, process as HANDLE) != 0 }
        }
    }
}

#[derive(Debug, PartialEq)]
enum Sse {
    Delta(String),
    Done,
}

fn parse_sse_line(line: &str) -> Option<Sse> {
    let data = line.trim().strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(Sse::Done);
    }
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    let delta = v["choices"][0]["delta"]["content"].as_str()?;
    (!delta.is_empty()).then(|| Sse::Delta(delta.to_string()))
}

fn display(model: &str) -> String {
    model.trim_end_matches(".gguf").to_string()
}

pub(crate) fn free_port() -> std::io::Result<u16> {
    Ok(std::net::TcpListener::bind(("127.0.0.1", 0))?
        .local_addr()?
        .port())
}

/// 128 bit từ CSPRNG của OS.
pub(crate) fn random_key() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("OS RNG");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_parsing() {
        let line = r#"data: {"choices":[{"delta":{"content":"Chào"}}]}"#;
        assert_eq!(parse_sse_line(line), Some(Sse::Delta("Chào".into())));
        assert_eq!(parse_sse_line("data: [DONE]"), Some(Sse::Done));
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
        assert_eq!(
            parse_sse_line(r#"data: {"choices":[{"delta":{"content":""}}]}"#),
            None
        );
        assert_eq!(parse_sse_line(": keep-alive"), None);
        assert_eq!(parse_sse_line(""), None);
    }

    #[test]
    fn keys_and_ports_differ() {
        assert_ne!(random_key(), random_key());
        assert_eq!(random_key().len(), 32);
        assert!(free_port().unwrap() > 0);
    }

    #[cfg(windows)]
    #[test]
    fn job_object_created() {
        assert!(job::Job::new().is_some());
    }
}
