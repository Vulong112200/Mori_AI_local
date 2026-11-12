# Kiến trúc Mori — trạng thái thực tế

> Tài liệu này mô tả hệ thống **như đang được xây** (code + runtime + số đo thật).
> [SPEC.md](SPEC.md) là đặc tả đích; [ROADMAP.md](ROADMAP.md) là tiến độ; [adr/](adr/) là lý do của từng quyết định.
> **Cập nhật:** 2026-11-12 — hết Phase 0, Phase 1 đã code xong, chờ người dùng thử để qua gate.

---

## 1. Tóm tắt một trang

Mori là cô bạn đồng hành AI **nữ, chibi, dễ thương**, chạy **100% trên laptop** (Windows 11, Core Ultra 9 285H, Arc 140T, 32 GB RAM), nói tiếng Việt. Không cloud, không telemetry.

| Chức năng AI | Đã chọn | Chạy trên | Số đo thật (máy đích) | ADR | Trạng thái |
|---|---|---|---|---|---|
| **Não (LLM)** | Qwen3-8B Q4_K_M · *chế độ nhanh:* Qwen3-4B-Instruct-2507 | llama.cpp b12284 **Vulkan** (iGPU) | TTFT 1.16 s lạnh / 0.40 s ấm, 11.8 tok/s, RAM 6.2 GB | [0008](adr/0008-llm-8b-mac-dinh.md) | ✅ **Đã tích hợp vào app (Phase 1)** |
| **Tai (STT)** | Whisper large-v3-turbo q5_0 | whisper.cpp v1.9.5 **tự build Vulkan** | WER 7.8 %, 1.53 s TB / 1.77 s P95 (giọng tổng hợp) | [0007](adr/0007-chon-stt.md) | Đã đo, chưa tích hợp (Phase 2) |
| **Giọng (TTS)** | ZeroTTS 0.1.5 **int8**, giọng `maichi` | ONNX Runtime CPU (Python 3.12) | Chunk đầu 0.06 s, RTF 0.31, RAM 0.82 GB | [0005](adr/0005-chon-tts.md) | Đã đo, chưa tích hợp (Phase 3) |
| **Hình (Avatar)** | Live2D Cubism SDK for Web; dev dùng mẫu **Koharu** | WebGL trong WebView2 | — | [0006](adr/0006-license-live2d.md) | Chưa làm (Phase 5) |
| **Trí nhớ** | SQLite + FTS5, người dùng duyệt | Rust (rusqlite) | — | SPEC §11 | Bảng hội thoại xong; memory ở Phase 4 |

**End-to-end đo thật** (wav → STT → LLM 8B tới hết câu đầu → TTS chunk đầu, 6 lượt): **TB 2.63 s, max 3.59 s** (mục tiêu ≤ 4 s), **0 kết nối mạng ra ngoài loopback**.

---

## 2. Sơ đồ thành phần

```text
┌──────────────────────── Mori.exe (Tauri 2.12, WebView2) ─────────────────────────┐
│ Frontend React 19 + TS 7 + Tailwind 4 (src/)                                      │
│   App.tsx: chat, trạng thái model, nút Dừng, Chế độ nhanh, Cuộc trò chuyện mới    │
│   api.ts : chỉ gọi lệnh Rust (invoke) + nhận Channel/event — KHÔNG tự gọi model   │
│        │ invoke: init · send_message · stop_generation · set_fast_mode ·          │
│        │         new_conversation          event: "llm-status"                    │
│ ───────▼──────────────────────────────────────────────────────────────────────── │
│ Rust core (src-tauri/src/)                                                        │
│   lib.rs    lệnh IPC, kiểm tra đầu vào, slot 1-lượt-sinh, logging                 │
│   llm.rs    supervisor llama-server + stream SSE + Job Object                     │
│   prompt.rs quy tắc an toàn (cố định) + personality + cắt lịch sử                 │
│   db.rs     SQLite: conversations, messages, settings (migration user_version)    │
└───────┬───────────────────────────────────────────────────────────────────────────┘
        │ HTTP 127.0.0.1:<cổng ngẫu nhiên>, Bearer <key ngẫu nhiên 128 bit>
┌───────▼───────────────┐   (Phase 2)                 (Phase 3)
│ llama-server.exe      │   whisper-server.exe        TTS worker Python
│ Qwen3-8B / 4B, Vulkan │   turbo q5_0, Vulkan        ZeroTTS int8, JSON Lines stdio
└───────────────────────┘
```

---

## 3. Luồng một tin nhắn (Phase 1 — đang chạy)

1. Người dùng gõ → `send_message(conversation_id, text, Channel)`.
2. Rust kiểm tra: không rỗng, ≤ 8.000 ký tự, model đang **Ready**, không có lượt sinh khác (`GenerationSlot`).
3. Lưu tin nhắn user vào SQLite → đọc lịch sử.
4. `prompt::build`: `[system = SAFETY + personality] + lịch sử` (tối đa 24 tin, ≤ 12.000 ký tự, luôn giữ tin mới nhất). Thứ tự cố định → llama-server tái dùng KV cache (TTFT ấm ~0.4 s).
5. `llm.chat`: POST `/v1/chat/completions` (`stream`, `cache_prompt`, `max_tokens` 1024) → parse SSE → gửi từng `Delta` qua Channel → UI hiện chữ chảy.
6. Bấm **Dừng** → `Notify` → drop stream (llama-server ngừng sinh). Câu trả lời dở dang vẫn được lưu.
7. Lưu câu trả lời → gửi `Done`.

Luồng giọng nói đầy đủ (Phase 2–3, đã đo e2e bằng script): giữ phím nói → cpal 16 kHz → whisper-server → (luồng trên) → cắt câu đầu → ZeroTTS stream → rodio phát + RMS cho lip-sync.

---

## 4. Vòng đời llama-server (`llm.rs`)

| Tình huống | Hành vi | Kiểm chứng |
|---|---|---|
| Mở app | Spawn với model theo setting `fast_mode` → chờ `/health` (≤ 180 s) → warm-up 1 token (biên dịch shader Vulkan) → **Ready** | Ready sau 6–9 s |
| llama-server chết | Restart, backoff 1-2-4-8-16 s; > 5 lần liên tiếp → **Error** (chờ người dùng đổi model) | Kill thủ công → Ready lại sau 6.3 s, PID + cổng mới |
| Bật/tắt Chế độ nhanh | Kill + spawn model kia (không tính là lỗi) | — (người dùng thử) |
| Thoát app bình thường | `shutdown()` → kill, chờ ≤ 3 s | — |
| **Mori crash / End Task** | Job Object `KILL_ON_JOB_CLOSE` → kernel tắt llama-server | Ép tắt `mori.exe` → llama-server chết theo |

---

## 5. Biện pháp bảo mật / riêng tư đã có (ADR-0009)

- Mọi service bind **127.0.0.1**, cổng ngẫu nhiên mỗi lần chạy; HTTP client `no_proxy`, timeout kết nối 2 s.
- **API key 128 bit** (`getrandom`) truyền qua env `LLAMA_API_KEY` (không lộ trên command line). Gọi không key → **401**.
- `--no-webui --no-slots --offline`; **xóa mọi biến môi trường `LLAMA_*`** trước khi spawn (chặn bật tools chạy shell, log file, tải model qua env). Đã thử: đặt `LLAMA_ARG_LOG_FILE` → file không được tạo.
- stdout/stderr của llama-server → null (log của nó có thể chứa prompt). Log của Mori chỉ ghi id + độ dài, **không nội dung**; giữ 14 ngày.
- Tauri: chỉ `core:default`, **không** plugin shell/fs/http; CSP `default-src 'self'`, `connect-src ipc:`, `base-uri/form-action/object-src 'none'`. Model output hiển thị dạng text (không HTML).
- SQLite: mọi query có tham số; `foreign_keys`, `secure_delete`.
- Quy tắc an toàn nằm trong code (`prompt.rs`), đứng trước `personality.md`.

Còn mở (xem ROADMAP "Bảo mật — để sau"): ACL thư mục `E:\Mori Data`, kiểm SHA-256 binary/model lúc chạy, thu hẹp capability, chặn Windows Error Reporting.

---

## 6. Mã nguồn

| Đường dẫn | Nội dung |
|---|---|
| `src/App.tsx`, `src/api.ts`, `src/main.tsx`, `src/index.css` | Giao diện chat |
| `src-tauri/src/lib.rs` | Lệnh IPC, state, logging, khởi động |
| `src-tauri/src/llm.rs` | Supervisor llama-server, stream SSE, Job Object, test SSE/key/job |
| `src-tauri/src/prompt.rs` | SAFETY + dựng prompt, test cắt lịch sử |
| `src-tauri/src/db.rs` | SQLite + migration, test |
| `src-tauri/tauri.conf.json`, `capabilities/default.json` | Cửa sổ, CSP, quyền |
| `config/personality.md` | Tính cách Mori (bản mặc định) |
| `scripts/phase0/` | Benchmark & công cụ Phase 0 (xem §8) |
| `tests/fixtures/` | Prompt LLM, câu TTS, manifest STT, system prompt mẫu |

**Lệnh IPC**

| Lệnh | Vào | Ra |
|---|---|---|
| `init` | — | `{conversation_id, messages, status, fast_mode}` (mở hội thoại gần nhất) |
| `send_message` | `conversationId, text, onEvent: Channel` | tin nhắn user đã lưu; Channel nhận `{type:"delta",text}` … `{type:"done",message,stopped}` |
| `stop_generation` | — | — |
| `set_fast_mode` | `enabled: bool` | — (đổi model) |
| `new_conversation` | — | id mới |
| sự kiện `llm-status` | — | `{state: starting|ready|restarting|error|stopped, ...}` |

---

## 7. Dữ liệu & runtime trên đĩa (`E:\Mori Data\`, ADR-0002)

```text
E:\Mori Data\
├── models\llm\        Qwen3-8B, Qwen3-4B-Instruct-2507 (+ Qwen3-4B, Qwen3.5-4B đã đo, không dùng)
├── models\stt\        ggml-large-v3-turbo-q5_0.bin, ggml-small.bin
├── models\tts\        zerotts-int8\ (dùng), hf\ (cache HF: ZeroTTS fp32, VieNeu, Korva)
├── runtime\llama.cpp\ b12284-cpu\, b12284-vulkan\ (app dùng bản vulkan)
├── runtime\whisper.cpp\ src-v1.9.5\, build-vulkan\, v1.9.5-vulkan\ (bản dùng)
├── runtime\python\    tts-venv\ (Python 3.12, zerotts/vieneu/korvatts/onnx), interpreters\, uv-cache\
├── database\mori.db   SQLite (WAL)
├── logs\mori.<ngày>.log
└── settings\personality.md  (tùy chọn — ghi đè bản trong repo)
```

Ghi đè data root bằng biến môi trường `MORI_DATA_DIR`. `E:\Memora Data` là dự án khác — Mori không ghi vào đó.

---

## 8. Cách chạy

```powershell
npm install                      # lần đầu
npm run tauri dev                # chạy app; app tự quản lý llama-server
cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test   # 7 test
npx tsc --noEmit
```

- Lần đầu mở: trạng thái "Mori đang thức dậy…" ~6–9 s (nạp 8B + warm-up).
- Nếu `tauri dev` báo lỗi dev server: cổng 1420 còn bị Vite cũ giữ → tắt tiến trình `node … vite.js` đó.
- Công cụ Phase 0: `scripts/phase0/` — `hw_inventory.ps1`, `bench_llm.ps1`, `bench_stt.ps1`, `bench_tts.py`, `wer.py`, `run_llm_prompts.py`, `quantize_zerotts_int8.py`, `e2e_latency.py` (cách gọi trong docstring/đầu file và [CLAUDE.md](../CLAUDE.md)).

---

## 9. Giới hạn đã biết

- **RAM chật với 8B**: còn ~6.8 GB RAM / 0.8 GB commit trống khi app chạy (pagefile tự nới). Thêm STT + TTS ở Phase 2–3 phải đo lại; nếu swap nhiều → Chế độ nhanh.
- Số đo STT/e2e dùng **giọng tổng hợp**; cần giọng thật trước gate Phase 2.
- Cắt lịch sử theo ký tự (ước lượng ~2 ký tự/token), chưa tóm tắt hội thoại (Phase 4).
- Model đôi khi chèn emoji → phải lọc trước TTS (Phase 3).
- Chưa có danh sách hội thoại cũ trong UI (chỉ "Cuộc trò chuyện mới" + mở hội thoại gần nhất).
