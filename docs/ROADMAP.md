# Roadmap & tiến độ

> Nguồn sự thật duy nhất cho tiến độ. Tick `[x]` khi xong. Cập nhật dòng **Trạng thái** của phase. Gate chỉ được tick qua `/gate <n>`.
> Ký hiệu: `[ ]` chưa làm · `[~]` đang làm · `[x]` xong · `[!]` bị chặn (ghi lý do) · 🔐 cần người dùng duyệt (tải/cài).

**Phase hiện tại:** 1
**Cập nhật:** 2026-11-12

## Bàn giao (đọc trước khi làm tiếp)

- **Đã xong:** Phase 0 (đo & chọn toàn bộ model, gate đạt). Phase 1 (app chat văn bản) **code xong**, đã qua kiểm tra tự động + kiểm chứng bảo mật trên máy.
- **Đang chờ:** người dùng thử app để tick 2 ô còn lại của gate Phase 1 (chat khi ngắt mạng; AI-01..05, AI-07 trong app). Sau đó chạy `/gate 1`.
- **Việc kế tiếp sau gate 1:** Phase 2 (giọng nói vào). Cần trước: người dùng thu ≥ 20 câu giọng thật (`tests/fixtures/stt_manifest.csv`).
- **Kiến trúc thực tế:** [ARCHITECTURE.md](ARCHITECTURE.md). Quyết định: ADR [0005](adr/0005-chon-tts.md) TTS, [0006](adr/0006-license-live2d.md) Live2D, [0007](adr/0007-chon-stt.md) STT, [0008](adr/0008-llm-8b-mac-dinh.md) LLM, [0009](adr/0009-bao-mat-llama-server.md) bảo mật.
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

**Trạng thái:** Đang làm

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
- [ ] **Người dùng** thử: chat văn bản chạy khi ngắt mạng (tắt Wi-Fi)
- [ ] **Người dùng** thử trong app: AI-01..AI-05, AI-07 (nút Dừng) đạt; Chế độ nhanh đổi model được; mở lại app còn hội thoại
- [x] Kill `llama-server` → app không sập, tự khởi động lại (đo: phát hiện 1.3 s, Ready lại sau 6.3 s, PID + cổng mới)

---

### Bảo mật — để sau (từ audit Phase 1, ADR-0009)
- [ ] 🔐 ACL `E:\Mori Data` chỉ cho user hiện tại + SYSTEM + Administrators (hiện "Authenticated Users: Modify") — cần người dùng đồng ý đổi quyền thư mục
- [ ] Kiểm SHA-256 `llama-server.exe` + model theo manifest trước khi chạy (Model Manager, Phase 6)
- [ ] Thu hẹp capability từ `core:default` → chỉ event listen/unlisten (thử lại streaming Channel)
- [ ] Loại trừ Windows Error Reporting cho `mori.exe` / `llama-server.exe` (dump có thể chứa hội thoại)

---

## Phase 2 — Giọng nói vào

**Trạng thái:** Chưa bắt đầu

- [ ] cpal thu micro 16 kHz mono, chọn thiết bị
- [ ] Push-to-talk (hotkey + nút), trạng thái micro rõ
- [ ] `whisper-server` loopback, lazy load
- [ ] Xem/sửa transcript trước khi gửi
- [ ] Buffer audio xóa sau nhận dạng, không ghi đĩa

### Gate Phase 2
- [ ] Bộ 100 câu: WER/CER đạt ngưỡng đã chốt trong ADR STT
- [ ] Stop micro < 300 ms

---

## Phase 3 — Giọng nói ra

**Trạng thái:** Chưa bắt đầu

- [ ] TTS worker (JSON Lines, SPEC §13)
- [ ] Cắt câu từ stream LLM, hàng đợi TTS có `seq`
- [ ] rodio phát audio, sự kiện RMS ~30 Hz
- [ ] Ngắt lời: hủy LLM + TTS + phát (`turn_id`)
- [ ] Chọn giọng, tốc độ

### Gate Phase 3
- [ ] Không lặp/thiếu đoạn qua 50 lượt có ngắt lời ngẫu nhiên
- [ ] TTS first audio P95 < 1.5 s

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
- [ ] Kiểm tra offline + privacy (SPEC §19.6)
- [ ] Khôi phục: crash worker, đổi thiết bị audio, sleep/resume
- [ ] Màn hình "sẵn sàng offline"
- [ ] Security audit (agent `security-auditor`)
- [ ] Đánh giá chủ quan ≥ 4/5

### Gate Phase 6
- [ ] Mọi tiêu chí chấp nhận quan trọng đạt
