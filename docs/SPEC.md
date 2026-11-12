# Mori Desktop Companion — Đặc tả sản phẩm & kỹ thuật

**Phiên bản:** 1.0 (đã rà soát, thay thế v0 — xem [SPEC_REVIEW.md](SPEC_REVIEW.md))
**Trạng thái:** Phase 0 — Khảo sát & khả thi
**Nền tảng:** Windows 11
**Cập nhật:** 2026-11-11

> Quy ước: **PHẢI** = bắt buộc; **NÊN** = khuyến nghị; **[P0]** = cần kiểm chứng ở Phase 0. Thay đổi quyết định kỹ thuật → ghi ADR trong [adr/](adr/), rồi cập nhật tài liệu này.

---

## 1. Tầm nhìn sản phẩm

Mori là trợ lý AI chạy hoàn toàn trên máy (local), đặt trên desktop Windows:

- Nói chuyện tự nhiên bằng tiếng Việt, qua giọng nói hoặc văn bản.
- Có tính cách nhất quán (bạn đồng hành thân thiện).
- Nhớ những điều **người dùng đã duyệt**.
- Hiển thị nhân vật 2D (Live2D) động, nổi trên desktop.
- Hỗ trợ lập trình, học tập, viết lách, xử lý sự cố, việc hằng ngày.
- Không dùng dịch vụ AI đám mây, không API ngoài.

### 1.1. Định hướng đã chốt

| Thuộc tính | Quyết định |
|---|---|
| Avatar | 2D, Live2D |
| Giọng nói | Tiếng Việt tự nhiên |
| Tương tác | Giọng nói (push-to-talk) + văn bản |
| Chạy AI | 100% local, không cloud, không cloud fallback |
| Tính cách | Bạn đồng hành thân thiện |
| Memory | Local, người dùng kiểm soát |
| Desktop | Cửa sổ nổi, luôn ở trên |

### 1.2. Nguyên tắc

1. **Local-first:** mọi suy luận AI và xử lý dữ liệu cá nhân chạy trên máy.
2. **Privacy-first:** không ghi âm ngầm, không telemetry, không gửi dữ liệu ra ngoài.
3. **Module hóa:** LLM, STT, TTS, avatar, memory thay thế độc lập được.
4. **CPU fallback:** dùng được khi không có tăng tốc GPU.
5. **Người dùng kiểm soát:** memory, micro, tải model đều cần thao tác rõ ràng.
6. **Độc lập avatar:** AI và giọng nói chạy được khi avatar bị ẩn hoặc lỗi.
7. **Minh bạch:** luôn hiện rõ trạng thái nghe / nghĩ / nói / nghỉ.
8. **Đo, không đoán:** mọi số liệu hiệu năng phải đo trên máy đích.

---

## 2. Phần cứng đích

| Thành phần | Thông số |
|---|---|
| Máy | ASUS Zenbook 14 UX3405CA |
| CPU | Intel Core Ultra 9 285H (16 luồng) |
| RAM | 32 GB (~31.4 GB dùng được) |
| GPU | Intel Arc 140T (iGPU, dùng chung RAM) |
| NPU | Intel AI Boost |
| Driver GPU / NPU | 32.0.101.8860 / 32.0.100.4023 |
| OS | Windows 11 build 26300 |
| Lưu trữ | NVMe ~1 TB; ổ E: còn ~154 GB |
| Pagefile | Windows tự quản lý, đang cấp 2 GB (xem §15.3) |
| Toolchain có sẵn | Git, Node 24, Rust 1.99, Python 3.14 |
| Toolchain thiếu | uv, CMake (cài khi cần, có duyệt) |

Chi tiết đo được ghi trong [HARDWARE_REPORT.md](HARDWARE_REPORT.md).

### 2.1. Chiến lược phần cứng

- Mọi thành phần AI thiết yếu **PHẢI** chạy được trên CPU.
- GPU (Arc 140T): thử **Vulkan trước**, SYCL sau. Chỉ bật khi đã đo ổn định. [P0]
- NPU: không bắt buộc trong MVP. OpenVINO là hướng dự phòng sau MVP.
- Nạp model khi cần (lazy). Không chạy đồng thời nhiều model lớn nếu không cần.
- Không coi TOPS hay benchmark máy khác là bằng chứng hiệu năng.

---

## 3. Kiến trúc hệ thống

### 3.1. Tổng quan

```text
┌──────────────────────── Tauri 2 (WebView2) ────────────────────────┐
│  React + TS: Chat UI · Avatar (Live2D/WebGL) · Settings · Memory UI │
└───────────────┬───────────────────────────── ▲ ────────────────────┘
                │ typed commands               │ events (token, state, rms)
┌───────────────▼──────────────── Rust core ───┴────────────────────┐
│ Conversation Manager · Audio (cpal capture, rodio playback)       │
│ Process Manager · Model Manager · SQLite (FTS5) · Settings · Log  │
└──────┬───────────────────┬───────────────────────┬────────────────┘
       │ HTTP 127.0.0.1    │ HTTP 127.0.0.1        │ JSON Lines (stdio)
┌──────▼──────┐     ┌──────▼────────┐       ┌──────▼──────────┐
│ llama-server│     │ whisper-server│       │ TTS worker      │
│ (LLM)       │     │ (STT)         │       │ (Python/ONNX    │
└─────────────┘     └───────────────┘       │  hoặc GGUF)     │
                                            └─────────────────┘
```

### 3.2. Quy tắc kiến trúc

- Frontend **không** nạp model, **không** chạy process, FFmpeg, Python.
- Rust sở hữu vòng đời app và mọi process con.
- Worker giao tiếp qua giao diện có phiên bản:
  - `llama-server`, `whisper-server`: HTTP, **PHẢI** chạy với `--host 127.0.0.1` và cổng ngẫu nhiên.
  - TTS worker: JSON Lines qua stdin/stdout (§13).
- Worker lỗi **không được** làm sập app. Rust phát hiện và khởi động lại.
- Không worker nào được chạy lệnh hệ điều hành tùy ý.
- Không có cloud fallback.

### 3.3. Module

| Module | Trách nhiệm |
|---|---|
| Desktop UI | Chat, avatar, settings, quản lý memory, cửa sổ nổi |
| Conversation Manager | Điều phối STT → LLM → TTS, ngắt lời, trạng thái |
| Audio | Thu micro (cpal), phát audio (rodio), tính biên độ RMS cho lip-sync |
| Process Manager | Khởi động, health check, khởi động lại, tắt worker |
| Model Manager | Manifest, checksum, kiểm tra offline |
| Memory | SQLite + FTS5, đề xuất / duyệt / xóa memory |
| Settings | Lưu trong SQLite |
| Logging | `tracing`, file log local, không ghi nội dung nhạy cảm |

---

## 4. Công nghệ

| Lớp | Công nghệ | Trạng thái |
|---|---|---|
| Desktop | Tauri 2 | Chốt |
| Frontend | React + TypeScript + Tailwind CSS | Chốt |
| Backend | Rust | Chốt |
| Avatar | Live2D Cubism SDK for Web (WebGL) | Chốt, rà license [P0] |
| LLM runtime | llama.cpp (`llama-server`) | Chốt |
| Tăng tốc LLM | Vulkan (Arc 140T); CPU fallback | Chốt — ADR-0004 |
| STT | whisper.cpp (`whisper-server`) | Chốt runtime; model [P0] |
| TTS | ZeroTTS int8 (dự phòng: VieNeu v3 Turbo) | Chốt — ADR-0005 |
| Python (nếu cần) | 3.12 qua uv, venv riêng (thư viện TTS hỗ trợ 3.10–3.13, máy đang có 3.14) | Chốt |
| Audio | cpal (thu), rodio (phát) | Chốt |
| DB | SQLite + FTS5 (`rusqlite`) | Chốt |
| Log | `tracing` | Chốt |
| VAD | Không có trong MVP | Hoãn (ADR-0003) |

---

## 5. LLM

### 5.1. Ứng viên

**Đã chọn ([ADR-0004](adr/0004-chon-llm.md)): Qwen3-4B-Instruct-2507 Q4_K_M trên Vulkan** — TTFT 0.55 s lạnh / 0.25 s ấm, 23.7 tok/s. Đang thử thêm Qwen3-8B.

| Model | Vai trò | Ghi chú |
|---|---|---|
| Qwen3-4B Q4_K_M | Baseline | Apache-2.0. Có thinking / non-thinking (`/no_think`) |
| Qwen3-4B-Instruct-2507 Q4_K_M | Ứng viên | Chỉ non-thinking, hợp hội thoại |
| Qwen3.5-4B Q4_K_M | Ứng viên | 03/2026. Cần build llama.cpp đủ mới |

Không thay model chỉ vì có bản mới hơn. So sánh theo: chất lượng tiếng Việt, TTFT, tốc độ sinh, RAM, độ ổn định context, license.

### 5.2. Cấu hình ban đầu

| Tham số | Giá trị |
|---|---|
| Context | 8K token |
| Chế độ mặc định | Non-thinking |
| Runtime | `llama-server` loopback |
| Backend | CPU, bật GPU khi Phase 0 xác nhận |
| Prompt cache | **Bật** (`cache_prompt`) |

### 5.3. Đo bắt buộc [P0]

Bằng `llama-bench`, cùng commit, cùng model, cho CPU / Vulkan / SYCL:
- **pp** (xử lý prompt, token/s) — quyết định TTFT.
- **tg** (sinh token, token/s) — quyết định tốc độ nói.
- RAM đỉnh, nhiệt/throttle sau 10 phút.

### 5.4. Cấu trúc prompt (để đạt TTFT)

Xếp prompt từ **ổn định nhất** đến **thay đổi nhất** để KV cache được tái dùng:

```text
1. System prompt (quy tắc an toàn + personality.md)   ← cố định
2. Memory đã duyệt liên quan                          ← ít đổi
3. Tóm tắt hội thoại cũ (nếu có)                       ← ít đổi
4. Các lượt gần đây                                    ← thêm dần
5. Tin nhắn mới                                        ← mới
```

Không chèn timestamp hay dữ liệu thay đổi vào phần 1.

---

## 6. Nhận dạng giọng nói (STT)

### 6.1. Ứng viên [P0]

| Model | Ghi chú |
|---|---|
| Whisper small (ggml) | Nhanh, chất lượng tiếng Việt có thể chưa đủ |
| Whisper large-v3-turbo (ggml q5/q8) | Chất lượng cao hơn, cần đo độ trễ CPU/GPU |
| PhoWhisper (VinAI) | Fine-tune tiếng Việt. Thử convert sang ggml bằng `convert-h5-to-ggml.py` của whisper.cpp |

Runtime: `whisper-server` (giữ model trong RAM khi đang trong phiên hội thoại, giải phóng khi nghỉ lâu).

**Đề xuất ([ADR-0007](adr/0007-chon-stt.md)): large-v3-turbo q5_0 trên whisper.cpp v1.9.5 build Vulkan** — WER 7.8 %, 1.53 s TB (giọng tổng hợp). Bản dựng sẵn của whisper.cpp không có Vulkan → phải build:

```text
cmake -S src-v1.9.5 -B build-vulkan -G "Visual Studio 17 2022" -A x64 -DGGML_VULKAN=ON -DWHISPER_BUILD_TESTS=OFF
cmake --build build-vulkan --config Release --target whisper-cli whisper-server
```
(cần MSVC Build Tools 2022 + C++ workload, Vulkan SDK, CMake; biến `VULKAN_SDK`)

### 6.2. Luồng

```text
Nhấn giữ phím → cpal thu PCM 16 kHz mono vào buffer RAM
Thả phím      → gửi buffer cho whisper-server → transcript
              → hiện transcript (cho sửa) → Conversation Manager
Buffer bị xóa ngay sau khi nhận dạng xong.
```

### 6.3. Yêu cầu

- Push-to-talk; không ghi âm nền mặc định.
- Chọn thiết bị micro; trạng thái micro luôn hiện rõ.
- Nút dừng tức thì (< 300 ms).
- Không ghi audio xuống đĩa. Nếu buộc phải ghi tạm, xóa ngay sau xử lý.
- Hỗ trợ "prompt" từ vựng (tên riêng, thuật ngữ lập trình) cho Whisper.

---

## 7. Tổng hợp giọng nói (TTS)

### 7.1. Engine đã chọn — [ADR-0005](adr/0005-chon-tts.md)

**ZeroTTS 0.1.5** ([zeroweight-ai/ZeroTTS](https://github.com/zeroweight-ai/ZeroTTS), MIT), ONNX/CPU, streaming `synthesize_stream`, 48 kHz, giọng `maichi`.

- Model **int8** tạo local từ weights fp32 chính thức bằng `scripts/phase0/quantize_zerotts_int8.py` → `E:\Mori Data\models\tts\zerotts-int8\`.
- `intra_op_num_threads=4`. Đo Phase 0: first audio 0.06 s, RTF 0.31, RAM 0.82 GB.
- Dự phòng: VieNeu-TTS v3 Turbo int8 (`vieneu` 3.8.4, Apache-2.0) — first audio 0.39 s, RTF 0.74.
- Đã loại: KorvaTTS (không streaming, quá chậm).

Engine mặc định tải weights từ Hugging Face ở lần chạy đầu → phải tải trước (có duyệt), rồi trỏ đường dẫn local và chạy với `HF_HUB_OFFLINE=1`.

**Bắt buộc trên Windows (đã kiểm chứng Phase 0):** đặt `HF_HUB_DISABLE_SYMLINKS=1` khi tải. Mặc định cache Hugging Face lưu file dạng symlink trỏ vào `blobs/`; ONNX Runtime ≥ 1.31 từ chối file external data (`*.data`) nằm khác thư mục với `.onnx` ("External data path escapes model directory") → ZeroTTS và VieNeu không nạp được. Biến môi trường chuẩn của worker TTS:

```text
HF_HOME=E:\Mori Data\models\tts\hf
HF_HUB_DISABLE_SYMLINKS=1
HF_HUB_OFFLINE=1          (khi chạy, sau khi đã tải)
```

### 7.2. Tiêu chí đánh giá

| Tiêu chí | Đo bằng |
|---|---|
| Tự nhiên, phát âm, nhịp | Nghe, chấm 1–5 |
| Đọc thuật ngữ tiếng Anh, số, ngày | Bộ câu [tests/fixtures/tts_sentences.txt](../tests/fixtures/tts_sentences.txt) |
| First-audio latency | giây |
| Real-time factor (RTF) | thời gian xử lý / độ dài audio |
| CPU, RAM | %, MB |
| Ổn định với văn bản dài | Stress test |

### 7.3. Streaming

1. LLM sinh token; Conversation Manager cắt theo câu (`.`, `?`, `!`, xuống dòng; câu quá dài cắt ở dấu phẩy). Trước khi gửi TTS: bỏ emoji và ký hiệu markdown (`**`, `#`, `-` đầu dòng, khối code) — Phase 0 cho thấy model hay chèn emoji.
2. Câu đầu tiên gửi TTS ngay.
3. Rust phát audio chunk theo thứ tự, có số thứ tự để chống phát lặp hoặc thiếu.
4. Trong lúc phát, Rust phát sự kiện RMS ~30 Hz cho avatar.
5. Người dùng ngắt lời → hủy LLM, hủy hàng đợi TTS, dừng phát.

---

## 8. VAD

Không có trong MVP. Push-to-talk đã xác định đầu/cuối câu. Hoãn cùng wake-word (ADR-0003).

---

## 9. Avatar

### 9.0. Nhân vật

Mori là **nhân vật nữ, phong cách chibi/SD, dễ thương** (người dùng chọn 2026-11-12), giọng ZeroTTS `maichi` (hơi giọng miền Nam). Phát triển dùng model mẫu Koharu của Live2D; phát hành dùng nhân vật riêng — xem [ADR-0006](adr/0006-license-live2d.md).

### 9.1. Công nghệ

Live2D Cubism SDK for Web, WebGL trong WebView2. Cubism Core là binary độc quyền: **không commit vào git**, tải thủ công theo hướng dẫn.

### 9.2. Trạng thái

| Trạng thái | Hành vi |
|---|---|
| Idle | Chớp mắt, thở, cử động nhẹ |
| Listening | Biểu cảm tập trung |
| Thinking | Nhìn sang bên |
| Speaking | Miệng theo biên độ RMS |
| Happy / Confused | Cười / nghiêng đầu (nếu model hỗ trợ) |
| Sleeping | Nhắm mắt, giảm animation |

### 9.3. Lip-sync

MVP: theo biên độ (RMS từ Rust) → độ mở miệng, có làm mượt (EMA) để tránh giật. Sau MVP: viseme.

### 9.4. Cửa sổ

Trong suốt, luôn ở trên, kéo thả, đổi kích thước, chỉnh độ mờ, tùy chọn click-through (`set_ignore_cursor_events`), nhớ vị trí. Idle giảm FPS.

Chế độ hiển thị: **Companion** (chỉ nhân vật), **Chat** (nhân vật + khung chat), **Focus** (ẩn nhân vật, dịch vụ vẫn chạy).

### 9.5. License

Rà điều khoản Cubism SDK, model mẫu, artwork trước khi phát hành. Model tải trên mạng không mặc định được phép phân phối lại.

Tóm tắt và điều kiện: [ADR-0006](adr/0006-license-live2d.md). Điểm chính: Mori dùng nhân vật cố định, **không** cho người dùng nạp model Live2D tùy ý (nếu không sẽ thành "Expandable Application" — phải xin duyệt, không miễn phí).

---

## 10. Tính cách

### 10.1. Định hướng

Tên: **Mori**. Thân thiện, thông minh, chu đáo, hơi hài hước, tinh tế cảm xúc. Là bạn đồng hành, không phải chatbot chăm sóc khách hàng.

- Tiếng Việt tự nhiên; nghiêm túc khi làm việc quan trọng.
- Không nịnh; không giả vờ biết; lịch sự phản biện khi người dùng sai.
- Hỏi lại khi chưa rõ.
- Không tự bắt chuyện khi chưa được phép.
- Câu trả lời khi nói bằng giọng NÊN ngắn gọn (vài câu), vì người nghe không lướt được như đọc.

Ví dụ:
- "Mori ơi." → "Mình đây, bạn cần gì nè?"
- "Code này bị lỗi." → Tập trung tìm nguyên nhân gốc và cách sửa.
- "Hôm nay hơi mệt." → Đáp nhẹ nhàng, không thúc ép làm tiếp.

### 10.2. Cấu hình

```text
config/personality.md     # prompt tính cách, người dùng sửa được
```

Quy tắc an toàn (không leo thang quyền, không chạy lệnh, chống prompt injection từ nội dung memory/tài liệu) **nằm trong code Rust**, ghép trước `personality.md`, người dùng và AI không sửa được.

---

## 11. Memory

### 11.1. Các lớp

- **Ngắn hạn:** hội thoại hiện tại (trong RAM + bảng `messages`).
- **Dài hạn:** sở thích, dự án, quyết định kỹ thuật, bối cảnh cá nhân **đã được duyệt**.
- **Tri thức:** tài liệu người dùng chọn nạp — **hoãn sau MVP**.

### 11.2. Lưu trữ

SQLite + FTS5. Vector search hoãn đến khi chứng minh được là cần.

### 11.3. Schema ban đầu

```sql
PRAGMA user_version = 1;          -- phiên bản schema, dùng cho migration
PRAGMA secure_delete = ON;        -- ghi đè dữ liệu khi xóa
PRAGMA foreign_keys = ON;

CREATE TABLE conversations (
    id          TEXT PRIMARY KEY,
    title       TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE messages (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL
);
CREATE INDEX idx_messages_conv ON messages(conversation_id, created_at);

CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    category    TEXT NOT NULL,
    content     TEXT NOT NULL,
    source_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    status      TEXT NOT NULL DEFAULT 'proposed'
                CHECK (status IN ('proposed','approved','rejected')),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE VIRTUAL TABLE memories_fts USING fts5(
    content, content='memories', content_rowid='rowid'
);
-- + trigger đồng bộ insert/update/delete giữa memories và memories_fts

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 11.4. Quy tắc

- AI không tự lưu mọi thứ; AI chỉ **đề xuất** (`status='proposed'`).
- Chỉ memory `approved` được đưa vào prompt.
- Người dùng xem, sửa, duyệt, từ chối, xóa từng memory; xóa toàn bộ.
- Xóa phải kiểm chứng: `secure_delete`, rebuild FTS, `VACUUM` khi xóa toàn bộ; có test đọc thô file DB không còn chuỗi đã xóa.
- Sao lưu/khôi phục: SQLite Online Backup API ra `E:\Mori Data\backups\`.
- Nội dung memory là **dữ liệu**, không phải chỉ thị (chống prompt injection).

---

## 12. Conversation Manager

### 12.1. Máy trạng thái

```text
Idle ──nhấn PTT──▶ Listening ──thả──▶ Transcribing ──▶ Thinking ──token đầu──▶ Speaking ──xong──▶ Idle
  ▲                    │                                  │                       │
  └──────── Stop / lỗi / nhấn PTT (ngắt lời) ◀─────────────┴───────────────────────┘
```

Mỗi lượt có một `turn_id`. Kết quả mang `turn_id` cũ bị bỏ qua (chống phát lặp sau khi ngắt).

### 12.2. Trách nhiệm

- Quản lý lịch sử, dựng prompt theo §5.4, tóm tắt khi gần đầy context.
- Hủy sinh LLM, hủy hàng đợi TTS, dừng phát khi bị ngắt.
- Phát sự kiện trạng thái cho UI/avatar.
- Truy xuất memory: FTS5 lấy top-k memory `approved` liên quan, đặt ở vị trí 2 của prompt.

---

## 13. Worker & IPC

| Worker | Giao thức | Ghi chú |
|---|---|---|
| `llama-server.exe` | HTTP OpenAI-compatible + SSE | `--host 127.0.0.1 --port <ngẫu nhiên>` |
| `whisper-server.exe` | HTTP | Loopback |
| TTS worker | JSON Lines stdio | Python 3.12 (uv venv) hoặc runtime GGUF nếu đủ tốt |

### 13.1. Giao thức JSON Lines (TTS)

Request:

```json
{"v":1,"id":"req-001","type":"synthesize","payload":{"text":"Chào bạn!","voice":"default"}}
```

Response (một hoặc nhiều dòng):

```json
{"v":1,"id":"req-001","type":"audio","seq":0,"sample_rate":24000,"pcm_b64":"..."}
{"v":1,"id":"req-001","type":"done"}
{"v":1,"id":"req-001","type":"error","message":"..."}
```

Các `type` khác: `cancel`, `health`, `shutdown`.

### 13.2. Yêu cầu

- Mỗi request có `id`, phiên bản `v`, timeout phía Rust.
- Hỗ trợ hủy, health check, tắt êm.
- Kiểm tra output, giới hạn kích thước payload (text ≤ 2.000 ký tự/request).
- stdout chỉ chứa protocol; log ra stderr.
- Worker chết → Rust log, khởi động lại có giới hạn (backoff), UI báo trạng thái.

---

## 14. Quyền riêng tư & bảo mật

### 14.1. Bắt buộc

| Thành phần | Chính sách |
|---|---|
| LLM, STT, TTS | Chỉ local |
| Memory | SQLite local |
| Ghi âm | Không lưu |
| Log | Không chứa nội dung hội thoại mặc định |
| Telemetry, crash report ra ngoài | Tắt / không có |
| Cloud API, cloud fallback | Không |
| Tải model | Chỉ khi người dùng duyệt |
| Truy cập web | Tắt trong MVP |

### 14.2. Yêu cầu bảo mật

- Mọi service bind `127.0.0.1`; không nhận kết nối LAN.
- Micro: trạng thái hiện rõ, Stop dừng thu ngay.
- Model file kiểm SHA-256 theo manifest; worker Python chạy với `HF_HUB_OFFLINE=1`.
- Tauri: CSP chặt, không bật plugin `shell`, capability tối thiểu.
- AI không chạy lệnh shell, không truy cập file tự do.
- Tính năng mở rộng (skills) cần quyền rõ ràng — sau MVP.

### 14.3. Offline

Cài đặt và tải model có thể cần Internet. Sau đó hội thoại cốt lõi **PHẢI** chạy offline. App có màn hình "kiểm tra sẵn sàng offline" (đủ model, checksum đúng, worker khởi động được).

---

## 15. Quản lý tài nguyên

### 15.1. Ngân sách RAM (ước tính, chờ đo)

| Thành phần | Ước tính |
|---|---:|
| Windows + nền | 6–9 GB |
| LLM 4B Q4 + KV 8K | 3–5 GB |
| STT (small → large-v3-turbo) | 0.5–2 GB |
| TTS (+ Python runtime) | 0.5–2.5 GB |
| UI + avatar | 0.3–1 GB |
| DB + buffer | 0.2–0.5 GB |

### 15.2. Chính sách

- Ưu tiên LLM trong hội thoại; STT/TTS nạp khi cần, nhả sau thời gian nghỉ.
- Tránh chạy đồng thời nhiều tác vụ GPU.
- Avatar 30 FPS khi hoạt động, giảm khi idle.
- Có nút **Pause AI** (dừng mọi worker).

### 15.3. Pagefile

Pagefile do Windows quản lý, hiện cấp 2 GB → giới hạn commit ~33 GB lúc đo. Lần đo đầu chỉ còn ~4 GB commit trống do các ứng dụng khác (xem [HARDWARE_REPORT.md](HARDWARE_REPORT.md)). Phase 0 phải đo RAM trống ở trạng thái làm việc thường ngày và theo dõi commit charge khi chạy đồng thời LLM + STT + TTS. Nếu thiếu: đóng bớt ứng dụng nền hoặc đặt pagefile tối thiểu ≥ 8 GB.

### 15.4. Phân bổ phần cứng ban đầu

| Thành phần | Chạy trên |
|---|---|
| LLM | GPU (Vulkan) — ADR-0004; CPU fallback |
| STT | GPU (Vulkan) — ADR-0007 |
| TTS | CPU |
| Avatar | WebGL |
| NPU | Không dùng trong MVP |

---

## 16. Dữ liệu trên đĩa

```text
E:\Mori Data\
├── models\        llm\ stt\ tts\   (+ manifest.json: tên, phiên bản, URL, license, sha256)
├── runtime\       llama.cpp\<build>\  whisper.cpp\<build>\  python\ (uv venv 3.12)
├── database\      mori.db
├── backups\
├── logs\
└── avatar\        (model Live2D, Cubism Core)
```

- Không commit model weights, audio, DB vào git.
- Không ghi vào `E:\Memora Data\` (dự án khác). Phase 0 được **đọc/chạy** `whisper-cli.exe` ở đó để benchmark.

---

## 17. Cấu trúc mã nguồn

Chỉ tạo thư mục khi phase cần đến.

```text
Mori_AI_Local/
├── CLAUDE.md, README.md
├── .claude/                 settings.json, commands/
├── docs/                    SPEC, SPEC_REVIEW, ROADMAP, HARDWARE_REPORT, MODEL_EVALUATION, adr/, archive/
├── scripts/phase0/          benchmark Phase 0
├── tests/fixtures/          bộ câu test
│ — từ Phase 1 —
├── src/                     React frontend
├── src-tauri/               Rust core
├── config/personality.md
│ — từ Phase 3 (nếu TTS cần Python) —
└── workers/tts/
```

---

## 18. Lộ trình MVP

Tiến độ chi tiết và checkbox: [ROADMAP.md](ROADMAP.md). Mỗi phase chỉ bắt đầu khi phase trước qua gate.

| Phase | Nội dung | Gate |
|---|---|---|
| 0 | Khảo sát phần cứng, đo LLM/STT/TTS, rà license | Có ít nhất một bộ LLM + STT + TTS chạy offline đạt ngưỡng chấp nhận |
| 1 | Tauri shell, Rust process manager, llama-server, chat UI streaming, dừng sinh, **personality prompt**, SQLite (conversations/messages) | Chat văn bản chạy offline, đúng giọng Mori |
| 2 | Thu micro, push-to-talk, whisper-server, xem/sửa transcript, chọn thiết bị | Nhận dạng ổn trên bộ test 100 câu |
| 3 | TTS worker, streaming theo câu, phát audio, ngắt lời | Giọng tự nhiên, không lặp/thiếu đoạn |
| 4 | Memory: đề xuất/duyệt/sửa/xóa, FTS5, tóm tắt hội thoại, sao lưu | Memory test + xóa có kiểm chứng đạt |
| 5 | Live2D, cửa sổ trong suốt, trạng thái, lip-sync RMS, hotkey | Avatar ổn định, không làm chậm AI đáng kể |
| 6 | Benchmark đầy đủ, kiểm tra offline, khôi phục lỗi, security audit, regression | Mọi tiêu chí chấp nhận quan trọng đạt |

---

## 19. Kiểm thử

### 19.1. LLM

| ID | Test | Kỳ vọng |
|---|---|---|
| AI-01 | Hội thoại tiếng Việt | Không lỗi ngôn ngữ, không chen chữ Trung/Anh vô cớ |
| AI-02 | Nhiều lượt | Giữ ngữ cảnh |
| AI-03 | Câu hỏi lập trình | Giải thích hữu ích |
| AI-04 | Thông tin không biết | Không bịa chắc chắn |
| AI-05 | Nhất quán tính cách | Giọng Mori ổn định |
| AI-06 | Prompt injection (cả qua memory) | Không leo thang quyền |
| AI-07 | Dừng sinh | Dừng ngay |
| AI-08 | Context dài | Giữ thông tin quan trọng |

Bộ prompt: [tests/fixtures/llm_prompts.md](../tests/fixtures/llm_prompts.md).

### 19.2. STT — bộ 100 câu

20 câu đời thường · 20 câu giọng miền Nam · 20 câu lẫn thuật ngữ tiếng Anh/kỹ thuật · 20 câu dài có số và dấu câu · 20 câu nhiễu hoặc nói nhanh.

Đo: WER, CER, lỗi tên riêng, lỗi thuật ngữ, lỗi dấu thanh, chấm điểm chủ quan. Danh sách file: [tests/fixtures/stt_manifest.csv](../tests/fixtures/stt_manifest.csv); tính WER/CER: `scripts/phase0/wer.py`.

### 19.3. TTS

Tự nhiên, phát âm, nhịp, ngắt nghỉ, đọc lẫn vi-en, first-audio, RTF, CPU, RAM, ổn định văn bản dài.

### 19.4. Mục tiêu hiệu năng (thiết kế, chờ đo)

| Chỉ số | Mục tiêu P95 |
|---|---:|
| STT câu ngắn (≤ 5 s audio) | < 2 s |
| LLM TTFT (prompt cache ấm) | < 2 s |
| TTS first audio | < 1.5 s |
| Thả PTT → nghe tiếng đầu tiên | < 4 s |
| Dừng micro / ngắt lời | < 300 ms |
| Avatar khi hoạt động | 30 FPS |

Đo riêng: cold start, warm start, pp, tg, TTFT cache lạnh/ấm, RTF.

### 19.5. Tài nguyên

| Test | Kỳ vọng |
|---|---|
| Idle 30 phút | RAM không tăng liên tục |
| Hội thoại 30 phút / 2 giờ | Không treo worker, RAM ổn định |
| Không có GPU | CPU fallback chạy |
| TTS liên tục | Không nghẽn pipeline audio |
| Đổi thiết bị audio | Thu/phát khôi phục |
| Sleep/resume | Worker khôi phục an toàn |

### 19.6. Quyền riêng tư

Ngắt mạng vẫn hội thoại được · không có kết nối ra ngoài (kiểm bằng Resource Monitor/`netstat`) · micro dừng sau Stop · không còn file audio tạm · DB không chứa credential · service không truy cập được từ máy khác trong LAN · xóa memory có kiểm chứng · không tự tải model · không tự cập nhật.

### 19.7. Đánh giá chủ quan (1–5)

Hiểu tiếng Việt · giọng tự nhiên · nhịp nói · nhất quán tính cách · memory hữu ích · trải nghiệm hội thoại · biểu cảm avatar.
Ngưỡng: trung bình ≥ 4/5 cho trải nghiệm cốt lõi, không lỗi nghiêm trọng về riêng tư hoặc mất dữ liệu.

---

## 20. Rủi ro

| Rủi ro | Mức | Giảm thiểu |
|---|---|---|
| TTFT cao trên CPU | Cao | Prompt cache, cấu trúc prompt §5.4, GPU nếu ổn |
| Vulkan/SYCL không ổn trên Arc 140T | Cao | CPU baseline; OpenVINO là hướng dự phòng sau MVP |
| TTS chưa đủ tự nhiên | Cao | So sánh 3 ứng viên, nghe thực tế |
| STT sai thuật ngữ | Cao | large-v3-turbo, PhoWhisper, prompt từ vựng, cho sửa transcript |
| Nhiều model chiếm RAM / chạm commit limit | Cao | Lazy load, nhả khi nghỉ, tăng pagefile |
| Lưu memory ngoài ý muốn | Cao | Chỉ đề xuất, duyệt mới lưu, xóa kiểm chứng |
| Python 3.14 không tương thích thư viện TTS | Trung bình | uv venv 3.12 riêng |
| TTS tự tải model ngầm | Trung bình | Tải trước, pin sha256, `HF_HUB_OFFLINE=1` |
| KorvaTTS non trẻ (v0.1.0) | Trung bình | Có 2 ứng viên khác |
| Worker crash | Trung bình | Health check, restart có backoff |
| Avatar tốn tài nguyên | Trung bình | Giảm FPS khi idle, tắt được |
| License Live2D | Trung bình | Rà trước khi phát hành; không commit Cubism Core |
| Echo | Thấp | PTT; dừng TTS trước khi mở micro |

---

## 21. Tính năng hoãn (ngoài MVP)

Wake-word và VAD · nhận diện khuôn mặt · hiểu màn hình · điều khiển Windows tự động · voice cloning · plugin bên thứ ba · dịch vụ cloud · tìm kiếm Internet · chạy shell tự do · quét file tự động · knowledge memory (nạp tài liệu) · vector search · NPU/OpenVINO.

Mỗi tính năng cần thiết kế, quyền và phê duyệt riêng.

---

## 22. Quy tắc phát triển với Claude Code

Claude Code là trợ lý phát triển, **không phải** AI runtime của Mori. Quy tắc chi tiết cho Claude nằm trong [../CLAUDE.md](../CLAUDE.md).

### 22.1. Phê duyệt

- Phase 0 chỉ khảo sát, đo đạc, lập kế hoạch; không viết code app.
- **Không tải model, không cài phần mềm/gói** khi người dùng chưa duyệt rõ ràng (cấu hình `ask` trong `.claude/settings.json`).
- Không commit/push khi người dùng chưa yêu cầu.
- Không ghi vào `E:\Memora Data\`.

### 22.2. Quy trình

- Mỗi phiên bắt đầu bằng `/status`.
- Quyết định kỹ thuật → `/adr`, rồi cập nhật SPEC.
- Kết quả đo → `/bench`, ghi vào MODEL_EVALUATION.md.
- Xong task → tick checkbox trong ROADMAP.md.
- Kết thúc phase → `/gate <n>`.

### 22.3. Chất lượng

- Code Rust: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` phải sạch.
- Code TS: `tsc --noEmit` và lint phải sạch.
- Không tắt test hay bỏ qua cảnh báo để "cho qua".
- Mọi thay đổi chạm tới quyền riêng tư/bảo mật → chạy agent `security-auditor` trước khi xong.
