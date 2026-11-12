# Roadmap & tiến độ

> Nguồn sự thật duy nhất cho tiến độ. Tick `[x]` khi xong. Cập nhật dòng **Trạng thái** của phase. Gate chỉ được tick qua `/gate <n>`.
> Ký hiệu: `[ ]` chưa làm · `[~]` đang làm · `[x]` xong · `[!]` bị chặn (ghi lý do) · 🔐 cần người dùng duyệt (tải/cài).

**Phase hiện tại:** 3
**Cập nhật:** 2026-11-13

## Bàn giao (đọc trước khi làm tiếp)

- **Đã xong:** Phase 0 (đo & chọn model), Phase 1 (chat văn bản), Phase 2 (giọng nói vào).
- **Đang làm:** Phase 3 (giọng nói ra) — code xong, 2 tiêu chí gate đạt bằng test tự động (`cargo test tts_gate -- --ignored`), audit 0 Critical/High (Medium/Low đã sửa). **Còn:** người dùng nghe thử trong app + đo e2e "thả F2 → nghe tiếng" ≤ 4 s, rồi `/gate 3`. Tốc độ đọc: hoãn (ZeroTTS không hỗ trợ).
- **Sửa 2026-11-12:** app sập lúc mở trên WebView2 157 (`scrollIntoView` trả Promise vào `useEffect`) — đã sửa `App.tsx`. Capability đã thu hẹp → lượt thử gate cũng kiểm chứng streaming.
- **Phase 0–2 đã qua gate.** Ngoại lệ còn mở: WebView2 tự kết nối ra Microsoft (ADR-0010, xử lý Phase 6).
- **Kiến trúc thực tế:** [ARCHITECTURE.md](ARCHITECTURE.md). Quyết định: ADR [0005](adr/0005-chon-tts.md) TTS, [0006](adr/0006-license-live2d.md) Live2D, [0007](adr/0007-chon-stt.md) STT, [0008](adr/0008-llm-8b-mac-dinh.md) LLM, [0009](adr/0009-bao-mat-llama-server.md) bảo mật, [0010](adr/0010-webview2-ket-noi-ra-ngoai.md) WebView2.
- **Chạy app:** `npm run tauri dev` (xem ARCHITECTURE §8).

---

## Phase 0 — Khảo sát & khả thi

**Trạng thái:** Hoàn thành (2026-11-12)

### 0.1. Khung dự án
- [x] Rà soát spec v0 → [SPEC_REVIEW.md](SPEC_REVIEW.md), [SPEC.md](SPEC.md)
- [x] Khung tài liệu, ADR, framework Claude (`CLAUDE.md`, `.claude/`)
- [x] Script Phase 0 + fixtures

### 0.2. Môi trường
- [x] Chạy `scripts/phase0/hw_inventory.ps1` → điền [HARDWARE_REPORT.md](HARDWARE_REPORT.md) (lần 1)
- [x] Đo lại RAM/commit trống (lần 2: 7.8 GB RAM / 3.0 GB commit trống, pagefile tự nới) → không cần đổi pagefile
- [x] 🔐 Cài `uv` (0.13.7, winget)
- [x] 🔐 Tạo `E:\Mori Data\` theo cấu trúc SPEC §16

### 0.3. LLM
- [x] 🔐 Tải llama.cpp bản Windows: CPU + Vulkan b12284 (sha256 OK) → `E:\Mori Data\runtime\llama.cpp\b12284-{cpu,vulkan}\`
- [x] 🔐 Tải Qwen3-4B Q4_K_M (baseline), ghi sha256
- [x] `/bench llm` CPU
- [x] `/bench llm` Vulkan — nhanh hơn CPU 7× (pp) / 1.5–3× (tg), ổn định
- [x] ~~🔐 SYCL build~~ — bỏ qua: Vulkan đã đạt mục tiêu
- [x] 🔐 Tải Qwen3-4B-Instruct-2507 và Qwen3.5-4B → bench Vulkan
- [x] Chạy bộ [llm_prompts.md](../tests/fixtures/llm_prompts.md) (AI-01..06) trên 3 model — `scripts/phase0/run_llm_prompts.py`; Claude chấm sơ bộ
- [x] Đo TTFT cache lạnh/ấm qua `llama-server` (Qwen3-4B Vulkan: 0.60 s / 0.23 s)
- [ ] **Người dùng** đọc `scripts/phase0/out/llm_answers_*.md` và chấm điểm tiếng Việt
- [x] 🔐 Thử Qwen3-8B Q4_K_M (Qwen chính thức, 4.68 GB, sha256 OK) trên Vulkan — TTFT 1.16 / 0.40 s, 11.8 tok/s, chất lượng 4* (tốt hơn 4B)
- [x] Người dùng chọn: **Qwen3-8B mặc định, 4B chế độ nhanh** → [ADR-0008](adr/0008-llm-8b-mac-dinh.md) (thay ADR-0004)
- [x] `/adr` chọn model + backend LLM — [ADR-0004](adr/0004-chon-llm.md) **Chấp nhận**

### 0.4. STT
- [ ] (Hoãn, không chặn gate Phase 0) **Người dùng** thu âm giọng thật theo [stt_manifest.csv](../tests/fixtures/stt_manifest.csv): ~20 câu trước khi chốt ADR STT cuối cùng, đủ 100 câu trước gate Phase 2
- [x] 🔐 Tải whisper small + large-v3-turbo q5_0 (ggml, sha256 OK)
- [x] `/bench stt` sơ bộ trên **giọng tổng hợp** (18 câu VieNeu, 16 kHz) với whisper-cli b5454 (CPU) — turbo WER 8.4 % nhưng 10.3 s; small 2.3 s / WER 17 %
- [x] 🔐 Build whisper.cpp v1.9.5 với Vulkan (CMake 4.4.4, Vulkan SDK 1.4.363, MSVC Build Tools 2022) → turbo trên GPU: **WER 7.8 %, 1.53 / 1.77 s**
- [ ] `/bench stt` trên giọng thật
- [ ] 🔐 Thử convert PhoWhisper → ggml, bench nếu được
- [x] `/adr` chọn model STT — [ADR-0007](adr/0007-chon-stt.md) **Chấp nhận**: turbo q5_0 + Vulkan

### 0.5. TTS
- [x] Kiểm tra license VieNeu-TTS v3 — Apache-2.0 (cả Turbo và Nano)
- [x] 🔐 uv venv Python 3.12 (`E:\Mori Data\runtime\python\tts-venv`) + korvatts 0.1.3, zerotts 0.1.5, vieneu 3.8.4; weights tải với `HF_HUB_DISABLE_SYMLINKS=1`
- [x] `/bench tts` cho cả 3 engine (+ VieNeu Nano, Turbo int8, Korva 8 bước)
- [x] **Người dùng** nghe: thích giọng **ZeroTTS**; VieNeu Turbo int8 không méo
- [x] Tăng tốc ZeroTTS → RTF < 1: **int8, 4 luồng → first audio 0.06 s, RTF 0.31** (`scripts/phase0/quantize_zerotts_int8.py`; cài thêm gói `onnx` vào venv)
- [x] **Người dùng** nghe ZeroTTS int8: ổn (hơi ngọng, giọng miền Nam vài chỗ, dễ thương)
- [x] Kiểm tra chạy offline — `HF_HUB_OFFLINE=1` + đo e2e không có kết nối nào ra ngoài loopback (rút mạng thật: test Phase 6)
- [x] `/adr` chọn TTS — [ADR-0005](adr/0005-chon-tts.md) **Chấp nhận: ZeroTTS int8**

### 0.6. License
- [x] Rà license Live2D Cubism SDK for Web + model mẫu → [ADR-0006](adr/0006-license-live2d.md) **Chấp nhận** (nhân vật nữ chibi; dev dùng Koharu; không cho nạp model tùy ý; Cubism Core không commit)
- [x] Ghi license từng model vào MODEL_EVALUATION.md (Qwen Apache-2.0, Whisper MIT, VieNeu v3 Apache-2.0, ZeroTTS MIT, Korva Apache-2.0)

### Gate Phase 0
- [x] Có ít nhất một bộ LLM + STT + TTS chạy **offline** trên máy đích (e2e, 0 kết nối ra ngoài)
- [x] Số đo pp/tg/TTFT, WER/CER, first-audio/RTF đã ghi trong MODEL_EVALUATION.md
- [x] Đã có ADR cho LLM, STT, TTS, Live2D (0008, 0007, 0005, 0006)
- [x] Ước tính "thả PTT → nghe tiếng đầu tiên" từ số đo ≤ 4 s — **đo thật e2e: TB 2.63 s, max 3.59 s**

---

## Phase 1 — Lõi AI local (chat văn bản)

**Trạng thái:** Hoàn thành (2026-11-12)

- [x] Scaffold Tauri 2.12 + React 19 + TS 7 + Tailwind 4 + Vite 8 (viết tay, không dùng template)
- [x] Tauri: CSP, capability tối thiểu (`core:default`), không plugin shell/fs/http
- [x] Rust process manager (`src-tauri/src/llm.rs`): khởi động/health/warm-up/restart (backoff)/tắt `llama-server`; loopback, cổng + API key ngẫu nhiên, `--no-webui`
- [x] Security audit Phase 1 (agent `security-auditor`: 0 Critical/High, 3 Medium, 5 Low) → sửa theo [ADR-0009](adr/0009-bao-mat-llama-server.md): xóa env `LLAMA_*`, key qua env (getrandom), `--no-slots --offline`, Job Object, `max_tokens`, cắt lịch sử theo ký tự, 1 lượt sinh/lúc, timeout HTTP, CSP chặt hơn, log 14 ngày. Kiểm chứng: 401 khi không key; key không ở command line; env bẫy không có tác dụng; kill `mori.exe` → llama-server chết theo
- [x] Kiểm tra tự động: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (7 test), `tsc --noEmit` — sạch
- [x] Chat UI streaming token (SSE → Tauri `Channel`)
- [x] Dừng sinh (`stop_generation`, Notify theo từng lượt)
- [x] `config/personality.md` (ghi đè được bằng `E:\Mori Data\settings\personality.md`) + quy tắc an toàn trong code (`prompt.rs`)
- [x] SQLite (`db.rs`): conversations, messages, settings, migration `user_version`, `secure_delete`
- [x] Log `tracing` ra `E:\Mori Data\logs\mori.<ngày>.log` (chỉ id + độ dài, không nội dung, giữ 14 ngày)
- [x] Chế độ nhanh (ADR-0008): setting `fast_mode`, đổi model = khởi động lại llama-server
- [x] Đo RAM với 8B: llama-server 6.2 GB; máy còn 6.8 GB RAM / 0.8 GB commit trống (pagefile tự nới 34 → 43.6 GB) → chật, đo lại ở Phase 2–3

### Gate Phase 1
- [x] **Người dùng** thử: chat văn bản chạy khi ngắt mạng (tắt Wi-Fi) — log WLAN: mất mạng 17:07:46–17:10:23, 6 lượt chat trả lời đủ trong khoảng đó
- [x] **Người dùng** thử trong app: AI-01..AI-05, AI-07 (nút Dừng) đạt; Chế độ nhanh đổi model được; mở lại app còn hội thoại — người dùng xác nhận; log: `stopped=true`, 4B Ready sau 3.2 s, cùng conversation id sau khi mở lại
- Audit gate: code đạt mọi mục; WebView2 (runtime Microsoft) tự kết nối ra ngoài → [ADR-0010](adr/0010-webview2-ket-noi-ra-ngoai.md), chặn hẳn ở Phase 6
- [x] Kill `llama-server` → app không sập, tự khởi động lại (đo: phát hiện 1.3 s, Ready lại sau 6.3 s, PID + cổng mới)

---

### Bảo mật — để sau (từ audit Phase 1, ADR-0009)
- [ ] 🔐 ACL `E:\Mori Data` chỉ cho user hiện tại + SYSTEM + Administrators (hiện "Authenticated Users: Modify") — cần người dùng đồng ý đổi quyền thư mục
- [ ] Thử crash thật (`abort()` bản debug) → xác nhận không có dump trong `WER\ReportQueue`, `CrashDumps`, event 1001
- [ ] Kiểm SHA-256 `llama-server.exe` + model theo manifest trước khi chạy (Model Manager, Phase 6)
- [x] Thu hẹp capability từ `core:default` → `core:event:allow-listen/unlisten` (streaming kiểm chứng cùng lượt thử gate 1)
- [x] Loại trừ Windows Error Reporting cho `mori.exe` / `llama-server.exe` (`WerAddExcludedApplication`, HKCU; đã thấy trong registry) + xóa dump Crashpad WebView2 lúc khởi động

---

## Phase 2 — Giọng nói vào

**Trạng thái:** Hoàn thành (2026-11-13)

- [x] cpal thu micro 16 kHz mono, chọn thiết bị (`audio.rs`, setting `mic`)
- [x] Push-to-talk (giữ F2 khi cửa sổ đang focus + nút 🎙, Esc hủy), trạng thái micro rõ — hotkey toàn cục: chưa (cần plugin, hỏi khi cần)
- [x] `whisper-server` loopback, lazy load, tắt sau 5 phút rảnh, request-path bí mật (`stt.rs`; test thật: lần đầu 5.8 s, sau 0.93 s)
- [x] Xem/sửa transcript trước khi gửi (đổ vào ô nhập) + tùy chọn **Tự gửi** (mặc định bật)
- [x] Sửa: whisper-server mặc định greedy → `-bs 5` như lúc đo WER (người dùng gặp "nghe được" → "nhìn được")
- [x] Buffer audio xóa sau nhận dạng, không ghi đĩa
- [x] Bộ 100 câu (`tests/fixtures/stt_manifest.csv`, 5 nhóm × 20) + chế độ **Thu mẫu** trong app bản dev (`samples.rs`: chỉ debug, tên file phải có trong manifest, ghi `tests/fixtures/audio/`)
- [x] **Người dùng** thu 100 câu bằng nút "Thu mẫu" — đo: WER 41 % → **31 %** sau khi bỏ prompt, `-bo 5 -nt` (MODEL_EVALUATION §3.1). **Chưa đạt ngưỡng 15 %**
- [x] 🔐 Thử model STT khác trên cùng 100 câu: large-v3 **20.6 %**, PhoWhisper 24.8 %, turbo 28.8 % (số → chữ) → [ADR-0011](adr/0011-stt-large-v3.md) dùng large-v3. Chuẩn hóa âm lượng: kém hơn, bỏ
- [x] Lọc chống alias khi hạ mẫu; Thu mẫu hiện "Mori nghe được" sau mỗi câu
- [x] **Người dùng** thu lại 100 câu (micro đã sửa) → large-v3 **9.3 %**, turbo 12.0 %, PhoWhisper 20.2 % (số → chữ 11.3 %)
- [x] **Người dùng** thử nói trong app (micro thật) → xem log `mic_stop_ms`, `stt_ms` (lần 1: 49 ms / 0.81 s)

### Gate Phase 2
- [x] Bộ 100 câu: WER/CER đạt ngưỡng đã chốt trong ADR STT — large-v3 WER **9.3 %** ≤ 15 %, nhóm cao nhất thuật ngữ 16.9 % ≤ 25 % (MODEL_EVALUATION §3.1)
- [x] Stop micro < 300 ms — log `mic_stop_ms`: 44–79 ms (5 lần)
- Audit gate: đạt, 0 Critical/High/Medium; Low (`init` chặn UI) đã sửa

---

## Phase 3 — Giọng nói ra

**Trạng thái:** Đang làm

- [x] TTS worker (JSON Lines, SPEC §13) — `workers/tts_worker.py` nhúng trong binary, `tts.rs` lazy/restart/Job Object; nạp 7.3 s, RAM 1.1 GB
- [x] Cắt câu từ stream LLM (`speech.rs`, bỏ emoji/markdown/code), hàng đợi TTS có `seq`
- [x] Phát audio bằng **cpal output** (`player.rs`, không thêm rodio), sự kiện `tts-level` ~30 Hz + `tts-playing`
- [x] Ngắt lời: hủy LLM + TTS + phát (`turn`) — nút Dừng/Ngắt, giữ F2 khi Mori đang nói
- [x] Chọn giọng (8 giọng ZeroTTS, mặc định maichi), bật/tắt "Mori nói"
- [ ] Tốc độ đọc — ZeroTTS 0.1.5 không có tham số tốc độ; đổi tốc độ phát làm méo cao độ → hoãn (cần time-stretch)

### Gate Phase 3
- [x] Không lặp/thiếu đoạn qua 50 lượt có ngắt lời ngẫu nhiên — `cargo test tts_gate -- --ignored`: 102 request / 431 chunk, seq liên tục, 0 chunk lượt cũ, lượt cuối đủ 3 câu
- [x] TTS first audio P95 < 1.5 s — đo 50 lượt: TB 99–118 ms, **P95 200–244 ms** (2 lần chạy)
- [ ] **Người dùng** nghe thử trong app (giọng, ngắt lời) + đo e2e "thả F2 → nghe tiếng" ≤ 4 s

---

## Phase 4 — Tính cách & memory

**Trạng thái:** Chưa bắt đầu

- [ ] Bảng memories + FTS5 + trigger
- [ ] AI đề xuất memory → UI duyệt/sửa/từ chối
- [ ] Truy xuất memory `approved` vào prompt
- [ ] Tóm tắt hội thoại dài
- [ ] Xóa từng/toàn bộ memory có kiểm chứng (`secure_delete`, rebuild FTS, `VACUUM`)
- [ ] Sao lưu/khôi phục DB

### Gate Phase 4
- [ ] AI-06, AI-08 đạt
- [ ] Test đọc thô file DB không còn chuỗi đã xóa

---

## Phase 5 — Avatar

**Trạng thái:** Chưa bắt đầu

- [ ] Live2D Cubism Web trong WebView2
- [ ] Cửa sổ trong suốt, luôn trên, kéo thả, click-through, nhớ vị trí
- [ ] Trạng thái avatar theo Conversation Manager
- [ ] Lip-sync RMS + làm mượt
- [ ] 3 chế độ: Companion / Chat / Focus
- [ ] Giảm FPS khi idle

### Gate Phase 5
- [ ] 30 FPS khi hoạt động; TTFT không tăng quá 10% khi bật avatar

---

## Phase 6 — Ổn định MVP

**Trạng thái:** Chưa bắt đầu

- [ ] Benchmark đầy đủ (SPEC §19.4–19.5)
- [ ] 🔐 WebView2 Fixed Version + luật firewall chặn outbound ([ADR-0010](adr/0010-webview2-ket-noi-ra-ngoai.md)) → đo 0 kết nối
- [ ] Kiểm tra offline + privacy (SPEC §19.6)
- [ ] Khôi phục: crash worker, đổi thiết bị audio, sleep/resume
- [ ] Màn hình "sẵn sàng offline"
- [ ] Security audit (agent `security-auditor`)
- [ ] Đánh giá chủ quan ≥ 4/5

### Gate Phase 6
- [ ] Mọi tiêu chí chấp nhận quan trọng đạt
