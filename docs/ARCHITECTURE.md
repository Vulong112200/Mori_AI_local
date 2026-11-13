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
| **Tai (STT)** | Whisper **large-v3** q5_0 | whisper.cpp v1.9.5 **tự build Vulkan** | **Giọng thật 100 câu (thu lại, micro đã sửa): WER 9.3 %** (lần thu đầu 20.6 %), 1.77 s TB / 2.41 s P95 | [0007](adr/0007-chon-stt.md), [0011](adr/0011-stt-large-v3.md) | Đang chạy trong app (Phase 2) |
| **Giọng (TTS)** | ZeroTTS 0.1.5 **int8**, 8 giọng, mặc định `maichi` | ONNX Runtime CPU (Python 3.12, worker JSON Lines) | Chunk đầu P95 181–244 ms (50 lượt), RAM 1.1 GB | [0005](adr/0005-chon-tts.md) | Chạy trong app (Phase 3, gate đạt) |
| **Hình (Avatar)** | Live2D Cubism SDK for Web 5-r.5; dev dùng mẫu **Koharu** | WebGL2 trong WebView2 (cửa sổ trong suốt riêng) | 30 FPS khi hoạt động, 15 khi rảnh; TTFT +0.9 % (A/B) | [0006](adr/0006-license-live2d.md) | Chạy trong app (Phase 5) |
| **Trí nhớ** | SQLite + FTS5 (không dấu), người dùng duyệt; tóm tắt hội thoại dài | Rust (rusqlite) | Xóa có kiểm chứng (test đọc thô file + WAL) | SPEC §11 | Tự ghi nhớ (ADR-0012), chờ người dùng thử |

**End-to-end đo thật** — script Phase 0 (6 lượt): TB 2.63 s, max 3.59 s, 0 kết nối ra ngoài loopback. **Trong app** (`cargo test e2e_gate -- --ignored`, 20 câu giọng thật, lịch sử thật): **P95 3.45–3.72 s**, max 3.87 s (mục tiêu ≤ 4 s) — MODEL_EVALUATION §5.1.

---

## 2. Sơ đồ thành phần

```text
┌──────────────────────── Mori.exe (Tauri 2.12, WebView2) ─────────────────────────┐
│ Frontend React 19 + TS 7 + Tailwind 4 (src/)                                      │
│   App.tsx: chat, trạng thái model, nút Dừng, Chế độ nhanh, micro (giữ F2), chọn mic │
│   api.ts : chỉ gọi lệnh Rust (invoke) + nhận Channel/event — KHÔNG tự gọi model   │
│        │ invoke: init · send_message · stop_generation · set_fast_mode ·          │
│        │         new_conversation · list_mics · set_mic · start/stop/cancel_listening │
│        │         set_voice_out · set_tts_voice · list/star/update/delete(_all)_memor… │
│        │         backup_db · list_backups · restore_backup                        │
│        │         event: "llm-status" · "tts-status" · "tts-playing" · "tts-level"  │
│   Memories.tsx: panel Ghi nhớ (duyệt/sửa/xóa) + sao lưu/khôi phục                 │
│ ───────▼──────────────────────────────────────────────────────────────────────── │
│ Rust core (src-tauri/src/)                                                        │
│   lib.rs    lệnh IPC, kiểm tra đầu vào, slot 1-lượt-sinh, logging                 │
│   llm.rs    supervisor llama-server + stream SSE + Job Object                     │
│   audio.rs  cpal thu micro → RAM, resample 16 kHz, WAV trong RAM                  │
│   stt.rs    whisper-server lazy (tắt sau 5 phút rảnh), request-path bí mật       │
│   prompt.rs an toàn + personality + memory + tóm tắt + cửa sổ lịch sử theo khối   │
│   memory.rs nhận lệnh "nhớ là …" · auto_remember (lib.rs) tự trích ghi nhớ        │
│   speech.rs cắt câu từ stream LLM · tts.rs worker TTS · player.rs cpal phát       │
│   db.rs     SQLite v2: conversations(+summary), messages, settings, memories+FTS5 │
└───────┬───────────────────────────────────────────────────────────────────────────┘
        │ HTTP 127.0.0.1:<cổng ngẫu nhiên>, Bearer <key ngẫu nhiên 128 bit>
┌───────▼───────────────┐   (Phase 2, đang chạy)      (Phase 3)
│ llama-server.exe      │   whisper-server.exe        TTS worker Python
│ Qwen3-8B / 4B, Vulkan │   large-v3 q5_0, Vulkan     ZeroTTS int8, JSON Lines stdio
└───────────────────────┘
```

---

## 3. Luồng một tin nhắn (Phase 1 — đang chạy)

1. Người dùng gõ → `send_message(conversation_id, text, Channel)`.
2. Rust kiểm tra: không rỗng, ≤ 12.000 ký tự, model đang **Ready**, không có lượt sinh khác (`GenerationSlot`).
3. Lưu tin nhắn user vào SQLite (khớp lệnh "nhớ là …" → tạo đề xuất ghi nhớ, báo UI qua Channel) → `context()`: memory đã duyệt + tóm tắt + các tin chưa tóm tắt.
4. `prompt::build`: `[system = SAFETY + personality + ghi nhớ đã duyệt (≤ 2.000 ký tự, vượt thì FTS5 theo tin mới) + tóm tắt] + lịch sử chưa tóm tắt` (tối đa 24 tin, ≤ 13.000 ký tự, **bỏ tin cũ theo khối 12** để prefix ổn định, luôn giữ tin mới nhất). Ngân sách token đo bằng tokenizer: tiếng Việt 2.8–3.5 ký tự/token → luôn vừa ctx 8192.
5. `llm.chat`: POST `/v1/chat/completions` (`stream`, `cache_prompt`, `max_tokens` 1024) → parse SSE → gửi từng `Delta` qua Channel → UI hiện chữ chảy.
6. Bấm **Dừng** → `Notify` → drop stream (llama-server ngừng sinh). Câu trả lời dở dang vẫn được lưu.
7. Lưu câu trả lời → gửi `Done`.
8. Nền (`after_reply`): nếu > 24 tin chưa tóm tắt → nhờ model tóm tắt phần cũ (giữ 12 tin gần nhất) vào `conversations.summary`; bị hủy ngay khi người dùng nói/gửi. Sau đó **làm ấm** prompt lượt sau (`Llm::prefill`, 1 request/lúc, llama-server `-np 1`); cũng làm ấm khi model vừa Ready.

**Ghi nhớ (Phase 4, [ADR-0012](adr/0012-tu-ghi-nho.md)):** sau mỗi lượt, `auto_remember` (nền) nhờ model trích điều lâu dài người dùng vừa nói → lưu `approved` ngay → sự kiện `memory-saved` → UI "Mori đã nhớ: … [Hoàn tác]". Lệnh "nhớ là …" (`memory.rs`) và ⭐ lưu ngay. Panel ⭐ Ghi nhớ: sửa/xóa (xóa hẳn: `secure_delete` + rebuild FTS + checkpoint WAL; xóa toàn bộ thêm `VACUUM`). Sao lưu = `VACUUM INTO E:\Mori Data\backups\mori-<thời gian>.db`; khôi phục chỉ nhận tên trong danh sách đó (kiểm `integrity_check`), tự sao lưu bản hiện tại trước. Việc nền (tự nhớ, tóm tắt) bị hủy khi người dùng nói/gửi; cache prompt của llama-server nằm trong RAM nên việc nền không phá cache hội thoại.

**Giọng nói vào (Phase 2 — đang chạy):** giữ **F2** (hoặc bấm 🎙) → `start_listening`: cpal mở micro (thread riêng, PCM vào RAM, tối đa 120 s) **và** nạp whisper-server song song → thả phím → `stop_listening`: tắt micro (log `mic_stop_ms`), bỏ qua nếu < 0.3 s hoặc RMS < 0.003 (chặn Whisper bịa chữ khi im lặng), resample 16 kHz → WAV trong RAM → POST multipart `/inference` → **Tự gửi** bật (mặc định): gửi luôn như hội thoại; tắt: transcript vào ô nhập để sửa trước khi gửi. Giải mã `-bs 5` (như lúc đo WER). Audio bị drop ngay sau nhận dạng. Esc = hủy. Đo (giọng tổng hợp, file Phase 0): lần đầu 5.8 s (gồm nạp model), sau đó 0.93 s.

**Giọng nói ra (Phase 3 — đang chạy):** mỗi `send_message` mở **lượt** mới (`tts.new_turn()`, dừng phát). Delta của LLM → `speech::Splitter` (cắt ở `. ? ! … : ;`/xuống dòng, câu > 220 ký tự cắt ở dấu phẩy; bỏ emoji, markdown, khối code) → `tts.speak(turn, câu)` → worker Python (ZeroTTS int8, JSON Lines, chunk PCM16 base64 48 kHz có `seq`) → `player` (cpal output, resample về tần số loa) → loa. Ngắt lời (`interrupt()`: nút Dừng/Ngắt, giữ F2, tắt "Mori nói") = notify LLM + tăng turn + `cancel` mọi request + xóa hàng đợi phát → audio lượt cũ bị bỏ ở `tts.rs`. Sự kiện `tts-level` (~30 Hz) cho lip-sync Phase 5. Đo: first audio P95 244 ms; worker nạp 7.3 s lúc mở app (khi bật "Mori nói"); RAM: llama-server 6.2 GB + TTS 1.1 GB (máy còn 6.4/31.4 GB, commit trống 1.9 GB).

Luồng giọng nói đầy đủ (Phase 2–3, đã đo e2e bằng script): giữ phím nói → cpal 16 kHz → whisper-server → (luồng trên) → cắt câu đầu → ZeroTTS stream → rodio phát + RMS cho lip-sync.

---

## 4. Vòng đời llama-server (`llm.rs`)

| Tình huống | Hành vi | Kiểm chứng |
|---|---|---|
| Mở app | Spawn với model theo setting `fast_mode` → chờ `/health` (≤ 180 s) → warm-up 1 token (biên dịch shader Vulkan) → **Ready** | Ready sau 6–9 s |
| llama-server chết | Restart, backoff 1-2-4-8-16 s; > 5 lần liên tiếp → **Error** (chờ người dùng đổi model) | Kill thủ công → Ready lại sau 6.3 s, PID + cổng mới |
| Bật/tắt Chế độ nhanh | Kill + spawn model kia (không tính là lỗi) | — (người dùng thử) |
| Thoát app bình thường | `shutdown()` → kill, chờ ≤ 3 s | — |
| whisper-server | Spawn khi bắt đầu nghe; chết → spawn lại ở lượt sau; rảnh > 5 phút → kill (giải phóng RAM/VRAM) | Test `cargo test -- --ignored stt` |
| **Mori crash / End Task** | Job Object `KILL_ON_JOB_CLOSE` → kernel tắt llama-server | Ép tắt `mori.exe` → llama-server chết theo |

---

## 5. Biện pháp bảo mật / riêng tư đã có (ADR-0009)

- Mọi service bind **127.0.0.1**, cổng ngẫu nhiên mỗi lần chạy; HTTP client `no_proxy`, timeout kết nối 2 s.
- **API key 128 bit** (`getrandom`) truyền qua env `LLAMA_API_KEY` (không lộ trên command line). Gọi không key → **401**.
- `--no-webui --no-slots --offline`; **xóa mọi biến môi trường `LLAMA_*`** trước khi spawn (chặn bật tools chạy shell, log file, tải model qua env). Đã thử: đặt `LLAMA_ARG_LOG_FILE` → file không được tạo.
- stdout/stderr của llama-server → null (log của nó có thể chứa prompt). Log của Mori chỉ ghi id + độ dài, **không nội dung**; giữ 14 ngày.
- Tauri: chỉ `core:event:allow-listen/unlisten` (lệnh app và `Channel` không cần permission), **không** plugin shell/fs/http; CSP `default-src 'self'`, `connect-src ipc:`, `base-uri/form-action/object-src 'none'`. Model output hiển thị dạng text (không HTML).
- SQLite: mọi query có tham số; `foreign_keys`, `secure_delete`.
- Quy tắc an toàn nằm trong code (`prompt.rs`), đứng trước `personality.md`.
- **whisper-server** không có API key → mọi route nằm sau `--request-path /<128 bit ngẫu nhiên>` (kể cả `/load` nạp model tùy ý); gọi thiếu path → 404 (test). Path nằm trên command line (tiến trình cùng user đọc được — chấp nhận, cùng user thì đã đọc được bộ nhớ Mori). Không `--convert` → không file tạm; stdout/stderr → null; Job Object.
- **Micro** chỉ mở giữa `start_listening` và `stop/cancel_listening`; audio chỉ trong RAM, không ghi đĩa; log chỉ ghi thời lượng + số ký tự transcript.
- Khởi động gọi `WerAddExcludedApplication` cho `mori.exe` + `llama-server.exe` + `whisper-server.exe` (HKCU) → Windows Error Reporting không lấy dump (có thể chứa hội thoại). Loại trừ theo **tên**, lưu vĩnh viễn trong HKCU (cũng áp cho `llama-server.exe` của dự án khác); gỡ: `reg delete "HKCU\Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications" /v llama-server.exe /f` (tương tự `mori.exe`).
- WebView2 (runtime Evergreen của Microsoft) tự mở 1–2 kết nối HTTPS ra Microsoft lúc khởi động; cờ `--disable-background-networking …` giảm còn 1, không chặn hết → [ADR-0010](adr/0010-webview2-ket-noi-ra-ngoai.md), chặn bằng Fixed Version + firewall ở Phase 6. **App chưa đạt "0 kết nối ra ngoài"** (số 0 ở §1 là đo pipeline bằng script, không có WebView).
- WebView2 có Crashpad riêng (`%LOCALAPPDATA%\dev.mori.companion\EBWebView\Crashpad`), không tắt được bằng `--disable-crash-reporter` (đã thử) → khởi động xóa `reports/` + `attachments/`. Còn hở: dump renderer có thể được upload trước lần khởi động sau, tùy cài đặt diagnostic data của Windows.

Còn mở (xem ROADMAP "Bảo mật — để sau"): ACL thư mục `E:\Mori Data`, kiểm SHA-256 binary/model lúc chạy.

---

## 6. Mã nguồn

| Đường dẫn | Nội dung |
|---|---|
| `src/App.tsx`, `src/api.ts`, `src/main.tsx`, `src/index.css` | Giao diện chat |
| `src-tauri/src/lib.rs` | Lệnh IPC, state, logging, khởi động |
| `src-tauri/src/llm.rs` | Supervisor llama-server, stream SSE, Job Object, test SSE/key/job |
| `src-tauri/src/audio.rs` | Thu micro (cpal), resample, WAV, test |
| `src-tauri/src/stt.rs` | whisper-server lazy + nhận dạng, test thật + đo WER (`--ignored`) |
| `src-tauri/src/samples.rs` | Thu mẫu 100 câu (chỉ bản dev) |
| `src-tauri/src/tts.rs`, `src-tauri/workers/tts_worker.py` | Worker TTS (Python ZeroTTS, JSON Lines), lượt/ngắt lời, test gate 50 lượt (`--ignored`) |
| `src-tauri/src/player.rs` | Phát âm thanh (cpal output), mức RMS |
| `src-tauri/src/speech.rs` | Cắt câu từ stream LLM, làm sạch văn bản đọc, test |
| `src-tauri/src/prompt.rs` | SAFETY + dựng prompt, test cắt lịch sử |
| `src-tauri/src/db.rs` | SQLite + migration, test |
| `src-tauri/tauri.conf.json`, `capabilities/default.json` | Cửa sổ, CSP, quyền |
| `config/personality.md` | Tính cách Mori (bản mặc định) |
| `scripts/phase0/` | Benchmark & công cụ Phase 0 (xem §8) |
| `tests/fixtures/` | Prompt LLM, câu TTS, manifest STT, system prompt mẫu |

**Lệnh IPC**

| Lệnh | Vào | Ra |
|---|---|---|
| `init` | — | `{conversation_id, messages, status, fast_mode, mic}` (mở hội thoại gần nhất) |
| `send_message` | `conversationId, text, onEvent: Channel` | tin nhắn user đã lưu; Channel nhận `{type:"delta",text}` … `{type:"done",message,stopped}` |
| `stop_generation` | — | — |
| `set_fast_mode` | `enabled: bool` | — (đổi model) |
| `new_conversation` | — | id mới |
| `list_mics` | — | `[{id, name}]` |
| `set_mic` | `id: string \| null` (≤ 1024 ký tự; null = mặc định) | — |
| `start_listening` | — | — (lỗi nếu không mở được micro) |
| `stop_listening` | — | transcript (rỗng nếu im lặng) |
| `cancel_listening` | — | — (bỏ audio) |
| `set_voice_out` | `enabled: bool` | — (bật: nạp worker; tắt: ngắt lời) |
| `set_tts_voice` | `voice` (phải là thư mục trong `voices/`) | — |
| `list_samples`, `save_sample` | `file` (phải có trong manifest) | chỉ bản dev: danh sách câu / `{secs, heard}` |

Sự kiện Rust → UI: `llm-status`, `tts-status` (`loading`/`ready`/`error`), `tts-playing` (bool), `tts-level` (RMS ~30 Hz). `init` trả thêm `voice_out`, `tts_voice`, `tts_voices`. `stop_generation` = ngắt lời (dừng sinh + dừng nói).
| sự kiện `llm-status` | — | `{state: starting|ready|restarting|error|stopped, ...}` |

---

## 7. Dữ liệu & runtime trên đĩa (`E:\Mori Data\`, ADR-0002)

```text
E:\Mori Data\
├── models\llm\        Qwen3-8B, Qwen3-4B-Instruct-2507 (+ Qwen3-4B, Qwen3.5-4B đã đo, không dùng)
├── models\stt\        ggml-large-v3-q5_0.bin (app), large-v3-turbo, phowhisper-large-q5_0, small; src (nguồn PhoWhisper)
├── models\tts\        zerotts-int8\ (dùng), hf\ (cache HF: ZeroTTS fp32, VieNeu, Korva)
├── runtime\llama.cpp\ b12284-cpu\, b12284-vulkan\ (app dùng bản vulkan)
├── runtime\whisper.cpp\ src-v1.9.5\, build-vulkan\, v1.9.5-vulkan\ (bản dùng)
├── runtime\python\    tts-venv\ (Python 3.12, zerotts/vieneu/korvatts/onnx), interpreters\, uv-cache\
├── avatar\           sdk\CubismSdkForWeb-5-r.5\, models\koharu_haruto\, downloads\ (zip gốc, sha256 trong ROADMAP)
├── backups\          mori-<thời gian>.db (sao lưu từ panel ⭐ Ghi nhớ)
├── database\mori.db   SQLite (WAL)
├── logs\mori.<ngày>.log
└── settings\personality.md  (tùy chọn — ghi đè bản trong repo)
```

Ghi đè data root bằng biến môi trường `MORI_DATA_DIR`. `E:\Memora Data` là dự án khác — Mori không ghi vào đó.

---

## 8. Cách chạy

```powershell
npm install                      # lần đầu
node scripts/setup_avatar.mjs    # lần đầu / sau khi đổi SDK hay model: chép Live2D vào thư mục gitignore
npm run tauri dev                # chạy app; app tự quản lý llama-server
cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test   # 29 test
cargo test e2e_gate -- --ignored --nocapture   # đo "thả F2 → nghe tiếng" (cần model thật)
cargo test llm_gate -- --ignored --nocapture   # AI-06, AI-08, tự ghi nhớ (cần model thật)
npx tsc --noEmit
```

- Lần đầu mở: trạng thái "Mori đang thức dậy…" ~6–9 s (nạp 8B + warm-up).
- Cửa sổ avatar (label `avatar`): `src/AvatarView.tsx` + `src/live2d/avatar.ts`; mã Live2D chỉ nạp trong cửa sổ này (lazy). Đo FPS khi vẽ liên tục: `VITE_AVATAR_BENCH=1 npm run tauri dev` (log `avatar_fps`).
- Nếu `tauri dev` báo lỗi dev server: cổng 1420 còn bị Vite cũ giữ → tắt tiến trình `node … vite.js` đó.
- Công cụ Phase 0: `scripts/phase0/` — `hw_inventory.ps1`, `bench_llm.ps1`, `bench_stt.ps1`, `bench_tts.py`, `wer.py`, `run_llm_prompts.py`, `quantize_zerotts_int8.py`, `e2e_latency.py` (cách gọi trong docstring/đầu file và [CLAUDE.md](../CLAUDE.md)).

---

## 9. Giới hạn đã biết

- **RAM chật với 8B**: còn ~6.8 GB RAM / 0.8 GB commit trống khi app chạy (pagefile tự nới). Thêm STT + TTS ở Phase 2–3 phải đo lại; nếu swap nhiều → Chế độ nhanh.
- Số đo STT/e2e dùng **giọng tổng hợp**; cần giọng thật trước gate Phase 2.
- Cắt lịch sử theo ký tự (ước lượng ~2 ký tự/token), chưa tóm tắt hội thoại (Phase 4).
- Model đôi khi chèn emoji → phải lọc trước TTS (Phase 3).
- Chưa có danh sách hội thoại cũ trong UI (chỉ "Cuộc trò chuyện mới" + mở hội thoại gần nhất).
