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

### 3.1. Giọng thật (100 câu người dùng thu trong app, 2026-11-12)

Thu bằng chế độ Thu mẫu (micro mặc định, cpal → 16 kHz nội suy tuyến tính). Âm lượng thấp (RMS 0.009–0.031, đỉnh 0.09–0.34), không clip; 140–190 ms đầu mỗi file là số 0 (micro khởi động). Đo qua đúng đường app (`cargo test stt_bench -- --ignored`), turbo q5_0, Vulkan, độ trễ không tính nạp model.

| Cấu hình whisper-server | WER | CER | Câu "bịa" | Độ trễ TB / P95 | đời thường | miền Nam | nhiễu/nhanh | số/dài | thuật ngữ |
|---|---|---|---|---|---|---|---|---|---|
| `-bs 5` + `--prompt` từ vựng | 0.409 | 0.288 | 10 | 1.14 / 1.41 s | 0.291 | 0.546 | 0.395 | 0.353 | 0.476 |
| `-bs 5`, không prompt | 0.408 | 0.291 | 11 | 1.28 / 1.67 s | 0.280 | 0.580 | 0.389 | 0.366 | 0.429 |
| **`-bs 5 -bo 5 -nt`, không prompt** (= whisper-cli) | **0.310** | 0.211 | 2 | **1.19 / 1.49 s** | **0.143** | 0.377 | 0.312 | 0.281 | 0.444 |

- `whisper-cli` mặc định, không prompt, cùng 100 file: WER 0.310 — trùng cấu hình cuối → app giờ khớp cấu hình đã đo.
- Bỏ 2 câu "bịa" → WER 0.286. Lỗi còn lại chủ yếu là **nhận nhầm âm** ("cất cánh" → "cách cánh", "bạn nữ" → "bệnh nữ"), không phải định dạng số → giới hạn của turbo với giọng/micro này.
- **Chưa đạt** ngưỡng đề xuất (ADR-0007: ≤ 15 %, nhóm ≤ 25 %). Chỉ nhóm đời thường gần đạt.

So model trên cùng 100 câu (`-bs 5 -bo 5 -nt`, không prompt; "số → chữ" = đổi số sang chữ cả hai phía, `wer.py`):

| Model (q5_0) | WER | WER số → chữ | đời thường | miền Nam | nhiễu/nhanh | số/dài | thuật ngữ | Độ trễ TB / P95 |
|---|---|---|---|---|---|---|---|---|
| large-v3-turbo | 0.310 | 0.288 | 0.143 | 0.372 | 0.312 | 0.228 | 0.427 | 1.19 / 1.49 s |
| **large-v3** | **0.230** | **0.206** | 0.148 | **0.271** | **0.255** | 0.114 | **0.323** | 1.77 / 2.41 s |
| large-v3 + chuẩn hóa âm lượng | 0.245 | 0.222 | 0.143 | 0.285 | 0.274 | 0.139 | 0.344 | 1.86 / 2.54 s |
| PhoWhisper-large | 0.344 | 0.248 | **0.088** | 0.314 | 0.306 | **0.106** | 0.547 | 2.13 / 3.15 s |

→ [ADR-0011](adr/0011-stt-large-v3.md): app dùng large-v3.

**Lần thu 2 (2026-11-13)** — người dùng thu lại cả 100 câu, đọc đúng chữ, sau khi sửa micro (chờ tín hiệu thật, lọc chống alias):

| Model (q5_0) | WER | WER số → chữ | CER | đời thường | miền Nam | nhiễu/nhanh | số/dài | thuật ngữ | Độ trễ TB / P95 |
|---|---|---|---|---|---|---|---|---|---|
| **large-v3** | **0.093** | **0.077** | 0.054 | 0.027 | 0.068 | 0.127 | 0.085 | 0.169 | 1.67 / 2.08 s |
| large-v3-turbo | 0.120 | 0.103 | 0.068 | 0.038 | 0.155 | 0.134 | 0.095 | 0.190 | 1.11 / 1.29 s |
| PhoWhisper-large | 0.202 | 0.113 | 0.145 | **0.022** | 0.116 | 0.115 | 0.310 (số → chữ 0.033) | 0.365 | 2.04 / 3.19 s |

→ **Đạt ngưỡng gate Phase 2** (≤ 15 %, nhóm ≤ 25 %) trên WER thô. Lỗi còn lại: từ tiếng Anh ("Unit test fail … parse JSON" → "Unitec failed a book by Jason"), tên "Mori" ("Mòri", "Emory"), vài âm ("cất cánh" → "kích cánh"). Lần 1 → lần 2 giảm mạnh cho thấy phần lớn lỗi lần 1 do cách thu (mất đầu câu, đọc lệch chữ), không phải model. 1/100 câu bị "bịa" — bộ lọc trong `stt.rs` bắt được. PhoWhisper giỏi tiếng Việt thuần, kém từ tiếng Anh (viết "cao gót lipi" cho "cargo clippy").

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

## 5. End-to-end (đo thật)

`scripts/phase0/e2e_latency.py`: wav → whisper-cli Vulkan (turbo q5_0, không tính nạp model) → llama-server router (Qwen3-8B, stream tới hết câu đầu) → ZeroTTS int8 chunk đầu. 6 câu giọng tổng hợp, sau warm-up. Pin, Balanced.

| Ngày | LLM | STT | TTFT | Tới hết câu đầu | TTS chunk đầu | **Tổng TB / max** | Mạng ngoài loopback |
|---|---|---|---|---|---|---|---|
| 2026-11-12 | Qwen3-8B | 1.13–1.42 s | 0.39–0.48 s | 0.66–2.25 s | 0.07–0.10 s | **2.63 / 3.59 s** ✅ | Không có (theo dõi mỗi 250 ms) |

Thời gian "tới hết câu đầu" phụ thuộc độ dài câu đầu model sinh ra → Phase 3 nên cắt cả ở dấu phẩy nếu câu đầu dài.

### 5.1. Trong app (gate Phase 3, 2026-11-13)

**Trong app thật, trước khi sửa** (log `micro tắt` → `phát câu đầu`, hội thoại 40 tin): 7.6–8 s (12/11) → nguyên nhân: cửa sổ lịch sử trượt mỗi lượt → xử lý lại ~1 300 token (5.3–5.9 s); sau khi ngắt lời request làm ấm rơi sang slot khác (TTFT 2.6–3.1 s); câu đầu dài 71–123 ký tự (1.8–2.6 s); lượt đầu sau khi mở app cache lạnh (TTFT 3.3 s).

Sửa: cửa sổ nhảy khối 12 tin, làm ấm prompt sau mỗi lượt + khi model sẵn sàng, llama-server `-np 1`, câu đầu cắt ở dấu phẩy (≥ 30 ký tự) / tối đa 50 ký tự. `--cache-reuse 256` đã thử: KV shift 2–3 s → không dùng.

**Sau khi sửa** — `cargo test e2e_gate -- --ignored --nocapture`: đúng các thành phần của app (whisper-server large-v3 Vulkan → llama-server Qwen3-8B với bản sao lịch sử thật 40+ tin → cắt câu → ZeroTTS int8 chunk đầu), 20 câu giọng thật người dùng thu (mỗi nhóm 4 câu), nghỉ 2 s giữa các lượt. Không tính micro tắt (log 35–85 ms) và loa (phát ngay khi có chunk).

| Ngày | STT | e2e TB | e2e P95 (n=20) | P95 audio ≤ 5 s (n=17) | max |
|---|---|---|---|---|---|
| 2026-11-13 | 1.22–2.24 s | **2.96 s** | **3.45 s** ✅ | 3.24 s | 4.22 s (s06, STT 2.24 s) |
| 2026-11-13 (lần 2, sau sửa audit: làm ấm 1 request/lúc) | 1.29–2.04 s | **3.27 s** | **3.72 s** ✅ | 3.72 s | 3.87 s |

Câu nói dài (7–11 s audio) trong app: STT 2.7–2.8 s → e2e có thể > 4 s; giới hạn của large-v3 (ADR-0011), không thuộc phạm vi sửa ở đây.

## 6. Kết luận (sơ bộ, 2026-11-12)

- **LLM:** [ADR-0008](adr/0008-llm-8b-mac-dinh.md): **Qwen3-8B mặc định**, Qwen3-4B-Instruct-2507 làm chế độ nhanh (thay ADR-0004). **Qwen3-8B** chất lượng tốt hơn rõ (4* so với 3.5*), vẫn đạt TTFT (0.4 s ấm) nhưng sinh chậm hơn một nửa (11.8 tok/s) và tốn thêm ~2 GB RAM → người dùng chọn 8B mặc định.
- **TTS:** người dùng **thích giọng ZeroTTS** hơn; VieNeu v3 Turbo int8 không méo và là phương án nhanh. **Chốt ZeroTTS int8 (4 luồng)**: first audio 0.06 s, RTF 0.31; người dùng nghe đạt. [ADR-0005](adr/0005-chon-tts.md) đã chấp nhận. Xem [ADR-0005](adr/0005-chon-tts.md).
- **STT:** chọn **Whisper large-v3-turbo q5_0 trên whisper.cpp Vulkan tự build** — WER 7.8 %, 1.53 / 1.77 s (giọng tổng hợp). Xem [ADR-0007](adr/0007-chon-stt.md). Còn phải kiểm chứng trên giọng thật.
- **Ước tính "thả PTT → nghe tiếng đầu tiên"** (cộng các số đo riêng, chưa đo nối liền): STT turbo Vulkan 1.5–1.8 s + LLM TTFT ấm + câu đầu tiên (~15 token) + TTS ZeroTTS int8 0.06 s:
  - Qwen3-4B-Instruct-2507: 0.25 + 0.65 s → **≈ 2.5–2.8 s** ✅
  - Qwen3-8B: 0.40 + 1.3 s → **≈ 3.3–3.6 s** ✅ (sát ngưỡng 4 s hơn)
