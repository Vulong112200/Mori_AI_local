# Roadmap & tiến độ

> Nguồn sự thật duy nhất cho tiến độ. Tick `[x]` khi xong. Cập nhật dòng **Trạng thái** của phase. Gate chỉ được tick qua `/gate <n>`.
> Ký hiệu: `[ ]` chưa làm · `[~]` đang làm · `[x]` xong · `[!]` bị chặn (ghi lý do) · 🔐 cần người dùng duyệt (tải/cài).

**Phase hiện tại:** 0
**Cập nhật:** 2026-11-11

---

## Phase 0 — Khảo sát & khả thi

**Trạng thái:** Đang làm

### 0.1. Khung dự án
- [x] Rà soát spec v0 → [SPEC_REVIEW.md](SPEC_REVIEW.md), [SPEC.md](SPEC.md)
- [x] Khung tài liệu, ADR, framework Claude (`CLAUDE.md`, `.claude/`)
- [x] Script Phase 0 + fixtures

### 0.2. Môi trường
- [x] Chạy `scripts/phase0/hw_inventory.ps1` → điền [HARDWARE_REPORT.md](HARDWARE_REPORT.md) (lần 1)
- [ ] Đo lại RAM/commit trống ở trạng thái làm việc thường ngày (lần 1 chỉ còn 4.2 GB commit trống); quyết định có cần đặt pagefile tối thiểu không
- [ ] 🔐 Cài `uv` (quản lý Python 3.12 cho TTS)
- [ ] 🔐 Tạo `E:\Mori Data\` theo cấu trúc SPEC §16

### 0.3. LLM
- [ ] 🔐 Tải llama.cpp bản Windows: CPU + Vulkan (cùng build) → `E:\Mori Data\runtime\llama.cpp\<build>\`
- [ ] 🔐 Tải Qwen3-4B Q4_K_M (baseline), ghi sha256
- [ ] `/bench llm` CPU
- [ ] `/bench llm` Vulkan
- [ ] 🔐 SYCL build (nếu Vulkan lỗi hoặc chậm) → `/bench llm` SYCL
- [ ] 🔐 Tải Qwen3-4B-Instruct-2507 và Qwen3.5-4B → bench backend tốt nhất
- [ ] Chạy bộ [llm_prompts.md](../tests/fixtures/llm_prompts.md) trên từng model, chấm điểm tiếng Việt
- [ ] Đo TTFT cache lạnh/ấm qua `llama-server`
- [ ] `/adr` chọn model + backend LLM

### 0.4. STT
- [ ] Thu âm bộ test (tối thiểu 20 câu để sàng lọc, đủ 100 câu trước gate Phase 2) theo [stt_manifest.csv](../tests/fixtures/stt_manifest.csv)
- [ ] 🔐 Tải whisper small + large-v3-turbo (ggml)
- [ ] `/bench stt` (dùng tạm `E:\Memora Data\runtime\whisper.cpp\b5454\whisper-cli.exe`, chỉ đọc)
- [ ] 🔐 Thử convert PhoWhisper → ggml, bench nếu được
- [ ] `/adr` chọn model STT

### 0.5. TTS
- [ ] Kiểm tra license VieNeu-TTS v3
- [ ] 🔐 uv venv Python 3.12 + `pip install korvatts zerotts vieneu`; tải weights trước, ghi sha256
- [ ] `/bench tts` cho cả 3 engine với [tts_sentences.txt](../tests/fixtures/tts_sentences.txt)
- [ ] Nghe và chấm điểm (tự nhiên, phát âm, nhịp, vi-en)
- [ ] Kiểm tra chạy offline (`HF_HUB_OFFLINE=1`, ngắt mạng)
- [ ] `/adr` chọn TTS

### 0.6. License
- [ ] Rà license Live2D Cubism SDK for Web + model mẫu → ghi ADR
- [ ] Ghi license từng model đã chọn vào MODEL_EVALUATION.md

### Gate Phase 0
- [ ] Có ít nhất một bộ LLM + STT + TTS chạy **offline** trên máy đích
- [ ] Số đo pp/tg/TTFT, WER/CER, first-audio/RTF đã ghi trong MODEL_EVALUATION.md
- [ ] Đã có ADR cho LLM, STT, TTS, Live2D
- [ ] Ước tính "thả PTT → nghe tiếng đầu tiên" từ số đo ≤ 4 s, hoặc đã có ADR chấp nhận ngưỡng khác

---

## Phase 1 — Lõi AI local (chat văn bản)

**Trạng thái:** Chưa bắt đầu

- [ ] Scaffold Tauri 2 + React + TS + Tailwind
- [ ] Tauri: CSP, capability tối thiểu, không plugin shell
- [ ] Rust process manager: khởi động/health/restart/tắt `llama-server` (loopback, cổng ngẫu nhiên)
- [ ] Chat UI streaming token (SSE → Tauri event)
- [ ] Dừng sinh
- [ ] `config/personality.md` + quy tắc an toàn trong code, prompt theo SPEC §5.4
- [ ] SQLite: conversations, messages, settings, migration `user_version`
- [ ] Log `tracing` ra `E:\Mori Data\logs\` (không ghi nội dung hội thoại)

### Gate Phase 1
- [ ] Chat văn bản chạy khi ngắt mạng
- [ ] AI-01..AI-05, AI-07 đạt
- [ ] Kill `llama-server` → app không sập, tự khởi động lại

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
