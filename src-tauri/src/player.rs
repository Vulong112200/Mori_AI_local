//! Phát giọng Mori qua loa (SPEC §7.3) bằng cpal output (đã có cho micro — không thêm rodio).
//! Hàng đợi mẫu mono ở tần số thiết bị; callback lấy dần, nhân ra mọi kênh, đo RMS cho avatar (Phase 5).
//! Stream mở lần đầu cần phát và giữ tới khi thiết bị lỗi (rút tai nghe) hoặc thiết bị mặc định của Windows đổi
//! (cắm tai nghe) → mở lại ở lượt kế tiếp (Phase 6). Stream sống trên thread riêng (không phải lúc nào cũng Send).
use crate::audio::resample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tracing::{info, warn};

#[derive(Default)]
struct Shared {
    /// (lượt đang phát, mẫu chờ phát). Kiểm lượt dưới cùng khóa với hàng đợi → chunk của lượt cũ không lọt
    /// vào sau khi ngắt lời dù worker đã decode xong trước đó.
    queue: Mutex<(u64, VecDeque<f32>)>,
    /// RMS của khối vừa phát (bit của f32).
    level: AtomicU32,
    /// Stream báo lỗi (thiết bị mất) → mở lại ở lần phát sau.
    broken: AtomicBool,
}

pub struct Player {
    shared: Arc<Shared>,
    /// Tần số thiết bị; `None` = chưa mở, `Some(0)` = mở lỗi (thử lại ở lượt sau).
    rate: Mutex<Option<u32>>,
    /// Lượt đã log "phát câu đầu" (đo e2e "thả F2 → nghe tiếng" từ log).
    logged_turn: AtomicU64,
    /// (id thiết bị đang mở, kênh đóng stream: drop sender → thread giữ stream kết thúc).
    opened: Mutex<Option<(String, mpsc::Sender<()>)>>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            shared: Arc::default(),
            rate: Mutex::new(None),
            logged_turn: AtomicU64::new(0),
            opened: Mutex::new(None),
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
        let current = default_name(); // hỏi Windows trước khi khóa hàng đợi (luồng phát cũng cần khóa này)
        let mut q = self.shared.queue.lock().unwrap();
        q.0 = turn;
        q.1.clear();
        self.shared.level.store(0, Ordering::Relaxed);
        let mut rate = self.rate.lock().unwrap();
        if *rate == Some(0) {
            *rate = None; // loa lỗi lần trước → thử mở lại ở lượt này
        }
        // Thiết bị mặc định đổi (cắm/rút tai nghe, đổi trong Windows) → phát qua thiết bị mới từ lượt này.
        let mut opened = self.opened.lock().unwrap();
        if opened
            .as_ref()
            .is_some_and(|(name, _)| Some(name) != current.as_ref())
        {
            info!("thiết bị phát mặc định đổi → mở lại loa");
            *opened = None;
            *rate = None;
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
        if self.shared.broken.swap(false, Ordering::SeqCst) {
            warn!("loa lỗi (thiết bị mất?) → mở lại");
            *self.opened.lock().unwrap() = None; // đóng stream cũ
            *rate = None;
        }
        if rate.is_none() {
            let (tx, rx) = mpsc::channel();
            let (close_tx, close_rx) = mpsc::channel::<()>();
            let shared = self.shared.clone();
            std::thread::spawn(move || match open(shared) {
                Ok((stream, r, name)) => {
                    let _ = tx.send((r, name));
                    let _ = close_rx.recv(); // giữ stream tới khi sender bị drop (đóng/mở lại)
                    drop(stream);
                }
                Err(e) => {
                    warn!(error = %e, "không mở được loa");
                    let _ = tx.send((0, String::new()));
                }
            });
            let (r, name) = rx.recv().unwrap_or((0, String::new()));
            *rate = Some(r);
            *self.opened.lock().unwrap() = (r > 0).then_some((name, close_tx));
            info!(rate = ?*rate, "mở loa");
        }
        rate.unwrap_or(0)
    }
}

/// Id thiết bị phát mặc định hiện tại của Windows (`None` = không có loa).
fn default_name() -> Option<String> {
    Some(
        cpal::default_host()
            .default_output_device()?
            .id()
            .ok()?
            .to_string(),
    )
}

fn open(shared: Arc<Shared>) -> Result<(cpal::Stream, u32, String), String> {
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
    let name = device.id().map(|i| i.to_string()).unwrap_or_default();
    Ok((stream, rate, name))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, cpal::Error> {
    let on_error = shared.clone();
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
        move |e| {
            warn!(error = %e, "lỗi stream loa");
            on_error.broken.store(true, Ordering::SeqCst);
        },
        None,
    )
}
