//! Phát giọng Mori qua loa (SPEC §7.3) bằng cpal output (đã có cho micro — không thêm rodio).
//! Hàng đợi mẫu mono ở tần số thiết bị; callback lấy dần, nhân ra mọi kênh, đo RMS cho avatar (Phase 5).
//! Stream mở lần đầu cần phát và giữ suốt phiên (sống trên thread riêng vì Stream không phải lúc nào cũng Send).
use crate::audio::resample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tracing::{info, warn};

#[derive(Default)]
struct Shared {
    /// (lượt đang phát, mẫu chờ phát). Kiểm lượt dưới cùng khóa với hàng đợi → chunk của lượt cũ không lọt
    /// vào sau khi ngắt lời dù worker đã decode xong trước đó.
    queue: Mutex<(u64, VecDeque<f32>)>,
    /// RMS của khối vừa phát (bit của f32).
    level: AtomicU32,
}

pub struct Player {
    shared: Arc<Shared>,
    /// Tần số thiết bị; `None` = chưa mở, `Some(0)` = mở lỗi (thử lại ở lượt sau).
    rate: Mutex<Option<u32>>,
    /// Lượt đã log "phát câu đầu" (đo e2e "thả F2 → nghe tiếng" từ log).
    logged_turn: AtomicU64,
}

impl Player {
    pub fn new() -> Self {
        Self {
            shared: Arc::default(),
            rate: Mutex::new(None),
            logged_turn: AtomicU64::new(0),
        }
    }

    /// Thêm PCM mono (tần số `from`) của lượt `turn` vào cuối hàng đợi phát.
    pub fn push(&self, turn: u64, pcm: &[f32], from: u32) {
        let rate = self.ensure_open();
        if rate == 0 {
            return;
        }
        let pcm = resample(pcm, from, rate);
        let mut q = self.shared.queue.lock().unwrap();
        if turn == q.0 {
            q.1.extend(pcm);
            if self.logged_turn.swap(turn, Ordering::Relaxed) != turn {
                info!(turn, "phát câu đầu");
            }
        }
    }

    /// Ngắt lời / lượt mới: bỏ mọi thứ chưa phát, chỉ nhận audio của `turn` từ giờ.
    pub fn stop(&self, turn: u64) {
        let mut q = self.shared.queue.lock().unwrap();
        q.0 = turn;
        q.1.clear();
        self.shared.level.store(0, Ordering::Relaxed);
        let mut rate = self.rate.lock().unwrap();
        if *rate == Some(0) {
            *rate = None; // loa lỗi lần trước → thử mở lại ở lượt này
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.shared.queue.lock().unwrap().1.is_empty()
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.shared.level.load(Ordering::Relaxed))
    }

    fn ensure_open(&self) -> u32 {
        let mut rate = self.rate.lock().unwrap();
        if rate.is_none() {
            let (tx, rx) = mpsc::channel();
            let shared = self.shared.clone();
            std::thread::spawn(move || match open(shared) {
                Ok((stream, r)) => {
                    let _ = tx.send(r);
                    // ponytail: giữ stream tới khi app thoát; đổi thiết bị loa giữa chừng → mở lại ở Phase 6.
                    let _stream = stream;
                    loop {
                        std::thread::park(); // park có thể thức dậy giả → lặp lại, không drop stream
                    }
                }
                Err(e) => {
                    warn!(error = %e, "không mở được loa");
                    let _ = tx.send(0);
                }
            });
            *rate = Some(rx.recv().unwrap_or(0));
            info!(rate = ?*rate, "mở loa");
        }
        rate.unwrap_or(0)
    }
}

fn open(shared: Arc<Shared>) -> Result<(cpal::Stream, u32), String> {
    let err = |e: cpal::Error| e.to_string();
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("không có thiết bị phát")?;
    let config = device.default_output_config().map_err(err)?;
    let (rate, channels) = (config.sample_rate(), config.channels() as usize);
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config.into(), channels, shared),
        SampleFormat::I16 => build::<i16>(&device, config.into(), channels, shared),
        SampleFormat::I32 => build::<i32>(&device, config.into(), channels, shared),
        SampleFormat::U16 => build::<u16>(&device, config.into(), channels, shared),
        f => return Err(format!("loa dùng định dạng chưa hỗ trợ: {f}")),
    }
    .map_err(err)?;
    stream.play().map_err(err)?;
    Ok((stream, rate))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, cpal::Error> {
    device.build_output_stream(
        config,
        move |data: &mut [T], _: &_| {
            let mut q = shared.queue.lock().unwrap();
            let (mut sum, mut n) = (0f32, 0usize);
            for frame in data.chunks_mut(channels) {
                let s = q.1.pop_front().unwrap_or(0.0);
                sum += s * s;
                n += 1;
                frame.fill(T::from_sample(s));
            }
            let rms = if n > 0 { (sum / n as f32).sqrt() } else { 0.0 };
            shared.level.store(rms.to_bits(), Ordering::Relaxed);
        },
        |e| warn!(error = %e, "lỗi stream loa"),
        None,
    )
}
