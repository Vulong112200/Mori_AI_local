//! Thu micro cho push-to-talk (SPEC §6.2–6.3): PCM nằm trong RAM, không ghi đĩa, bị drop ngay sau nhận dạng.
//! Stream cpal sống trên thread riêng (không phải mọi backend đều Send) và bị drop ngay khi Stop.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SizedSample};
use serde::Serialize;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::warn;

/// Whisper cần 16 kHz mono.
pub const RATE: u32 = 16_000;
/// Một lượt nói tối đa 2 phút: quên thả phím cũng không làm buffer phình mãi.
const MAX_SECONDS: usize = 120;
/// Driver treo khi mở thiết bị (Bluetooth/USB rút giữa chừng) → không giữ khóa mãi.
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
pub struct Mic {
    pub id: String,
    pub name: String,
}

pub fn list() -> Vec<Mic> {
    let Ok(devices) = cpal::default_host().input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            Some(Mic {
                id: d.id().ok()?.to_string(),
                name: d.description().ok()?.name().to_string(),
            })
        })
        .collect()
}

type Pcm = Result<(Vec<f32>, u32), String>;
type Buf = Arc<Mutex<Vec<f32>>>;

struct Take {
    stop: mpsc::Sender<()>,
    thread: JoinHandle<Pcm>,
}

#[derive(Default)]
pub struct Recorder(Mutex<Option<Take>>);

impl Recorder {
    /// Mở micro (`None` = mặc định). Trả lỗi ngay nếu không mở được (thiếu quyền micro, thiết bị rút ra…).
    pub fn start(&self, device: Option<String>) -> Result<(), String> {
        let mut slot = self.0.lock().unwrap();
        if slot.is_some() {
            return Err("micro đang thu".into());
        }
        let (stop, stop_rx) = mpsc::channel();
        let (ready, ready_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || capture(device, stop_rx, ready));
        match ready_rx.recv_timeout(OPEN_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let _ = thread.join();
                return Err(e);
            }
            // Không join thread đang kẹt: drop `stop` → khi driver trả về, thread tự đóng stream và thoát.
            Err(mpsc::RecvTimeoutError::Timeout) => return Err("micro không phản hồi".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                return Err("luồng micro dừng bất ngờ".into());
            }
        }
        *slot = Some(Take { stop, thread });
        Ok(())
    }

    /// Đang thu hoặc đang mở micro (khóa đang bận = `start` đang mở thiết bị).
    pub fn is_recording(&self) -> bool {
        self.0.try_lock().map_or(true, |g| g.is_some())
    }

    /// Tắt micro, trả PCM mono 16 kHz. Gọi khi không thu cũng không sao (trả lỗi).
    pub fn stop(&self) -> Result<Vec<f32>, String> {
        let take = self.0.lock().unwrap().take().ok_or("micro chưa bật")?;
        let _ = take.stop.send(());
        let (pcm, rate) = take.thread.join().map_err(|_| "luồng micro bị lỗi")??;
        Ok(resample(&pcm, rate, RATE))
    }
}

fn capture(
    device: Option<String>,
    stop: mpsc::Receiver<()>,
    ready: mpsc::Sender<Result<(), String>>,
) -> Pcm {
    let (stream, buf, rate) = match open(device.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    // Driver trả toàn số 0 trong ~150 ms đầu (đo trên máy đích) → chỉ báo "sẵn sàng" khi có tín hiệu thật,
    // để UI không mời người dùng nói lúc micro chưa thu được (mất chữ đầu câu).
    let t = std::time::Instant::now();
    while t.elapsed() < Duration::from_secs(1) && buf.lock().unwrap().iter().all(|&s| s == 0.0) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = ready.send(Ok(()));
    // Stop, Recorder bị drop, hoặc hết hạn: micro không bao giờ mở quá MAX_SECONDS dù UI mất trạng thái.
    let _ = stop.recv_timeout(Duration::from_secs(MAX_SECONDS as u64 + 5));
    drop(stream); // micro tắt tại đây
    let mut pcm = std::mem::take(&mut *buf.lock().unwrap());
    let lead = pcm.iter().take_while(|&&s| s == 0.0).count();
    pcm.drain(..lead);
    Ok((pcm, rate))
}

fn open(device: Option<&str>) -> Result<(cpal::Stream, Buf, u32), String> {
    let err = |e: cpal::Error| format!("lỗi micro: {e}");
    let host = cpal::default_host();
    let device = match device {
        Some(id) => id.parse().ok().and_then(|id| host.device_by_id(&id)),
        None => host.default_input_device(),
    }
    .ok_or("không tìm thấy micro — kiểm tra thiết bị và quyền micro trong Windows")?;
    let config = device.default_input_config().map_err(err)?;
    let (rate, channels) = (config.sample_rate(), config.channels() as usize);
    let buf = Arc::new(Mutex::new(Vec::with_capacity(rate as usize * 10)));
    let max = rate as usize * MAX_SECONDS;
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config.into(), channels, buf.clone(), max),
        SampleFormat::I16 => build::<i16>(&device, config.into(), channels, buf.clone(), max),
        SampleFormat::I32 => build::<i32>(&device, config.into(), channels, buf.clone(), max),
        SampleFormat::U16 => build::<u16>(&device, config.into(), channels, buf.clone(), max),
        f => return Err(format!("micro dùng định dạng chưa hỗ trợ: {f}")),
    }
    .map_err(err)?;
    stream.play().map_err(err)?;
    Ok((stream, buf, rate))
}

fn build<T: SizedSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    buf: Buf,
    max: usize,
) -> Result<cpal::Stream, cpal::Error>
where
    f32: cpal::FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            let mut b = buf.lock().unwrap();
            for frame in data.chunks(channels) {
                if b.len() >= max {
                    return;
                }
                let sum: f32 = frame.iter().map(|s| s.to_sample::<f32>()).sum();
                b.push(sum / channels as f32);
            }
        },
        |e| warn!(error = %e, "lỗi stream micro"),
        None,
    )
}

/// Lọc trung bình trượt (độ rộng = tỉ lệ giảm mẫu) rồi nội suy tuyến tính.
/// ponytail: bộ lọc hộp chặn alias thô (~-12 dB ở 20 kHz khi 48→16 kHz); cần sạch hơn → FIR windowed-sinc.
pub fn resample(pcm: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || pcm.is_empty() {
        return pcm.to_vec();
    }
    let w = (from as f64 / to as f64).ceil() as usize;
    let mut acc = 0.0;
    let pcm: Vec<f32> = (0..pcm.len())
        .map(|i| {
            acc += pcm[i];
            if i >= w {
                acc -= pcm[i - w];
            }
            acc / (i + 1).min(w) as f32
        })
        .collect();
    let n = (pcm.len() as u64 * to as u64 / from as u64) as usize;
    let step = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * step;
            let j = x as usize;
            let a = pcm[j];
            let b = *pcm.get(j + 1).unwrap_or(&a);
            a + (b - a) * (x - j as f64) as f32
        })
        .collect()
}

pub fn rms(pcm: &[f32]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt()
}

/// WAV PCM 16-bit mono 16 kHz trong RAM (gửi whisper-server, không ghi đĩa).
pub fn wav(pcm: &[f32]) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes()); // độ dài khối fmt
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 2).to_le_bytes()); // byte/giây
    w.extend_from_slice(&2u16.to_le_bytes()); // byte/khung
    w.extend_from_slice(&16u16.to_le_bytes()); // bit/mẫu
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        w.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_48k_to_16k_keeps_duration_and_shape() {
        let pcm: Vec<f32> = (0..48_000).map(|i| i as f32 / 48_000.0).collect();
        let out = resample(&pcm, 48_000, RATE);
        assert_eq!(out.len(), 16_000);
        assert!((out[8_000] - 0.5).abs() < 1e-3); // giữa đoạn ramp vẫn là 0.5
        assert_eq!(resample(&pcm[..10], RATE, RATE).len(), 10);
    }

    #[test]
    fn resample_attenuates_above_nyquist() {
        let tone = |hz: f32| -> Vec<f32> {
            (0..48_000)
                .map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / 48_000.0).sin())
                .collect()
        };
        let speech = rms(&resample(&tone(1_000.0), 48_000, RATE));
        let alias = rms(&resample(&tone(20_000.0), 48_000, RATE));
        assert!(speech > 0.65, "1 kHz phải gần như nguyên vẹn: {speech}");
        assert!(alias < speech * 0.3, "20 kHz phải bị chặn: {alias}");
    }

    #[test]
    fn wav_header_and_samples() {
        let w = wav(&[0.0, 1.0, -2.0]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), RATE);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 6);
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), 32767);
        assert_eq!(i16::from_le_bytes([w[48], w[49]]), -32767); // clamp
    }

    #[test]
    fn rms_of_constant() {
        assert_eq!(rms(&[]), 0.0);
        assert!((rms(&[0.5, -0.5]) - 0.5).abs() < 1e-6);
    }
}
