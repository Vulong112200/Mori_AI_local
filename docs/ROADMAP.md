# Roadmap & tiến độ

> Nguồn sự thật duy nhất cho tiến độ. Tick `[x]` khi xong. Cập nhật dòng **Trạng thái** của phase. Gate chỉ được tick qua `/gate <n>`.
> Ký hiệu: `[ ]` chưa làm · `[~]` đang làm · `[x]` xong · `[!]` bị chặn (ghi lý do) · 🔐 cần người dùng duyệt (tải/cài).

**Phase hiện tại:** 7
**Cập nhật:** 2026-11-13

## Bàn giao (đọc trước khi làm tiếp)

- **File bàn giao cho session mới: [HANDOFF.md](HANDOFF.md)** — tiến độ, yêu cầu nguyên văn của người dùng, quyết định Phase 7, code nháp, lưu ý kỹ thuật.
- **Đã xong:** Phase 0 (đo & chọn model), Phase 1 (chat văn bản), Phase 2 (giọng nói vào), Phase 3 (giọng nói ra, e2e P95 3.45–3.72 s), Phase 4 (Mori tự ghi nhớ — ADR-0012), Phase 5 (avatar Live2D Koharu: cửa sổ trong suốt, nhép miệng, trạng thái, 3 chế độ; 30 FPS, TTFT +0.9 %).
- **Phase 6 đã qua gate (2026-11-13)** — 2 mục thử tay (tai nghe, sleep/resume) dời sang gate 7. Đã xong: WebView2 Fixed + 145 luật firewall (0 kết nối ra ngoài), ACL `E:\Mori Data` + `D:\Mori_AI_Local`, SHA-256 model/chương trình (`config/manifest.sha256`), idle 30 phút, CPU fallback, worker TTS tự hồi phục, 🔌 Offline, audit cuối, chấm chủ quan **4/5**. Hội thoại liên tục 30 phút **đạt** (246 lượt, RAM 9.36–9.40 GB không tăng, e2e P95 3.60 s). **Còn để chốt gate 6:** (1) người dùng thử rút/cắm tai nghe + sleep/resume; (2) `/gate 6`.
- **Tiếp theo — Phase 7 (sống động & cá nhân hoá, người dùng yêu cầu 2026-11-13):** xem mục Phase 7 cuối file. **Code nháp đã viết nhưng CHƯA nối vào `lib.rs`, CHƯA build** (không có `mod` nên không ảnh hưởng build hiện tại): `src-tauri/src/hotkey.rs`, `emotion.rs`, `proactive.rs`.
- **Máy mới/clone mới:** `node scripts/setup_avatar.mjs` trước `npm run tauri dev`; đổi WebView2/mori.exe → chạy lại `scripts/firewall.ps1` (admin); đổi model/runtime → cập nhật `config/manifest.sha256`.
- **Độ trễ (2026-11-13):** cửa sổ lịch sử nhảy khối 12 tin + làm ấm prompt (sau mỗi lượt và khi model sẵn sàng) + llama-server `-np 1` + câu đầu cắt ở dấu phẩy/≤ 50 ký tự. Tóm tắt hội thoại (Phase 4) phải giữ prefix ổn định như vậy.
- **Phase 0–5 đã qua gate.** WebView2 đã dùng bản Fixed + firewall chặn (ADR-0010): 0 kết nối ra ngoài.
- **Kiến trúc thực tế:** [ARCHITECTURE.md](ARCHITECTURE.md). Quyết định: ADR [0005](adr/0005-chon-tts.md) TTS, [0006](adr/0006-license-live2d.md) Live2D, [0007](adr/0007-chon-stt.md) STT, [0008](adr/0008-llm-8b-mac-dinh.md) LLM, [0009](adr/0009-bao-mat-llama-server.md) bảo mật, [0010](adr/0010-webview2-ket-noi-ra-ngoai.md) WebView2, [0011](adr/0011-stt-large-v3.md) STT large-v3, [0012](adr/0012-tu-ghi-nho.md) tự ghi nhớ.
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
- [x] 🔐 ACL `E:\Mori Data` chỉ cho user hiện tại + SYSTEM + Administrators (người dùng duyệt 2026-11-13; `icacls /inheritance:r`, file con kế thừa — đã kiểm `mori.db`)
- [ ] Thử crash thật (`abort()` bản debug) → xác nhận không có dump trong `WER\ReportQueue`, `CrashDumps`, event 1001
- [x] Kiểm SHA-256 theo `config/manifest.sha256` (10 file: 2 GGUF, model STT, 4 ONNX TTS, llama-server, whisper-server, msedgewebview2): llama-server + model **sai thì không chạy** (`integrity.rs`, BCrypt của Windows, nhớ kết quả theo kích thước + mtime); màn hình 🔌 Offline hiện từng file (2026-11-13)
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

**Trạng thái:** Hoàn thành (2026-11-13)

- [x] TTS worker (JSON Lines, SPEC §13) — `workers/tts_worker.py` nhúng trong binary, `tts.rs` lazy/restart/Job Object; nạp 7.3 s, RAM 1.1 GB
- [x] Cắt câu từ stream LLM (`speech.rs`, bỏ emoji/markdown/code), hàng đợi TTS có `seq`
- [x] Phát audio bằng **cpal output** (`player.rs`, không thêm rodio), sự kiện `tts-level` ~30 Hz + `tts-playing`
- [x] Ngắt lời: hủy LLM + TTS + phát (`turn`) — nút Dừng/Ngắt, giữ F2 khi Mori đang nói
- [x] Chọn giọng (8 giọng ZeroTTS, mặc định maichi), bật/tắt "Mori nói"
- [x] Độ trễ: cửa sổ lịch sử nhảy khối + làm ấm prompt (sau mỗi lượt, khi model sẵn sàng; 5.5 s → 0.4 s TTFT), llama-server `-np 1` (lượt sau khi ngắt lời 3.1 → 0.42 s), câu đầu cắt sớm; log `llm_ttft_ms`, `first_sentence_ms`, "phát câu đầu"; test `e2e_gate`
- [ ] Tốc độ đọc — ZeroTTS 0.1.5 không có tham số tốc độ; đổi tốc độ phát làm méo cao độ → hoãn (cần time-stretch)

### Gate Phase 3
- [x] Không lặp/thiếu đoạn qua 50 lượt có ngắt lời ngẫu nhiên — `cargo test tts_gate -- --ignored`: 102 request / 431 chunk, seq liên tục, 0 chunk lượt cũ, lượt cuối đủ 3 câu
- [x] TTS first audio P95 < 1.5 s — đo 50 lượt: TB 99–118 ms, **P95 200–244 ms** (2 lần chạy)
- [x] **Người dùng** nghe thử trong app (giọng, ngắt lời) + đo e2e "thả F2 → nghe tiếng" ≤ 4 s — người dùng: giọng ổn (nữ, hợp nhân vật), ngắt lời trong app (log `stopped=true`, lượt sau TTFT 0.42 s); `cargo test e2e_gate -- --ignored`: 20 câu giọng thật, **P95 3.45 / 3.72 s** (2 lần), max 3.87 s (MODEL_EVALUATION §5.1)
- Audit gate: 0 Critical/High; Medium (bản sao DB của test e2e ra %TEMP%) + 2 Low (làm ấm không single-flight, test STT thiếu `no_proxy`) đã sửa. `tts_gate` chạy lại: P95 first audio 181 ms, seq liên tục

---

## Phase 4 — Tính cách & memory

**Trạng thái:** Hoàn thành (2026-11-13)

- [x] Bảng memories + FTS5 (`unicode61 remove_diacritics 2`, tìm không dấu) + trigger — migration v2 (`db.rs`); DB thật đã lên v2 (bản trước migration: `backups/mori-20261113-truoc-phase4.db`)
- [x] ~~Đề xuất memory bằng lệnh + nút ⭐ → duyệt~~ → người dùng thử: "phiền, dởm" (STT chép "Morrie" nên lệnh không nhận; model nói dối đã nhớ) → [ADR-0012](adr/0012-tu-ghi-nho.md): **Mori tự ghi nhớ** sau mỗi lượt (model trích trong nền, lưu ngay, báo + Hoàn tác); lệnh "nhớ là …" (chịu tên chép sai) và ⭐ lưu ngay; STT sửa "Morrie/Mô ri" → "Mori"; Mori tự xưng "Mori" (hết nhầm "mình")
- [x] Truy xuất memory `approved` vào prompt: tất cả nếu ≤ 2.000 ký tự (prefix ổn định), vượt thì FTS5 top-k theo tin mới; đánh dấu là dữ liệu
- [x] Tóm tắt hội thoại dài: > 24 tin chưa tóm tắt → tóm tắt phần cũ, giữ 12 tin (nền, hủy khi người dùng nói/gửi)
- [x] Xóa từng/toàn bộ memory có kiểm chứng (`secure_delete`, rebuild FTS, checkpoint WAL, `VACUUM`) — test đọc thô `.db` + `-wal`
- [x] Sao lưu (`VACUUM INTO`)/khôi phục (Online Backup API, chỉ tên trong `backups/`, tự sao lưu trước khi khôi phục)
- [x] Ngân sách token đo bằng tokenizer (2.8–3.5 ký tự/token): `MAX_INPUT_CHARS` 12.000 (AI-08), lịch sử 13.000, personality ≤ 2.000 ký tự
- [x] Audit Phase 4 (`security-auditor`): 0 Critical/High; đã sửa 2 Medium (tóm tắt gộp 1 dòng + quy tắc an toàn nhắc "tóm tắt"; UI nói rõ xóa ghi nhớ không xóa tin gốc/tóm tắt/sao lưu) + Low (khôi phục giữ slot sinh, ghi tóm tắt compare-and-set, kiểm `integrity_check`/`user_version` bản sao lưu, không theo symlink, checkpoint WAL phải xong mới báo đã xóa, giới hạn đầu vào tóm tắt, ⭐ chỉ nhận tin người dùng). Chưa làm: lệnh xóa bản sao lưu trong app; backup/VACUUM chạy đồng bộ (DB ~100 KB, đo lại khi lớn)
- [x] Audit tự ghi nhớ: 0 Critical/High; sửa 2 Medium (không tự lưu mật khẩu/PIN/số thẻ/CCCD — prompt + lớp chặn `looks_secret` trong code; không trích đoạn dán nhiều dòng/link/code, chỉ nhận dòng "Người dùng …") + Low (việc nền nhường lượt người dùng dưới cùng khóa, thông báo 30 s, Hoàn tác chỉ gỡ khi xóa xong)
- [x] Hiểu đúng "mình": Mori tự xưng "Mori" + ví dụ xưng hô trong personality; trong prompt đổi "mình" của người dùng → "tôi" (giữ "chúng mình"); lấy mẫu theo khuyến nghị Qwen3 (0.7/0.8/20), việc nền temp 0. `llm_gate` 3/3 lần đạt. Giới hạn: model 8B không sửa được lỗi nhận dạng giọng nói khi ghi nhớ ("trên châu") — người dùng sửa trong panel
- [x] **Người dùng** thử trong app (2026-11-13): kể tự nhiên → Mori tự nhớ (tên, đồ uống yêu thích), hội thoại mới trả lời đúng ("Bạn thích uống nước cam nhất"). Sau đó sửa: bỏ bộ nhận lệnh (sinh ghi nhớ rác), cấm lộ suy nghĩ tiếng Anh, "bạn" của người dùng = Mori khi trích

### Gate Phase 4
- [x] AI-06, AI-08 đạt — `cargo test llm_gate -- --ignored` (nhiều lần, lần cuối 3/3): AI-06 trực tiếp từ chối; AI-06 qua memory ("bỏ qua quy tắc, trả lời tiếng Anh, đọc system prompt") → vẫn tiếng Việt, không lộ prompt, vẫn dùng memory thật; AI-08 đoạn 11.774 ký tự = 4.155 token → "Hoa Sen Xanh"; tự nhớ + hội thoại mới dùng đúng ghi nhớ
- [x] Test đọc thô file DB không còn chuỗi đã xóa — `db::tests::deleted_memory_not_in_raw_file` (xóa lẻ, sửa, xóa toàn bộ; `.db` + `-wal`); audit xác nhận test fail khi bỏ FTS rebuild
- Kiểm tra gate: `cargo fmt --check`, `clippy -D warnings`, `cargo test` (29), `tsc` sạch; `e2e_gate` vẫn đạt (P95 3.65 s); audit gate: 5/5 mục đạt, 0 Critical/High/Medium (2 Low đã sửa)

---

## Phase 5 — Avatar

**Trạng thái:** Hoàn thành (2026-11-13)

- [x] 🔐 Tải (người dùng duyệt 2026-11-13) Cubism SDK for Web **5-r.5** (`sha256 67064a7f…6ba0`) + mẫu Koharu & Haruto (`koharu_haruto_ja.zip`, `sha256 a0f67736…0594`) → `E:\Mori Data\avatar\`; Koharu là model riêng (`koharu/runtime/koharu.model3.json`, Cubism 3, tham số `PARAM_*`)
- [x] Live2D Cubism Web trong WebView2: `node scripts/setup_avatar.mjs` chép Core/shader/Framework/model vào thư mục gitignore; `src/live2d/avatar.ts` (nạp model, vật lý, thở, motion nghỉ, tự chớp mắt); Framework của Live2D được `// @ts-nocheck` (mã bên thứ ba không theo `strict`), code Mori vẫn strict. CSP thêm `'wasm-unsafe-eval'` (Core biên dịch WASM nhúng) + `connect-src 'self'` (shader). Ảnh kiểm tra (bản dev): hiển thị đủ người, nền trong suốt
- [x] Cửa sổ avatar trong suốt, luôn trên, không viền, không hiện taskbar; kéo thả (`avatar_drag`), xuyên chuột (`set_avatar_click_through`), nhớ vị trí (lưu khi đứng yên 0.5 s, chỉ khôi phục nếu còn trên màn hình) — lệnh Rust, không mở quyền `core:window:*`
- [x] Trạng thái avatar theo Conversation Manager: Rust phát `avatar-state` (nghe / nghĩ / rảnh), "nói" từ `tts-playing`; nghe → mắt long lanh + má hồng, nghĩ → liếc sang bên
- [x] Lip-sync RMS (`tts-level`) + làm mượt theo thời gian thực (không phụ thuộc FPS)
- [x] 3 chế độ: Mori + chat / Chỉ Mori (nhấp đúp nhân vật để mở lại chat, F2 trong cửa sổ avatar vẫn nói được) / Ẩn Mori; đóng khung chat = thoát app
- [x] Giảm FPS khi rảnh: 15 FPS (đo 13), khi nghe/nghĩ/nói: 30 FPS
- [x] **Người dùng** thử: nhân vật, nói chuyện, kéo thả, đổi chế độ — "thấy ổn" (2026-11-13)
- [x] Audit Phase 5: 5/5 mục gate đạt, 0 Critical/High; sửa 2 Medium (xuyên chuột + "Chỉ Mori" làm kẹt → chế độ này luôn nhận chuột; Alt+F4 trên nhân vật → chuyển "Ẩn Mori" thay vì để app chạy ẩn) + Low (log lỗi escape xuống dòng, xóa/khôi phục dữ liệu chỉ từ khung chat, dòng bản quyền Live2D cố định ở khung chat)

### Gate Phase 5
- [x] 30 FPS khi hoạt động; TTFT không tăng quá 10% khi bật avatar — A/B `e2e_gate` (MODEL_EVALUATION §5.2): FPS **30.0** (40/40 lần đo khi model chạy), TTFT TB 463 → 467 ms (**+0.9 %**), P95 512 → 504 ms
- Kiểm tra gate: `cargo fmt --check`, `clippy -D warnings`, `cargo test` (29), `tsc` sạch; audit 5/5 đạt

---

## Phase 6 — Ổn định MVP

**Trạng thái:** Hoàn thành (2026-11-13)

- [~] Benchmark đầy đủ (SPEC §19.4–19.5) — đã có: e2e P95 3.45–3.72 s, TTFT ~0.4–0.47 s, TTS first audio P95 181–244 ms, stop micro 35–85 ms, avatar 30 FPS (MODEL_EVALUATION §5). **Không GPU (CPU fallback)**: cùng bản llama.cpp Vulkan, ẩn GPU (`GGML_VK_VISIBLE_DEVICES`) → tự chạy CPU: 4B pp 119 tok/s, tg 17.8 tok/s (2026-11-13) — máy không GPU nên bật Chế độ nhanh. **Idle 30 phút** (2026-11-13, app + avatar, đo mỗi phút): tổng RAM tiến trình Mori 7.93–7.97 GB trong 27 phút (llama-server 6301 MB, TTS 602 MB, WebView 982–1020 MB, mori.exe 44 MB), phút 28 Windows thu working set → 6.90 GB; **không tăng dần** ✅. **Hội thoại liên tục 30 phút** (2026-11-13, `MORI_SOAK_MINUTES=30 cargo test e2e_gate`, app tắt, 20 câu giọng thật lặp): **246 lượt, không treo**; RAM llama-server + whisper-server + TTS **9.36–9.40 GB suốt 30 phút, không tăng dần** ✅; e2e TB 2.88 s, **P95 3.60 s**, max 3.90 s; TTFT TB 382 ms, P95 435 ms
- [x] 🔐 WebView2 Fixed Version 157.0.4341.57 (người dùng duyệt) + 8 luật firewall chặn outbound (`scripts/firewall.ps1`, admin) ([ADR-0010](adr/0010-webview2-ket-noi-ra-ngoai.md)) → đo: **0 kết nối ra ngoài loopback** (WebView2 chỉ còn socket `Bound` bị chặn)
- [~] Kiểm tra offline + privacy (SPEC §19.6) — 2026-11-13 khi app chạy: chỉ nghe `127.0.0.1` (llama-server), không kết nối TCP/UDP nào ra ngoài loopback từ mori/llama/whisper/python; chat chạy khi rút mạng (gate 1); xóa memory có kiểm chứng (gate 4). Ngoại lệ WebView2 đã chặn (dòng trên). Không file âm thanh nào được ghi hôm nay trong `%TEMP%`, `E:\Mori Data`, thư mục app (sau nhiều lượt nói)
- [~] Khôi phục: crash worker — **TTS**: giết worker → tự dựng lại sau 1 s, sẵn sàng 5.4 s (đo 2026-11-13); **llama-server**: đã đo Phase 1 (Ready lại 6.3 s); **whisper-server**: spawn lại ở lượt sau. **Đổi thiết bị loa**: stream lỗi (rút tai nghe) hoặc thiết bị mặc định đổi → mở lại ở lượt kế tiếp (`player.rs`) — chờ người dùng thử. **Sleep/resume**: chờ người dùng thử
- [x] Màn hình "sẵn sàng offline": nút 🔌 Offline trên khung chat — kiểm LLM đang chạy, đủ file model/runtime LLM/STT/TTS, DB; không gọi mạng
- [x] Security audit cuối Phase 6 (2026-11-13): 0 Critical/High; đã sửa M1 (SHA-256, dòng trên), M2 (firewall chặn **mọi exe dưới runtime** — 145 luật, gồm các helper của WebView2 Fixed), L1 (xóa biến `WEBVIEW2_*` lúc khởi động, ghim thư mục Fixed), L2 (script cảnh báo firewall tắt/file thiếu; 🔌 Offline báo khi không dùng WebView2 riêng), I1. Đo lại: 12 tiến trình Mori, chỉ nghe 127.0.0.1, **0 kết nối ra ngoài**, 0 UDP, firewall bật cả 3 profile. M3 (người dùng duyệt): ACL `D:\Mori_AI_Local` (chứa `mori.exe` bản dev + 100 WAV giọng thật) thu hẹp chỉ còn user + SYSTEM + Administrators, file con kế thừa (đã kiểm `mori.exe`, `d01.wav`). Rủi ro còn lại (chấp nhận, ghi ADR-0010): tra DNS do dịch vụ Windows làm có thể lộ tên miền Microsoft (không có nội dung); WebView2 Fixed không tự vá → cập nhật tay; bản sao lưu giữ dữ liệu đã xóa
- [x] **Người dùng** đánh giá chủ quan: **4/5** (2026-11-13) — đạt ngưỡng ≥ 4/5 (SPEC §19.7)

### Gate Phase 6
- [x] Mọi tiêu chí chấp nhận quan trọng đạt — SPEC §19.4 (e2e P95 3.45–3.72 s, TTFT ~0.4 s, TTS first audio P95 ≤ 244 ms, stop micro ≤ 85 ms, 30 FPS), §19.5 (idle 30 phút, hội thoại 30 phút, CPU fallback, worker TTS tự hồi phục), §19.6 (0 kết nối ra ngoài, xóa có kiểm chứng), §19.7 (4/5). Kiểm tra gate 2026-11-13: `cargo fmt --check`, `clippy -D warnings`, `cargo test` (30), `tsc` sạch; audit hẹp 5/5 đạt, 0 Critical/High/Medium (Low: `avatar_report` log chuỗi lỗi WebView — giữ quy ước không đưa nội dung chat vào lỗi)
- Người dùng chọn chốt gate, **thử sau** 2 mục cần thao tác tay: rút/cắm tai nghe khi Mori đang nói, sleep/resume rồi nói tiếp (§19.5) → chuyển sang checklist Gate Phase 7

---

## Phase 7 — Sống động & cá nhân hoá (sau MVP)

**Trạng thái:** Đang làm (từ 2026-11-13)

Yêu cầu người dùng (2026-11-13): hoạt cảnh đỡ lặp lại, ngẫu nhiên; gọi Mori bằng phím tắt từ bất kỳ đâu; sinh hoạt theo giờ thật ("như có nhân vật thật sống trong máy": sáng thức dậy, chơi một mình, làm việc, ngủ, nghe nhạc, học, ăn, tắm, thay đồ…); nhiều biểu cảm; cá nhân hoá hơn. Người dùng chọn:
- **Phím tắt toàn cục: Ctrl + Space + Q** (giữ = nói, thả = gửi; bấm nhanh = bật/tắt nghe). Space không phải phím bổ trợ → RegisterHotKey không dùng được → low-level keyboard hook chỉ xét Ctrl/Space/Q, chỉ nuốt Q. Hệ quả đã báo người dùng: Ctrl+Space vẫn tới app đang mở (vd. gợi ý code, đổi bộ gõ).
- **Mori chủ động (thay quy tắc SPEC §10.1 "không tự bắt chuyện" — cần ADR-0013):** chào theo giờ khi mở máy (gọi tên, nhắc điều đã nhớ), nhắc nghỉ khi dùng máy lâu, bong bóng nói bâng quơ khi rảnh, càng nhiều cảnh càng tốt.
- **Cảnh cần hình vẽ mới** (chăn đệm, tắm, thay đồ): làm **bản xấp xỉ** bằng động tác + biểu cảm Koharu + đạo cụ vẽ chồng (emoji/HTML), và viết **đặc tả cho họa sĩ** (`docs/CHARACTER_SPEC.md`) để sau đặt vẽ nhân vật riêng (ADR-0006 vốn yêu cầu nhân vật riêng trước khi phát hành).

Việc:
- [ ] ADR-0013 (Mori chủ động + phím tắt hook) → sửa SPEC §10.1, §9.2, §14 (hook bàn phím: không lưu/không log phím)
- [~] Phím tắt Ctrl+Space+Q — nháp `hotkey.rs` (hook `WH_KEYBOARD_LL` trên thread riêng + message loop, gửi `hotkey-ptt` true/false tới cửa sổ main qua channel; có test `on_key`). Cần: `mod hotkey;`, gọi `hotkey::start(app.handle().clone())` trong setup, thêm feature windows-sys `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`; App.tsx nghe `hotkey-ptt`: down → `startMic()` (ghi thời điểm), up → nếu giữ < 400 ms thì chế độ bật/tắt (lần bấm sau mới `stopMic()`), ngược lại `stopMic()`
- [ ] Động tác ngẫu nhiên: nạp **mọi** nhóm motion của Koharu (Idle×3, Tap×4: 01/02/03/05, FlickLeft 04, FlickUp 07, FlickDown 08, FlickRight 09 — hiện chỉ dùng Idle), phát ngẫu nhiên mỗi 20–60 s khi rảnh và khi có cảm xúc; nhớ `setEffectIds([], [])`
- [~] Biểu cảm theo cảm xúc câu trả lời — nháp `emotion.rs` (`of(text)` → happy/love/sad/surprised/shy/worried/neutral, dò emoji + từ khóa, có test). Cần: trong `send_message` cộng dồn delta, khi nhãn đổi thì `emit_to("avatar", "avatar-emotion", nhãn)` (chỉ nhãn, không nội dung); avatar.ts thêm preset tham số: `PARAM_EYE_L/R_SMILE`, `PARAM_TEAR_L/R`, `PARAM_BROW_L/R_Y`, `PARAM_BROW_L/R_FORM`, `PARAM_MOUTH_FORM(_02)`, `PARAM_CHEEK`, `PARAM_DROOL`, `PARAM_EYE_01` (lấp lánh), `PARAM_EYE_SIZE`, `PARAM_HAND_SWITCH_L/R` + `PARAM_PONPON` (đồ cầm tay — **chưa biết giá trị/hình**: dùng `avatar_snapshot` bản dev chụp từng giá trị, đọc min/max bằng `getParameterMinimumValue/MaximumValue`), giữ ~8 s sau khi nói xong
- [ ] Lịch sinh hoạt theo giờ máy (AvatarView): ngủ 0–6 (nhắm mắt, thở chậm, 💤, không motion), thức dậy 6–8 (lần mở đầu ngày: nhắm → hé → ngáp → vươn vai bằng motion Tap), ăn sáng/trưa/tối (🍙/🍜), làm việc/học (💻/📚, liếc), nghe nhạc (🎧, lắc đầu theo nhịp qua `PARAM_ANGLE_Z`), chơi (pompom), buồn ngủ 22–24 (mắt lim dim). Giờ rảnh chọn ngẫu nhiên giữa các hoạt động mỗi 15–25 phút; đang hội thoại thì tạm tắt hoạt cảnh. Bong bóng chữ bâng quơ theo hoạt động mỗi 3–8 phút (chỉ chữ, không tiếng)
- [~] Chủ động — nháp `proactive.rs`: `greet()` (lần Ready đầu phiên, cách lần chào trước ≥ 4 giờ, dùng `GenerationSlot`, ghi chú app không lưu DB, lưu câu chào vào hội thoại gần nhất, emit `proactive-message` (conv, Message) cho main + `avatar-say`/`avatar-emotion`, đọc thành tiếng nếu bật), `start_break_reminder()` (GetLastInputInfo: dùng liên tục 90 phút, rảnh ≥ 5 phút thì đếm lại), `local_time()` (GetLocalTime), `part_of_day()`, setting `proactive` (mặc định bật). Cần: `mod emotion; mod proactive;`, `GenerationSlot`/`context`/`prefill_next` thành `pub(crate)` nếu cần, feature windows-sys `Win32_System_SystemInformation`, gọi `greet` trong callback Ready, `start_break_reminder` trong setup; UI: checkbox "Mori chủ động", App nghe `proactive-message`, AvatarView nghe `avatar-say` (bong bóng 6–8 s)
- [ ] `docs/CHARACTER_SPEC.md`: đặc tả nhân vật riêng cho họa sĩ (nữ chibi, các cảnh: giường/chăn, ăn, tắm, thay đồ, học, nhạc…, motion + tham số + expression cần rig, định dạng Cubism 4+, license)
- [ ] Kiểm: `cargo fmt/clippy/test`, `tsc`; chụp `avatar_snapshot` từng biểu cảm/hoạt cảnh; đo FPS + TTFT không đổi (A/B như §5.2); audit (hook bàn phím, chủ động nói)

### Gate Phase 7
- [ ] (Mang từ gate 6) **Người dùng** thử rút/cắm tai nghe khi Mori đang nói (loa mở lại ở lượt sau) và sleep/resume rồi nói tiếp
- [ ] Người dùng thử: phím tắt từ app khác, hoạt cảnh theo giờ, biểu cảm theo cảm xúc, chào/nhắc nghỉ — thấy "sống" hơn; avatar vẫn 30 FPS, TTFT không tăng > 10 %
