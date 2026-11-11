# Rà soát Spec Mori v0

**Đối tượng rà soát:** [archive/MORI_LOCAL_AI_TECHNICAL_SPEC_v0.md](archive/MORI_LOCAL_AI_TECHNICAL_SPEC_v0.md) (do AI free tạo, chưa kiểm chứng)
**Kết quả:** [SPEC.md](SPEC.md) là bản đã sửa và là nguồn sự thật từ nay.
**Ngày rà soát:** 2026-11-11

Phương pháp: tra cứu web từng tuyên bố có thể kiểm chứng (repo, license, phiên bản model), kiểm tra trực tiếp máy đích (ổ đĩa, công cụ cài sẵn), và đánh giá tính khả thi kỹ thuật của từng mục tiêu.

---

## 1. Đánh giá tổng quan

Spec v0 có định hướng **đúng**: local-first, module hóa, CPU fallback, Phase 0 đo đạc trước khi viết code, memory do người dùng kiểm soát. Khung tổng thể giữ nguyên.

Vấn đề chính:
- Một số tham chiếu sai hoặc thiếu (URL repo, license, model mới hơn).
- Một số mục tiêu hiệu năng không khả thi nếu không có kỹ thuật bổ sung (prompt cache).
- Có chỗ thiết kế thừa so với MVP (VAD, 4 file YAML, nhiều tài liệu trùng lặp).
- Tài liệu bị cắt cụt ở §22.1.
- Mâu thuẫn nội bộ (data root, tên tài liệu).

---

## 2. Tuyên bố đã kiểm chứng

| Mục | Kết luận | Ghi chú |
|---|---|---|
| Qwen3-4B, GGUF, llama.cpp | ✅ Đúng | Apache-2.0 |
| whisper.cpp | ✅ Đúng | Máy đã có `whisper-cli.exe` b5454 trong `E:\Memora Data\runtime` (dự án khác) |
| PhoWhisper (VinAI) | ✅ Tồn tại | Là fine-tune của Whisper (định dạng HF Transformers) |
| KorvaTTS (`dogenthq/KorvaTTS`) | ✅ Tồn tại | Apache-2.0, ONNX Runtime, CPU, vi + en lẫn, 44.1 kHz. **Nhưng:** mới v0.1.3 (non trẻ), **không có streaming**, và **tự tải weights từ Hugging Face ở lần chạy đầu** (có tùy chọn `assets_dir` để chạy offline) |
| ZeroTTS (`zeroweight-ai/ZeroTTS`) | ✅ Tồn tại | MIT, v0.1.5, 48 kHz, có `synthesize_stream`. Tác giả công bố RTF ~0.5, chunk đầu ~70 ms trên CPU. Weights fp32 ~900 MB. Có bản GGUF repackage của cộng đồng (`thuongvv/ZeroTTS-GGUF`). Số liệu tự công bố → phải đo lại |
| Live2D Cubism SDK for Web | ✅ Tồn tại | SDK độc quyền, có điều khoản license riêng |
| Phần cứng (Core Ultra 9 285H, Arc 140T, NPU) | ✅ Hợp lý | Arc 140T có trong danh sách hỗ trợ của llama.cpp SYCL |

## 3. Sai sót và chỉnh sửa

### 3.1. Lỗi thực tế

| # | Spec v0 | Thực tế | Sửa |
|---|---|---|---|
| E1 | Tài liệu kết thúc giữa câu ở §22.1 | Bị cắt cụt | Viết lại đầy đủ §22 (quy tắc Claude Code) |
| E2 | VieNeu-TTS: `github.com/bonelag/VieNeuTTS` | Repo chính thức là `github.com/pnnbao97/VieNeu-TTS`, SDK PyPI `vieneu` (3.8.4, 10/11/2026). Bản hiện tại là **v3** (`v3turbo` 48 kHz, `v3nano` 24 kHz ONNX CPU), có streaming, không cần eSpeak NG. Bản v2 (0.5B Apache-2.0 / 0.3B CC BY-NC, GGUF) đã **deprecated**. v4 không mở nguồn | Sửa URL. Đánh giá v3; **license v3 phải kiểm tra** trước khi chọn |
| E3 | Data root `E:\Memora Data\` (§2.1) và `E:\Mori Data\` (§16) | `E:\Memora Data` là dữ liệu của **một dự án khác** đang tồn tại trên máy | Chốt `E:\Mori Data\`. Không ghi vào `E:\Memora Data` (ADR-0002) |
| E4 | `HARDWARE_VALIDATION.md` (§17) vs `HARDWARE_REPORT.md` (§18), thêm `TECHNICAL_DECISIONS.md` | Trùng lặp | Một `HARDWARE_REPORT.md`, một `MODEL_EVALUATION.md`, quyết định ghi bằng ADR |

### 3.2. Thông tin lỗi thời / thiếu ứng viên

| # | Vấn đề | Sửa |
|---|---|---|
| U1 | Chỉ có Qwen3-4B | Thêm **Qwen3.5-4B** (phát hành 02/03/2026, context 256K, đa phương thức) và **Qwen3-4B-Instruct-2507** (non-thinking, phù hợp hội thoại). Qwen3-4B giữ làm baseline. Qwen3.5 cần bản llama.cpp mới hỗ trợ kiến trúc |
| U2 | GPU chỉ nói SYCL | Thử **Vulkan trước**: có binary Windows dựng sẵn, cài ít công hơn. Cộng đồng báo trên Arc rời (B580) Vulkan thường nhanh hơn SYCL; nhưng có báo lỗi Vulkan trên iGPU Lunar Lake/Windows (lỗi tính bộ nhớ). **Không có benchmark công khai cho Arc 140T** → bắt buộc đo cả CPU / Vulkan / SYCL bằng `llama-bench` cùng commit, cùng model |
| U3 | STT chỉ có Whisper base/small | Whisper small cho tiếng Việt thường có WER cao. Thêm **whisper large-v3-turbo (lượng tử hóa q5/q8)** |
| U4 | "PhoWhisper không mặc định tương thích whisper.cpp" | Đúng là cần kiểm chứng, nhưng PhoWhisper cùng kiến trúc Whisper nên **có đường convert** qua script `convert-h5-to-ggml.py` của whisper.cpp. Đưa vào Phase 0 như một bước thử |
| U5 | Không nhắc OpenVINO | OpenVINO là runtime chính thức của Intel cho CPU/GPU/NPU. Ghi là **hướng dự phòng** nếu cả Vulkan và SYCL đều không ổn, không làm trong MVP |

### 3.3. Mục tiêu hiệu năng không khả thi nếu giữ nguyên thiết kế

**TTFT P95 < 2 giây trên CPU.** Model 4B Q4 trên CPU laptop xử lý prompt ở mức khoảng vài chục đến vài trăm token/giây (cần đo). Nếu mỗi lượt gửi lại toàn bộ system prompt + memory + lịch sử (1.5–3K token) thì TTFT có thể lên **10–30 giây**.

Sửa (đưa vào SPEC §5.4):
- Dùng **prompt cache / KV reuse** của `llama-server` (`cache_prompt`).
- Cấu trúc prompt theo thứ tự **ổn định → thay đổi**: system prompt cố định, rồi memory đã duyệt, rồi lịch sử, rồi tin nhắn mới. Khi phần đầu giữ nguyên, mỗi lượt chỉ phải xử lý phần mới.
- Phase 0 phải đo riêng **tốc độ xử lý prompt (pp)** và **tốc độ sinh (tg)**, không chỉ "tokens/s".

### 3.4. Thiết kế thừa so với MVP

| # | Spec v0 | Vì sao thừa | Sửa |
|---|---|---|---|
| S1 | VAD (Silero/WebRTC) trong MVP + VAD worker | MVP dùng **push-to-talk**: người dùng nhấn/thả đã xác định đầu và cuối câu | Bỏ VAD khỏi MVP, hoãn cùng wake-word (ADR-0003) |
| S2 | 4 file YAML (`personality`, `conversation_policy`, `voice_style`, `assistant_settings`) | Tính cách thực chất là một system prompt. Quy tắc an toàn không nên để người dùng/AI sửa | Một file `personality.md`. Settings lưu trong SQLite. Quy tắc an toàn nằm trong code |
| S3 | Gọi `whisper-cli.exe` mỗi lượt | Mỗi lần gọi phải nạp lại model (vài giây) | Dùng `whisper-server` (có sẵn trong whisper.cpp) chạy loopback, giữ model trong RAM khi đang hội thoại |
| S4 | JSON Lines cho mọi worker | llama-server và whisper-server đã có HTTP API | HTTP loopback cho llama/whisper. JSON Lines chỉ dùng cho TTS worker Python |
| S5 | Cây thư mục nguồn đầy đủ từ đầu | Thư mục rỗng "để dành" | Chỉ tạo khi phase cần đến |

### 3.5. Thiếu sót trong thiết kế

| # | Vấn đề | Sửa |
|---|---|---|
| D1 | Không nói rõ ai phát audio và lấy dữ liệu lip-sync từ đâu | **Rust phát audio** (rodio). Rust phát sự kiện biên độ RMS ~30 Hz cho frontend để làm lip-sync. Như vậy giọng nói vẫn chạy khi ẩn avatar (đúng nguyên tắc 6) |
| D2 | Schema memory: `user_approved` + `confidence` | Thiếu trạng thái "bị từ chối", và `confidence` không ai dùng | Thay bằng `status` (`proposed` / `approved` / `rejected`). Thêm bảng FTS5, `PRAGMA user_version` cho migration, index cho `messages` |
| D3 | "Xóa phải được kiểm chứng" nhưng không nói cách | SQLite mặc định để lại dữ liệu cũ trong trang trống | `PRAGMA secure_delete=ON`, rebuild FTS sau khi xóa, `VACUUM` khi xóa toàn bộ. Có test đọc thô file DB để chứng minh |
| D4 | Personality ở Phase 4 | Prompt tính cách rất rẻ, nên áp từ đầu để test hội thoại đúng giọng | Chuyển personality prompt sang Phase 1. Memory giữ ở Phase 4 |
| D5 | Spec ghi pagefile "cố định 2 GB" | Đo thực tế: pagefile **do Windows quản lý**, đang cấp 2 GB, giới hạn commit 33.4 GB. Lúc đo chỉ còn **4.2 GB commit trống** và 8.6 GB RAM trống do ứng dụng khác | Đo RAM trống ở trạng thái làm việc thường ngày trước khi chốt ngân sách. Nếu thiếu: đóng ứng dụng nền hoặc đặt pagefile tối thiểu ≥ 8 GB |
| D6 | Python không nói phiên bản | Máy có **Python 3.14**. `vieneu` khai báo hỗ trợ 3.10–3.13; ONNX Runtime/PyTorch có thể chưa hỗ trợ 3.14 | Dùng **uv** tạo venv Python 3.12 riêng trong `E:\Mori Data\runtime\` |
| D7 | Live2D: chỉ ghi "phải rà license" | Cubism Core là binary độc quyền | Không commit Cubism Core vào git. Dùng model mẫu chính thức khi phát triển. Rà license trước khi phát hành |
| D8 | Rủi ro echo mức Medium | Với push-to-talk, mic không mở trong khi Mori nói | Hạ xuống Low cho MVP. Khi nhấn nói lúc Mori đang nói → dừng TTS trước khi mở mic |
| D9 | Thiếu bảo mật cho WebView | — | Tauri: CSP chặt, không bật plugin `shell`, capability tối thiểu, frontend chỉ gọi lệnh Rust được khai báo |
| D10 | KorvaTTS tự tải model lần đầu | Vi phạm "không tự động tải" | Tải trước có duyệt, pin phiên bản + SHA-256, chạy với biến môi trường offline (`HF_HUB_OFFLINE=1`) |

## 4. Những gì giữ nguyên

- Tauri 2 + React + TypeScript + Rust core.
- llama.cpp làm runtime LLM. CPU là baseline bắt buộc.
- SQLite + FTS5. Vector search hoãn lại.
- Push-to-talk, không ghi âm nền, không lưu audio.
- Live2D, volume-based lip-sync trước, viseme sau.
- Ba ứng viên TTS (KorvaTTS, VieNeu-TTS 0.5B, ZeroTTS), so sánh bằng nghe thực tế + đo trên máy.
- Thứ tự phase tổng thể, nguyên tắc gate.
- Danh sách tính năng hoãn (§21).

## 5. Câu hỏi còn mở (giải quyết trong Phase 0)

1. Backend GPU nào chạy ổn định trên Arc 140T: Vulkan, SYCL, hay không cái nào?
2. Model LLM nào có tiếng Việt tốt nhất trong ngân sách độ trễ: Qwen3-4B, Qwen3-4B-Instruct-2507, hay Qwen3.5-4B?
3. STT: whisper large-v3-turbo có đủ nhanh trên CPU không? PhoWhisper convert được và có tốt hơn không?
4. TTS nào tự nhiên nhất, và có đạt first-audio < 1.5 s không?
5. Có cần Python cho TTS không (ZeroTTS có bản GGUF cộng đồng có thể chạy không cần Python)?
6. License của VieNeu-TTS v3 là gì?

## 6. Nguồn

- [KorvaTTS — PyPI](https://pypi.org/project/korvatts/) · [GitHub](https://github.com/dogenthq/KorvaTTS)
- [VieNeu-TTS — GitHub](https://github.com/pnnbao97/VieNeu-TTS)
- [ZeroTTS — PyPI](https://pypi.org/project/zerotts/)
- [ZeroTTS — GitHub](https://github.com/zeroweight-ai/ZeroTTS) · [Hugging Face](https://huggingface.co/zeroweight-ai/ZeroTTS) · [GGUF](https://huggingface.co/thuongvv/ZeroTTS-GGUF)
- [VieNeu-TTS — PyPI `vieneu`](https://pypi.org/project/vieneu/) · [tổng quan dự án](https://www.sourcepulse.org/projects/20499710)
- [Qwen3.5 Small series — thông báo của Qwen](https://x.com/Alibaba_Qwen/status/2028460046510965160) · [Unsloth GGUF](https://huggingface.co/collections/unsloth/qwen35)
- [llama.cpp SYCL trên Intel GPU — thảo luận](https://github.com/ggml-org/llama.cpp/discussions/23313)
- [Lỗi llama.cpp Vulkan trên Lunar Lake/Windows](https://github.com/ggml-org/llama.cpp/issues/18946)
