//! whisper-server (ADR-0007): lazy — chạy khi bắt đầu nghe, tắt sau `IDLE` không dùng (RAM chật với LLM 8B).
//!
//! Bảo mật (theo ADR-0009):
//! - Chỉ bind 127.0.0.1, cổng ngẫu nhiên. whisper-server **không có API key** → mọi route đặt sau
//!   `--request-path` ngẫu nhiên 128 bit: trang web/tiến trình khác không đoán được URL để gọi
//!   `/inference` hay `/load` (nạp model tùy ý).
//! - Không dùng `--convert` → audio đọc thẳng từ body request, không có file tạm.
//! - stdout/stderr → null (whisper-server in transcript ra đó). Job Object như llama-server.
use crate::llm::{free_port, random_key};
use std::{
    io::Write,
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Mutex};
use tracing::{info, warn};

pub const IDLE: Duration = Duration::from_secs(5 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(90);

struct Server {
    child: tokio::process::Child,
    base: String,
    last_used: Instant,
}

pub struct Stt {
    exe: PathBuf,
    model: PathBuf,
    server: Mutex<Option<Server>>,
    http: reqwest::Client,
    #[cfg(windows)]
    job: Option<crate::llm::job::Job>,
}

impl Stt {
    pub fn new(exe: PathBuf, model: PathBuf) -> Self {
        Self {
            exe,
            model,
            server: Mutex::new(None),
            http: reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(2))
                .build()
                .expect("reqwest client"),
            #[cfg(windows)]
            job: crate::llm::job::Job::new(),
        }
    }

    /// Bật sẵn trong lúc người dùng còn đang nói (nạp model ~vài giây).
    pub async fn warm(&self) {
        if let Err(e) = self.ensure().await {
            warn!(error = %e, "không khởi động được whisper-server");
        }
    }

    pub async fn transcribe(&self, wav: Vec<u8>) -> Result<String, String> {
        let base = self.ensure().await?;
        let boundary = format!("mori{}", random_key());
        let mut body = Vec::with_capacity(wav.len() + 512);
        let _ = write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        );
        body.extend_from_slice(&wav);
        drop(wav);
        for (k, v) in [("response_format", "json"), ("temperature", "0")] {
            let _ = write!(
                body,
                "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}"
            );
        }
        let _ = write!(body, "\r\n--{boundary}--\r\n");
        let resp = self
            .http
            .post(format!("{base}/inference"))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("lỗi nhận dạng giọng nói: {}", e.without_url()))?; // URL chứa path bí mật
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("whisper trả về không hợp lệ: {}", e.without_url()))?;
        if let Some(s) = self.server.lock().await.as_mut() {
            s.last_used = Instant::now();
        }
        let text = json["text"].as_str().unwrap_or_default().trim();
        Ok(if is_hallucination(text) {
            String::new()
        } else {
            fix_name(text)
        })
    }

    /// Gọi định kỳ: tắt whisper-server nếu nhàn rỗi quá `IDLE`.
    pub async fn reap_idle(&self) {
        let mut g = self.server.lock().await;
        if g.as_ref().is_some_and(|s| s.last_used.elapsed() > IDLE) {
            if let Some(mut s) = g.take() {
                let _ = s.child.kill().await;
                info!("tắt whisper-server (nhàn rỗi)");
            }
        }
    }

    /// Trả base URL (đã gồm request-path bí mật) của server đang sẵn sàng; chưa có thì khởi động.
    async fn ensure(&self) -> Result<String, String> {
        let mut g = self.server.lock().await;
        if let Some(s) = g.as_mut() {
            if matches!(s.child.try_wait(), Ok(None)) {
                s.last_used = Instant::now();
                return Ok(s.base.clone());
            }
            warn!("whisper-server đã thoát, khởi động lại");
        }
        *g = None;
        let port = free_port().map_err(|e| e.to_string())?;
        let path = format!("/{}", random_key());
        let mut child = self
            .spawn(port, &path)
            .map_err(|e| format!("không chạy được whisper-server: {e}"))?;
        let base = format!("http://127.0.0.1:{port}{path}");
        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            if !matches!(child.try_wait(), Ok(None)) {
                return Err("whisper-server thoát khi đang nạp model".into());
            }
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
            if Instant::now() > deadline {
                let _ = child.kill().await;
                return Err("whisper-server nạp model quá lâu".into());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        info!(port, "whisper-server sẵn sàng");
        *g = Some(Server {
            child,
            base: base.clone(),
            last_used: Instant::now(),
        });
        Ok(base)
    }

    fn spawn(&self, port: u16, path: &str) -> std::io::Result<tokio::process::Child> {
        let mut cmd = Command::new(&self.exe);
        cmd.arg("-m")
            .arg(&self.model)
            .args(["--host", "127.0.0.1", "--port", &port.to_string()])
            .args(["--request-path", path])
            // Không dùng --prompt: đo 100 câu giọng thật, prompt từ vựng làm WER 31 % → 41 % và Whisper bịa câu 10 lần.
            .args(["-l", "vi", "-t", "4"])
            // Beam search 5 = cấu hình whisper-cli đã đo WER ở Phase 0; mặc định của server là greedy (kém hơn).
            // `-nt -bo 5`: như whisper-cli lúc đo; server mặc định sinh timestamp + best_of 2 → bịa câu nhiều hơn.
            .args(["-bs", "5", "-bo", "5", "-nt"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(dir) = self.exe.parent() {
            cmd.current_dir(dir); // thư mục public mặc định (examples/server/public) không tồn tại ở đây
        }
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        info!(port, "spawn whisper-server");
        let child = cmd.spawn()?;
        #[cfg(windows)]
        match (&self.job, child.raw_handle()) {
            (Some(job), Some(handle)) if job.assign(handle) => {}
            _ => warn!("không gắn được whisper-server vào Job Object"),
        }
        Ok(child)
    }
}

/// ponytail: danh sách câu Whisper hay "bịa" khi đoạn âm thanh khó nghe (đo: 2/100 câu giọng thật).
/// Thêm mẫu khi gặp; câu thật chứa cụm này gần như không có trong hội thoại với Mori.
fn is_hallucination(text: &str) -> bool {
    let t = text.to_lowercase();
    ["ghiền mì gõ", "subscribe cho kênh", "đăng ký kênh"]
        .iter()
        .any(|p| t.contains(p))
}

/// Whisper hay chép tên "Mori" thành "Morrie", "Mô ri"… (người dùng gặp 2026-11-13) → đưa về "Mori".
/// Chỉ thay cả từ (không đụng "morning", "memory").
fn fix_name(text: &str) -> String {
    const VARIANTS: &[&str] = &[
        "morrie", "morie", "morri", "mory", "moori", "mô ri", "mo ri", "mơ ri", "mô-ri", "mo-ri",
        "mori",
    ];
    if text
        .chars()
        .any(|c| c.to_lowercase().map(char::len_utf8).sum::<usize>() != c.len_utf8())
    {
        return text.to_string(); // vị trí byte phải khớp 1:1 giữa bản gốc và bản chữ thường
    }
    let lower = text.to_lowercase();
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let (mut out, mut i) = (String::with_capacity(text.len()), 0);
    'next: while i < text.len() {
        if !word(text[..i].chars().next_back()) {
            for v in VARIANTS {
                let end = i + v.len();
                if lower.get(i..end) == Some(v)
                    && text.is_char_boundary(end)
                    && !word(text[end..].chars().next())
                {
                    out.push_str("Mori");
                    i = end;
                    continue 'next;
                }
            }
        }
        let c = text[i..].chars().next().unwrap_or_default();
        out.push(c);
        i += c.len_utf8().max(1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mori_name_variants_fixed() {
        assert_eq!(
            fix_name("Morrie ơi, hãy nhớ là mình thích trà sữa."),
            "Mori ơi, hãy nhớ là mình thích trà sữa."
        );
        assert_eq!(
            fix_name("Mô ri, chào nha. MORIE à?"),
            "Mori, chào nha. Mori à?"
        );
        assert_eq!(
            fix_name("good morning, memory, Moriarty"),
            "good morning, memory, Moriarty"
        );
    }

    #[test]
    fn known_hallucinations_dropped() {
        assert!(is_hallucination(
            "Hãy subscribe cho kênh Ghiền Mì Gõ Để không bỏ lỡ những video hấp dẫn"
        ));
        assert!(!is_hallucination("Bạn nghe được mình nói không?"));
    }

    /// Chạy thật whisper-server trên máy đích: `cargo test -- --ignored stt`.
    #[test]
    #[ignore = "cần whisper-server + model ở E:/Mori Data"]
    fn stt_real_server_transcribes_and_hides_routes() {
        let data = PathBuf::from(r"E:\Mori Data");
        let stt = Stt::new(
            data.join(r"runtime\whisper.cpp\v1.9.5-vulkan\whisper-server.exe"),
            data.join(r"models\stt").join(crate::STT_MODEL),
        );
        let wav = std::fs::read("../scripts/phase0/out/stt_synth/01.wav").expect("fixture Phase 0");
        tauri::async_runtime::block_on(async {
            let t = Instant::now();
            let text = stt.transcribe(wav.clone()).await.unwrap();
            println!("lần 1 (gồm nạp model): {:?} → {text}", t.elapsed());
            assert!(!text.is_empty());
            let t = Instant::now();
            let again = stt.transcribe(wav).await.unwrap();
            println!("lần 2: {:?}", t.elapsed());
            assert_eq!(text, again);
            // Không có request-path bí mật → không gọi được route nào.
            let base = stt.server.lock().await.as_ref().unwrap().base.clone();
            let root = base.rsplit_once('/').unwrap().0;
            let http = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            for route in ["/health", "/inference", "/load", "/"] {
                let r = http.get(format!("{root}{route}")).send().await.unwrap();
                assert_eq!(r.status(), 404, "GET {route}");
            }
            for route in ["/inference", "/load"] {
                // Body có độ dài: POST rỗng không có Content-Length thì httplib đợi body mãi.
                let r = http
                    .post(format!("{root}{route}"))
                    .body("{}")
                    .send()
                    .await
                    .unwrap();
                assert_eq!(r.status(), 404, "POST {route}");
            }
        });
    }

    /// Đo WER giọng thật qua đúng đường của app (gate Phase 2):
    /// `cargo test stt_bench -- --ignored --nocapture` rồi `python scripts/phase0/wer.py --csv scripts/phase0/out/stt_app_real.csv`.
    #[test]
    #[ignore = "cần các câu đã thu ở tests/fixtures/audio"]
    fn stt_bench_real_voice() {
        // So model: MORI_STT_MODEL=<file trong models\stt> (mặc định = model của app).
        let model = std::env::var("MORI_STT_MODEL").unwrap_or(crate::STT_MODEL.into());
        let data = PathBuf::from(r"E:\Mori Data");
        let stt = Stt::new(
            data.join(r"runtime\whisper.cpp\v1.9.5-vulkan\whisper-server.exe"),
            data.join(r"models\stt").join(&model),
        );
        let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let mut csv = String::from("file,category,ref,hyp,latency_s\n");
        tauri::async_runtime::block_on(async {
            stt.warm().await; // không tính thời gian nạp model
            for s in crate::samples::list().into_iter().filter(|s| s.recorded) {
                let wav = std::fs::read(crate::samples::dir().join(&s.file)).unwrap();
                let t = Instant::now();
                let hyp = stt.transcribe(wav).await.unwrap();
                let secs = t.elapsed().as_secs_f32();
                println!("{} {secs:.2}s | {hyp}", s.file);
                csv += &format!(
                    "{},{},{},{},{secs:.3}\n",
                    s.file,
                    s.category,
                    q(&s.text),
                    q(&hyp)
                );
            }
        });
        let name = model.trim_end_matches(".bin");
        std::fs::write(
            format!("../scripts/phase0/out/stt_app_real_{name}.csv"),
            csv,
        )
        .unwrap();
    }
}
