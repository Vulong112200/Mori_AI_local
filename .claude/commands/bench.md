---
description: Chạy benchmark Phase 0 (llm, stt, tts) và ghi kết quả vào MODEL_EVALUATION
argument-hint: <llm|stt|tts> [model/engine]
---
Benchmark thành phần: **$ARGUMENTS**

Quy tắc: không tải model, không cài gói nếu người dùng chưa duyệt trong lượt này. Thiếu runtime/model → dừng, nói rõ thiếu gì và task 🔐 nào trong `docs/ROADMAP.md` cần làm trước.

Theo thành phần:
- **llm**: `scripts/phase0/bench_llm.ps1 -LlamaDir <dir> -Model <gguf> -Backend <cpu|vulkan|sycl> [-Server]`. Runtime ở `E:\Mori Data\runtime\llama.cpp\`, model ở `E:\Mori Data\models\llm\`. Chạy CPU trước nếu chưa có dòng CPU cho model này. Với `-Server`: ghi TTFT lạnh/ấm (prompt_ms) và số token prompt được xử lý.
- **stt**: `scripts/phase0/bench_stt.ps1 -Model "E:\Mori Data\models\stt\<ggml-small.bin|ggml-large-v3-turbo-q5_0.bin>"`. Mặc định dùng bản Vulkan tự build `E:\Mori Data\runtime\whisper.cpp\v1.9.5-vulkan\whisper-cli.exe`. Cần file wav trong `tests/fixtures/audio/` theo `tests/fixtures/stt_manifest.csv`.
- **tts**: `"E:\Mori Data\runtime\python\tts-venv\Scripts\python.exe" -I -X utf8 scripts/phase0/bench_tts.py <korva|zerotts|vieneu> [--mode v3turbo|v3nano] [--voice V]`. Đặt `HF_HOME=E:\Mori Data\models\tts\hf`, `HF_HUB_DISABLE_SYMLINKS=1`, `HF_HUB_OFFLINE=1`. Sau khi chạy, nhắc người dùng nghe file wav trong `scripts/phase0/out/tts/` và cho điểm 1–5.

Sau khi chạy:
1. Thêm **một dòng mới** vào bảng tương ứng trong `docs/MODEL_EVALUATION.md` (không xóa dòng cũ). Ghi ngày, build runtime, điều kiện (cắm sạc/pin).
2. Chỉ ghi số lấy từ output thật. Điểm chủ quan để trống cho người dùng điền.
3. Tick task tương ứng trong `docs/ROADMAP.md`.
4. Tóm tắt 3–5 dòng: số chính, so với mục tiêu SPEC §19.4, có bất thường gì (throttle, lỗi backend).
