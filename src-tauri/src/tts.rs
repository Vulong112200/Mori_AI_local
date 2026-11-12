//! TTS worker (ADR-0005, SPEC §13): Python + ZeroTTS int8, JSON Lines qua stdin/stdout.
//!
//! - Lazy: worker chỉ khởi động ở lệnh đầu tiên (speak/warm) — ~0.8 GB RAM.
//! - Mỗi lượt hội thoại có `turn`; ngắt lời = tăng turn + hủy mọi request đang chờ → audio của lượt cũ bị bỏ,
//!   không bao giờ phát lẫn vào lượt mới.
//! - Script nhúng trong binary, chạy `python -I` (bỏ PYTHON*, user site); stderr → null (traceback có thể chứa
//!   văn bản hội thoại); Job Object như llama-server. Chết → khởi động lại, backoff, tối đa 5 lần liên tiếp.
use base64::Engine;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::mpsc;
use tracing::{info, warn};

const SCRIPT: &str = include_str!("../workers/tts_worker.py");
const READY_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_RESTARTS: u32 = 5;
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Kiểm tra output worker (SPEC §13.2): tần số hợp lý, chunk ≤ 4 s PCM16 48 kHz.
const RATES: std::ops::RangeInclusive<u32> = 8_000..=192_000;
const MAX_CHUNK_BYTES: usize = 4 * 48_000 * 2;
/// SPEC §13.2: ≤ 2.000 ký tự / request.
const MAX_TEXT: usize = 2000;

/// Nhận audio của lượt hiện tại: (turn, request id, seq, PCM mono, sample rate).
pub type Sink = Arc<dyn Fn(u64, &str, u32, Vec<f32>, u32) + Send + Sync>;
/// Trạng thái worker cho UI: "loading" | "ready" | "error".
pub type OnStatus = Arc<dyn Fn(&'static str) + Send + Sync>;

enum Cmd {
    Warm,
    Speak {
        turn: u64,
        text: String,
        voice: String,
    },
    CancelAll,
}

pub struct Tts {
    tx: mpsc::UnboundedSender<Cmd>,
    turn: Arc<AtomicU64>,
}

struct Pending {
    turn: u64,
    sent: Instant,
    /// Câu đầu của lượt → đo "first audio".
    first: bool,
    next_seq: u32,
}

impl Tts {
    pub fn new(
        python: PathBuf,
        model: PathBuf,
        hf_home: PathBuf,
        sink: Sink,
        on_status: OnStatus,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let turn = Arc::new(AtomicU64::new(0));
        let worker = Worker {
            python,
            model,
            hf_home,
            sink,
            on_status,
            turn: turn.clone(),
            #[cfg(windows)]
            job: crate::llm::job::Job::new(),
        };
        tauri::async_runtime::spawn(worker.run(rx));
        Self { tx, turn }
    }

    /// Bắt đầu lượt mới (cũng là ngắt lời): audio của mọi lượt trước bị bỏ.
    pub fn new_turn(&self) -> u64 {
        let t = self.turn.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.tx.send(Cmd::CancelAll);
        t
    }

    pub fn speak(&self, turn: u64, text: &str, voice: &str) {
        let text: String = text.chars().take(MAX_TEXT).collect();
        let _ = self.tx.send(Cmd::Speak {
            turn,
            text,
            voice: voice.into(),
        });
    }

    /// Nạp worker sẵn (khi bật "Mori nói").
    pub fn warm(&self) {
        let _ = self.tx.send(Cmd::Warm);
    }
}

struct Worker {
    python: PathBuf,
    model: PathBuf,
    hf_home: PathBuf,
    sink: Sink,
    on_status: OnStatus,
    turn: Arc<AtomicU64>,
    #[cfg(windows)]
    job: Option<crate::llm::job::Job>,
}

impl Worker {
    async fn run(self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        // Lazy: chờ lệnh thật (speak/warm) mới khởi động — CancelAll gửi ở mọi lượt chat, kể cả khi tắt "Mori nói".
        let first = loop {
            match rx.recv().await {
                Some(Cmd::CancelAll) => continue,
                Some(c) => break c,
                None => return,
            }
        };
        let mut first = Some(first);
        let mut failures = 0u32;
        loop {
            (self.on_status)("loading");
            let mut reached_ready = false;
            match self
                .session(&mut rx, first.take(), &mut reached_ready)
                .await
            {
                Ok(()) => return, // app thoát (kênh đóng)
                Err(e) => warn!(error = %e, "TTS worker dừng"),
            }
            if reached_ready {
                failures = 0; // chỉ đếm lỗi liên tiếp
            }
            failures += 1;
            if failures > MAX_RESTARTS {
                (self.on_status)("error");
                warn!("TTS worker lỗi {MAX_RESTARTS} lần liên tiếp — tắt giọng nói");
                return;
            }
            tokio::time::sleep(Duration::from_secs(1 << (failures - 1).min(4))).await;
        }
    }

    /// Một vòng đời worker. `Ok` = app thoát; `Err` = worker chết/lỗi → khởi động lại.
    async fn session(
        &self,
        rx: &mut mpsc::UnboundedReceiver<Cmd>,
        mut first: Option<Cmd>,
        reached_ready: &mut bool,
    ) -> Result<(), String> {
        let t0 = Instant::now();
        let mut child = self
            .spawn()
            .map_err(|e| format!("không chạy được Python TTS: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("stdin")?;
        let mut lines = BufReader::new(child.stdout.take().ok_or("stdout")?).lines();
        let ready = tokio::time::timeout(READY_TIMEOUT, async {
            while let Ok(Some(l)) = lines.next_line().await {
                if serde_json::from_str::<serde_json::Value>(&l).is_ok_and(|v| v["type"] == "ready")
                {
                    return true;
                }
            }
            false
        })
        .await;
        if ready != Ok(true) {
            return Err("worker không sẵn sàng".into());
        }
        info!(
            load_ms = t0.elapsed().as_millis() as u64,
            "TTS worker sẵn sàng"
        );
        (self.on_status)("ready");
        *reached_ready = true;

        let mut pending: HashMap<String, Pending> = HashMap::new();
        let mut started_turn = 0u64; // lượt đã gửi câu đầu
        let mut n = 0u64;
        loop {
            let cmd = match first.take() {
                Some(c) => Some(c),
                None => tokio::select! {
                    c = rx.recv() => c,
                    line = lines.next_line() => match line {
                        Ok(Some(l)) => { self.on_line(&l, &mut pending); continue; }
                        _ => return Err("worker thoát".into()),
                    },
                    // SPEC §13.2: đang chờ audio mà worker im lặng quá lâu → coi như treo, khởi động lại.
                    _ = tokio::time::sleep(STALL_TIMEOUT), if !pending.is_empty() => {
                        return Err("worker không phản hồi".into());
                    }
                },
            };
            match cmd {
                None => {
                    let _ = send(&mut stdin, serde_json::json!({"v": 1, "type": "shutdown"})).await;
                    return Ok(());
                }
                Some(Cmd::Warm) => {}
                Some(Cmd::CancelAll) => {
                    for id in pending.keys() {
                        send(
                            &mut stdin,
                            serde_json::json!({"v": 1, "id": id, "type": "cancel"}),
                        )
                        .await?;
                    }
                    pending.clear();
                }
                Some(Cmd::Speak { turn, text, voice }) => {
                    if turn != self.turn.load(Ordering::SeqCst) {
                        continue; // lượt đã bị ngắt
                    }
                    n += 1;
                    let id = format!("{turn}-{n}");
                    send(
                        &mut stdin,
                        serde_json::json!({"v": 1, "id": id, "type": "synthesize", "payload": {"text": text, "voice": voice}}),
                    )
                    .await?;
                    let first = started_turn != turn;
                    started_turn = turn;
                    pending.insert(
                        id,
                        Pending {
                            turn,
                            sent: Instant::now(),
                            first,
                            next_seq: 0,
                        },
                    );
                }
            }
        }
    }

    fn on_line(&self, line: &str, pending: &mut HashMap<String, Pending>) {
        let Ok(m) = serde_json::from_str::<serde_json::Value>(line) else {
            return;
        };
        let Some(id) = m["id"].as_str() else {
            return;
        };
        match m["type"].as_str() {
            Some("audio") => {
                let Some(p) = pending.get_mut(id) else {
                    return; // request đã hủy
                };
                let seq = m["seq"].as_u64().unwrap_or(u64::MAX) as u32;
                if seq != p.next_seq {
                    warn!(id, seq, expected = p.next_seq, "TTS chunk lệch thứ tự");
                }
                p.next_seq = seq.wrapping_add(1);
                if seq == 0 && p.first {
                    info!(
                        tts_first_ms = p.sent.elapsed().as_millis() as u64,
                        "TTS câu đầu có tiếng"
                    );
                }
                if p.turn != self.turn.load(Ordering::SeqCst) {
                    return;
                }
                let rate = m["sample_rate"].as_u64().unwrap_or(0).min(u32::MAX as u64) as u32;
                let b64 = m["pcm_b64"].as_str().unwrap_or_default();
                let bytes = (RATES.contains(&rate) && b64.len() <= MAX_CHUNK_BYTES * 4 / 3 + 4)
                    .then(|| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                    .flatten();
                let Some(bytes) = bytes else {
                    warn!(id, rate, "TTS audio không hợp lệ");
                    return;
                };
                let pcm = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| i16::from_le_bytes(*b) as f32 / 32768.0)
                    .collect();
                (self.sink)(p.turn, id, seq, pcm, rate);
            }
            Some("done") => {
                pending.remove(id);
            }
            Some("error") => {
                warn!(id, kind = m["message"].as_str().unwrap_or("?"), "TTS lỗi");
                pending.remove(id);
            }
            _ => {}
        }
    }

    fn spawn(&self) -> std::io::Result<tokio::process::Child> {
        let mut cmd = Command::new(&self.python);
        cmd.args(["-I", "-X", "utf8", "-c", SCRIPT])
            .arg(&self.model)
            .arg("4") // ADR-0005: intra_op 4 luồng
            .env("HF_HOME", &self.hf_home)
            .env("HF_HUB_DISABLE_SYMLINKS", "1")
            .env("HF_HUB_OFFLINE", "1")
            // Chỉ dùng giọng trong data root (mặc định ZeroTTS còn đọc thư mục .zerotts của user).
            .env("ZEROTTS_VOICES_HOME", self.model.join("voices"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        info!("spawn TTS worker");
        let child = cmd.spawn()?;
        #[cfg(windows)]
        match (&self.job, child.raw_handle()) {
            (Some(job), Some(h)) if job.assign(h) => {}
            _ => warn!("không gắn được TTS worker vào Job Object"),
        }
        Ok(child)
    }
}

async fn send(stdin: &mut ChildStdin, msg: serde_json::Value) -> Result<(), String> {
    let mut line = msg.to_string();
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Gate Phase 3: 50 lượt, mỗi lượt 3 câu, ngắt lời ở thời điểm ngẫu nhiên.
    /// Kiểm: không có chunk của lượt cũ, mỗi request nhận seq liên tục 0,1,2… (không lặp/thiếu),
    /// lượt cuối (không bị ngắt) đủ cả 3 câu. In P95 "first audio".
    /// `cargo test tts_gate -- --ignored --nocapture`
    #[test]
    #[ignore = "cần Python TTS venv + ZeroTTS int8 ở E:/Mori Data"]
    fn tts_gate_50_turns_with_random_interrupts() {
        let data = PathBuf::from(r"E:\Mori Data");
        type Log = Arc<Mutex<Vec<(u64, String, u32, Instant)>>>;
        let log: Log = Arc::default();
        let current = Arc::new(AtomicU64::new(0));
        let (l, c) = (log.clone(), current.clone());
        let sink: Sink = Arc::new(move |turn, id, seq, _pcm, _rate| {
            assert_eq!(turn, c.load(Ordering::SeqCst), "audio của lượt cũ lọt ra");
            l.lock()
                .unwrap()
                .push((turn, id.to_string(), seq, Instant::now()));
        });
        tauri::async_runtime::block_on(async {
            let tts = Tts::new(
                data.join(r"runtime\python\tts-venv\Scripts\python.exe"),
                data.join(r"models\tts\zerotts-int8"),
                data.join(r"models\tts\hf"),
                sink,
                Arc::new(|_| {}),
            );
            tts.warm();
            tokio::time::sleep(Duration::from_secs(30)).await; // nạp model, không tính vào first-audio
            let sentences = [
                "Chào bạn, mình là Mori.",
                "Hôm nay bạn thế nào?",
                "Mình đang nghe nè.",
            ];
            let mut rng = 12345u64;
            let mut starts = Vec::new();
            for i in 0..50 {
                let t = tts.new_turn();
                current.store(t, Ordering::SeqCst);
                starts.push((t, Instant::now()));
                for s in sentences {
                    tts.speak(t, s, "maichi");
                }
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let wait = if i == 49 { 15_000 } else { (rng >> 33) % 2_500 };
                tokio::time::sleep(Duration::from_millis(wait)).await;
            }
            let log = log.lock().unwrap();
            let mut by_id: HashMap<&str, Vec<u32>> = HashMap::new();
            for (_, id, seq, _) in log.iter() {
                by_id.entry(id).or_default().push(*seq);
            }
            for (id, seqs) in &by_id {
                let want: Vec<u32> = (0..seqs.len() as u32).collect();
                assert_eq!(seqs, &want, "request {id}: seq lặp/thiếu");
            }
            let last = starts.last().unwrap().0;
            let last_ids = by_id
                .keys()
                .filter(|id| id.starts_with(&format!("{last}-")))
                .count();
            assert_eq!(last_ids, 3, "lượt cuối phải đọc đủ 3 câu");
            let mut first: Vec<u128> = starts
                .iter()
                .filter_map(|(t, at)| {
                    log.iter()
                        .find(|e| e.0 == *t)
                        .map(|e| (e.3 - *at).as_millis())
                })
                .collect();
            first.sort();
            let p95 = first[(first.len() - 1) * 95 / 100];
            println!(
                "lượt có tiếng: {}/50, first audio TB {} ms, P95 {p95} ms, max {} ms",
                first.len(),
                first.iter().sum::<u128>() / first.len() as u128,
                first.last().unwrap()
            );
            println!("request có audio: {}, chunk: {}", by_id.len(), log.len());
            assert!(p95 < 1_500, "first audio P95 {p95} ms ≥ 1.5 s");
        });
    }
}
