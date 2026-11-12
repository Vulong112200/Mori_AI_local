# ADR-0007: STT — Whisper large-v3-turbo q5_0 trên whisper.cpp Vulkan (tự build)

**Trạng thái:** Chấp nhận (2026-11-12) — vẫn phải kiểm chứng trên giọng thật trước gate Phase 2
**Ngày:** 2026-11-12

## Bối cảnh
whisper.cpp không phát hành bản Windows dựng sẵn có Vulkan (bản dựng sẵn mới nhất b5454 chỉ có CPU). Trên CPU không có cấu hình nào vừa nhanh vừa chính xác. Đã build whisper.cpp v1.9.5 với `GGML_VULKAN=ON` (MSVC Build Tools 2022, Vulkan SDK 1.4.363, CMake 4.4.4).

Số đo trên 18 câu **giọng tổng hợp** ([MODEL_EVALUATION.md](../MODEL_EVALUATION.md) §3):

| Cấu hình | WER | Độ trễ TB / P95 |
|---|---|---|
| turbo q5_0, CPU b5454 | 8.4 % | 10.3 / 10.9 s |
| turbo q5_0, CPU `-ac 768 -fa` | 13.9 % | 4.2 / 5.1 s |
| small, CPU `-ac 768 -fa` | 20.9 % | 1.2 / 1.8 s |
| **turbo q5_0, Vulkan v1.9.5** | **7.8 %** | **1.53 / 1.77 s** |
| small, Vulkan v1.9.5 | 16.9 % | 1.12 / 1.73 s |

## Quyết định
- Model: **Whisper large-v3-turbo q5_0** (ggml, MIT, sha256 `39422170…5a55ffa7e2`).
- Runtime: **whisper.cpp v1.9.5 build Vulkan** ở `E:\Mori Data\runtime\whisper.cpp\v1.9.5-vulkan\` (mã nguồn `src-v1.9.5`, thư mục build `build-vulkan`). Dùng `whisper-server` (loopback) trong app.
- Không còn phụ thuộc `E:\Memora Data`.
- ~~Phase 2: thêm prompt từ vựng~~ — **đã bỏ** (đo 100 câu giọng thật: prompt làm Whisper bịa câu nhiều, WER không giảm). whisper-server chạy `-l vi -bs 5 -bo 5 -nt` (= cấu hình whisper-cli đã đo); lọc câu "bịa" đã biết.

## Phương án đã cân nhắc
- small (CPU hoặc GPU) — nhanh hơn chút nhưng WER gấp đôi.
- turbo trên CPU — chính xác nhưng 10 s.
- PhoWhisper — chưa thử; xem lại nếu giọng thật cho WER cao.

## Hệ quả
- Cập nhật SPEC §6.1, §15.4 (STT chạy GPU).
- LLM và STT cùng dùng iGPU nhưng chạy tuần tự trong một lượt hội thoại (STT xong mới tới LLM) → không tranh chấp; Phase 2 phải đo lại khi cả hai model cùng nằm trong bộ nhớ GPU.
- Build từ mã nguồn → cần ghi lại lệnh build (SPEC/README) để tái tạo; khi nâng cấp whisper.cpp phải build lại.
- **Ngưỡng gate Phase 2 (chốt 2026-11-12 trước khi thu; người dùng thu và đo theo ngưỡng này, chốt trước khi thu để không bị kết quả dẫn dắt):** trên 100 câu giọng thật, cấu hình đúng như app: **WER tổng ≤ 15 %** (≈ 2× giọng tổng hợp) và **không nhóm nào > 25 %**. Không đạt → thử model khác (PhoWhisper, large-v3), low-pass trước khi resample, đo lại.
- **Kết quả đo lần 1 (2026-11-12):** WER **31.0 %** (đời thường 14 %, miền Nam 38 %, thuật ngữ 44 %) → **chưa đạt**, kích hoạt điều kiện xem lại bên dưới. Chi tiết: MODEL_EVALUATION §3.1.
- **Đo lần 2 (2026-11-13, thu lại sau khi sửa micro):** large-v3 9.3 %, turbo 12.0 % → **đạt**. Model đổi sang large-v3 theo [ADR-0011](0011-stt-large-v3.md).
- **Điều kiện xem lại:** WER trên bộ giọng thật (≥ 20 câu) cao bất thường so với giọng tổng hợp.
