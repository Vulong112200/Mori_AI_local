# Kiến trúc Mori — trạng thái thực tế

> Tài liệu này mô tả hệ thống **như đang được xây** (code + runtime + số đo thật): mã nguồn, lệnh IPC, dữ liệu trên đĩa, cách chạy.
> Logic AI chi tiết (một lượt hội thoại, prompt, việc nền, trí nhớ, chủ động): [AI_SYSTEM.md](AI_SYSTEM.md).
> [SPEC.md](SPEC.md) là đặc tả đích; [ROADMAP.md](ROADMAP.md) là tiến độ; [adr/](adr/) là lý do của từng quyết định.
> **Cập nhật:** 2026-11-13 — Phase 0–6 qua gate; Phase 7 (sống động & cá nhân hoá) đã code, đã cài thành app, chờ người dùng thử.

---

## 1. Tóm tắt một trang

Mori là cô bạn đồng hành AI **nữ, chibi, dễ thương**, chạy **100% trên laptop** (Windows 11, Core Ultra 9 285H, Arc 140T, 32 GB RAM), nói tiếng Việt. Không cloud, không telemetry, 0 kết nối ra ngoài (đo).

| Chức năng | Đã chọn | Chạy trên | Số đo thật (máy đích) | ADR |
|---|---|---|---|---|
| **Não (LLM)** | Qwen3-8B Q4_K_M · *chế độ nhanh:* Qwen3-4B-Instruct-2507 | llama.cpp b12284 **Vulkan**, `-fa on -ctk q8_0 -ctv q8_0 --cache-ram 1024` | TTFT ấm 373–393 ms, ~11–12 tok/s, RAM **5.8 GB** (trước tối ưu 6.3 GB) | [0008](adr/0008-llm-8b-mac-dinh.md), [0014](adr/0014-toi-uu-tai-nguyen.md) |
| **Tai (STT)** | Whisper **large-v3** q5_0, `-bs 5 -bo 5 -nt` | whisper.cpp v1.9.5 **tự build Vulkan** | Giọng thật 100 câu: **WER 9.3 %**, 1.66 s TB / 2.21 s P95; RAM 2.3 GB, tắt sau 5 phút rảnh | [0007](adr/0007-chon-stt.md), [0011](adr/0011-stt-large-v3.md) |
| **Giọng (TTS)** | ZeroTTS 0.1.5 **int8**, 8 giọng, mặc định `maichi` | ONNX Runtime CPU (Python 3.12, worker JSON Lines) | Chunk đầu P95 181–244 ms, RAM 0.7–1.3 GB | [0005](adr/0005-chon-tts.md) |
| **Hình (Avatar)** | Live2D Cubism SDK for Web 5-r.5; dev dùng mẫu **Koharu** | WebGL2 trong WebView2 (cửa sổ trong suốt riêng) | 30 FPS khi hoạt động, 15 rảnh, 10 ngủ; TTFT +0.9 % (A/B) | [0006](adr/0006-license-live2d.md), [0013](adr/0013-mori-chu-dong-phim-tat.md) |
| **Trí nhớ** | SQLite + FTS5 (không dấu), **Mori tự ghi nhớ** + Hoàn tác; tóm tắt hội thoại dài | Rust (rusqlite) | Xóa có kiểm chứng (test đọc thô file + WAL); AI-06/AI-08 đạt | [0012](adr/0012-tu-ghi-nho.md) |
| **Chủ động** | Chào theo giờ, nhắc nghỉ, bong bóng, phím **Ctrl+Space+Q**, giờ máy trong prompt | Rust + UI avatar | Lời chào thật: gọi tên, đúng buổi, có tiếng sau 75 ms | [0013](adr/0013-mori-chu-dong-phim-tat.md) |

**End-to-end trong app** (`cargo test e2e_gate -- --ignored`, 20 câu giọng thật, lịch sử thật): "thả phím → nghe tiếng" **P95 3.1–3.7 s** (mục tiêu < 4 s); hội thoại liên tục 30 phút 246 lượt, RAM không tăng — MODEL_EVALUATION §5.

---

## 2. Sơ đồ thành phần

```text
┌──────────── mori.exe (Tauri 2.12, WebView2 Fixed 157) ── E:\Mori Data\runtime\app\ (bản cài) ────────────┐
│ Cửa sổ "main" (src/App.tsx, Memories.tsx)      Cửa sổ "avatar" (src/AvatarView.tsx + live2d/avatar.ts)    │
│   chat stream, micro (F2/🎙/Ctrl+Space+Q),        Live2D Koharu: trạng thái, nhép miệng, biểu cảm,          │
│   chọn mic/giọng, Chế độ nhanh, Mori chủ động,    motion ngẫu nhiên, hoạt cảnh theo giờ, đạo cụ, bong bóng   │
│   ⭐ Ghi nhớ, 🔌 Offline, chế độ hiển thị                                                                   │
│        │ src/api.ts: invoke lệnh Rust + nghe event/Channel — UI KHÔNG tự gọi model                         │
│ ───────▼────────────────────────────────────────────────────────────────────────────────────────────────── │
│ Rust core (src-tauri/src/)                                                                                  │
│   lib.rs       lệnh IPC, kiểm đầu vào, GenerationSlot/BackgroundSlot, context(), after_reply, khởi động      │
│   llm.rs       supervisor llama-server, stream SSE, làm ấm, Job Object        integrity.rs  SHA-256 manifest │
│   prompt.rs    SAFETY + personality + ghi nhớ + tóm tắt + lịch sử khối + giờ máy; prompt trích/tóm tắt     │
│   db.rs        SQLite v2: conversations(+summary), messages, settings, memories + FTS5                     │
│   audio.rs     micro cpal → RAM 16 kHz      stt.rs  whisper-server lazy, lọc câu bịa, sửa tên "Mori"        │
│   speech.rs    cắt câu từ stream            tts.rs  worker TTS (lượt/seq/tự dựng lại)   player.rs  loa cpal │
│   avatar.rs    cửa sổ avatar (chế độ, kéo, xuyên chuột)   emotion.rs  nhãn cảm xúc   hotkey.rs  Ctrl+Space+Q │
│   proactive.rs lời chào, nhắc nghỉ, giờ máy                samples.rs  thu mẫu 100 câu (chỉ bản dev)        │
└───────┬───────────────────────────────┬─────────────────────────────────┬───────────────────────────────────┘
        │ HTTP 127.0.0.1:<cổng ngẫu nhiên> │ HTTP 127.0.0.1/<path bí mật>     │ stdin/stdout JSON Lines
        │ Bearer key 128 bit               │                                  │
┌───────▼───────────────┐   ┌──────────────▼──────────┐   ┌──────────────────▼─────────┐
│ llama-server.exe      │   │ whisper-server.exe      │   │ python.exe tts_worker      │
│ Qwen3-8B / 4B, Vulkan │   │ large-v3 q5_0, Vulkan   │   │ ZeroTTS int8, ONNX CPU     │
└───────────────────────┘   └─────────────────────────┘   └────────────────────────────┘
     Tất cả nằm trong Job Object của mori.exe (Mori chết → kernel giết theo)
```

---

## 3. Luồng chính (tóm tắt — chi tiết ở [AI_SYSTEM.md](AI_SYSTEM.md) §4–§8)

1. **Nói:** giữ F2 / 🎙 / Ctrl+Space+Q → `start_listening` (ngắt lời nếu Mori đang nói, mở micro, nạp whisper-server song song) → thả → `stop_listening` (lọc im lặng, whisper, sửa tên) → transcript → **Tự gửi**.
2. **Trả lời:** `send_message` → kiểm đầu vào + `GenerationSlot` → lưu tin → `context()` (ghi nhớ + tóm tắt + lịch sử khối) + giờ máy → stream SSE → UI (Channel) + cắt câu → TTS → loa (RMS → nhép miệng) + nhãn cảm xúc → avatar.
3. **Sau lượt (nền):** tự ghi nhớ → tóm tắt (khi > 24 tin) → làm ấm prompt lượt sau. Người dùng nói/gửi → hủy việc nền ngay.
4. **Chủ động:** model Ready lần đầu (cách lần chào trước ≥ 4 h) → lời chào (việc nền); 90 phút dùng máy liên tục → nhắc nghỉ; avatar tự đổi hoạt cảnh theo giờ + bong bóng.

---

## 4. Vòng đời tiến trình

| Tình huống | Hành vi | Kiểm chứng |
|---|---|---|
| Mở app | Kiểm SHA-256 llama-server + model → spawn (model theo `fast_mode`) → `/health` (≤ 180 s) → warm-up 1 token → **Ready** → làm ấm hội thoại gần nhất → lời chào nếu đủ điều kiện | Ready sau 3.6–6.6 s |
| llama-server chết | Restart, backoff 1-2-4-8-16 s; > 5 lần liên tiếp → **Error** | Kill thủ công → Ready lại sau 6.3 s, PID + cổng mới |
| Bật/tắt Chế độ nhanh | Kill + spawn model kia (không tính là lỗi) | Người dùng thử: 4B Ready sau 3.2 s |
| whisper-server | Spawn khi bắt đầu nghe; chết → spawn lại ở lượt sau; rảnh > 5 phút → kill | `cargo test -- --ignored stt` |
| TTS worker | Nạp lúc mở app nếu bật "Mori nói"; chết / treo > 30 s → dựng lại (backoff, ≤ 5 lần liên tiếp) | Giết worker → sẵn sàng lại 5.4 s |
| Loa | Lỗi stream (rút tai nghe) hoặc đổi thiết bị mặc định → mở lại ở lượt kế | Chờ người dùng thử (gate 7) |
| Thoát app bình thường | `shutdown()` → kill llama-server, chờ ≤ 3 s | — |
| **Mori crash / End Task** | Job Object `KILL_ON_JOB_CLOSE` → kernel tắt mọi tiến trình con | Ép tắt `mori.exe` → llama-server chết theo |
| Đăng nhập Windows | Shortcut trong thư mục Startup chạy bản cài | `scripts/install.ps1` |

---

## 5. Biện pháp bảo mật / riêng tư (ADR-0009, 0010, 0013)

- Mọi service bind **127.0.0.1**, cổng ngẫu nhiên mỗi lần chạy; HTTP client `no_proxy`, timeout kết nối 2 s.
- **API key 128 bit** (`getrandom`) truyền qua env `LLAMA_API_KEY` (không lộ trên command line). Gọi không key → **401**.
- `--no-webui --no-slots --offline`; **xóa mọi biến môi trường `LLAMA_*`** trước khi spawn. Cache prompt (`--cache-ram`) chỉ trong RAM, không ghi đĩa.
- stdout/stderr của tiến trình con → null. Log của Mori chỉ ghi id + độ dài + thời gian, **không nội dung**; giữ 14 ngày.
- Tauri: chỉ `core:event:allow-listen/unlisten`, **không** plugin shell/fs/http; CSP `default-src 'self'`, `connect-src 'self' ipc: http://ipc.localhost`, `base-uri/form-action/object-src 'none'`. Output model hiển thị dạng text (không HTML). Lệnh xóa/khôi phục dữ liệu chỉ nhận từ cửa sổ chat.
- SQLite: query có tham số; `foreign_keys`, `secure_delete`. Quy tắc an toàn nằm trong code (`prompt.rs`), đứng trước `personality.md`.
- **whisper-server** không có API key → mọi route sau `--request-path /<128 bit ngẫu nhiên>`; thiếu path → 404 (test).
- **Micro** chỉ mở giữa bắt đầu/kết thúc nghe; audio chỉ trong RAM, không ghi đĩa (ghi WAV chỉ ở chế độ Thu mẫu của bản dev).
- **Phím tắt toàn cục** (`hotkey.rs`): hook chỉ xét phím Q, Ctrl/Space đọc bằng `GetAsyncKeyState`, không lưu/log/gửi phím; bỏ phím giả lập (`LLKHF_INJECTED`). Nhắc nghỉ dùng `GetLastInputInfo` (chỉ là thời gian rảnh).
- Windows Error Reporting loại trừ `mori.exe`, `llama-server.exe`, `whisper-server.exe` (HKCU); Job Object `DIE_ON_UNHANDLED_EXCEPTION`; dump Crashpad của WebView2 bị xóa lúc khởi động.
- **WebView2 Fixed Version** riêng + **146 luật firewall chặn outbound** cho mọi exe Mori (`scripts/firewall.ps1`, admin) → **0 kết nối ra ngoài loopback** (đo cả bản dev lẫn bản cài).
- ACL `E:\Mori Data` và `D:\Mori_AI_Local`: chỉ người dùng + SYSTEM + Administrators.
- **SHA-256** model/chương trình theo `config/manifest.sha256` (`integrity.rs`): llama-server + model sai thì không chạy; 🔌 Offline hiện từng file.
- Audit gần nhất (Phase 7, 2026-11-13): 0 Critical/High/Medium. Rủi ro chấp nhận: tra DNS do dịch vụ Windows; WebView2 Fixed không tự vá; bản sao lưu giữ dữ liệu đã xóa; lời chào có thể nhắc điều đã nhớ thành tiếng (tắt bằng "Mori chủ động").

---

## 6. Mã nguồn

| Đường dẫn | Nội dung |
|---|---|
| `src/App.tsx`, `src/Memories.tsx`, `src/api.ts`, `src/main.tsx`, `src/index.css` | Khung chat, panel ⭐ Ghi nhớ + sao lưu/khôi phục, lớp gọi lệnh Rust |
| `src/AvatarView.tsx`, `src/live2d/avatar.ts` | Nhân vật: trạng thái, nhép miệng, motion ngẫu nhiên, preset biểu cảm, hoạt cảnh theo giờ, đạo cụ emoji, bong bóng |
| `src/live2d/framework/`, `public/live2d/`, `public/avatar/` | Mã + tài nguyên Live2D (**gitignored**, sinh bằng `node scripts/setup_avatar.mjs`) |
| `src-tauri/src/lib.rs` | Lệnh IPC, state, slot sinh/việc nền, `context`, `after_reply`, tự ghi nhớ, tóm tắt, làm ấm, khởi động, test gate (`--ignored`) |
| `src-tauri/src/llm.rs` | Supervisor llama-server, stream SSE, làm ấm, Job Object |
| `src-tauri/src/prompt.rs` | SAFETY, dựng prompt, giờ máy (`clock_note`), prompt trích ghi nhớ/tóm tắt, `looks_secret` |
| `src-tauri/src/db.rs` | SQLite + migration v2, memories + FTS5, sao lưu/khôi phục |
| `src-tauri/src/audio.rs` | Thu micro (cpal), resample, WAV trong RAM |
| `src-tauri/src/stt.rs` | whisper-server lazy + nhận dạng, lọc câu bịa, sửa tên, test thật + đo WER (`--ignored`) |
| `src-tauri/src/speech.rs` | Cắt câu từ stream LLM, làm sạch văn bản đọc |
| `src-tauri/src/tts.rs`, `src-tauri/workers/tts_worker.py` | Worker TTS (ZeroTTS, JSON Lines), lượt/ngắt lời, tự dựng lại, test gate 50 lượt (`--ignored`) |
| `src-tauri/src/player.rs` | Phát âm thanh (cpal output), mức RMS, mở lại khi đổi thiết bị |
| `src-tauri/src/avatar.rs` | Cửa sổ avatar: chế độ, kéo, xuyên chuột, nhớ vị trí, FPS/ảnh chụp (bản dev) |
| `src-tauri/src/emotion.rs` | Nhãn cảm xúc câu trả lời (emoji + từ khóa) |
| `src-tauri/src/hotkey.rs` | Phím gọi toàn cục Ctrl+Space+Q |
| `src-tauri/src/proactive.rs` | Lời chào theo giờ, nhắc nghỉ, giờ máy |
| `src-tauri/src/integrity.rs` | Kiểm SHA-256 theo manifest (BCrypt, nhớ theo kích thước + mtime) |
| `src-tauri/src/samples.rs` | Thu mẫu 100 câu (chỉ bản dev) |
| `src-tauri/tauri.conf.json`, `capabilities/default.json` | Cửa sổ, CSP, quyền, cờ WebView2 |
| `config/personality.md`, `config/manifest.sha256` | Tính cách Mori (bản mặc định), SHA-256 model/runtime |
| `scripts/install.ps1`, `scripts/firewall.ps1`, `scripts/setup_avatar.mjs` | Cài app + Startup, luật firewall (admin), chép Live2D |
| `scripts/phase0/` | Benchmark & công cụ đo (LLM/STT/TTS/WER/e2e) |
| `tests/fixtures/` | Prompt LLM, câu TTS, manifest + audio STT (audio gitignored) |

**Lệnh IPC**

| Lệnh | Vào | Ra / tác dụng |
|---|---|---|
| `init` | — | `{conversation_id, messages, status, fast_mode, mic, voice_out, tts_voice, tts_voices, avatar_mode, avatar_click_through, proactive}` (mở hội thoại gần nhất) |
| `send_message` | `conversationId, text (≤ 12 000), onEvent: Channel` | tin user đã lưu; Channel: `{type:"delta",text}` … `{type:"done",message,stopped}` |
| `stop_generation` | — | ngắt lời (dừng sinh + dừng nói) |
| `new_conversation` | — | id mới |
| `set_fast_mode` | `enabled` | đổi model |
| `list_mics`, `set_mic` | `id \| null` (≤ 1024 ký tự) | danh sách / chọn micro |
| `start_listening`, `stop_listening`, `cancel_listening` | — | bật micro / transcript / bỏ audio |
| `set_voice_out`, `set_tts_voice` | `enabled` / `voice` (phải có trong `voices/`) | bật tắt "Mori nói" / chọn giọng |
| `get_proactive`, `set_proactive` | `enabled` | setting "Mori chủ động" (mặc định bật) |
| `list_memories`, `star_message`, `update_memory`, `delete_memory`, `delete_all_memories` | id 32 hex, nội dung ≤ 500 ký tự, nhóm hợp lệ | ghi nhớ (xóa chỉ từ cửa sổ chat) |
| `backup_db`, `list_backups`, `restore_backup` | tên phải có trong `backups/` | sao lưu / khôi phục (khôi phục chỉ từ cửa sổ chat) |
| `readiness` | — | danh sách kiểm "sẵn sàng offline" (🔌) |
| `set_avatar_mode`, `set_avatar_click_through`, `avatar_drag`, `avatar_ptt` | `companion\|chat\|focus`, bool | cửa sổ nhân vật |
| `avatar_report`, `avatar_snapshot` | FPS/lỗi; ảnh PNG + tên `[a-z0-9_]{1,24}` | log FPS; chụp ảnh (chỉ bản dev) |
| `list_samples`, `save_sample` | `file` (phải có trong manifest) | chỉ bản dev |

**Sự kiện Rust → UI:** `llm-status` (`starting|ready|restarting|error|stopped`), `tts-status` (`loading|ready|error`), `tts-playing`, `tts-level` (RMS ~30 Hz), `avatar-state` (`idle|listening|thinking`), `avatar-mode`, `avatar-ptt`, `memory-saved`, `hotkey-ptt`, `avatar-emotion` (chỉ nhãn), `avatar-say`, `proactive-message` (`[conversationId, Message]`), `proactive-changed`.

---

## 7. Dữ liệu & runtime trên đĩa (`E:\Mori Data\`, ADR-0002)

```text
E:\Mori Data\
├── models\llm\         Qwen3-8B, Qwen3-4B-Instruct-2507 (+ Qwen3-4B, Qwen3.5-4B đã đo, không dùng)
├── models\stt\         ggml-large-v3-q5_0.bin (app), large-v3-turbo, phowhisper-large-q5_0, small
├── models\tts\         zerotts-int8\ (dùng), hf\ (cache HF: ZeroTTS fp32, VieNeu, Korva)
├── runtime\app\        mori.exe — bản cài (scripts/install.ps1)
├── runtime\llama.cpp\  b12284-cpu\, b12284-vulkan\ (app dùng bản vulkan)
├── runtime\whisper.cpp\ src-v1.9.5\, build-vulkan\, v1.9.5-vulkan\ (bản dùng)
├── runtime\python\     tts-venv\ (Python 3.12), interpreters\, uv-cache\
├── runtime\webview2\   Microsoft.WebView2.FixedVersionRuntime.157.0.4341.57.x64\
├── avatar\             sdk\CubismSdkForWeb-5-r.5\, models\koharu_haruto\, downloads\
├── backups\            mori-<thời gian>.db
├── database\mori.db    SQLite (WAL)
├── logs\               mori.<ngày>.log (14 ngày), avatar_snapshot*.png (bản dev)
└── settings\personality.md  (tùy chọn — ghi đè bản trong repo)
```

Ghi đè data root bằng biến môi trường `MORI_DATA_DIR`. `E:\Memora Data` là dự án khác — Mori không ghi vào đó. Dữ liệu WebView2 (localStorage, Crashpad): `%LOCALAPPDATA%\dev.mori.companion\`.

---

## 8. Cách chạy

```powershell
npm install                      # lần đầu
node scripts/setup_avatar.mjs    # lần đầu / sau khi đổi SDK hay model: chép Live2D vào thư mục gitignore
npm run tauri dev                # chạy bản dev; app tự quản lý llama-server (tắt bản cài trước)
cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test   # 34 test
cargo test e2e_gate -- --ignored --nocapture   # "thả phím → nghe tiếng" (cần model thật, tắt app)
cargo test llm_gate -- --ignored --nocapture   # AI-06, AI-08, tự ghi nhớ, giờ máy (cần model thật)
cargo test tts_gate -- --ignored --nocapture   # 50 lượt có ngắt lời
npx --no-install tsc --noEmit
```

**Cài thành app:**
```powershell
npm run tauri build -- --no-bundle                                        # target\release\mori.exe (tự chứa: UI, Live2D, worker TTS)
powershell -ExecutionPolicy Bypass -File scripts\install.ps1              # chép vào E:\Mori Data\runtime\app\, shortcut Start Menu + Startup
powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -NoStartup   # không tự chạy khi đăng nhập
powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -Uninstall   # gỡ app + shortcut, giữ dữ liệu
```
Đường dẫn exe mới chưa có luật firewall → script tự chạy `firewall.ps1` (UAC). Ghim taskbar: Windows 11 không cho chương trình tự ghim → chuột phải biểu tượng Mori trên taskbar → Ghim. Tắt tự chạy: Task Manager → Startup apps. Không chạy song song bản cài với `npm run tauri dev` (2 bộ model).

- Đo FPS khi vẽ liên tục: `VITE_AVATAR_BENCH=1 npm run tauri dev` (log `avatar_fps`). Chụp lần lượt 7 biểu cảm + 8 hoạt cảnh: `VITE_AVATAR_DEMO=1 npm run tauri dev` → `E:\Mori Data\logs\avatar_snapshot_<tên>.png`.
- `tauri dev` lỗi dev server: cổng 1420 còn bị Vite cũ giữ → tắt tiến trình `node … vite.js` đó. Cổng 8080 bị phần mềm khác chiếm.
- Công cụ đo: `scripts/phase0/` — `hw_inventory.ps1`, `bench_llm.ps1`, `bench_stt.ps1`, `bench_tts.py`, `wer.py`, `run_llm_prompts.py`, `quantize_zerotts_int8.py`, `e2e_latency.py`.

---

## 9. Giới hạn đã biết

- **RAM**: đủ bộ khi đang nói chuyện ~9–10 GB (llama 5.8 + whisper 2.3 + TTS 0.7–1.3 + WebView 0.6–1.0 GB); whisper tự nhả sau 5 phút rảnh. Máy nhiều ứng dụng nặng → swap; máy không GPU → Chế độ nhanh (4B).
- **STT**: câu dài 7–11 s mất 2.7–2.8 s nhận dạng (e2e có thể > 4 s); từ tiếng Anh/thuật ngữ còn sai (nhóm thuật ngữ WER 16.9 %).
- **LLM 8B**: đôi khi xưng "mình"; gần như câu nào cũng thêm 😊; không sửa đáng tin lỗi nhận dạng khi ghi nhớ (người dùng sửa trong panel).
- **Avatar**: Koharu là model mẫu (chỉ dùng dev, ADR-0006); cảnh chăn đệm/tắm/thay đồ chỉ xấp xỉ bằng emoji → [CHARACTER_SPEC.md](CHARACTER_SPEC.md). Cảm xúc dò bằng emoji + từ khóa.
- Chưa có danh sách hội thoại cũ trong UI; chưa chỉnh được tốc độ đọc (ZeroTTS không có tham số).
- Chờ người dùng thử (gate 7): phím tắt, hoạt cảnh, chào/nhắc nghỉ, rút/cắm tai nghe, sleep/resume; đo lại 30 FPS khi hoạt động.
