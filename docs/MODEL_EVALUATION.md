# Đánh giá model

> Mỗi lần đo dùng `/bench <llm|stt|tts>`. Thêm dòng mới, **không xóa** dòng cũ. Ghi build runtime, ngày, điều kiện (cắm sạc/pin).
> Chỉ ghi số đo trên máy đích. Số tác giả công bố ghi ở cột "Ghi chú" và đánh dấu "(công bố)".
> Điểm có dấu `*` là **chấm sơ bộ của Claude**, chờ người dùng chấm lại.

## 1. Danh sách ứng viên

Tất cả file đã kiểm sha256 sau khi tải. Vị trí: `E:\Mori Data\models\{llm,stt,tts}\`.

### LLM
| Model | Nguồn | License | Kích thước | sha256 | Trạng thái |
|---|---|---|---|---|---|
| Qwen3-4B Q4_K_M | huggingface.co/Qwen/Qwen3-4B-GGUF | Apache-2.0 | 2.33 GB | `7485fe6f…34fdf5` | Đã bench |
| Qwen3-4B-Instruct-2507 Q4_K_M | huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF (Qwen không có GGUF chính thức) | Apache-2.0 | 2.33 GB | `3605803b…c67e597` | Đã bench |
| Qwen3.5-4B Q4_K_M | huggingface.co/unsloth/Qwen3.5-4B-GGUF (không tải mmproj/vision) | Apache-2.0 | 2.55 GB | `00fe7986…69ef11a4` | Đã bench |
| Qwen3-8B Q4_K_M | huggingface.co/Qwen/Qwen3-8B-GGUF | Apache-2.0 | 4.68 GB | `d98cdcbd…2c5745785` | Đã bench |

### STT
| Model | Nguồn | License | sha256 | Trạng thái |
|---|---|---|---|---|
| Whisper small (ggml) | huggingface.co/ggerganov/whisper.cpp | MIT | `1be3a9b2…1fffea987b` | Đã bench (giọng tổng hợp) |
| Whisper large-v3-turbo q5_0 (ggml) | huggingface.co/ggerganov/whisper.cpp | MIT | `39422170…5a55ffa7e2` | Đã bench (giọng tổng hợp) |
| PhoWhisper (small/medium) | huggingface.co/vinai | Kiểm tra model card | | Chưa convert |

### TTS
Weights trong `E:\Mori Data\models\tts\hf` (cache HF, tải với `HF_HUB_DISABLE_SYMLINKS=1`). venv: `E:\Mori Data\runtime\python\tts-venv` (Python 3.12.16).

| Engine | Nguồn | License | Trạng thái |
|---|---|---|---|
| KorvaTTS 0.1.3 | github.com/dogenthq/KorvaTTS · PyPI `korvatts` | Apache-2.0 | Đã bench — **loại** (quá chậm, không streaming) |
| VieNeu-TTS v3 Turbo / Nano | github.com/pnnbao97/VieNeu-TTS · PyPI `vieneu` 3.8.4 · HF `pnnbao-ump/VieNeu-TTS-v3-{Turbo,Nano}` | Apache-2.0 (v3) | Đã bench |
| ZeroTTS 0.1.5 | github.com/zeroweight-ai/ZeroTTS · PyPI `zerotts` | MIT | Đã bench |

## 2. Kết quả LLM

TTFT đo qua `llama-server` với [system_prompt_sample.md](../tests/fixtures/system_prompt_sample.md) (~460 token), ctx 8192, sau 1 lượt khởi động. "Ấm" = prompt cache tái dùng system prompt, chỉ xử lý ~20 token mới.

| Ngày | Model | Backend | Build | pp512 (t/s) | tg128 (t/s) | TTFT lạnh (s) | TTFT ấm (s) | RAM đỉnh | Điểm VN (1–5) | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-11-11 | Qwen3-4B Q4_K_M | CPU (16 luồng) | b12284 | 108–133 (±20–28) | 7.8–16.2 | 4.38 | 0.24–0.28 | 5.3 GB | | Pin, Balanced. Dao động lớn giữa các lần chạy (nhiệt/điện năng). Chưa thử giảm số luồng |
| 2026-11-11 | Qwen3-4B Q4_K_M | Vulkan (Arc 140T, ngl 99) | b12284 | **887–889** (±4–13) | **23.1–23.3** (±0.2) | **0.60** | **0.23–0.26** | 4.1 GB | 3* | Pin, Balanced. Rất ổn định. Lượt đầu sau khi khởi động server mất ~4.8 s (biên dịch shader) → app phải warm-up lúc khởi động. **Lẫn chữ tiếng Nga** ("процесс") trong AI-05; xưng hô sai; bịa món ăn |
| 2026-11-12 | Qwen3-4B-Instruct-2507 Q4_K_M | Vulkan | b12284 | **983** (±11) | **23.7** (±0.03) | **0.55** | **0.22–0.28** | 4.1 GB | 3.5* | Pin, Balanced. Tiếng Việt trôi chảy, ấm áp nhất; AI-03/04/06 tốt. Lỗi: **dùng nhiều emoji** (phải lọc trước TTS), sai kiến thức (bún bò Huế = "miền Nam", "Tauri không viết backend"), nhầm xưng hô 1 lượt |
| 2026-11-12 | Qwen3.5-4B Q4_K_M | Vulkan | b12284 | 784 (±12) | 20.5 (±0.1) | 2.25 | **1.63–1.88** ❌ | 4.2 GB | 2.5* | Kiến trúc lai: TTFT ấm chậm dù chỉ ~22 token mới (kernel Vulkan chưa tối ưu?). Dài dòng (7–17 s/câu), sai nhiều (phở "nước cốt dừa", `mpmc::sync::channel`, "AI chạy trên trình duyệt") |
| 2026-11-12 | Qwen3-8B Q4_K_M | Vulkan | b12284 | 466 (±10) | 11.8 (±0.2) | 1.16 | 0.38–0.41 | 6.2 GB | **4*** | Đo khi bộ cài MSVC Build Tools đang chạy song song (có thể thấp hơn thực tế chút). Đúng kiến thức hơn rõ (bánh mì xíu mại; Tauri = Rust), không lẫn ngôn ngữ. Lỗi: AI-02 lượt 3 nói tên Mori thay vì tên người dùng; "thời thực"; vẫn có emoji. tg 11.8 ≈ 2× tốc độ cần cho giọng nói |

Câu trả lời đầy đủ: `scripts/phase0/out/llm_answers_<model>.md` (sinh bằng `scripts/phase0/run_llm_prompts.py`, không commit).

## 3. Kết quả STT

**Lưu ý:** bộ test hiện tại là **giọng tổng hợp** (18 câu từ VieNeu v3 Turbo, hạ mẫu 16 kHz) — chỉ để kiểm tra pipeline và so sánh tương đối. Chưa thay được bộ thu âm giọng thật (SPEC §19.2). whisper.cpp b5454 (bản dựng sẵn mới nhất, **chỉ CPU**), 8 luồng. Độ trễ không tính thời gian nạp model.

| Ngày | Model | Backend | Số câu | WER | CER | Độ trễ TB (s) | P95 (s) | RAM | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|
| 2026-11-11 | Whisper small | CPU | 18 (TTS) | 0.172 | 0.073 | 2.31 | 3.06 | | Mặc định |
| 2026-11-11 | Whisper large-v3-turbo q5_0 | CPU | 18 (TTS) | **0.084** | 0.042 | 10.27 ❌ | 10.94 | | Mặc định. Whisper luôn xử lý khung 30 s → độ trễ gần như cố định |
| 2026-11-12 | Whisper large-v3-turbo q5_0 | CPU | 18 (TTS) | 0.139 | 0.083 | 4.22 | 5.07 | | `-ac 768 -fa` (khung 15 s): nhanh 2.4× nhưng kém chính xác |
| 2026-11-12 | Whisper small | CPU | 18 (TTS) | 0.209 | 0.090 | **1.21** | 1.77 | | `-ac 768 -fa` |
| 2026-11-12 | Whisper large-v3-turbo q5_0 | **Vulkan** (v1.9.5 tự build) | 18 (TTS) | **0.078** | 0.039 | **1.53** | **1.77** | | Mặc định (khung 30 s). **Đạt mục tiêu < 2 s và chính xác nhất** |
| 2026-11-12 | Whisper small | Vulkan (v1.9.5) | 18 (TTS) | 0.169 | 0.071 | 1.12 | 1.73 | | Mặc định |

Phân tích lỗi (turbo, mặc định): phần lớn là khác **định dạng** ("14h30" vs "14 giờ 30", "32GB") hoặc TTS đọc chưa chuẩn ("PostgreSQL", "Pull request"). Lỗi thật: tên riêng ("Mori" → "Maury") → dùng `--prompt` từ vựng ở Phase 2. CER không dấu ≈ CER → rất ít lỗi chỉ do sai dấu thanh.

## 4. Kết quả TTS

18 câu trong [tts_sentences.txt](../tests/fixtures/tts_sentences.txt), chạy **offline** (`HF_HUB_OFFLINE=1`), CPU, sau 1 câu warm-up. Pin, Balanced. File wav: `scripts/phase0/out/tts/<engine>/` (không commit).

| Ngày | Engine | Giọng | First audio TB / P95 (s) | RTF | RAM | Tự nhiên | Phát âm | Nhịp | vi-en | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|---|
| 2026-11-11 | ZeroTTS 0.1.5 (fp32) | maichi | **0.52 / 0.66** | 1.87 ❌ | 1.37 GB | **Người dùng thích nhất** | | | | Streaming. RTF > 1 → phát sẽ bị ngắt quãng. Công bố RTF 0.5 (chưa đạt với fp32; có tool int8 trong repo GitHub, chưa thử) |
| 2026-11-12 | ZeroTTS 0.1.5 (fp32, **6 luồng**) | maichi | **0.32 / 0.50** | **1.05** ⚠️ | 1.37 GB | Người dùng thích | | | | Mặc định thư viện 4 luồng. Gần real-time nhưng vẫn > 1 |
| 2026-11-12 | ZeroTTS 0.1.5 (**int8**, 4 luồng) | maichi | **0.06 / 0.07** ✅ | **0.31** ✅ | 0.82 GB | **Đạt** — "hơi ngọng, giọng miền Nam vài chỗ nhưng rất dễ thương" | | | | int8 tạo bằng `scripts/phase0/quantize_zerotts_int8.py` (425 MatMulInteger ở local_frame_decode). Khớp số công bố (~70 ms). **Tốt nhất mọi chỉ số** |
| 2026-11-12 | ZeroTTS 0.1.5 (int8, 6 luồng) | maichi | 0.10 / 0.16 | 0.54 | 0.83 GB | | | | | Nhiều luồng hơn lại chậm hơn |
| 2026-11-12 | ZeroTTS 0.1.5 (fp32, 8 luồng) | maichi | 0.58 / 0.73 | 1.64 ❌ | 1.39 GB | | | | | Chậm hơn 6 luồng (nhân E/LP-E?) |
| 2026-11-11 | KorvaTTS 0.1.3 (32 bước) | khanh_vy | 30.7 / 46.8 ❌ | 6.09 ❌ | 0.69 GB | | | | | Không streaming |
| 2026-11-12 | KorvaTTS 0.1.3 (8 bước) | khanh_vy | 9.4 / 12.4 ❌ | 1.87 ❌ | 0.69 GB | | | | | Vẫn không dùng được → **loại** |
| 2026-11-11 | VieNeu v3 Turbo (fp32) | Hải Đăng | **0.58 / 0.75** | **0.89** | 1.35 GB | | | | | Streaming, 48 kHz. Đạt cả hai chỉ số nhưng RTF sát 1 |
| 2026-11-12 | VieNeu v3 Turbo (**int8**) | Hải Đăng | **0.39 / 0.50** | **0.74** | 0.85 GB | Đạt | | | | Tốt nhất về tốc độ. Người dùng nghe: **không méo** |
| 2026-11-11 | VieNeu v3 Nano | Ái Hân | 2.12 / 3.14 ⚠️ | 0.42 | 0.45 GB | | | | | 24 kHz, trả từng đoạn hoàn chỉnh → first audio cao |

Các cột Tự nhiên / Phát âm / Nhịp / vi-en: **người dùng nghe và chấm**.

## 5. Kết luận (sơ bộ, 2026-11-12)

- **LLM:** [ADR-0004](adr/0004-chon-llm.md) chấp nhận **Qwen3-4B-Instruct-2507 trên Vulkan** (nhanh nhất). **Qwen3-8B** chất lượng tốt hơn rõ (4* so với 3.5*), vẫn đạt TTFT (0.4 s ấm) nhưng sinh chậm hơn một nửa (11.8 tok/s) và tốn thêm ~2 GB RAM → chờ người dùng thử trực tiếp rồi quyết định có thay ADR-0004 không.
- **TTS:** người dùng **thích giọng ZeroTTS** hơn; VieNeu v3 Turbo int8 không méo và là phương án nhanh. **Chốt ZeroTTS int8 (4 luồng)**: first audio 0.06 s, RTF 0.31; người dùng nghe đạt. [ADR-0005](adr/0005-chon-tts.md) đã chấp nhận. Xem [ADR-0005](adr/0005-chon-tts.md).
- **STT:** đề xuất **Whisper large-v3-turbo q5_0 trên whisper.cpp Vulkan tự build** — WER 7.8 %, 1.53 / 1.77 s (giọng tổng hợp). Xem [ADR-0007](adr/0007-chon-stt.md). Còn phải kiểm chứng trên giọng thật.
- **Ước tính "thả PTT → nghe tiếng đầu tiên"** (cộng các số đo riêng, chưa đo nối liền): STT turbo Vulkan 1.5–1.8 s + LLM TTFT ấm + câu đầu tiên (~15 token) + TTS ZeroTTS int8 0.06 s:
  - Qwen3-4B-Instruct-2507: 0.25 + 0.65 s → **≈ 2.5–2.8 s** ✅
  - Qwen3-8B: 0.40 + 1.3 s → **≈ 3.3–3.6 s** ✅ (sát ngưỡng 4 s hơn)
