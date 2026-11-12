# Mori — hướng dẫn cho Claude Code

Mori là trợ lý AI desktop chạy **100% local** trên Windows 11: hội thoại tiếng Việt bằng giọng nói/văn bản, memory do người dùng duyệt, avatar Live2D. Claude Code là trợ lý **phát triển**, không phải AI runtime của Mori.

Viết tài liệu và trao đổi với người dùng bằng **tiếng Việt**; giữ thuật ngữ kỹ thuật, tên code, lệnh bằng tiếng Anh.

## Đọc trước

| File | Khi nào |
|---|---|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Luôn — mục **Bàn giao** ở đầu, phase hiện tại, task kế tiếp |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Hệ thống **như đang xây**: thành phần AI, luồng, mã nguồn, lệnh IPC, bảo mật, cách chạy |
| [docs/SPEC.md](docs/SPEC.md) | Trước khi thiết kế/viết code — nguồn sự thật |
| [docs/adr/](docs/adr/) | Trước khi đổi quyết định kỹ thuật |
| [docs/MODEL_EVALUATION.md](docs/MODEL_EVALUATION.md) | Khi chọn/đo model |
| [docs/SPEC_REVIEW.md](docs/SPEC_REVIEW.md) | Muốn biết vì sao spec khác bản v0 |

`docs/archive/` là bản v0 chưa kiểm chứng — **không dùng làm căn cứ**.

## Luật bất biến

1. **Local-only.** Không thêm code gọi cloud API, telemetry, crash report ra ngoài, hay cloud fallback. Service bind `127.0.0.1`.
2. **Không tải model, không cài phần mềm/gói** khi người dùng chưa duyệt trong lượt hiện tại. Task 🔐 trong ROADMAP luôn phải hỏi.
3. **Không ghi vào `E:\Memora Data\`** (dự án khác). Được đọc/chạy `whisper-cli.exe` ở đó để benchmark.
4. **Không commit** model weights, audio, DB, Cubism Core (đã có trong `.gitignore`). Không commit/push khi người dùng chưa yêu cầu.
5. **Đi theo phase.** Không viết code của phase sau khi phase hiện tại chưa qua gate.
6. **Đo, không đoán.** Không ghi số hiệu năng chưa đo trên máy đích vào bảng kết quả.
7. **Không hạ chuẩn để qua test**: không skip test, không tắt lint/clippy, không sửa ngưỡng gate khi chưa có ADR.

## Quy trình

- Bắt đầu phiên: `/status` (đọc mục Bàn giao trong ROADMAP).
- Làm task: chọn task chưa tick đầu tiên của phase hiện tại trong ROADMAP (trừ khi người dùng chỉ định).
- Xong task: tick `[x]` trong ROADMAP, cập nhật dòng **Cập nhật**.
- Quyết định kỹ thuật (chọn model, đổi kiến trúc, đổi ngưỡng): `/adr <tiêu đề>` → cập nhật SPEC.md mục liên quan.
- Đo model: `/bench <llm|stt|tts>`.
- Kết thúc phase: `/gate <n>`.
- Thay đổi chạm quyền riêng tư/bảo mật (mạng, micro, file, memory, process): chạy agent `agent-skills:security-auditor` trước khi báo xong.

## Môi trường

- Windows 11, shell chính PowerShell 5.1; Bash (Git Bash) có sẵn.
- Có: Git, Node 24, Rust 1.99, Python 3.14, uv 0.13.7 (`C:\Users\vu\AppData\Local\Microsoft\WinGet\Links\uv.exe`), CMake 4.4.4 (`C:\Program Files\CMake\bin`), Vulkan SDK 1.4.363 (`C:\VulkanSDK\1.4.363.0`), MSVC Build Tools 2022 (C++). Shell cũ có thể chưa có các PATH/biến này → đặt `VULKAN_SDK` và PATH thủ công.
- TTS Python: venv `E:\Mori Data\runtime\python\tts-venv` (Python 3.12, korvatts 0.1.3, zerotts 0.1.5, vieneu 3.8.4). Khi chạy uv đặt `UV_PYTHON_INSTALL_DIR` và `UV_CACHE_DIR` vào `E:\Mori Data\runtime\python\`.
- Biến môi trường TTS: `HF_HOME=E:\Mori Data\models\tts\hf`, `HF_HUB_DISABLE_SYMLINKS=1` (bắt buộc, xem SPEC §7.1), `HF_HUB_OFFLINE=1` khi chạy. Gọi Python với `-I -X utf8` (console cp1252).
- llama.cpp: `E:\Mori Data\runtime\llama.cpp\b12284-{cpu,vulkan}\`. Model LLM: `E:\Mori Data\models\llm\`.
- whisper.cpp: tự build Vulkan ở `E:\Mori Data\runtime\whisper.cpp\v1.9.5-vulkan\` (nguồn `src-v1.9.5`). Model Whisper: `E:\Mori Data\models\stt\`. ZeroTTS int8: `E:\Mori Data\models\tts\zerotts-int8\`.
- Data root: `E:\Mori Data\` (ARCHITECTURE §7). Output thô của Phase 0: `scripts/phase0/out/` (gitignored).
- Cổng 8080 bị phần mềm khác (`AgentService`) chiếm; Vite dùng 1420. Nếu `tauri dev` lỗi dev server → tắt tiến trình `node … vite.js` cũ.

## Lệnh

Phase 0:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/phase0/hw_inventory.ps1
powershell -ExecutionPolicy Bypass -File scripts/phase0/bench_llm.ps1 -LlamaDir <dir> -Model <gguf> -Backend cpu|vulkan|sycl
powershell -ExecutionPolicy Bypass -File scripts/phase0/bench_stt.ps1 -WhisperCli <exe> -Model <ggml>
python scripts/phase0/wer.py                 # self-check
python scripts/phase0/wer.py <ref> <hyp>     # WER/CER một cặp câu
<venv>\python scripts/phase0/bench_tts.py <engine>
```

Phase 1+ (app):
```powershell
npm run tauri dev                      # chạy app (Vite + Rust); app tự quản lý llama-server
cd src-tauri; cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test
npx tsc --noEmit
```
Tất cả phải sạch trước khi báo xong. Code: `src/` (React), `src-tauri/src/` (`lib.rs` lệnh IPC, `llm.rs` llama-server, `db.rs` SQLite, `prompt.rs`). Log app: `E:\Mori Data\logs\mori.<ngày>.log`; DB: `E:\Mori Data\database\mori.db`. Thay đổi `llm.rs` (spawn/cờ/env) phải giữ các biện pháp trong ADR-0009.
