# ADR-0011: STT đổi sang Whisper large-v3 đầy đủ (q5_0), thay turbo

**Trạng thái:** Chấp nhận (2026-11-13) — thay phần chọn model của [ADR-0007](0007-chon-stt.md); runtime giữ nguyên
**Ngày:** 2026-11-13

## Bối cảnh
ADR-0007 chọn turbo dựa trên 18 câu **giọng tổng hợp** (WER 7.8 %). Đo trên **100 câu giọng thật** người dùng thu trong app (MODEL_EVALUATION §3.1), đúng pipeline app (whisper-server `-l vi -bs 5 -bo 5 -nt`, không prompt), Vulkan, độ trễ không tính nạp model. Cột "số → chữ": đổi số sang chữ ở cả câu gốc và kết quả (PhoWhisper viết số bằng chữ, câu gốc viết bằng số).

| Model (q5_0) | WER | WER số → chữ | đời thường | miền Nam | nhiễu/nhanh | số/dài | thuật ngữ | Độ trễ TB / P95 | Dung lượng |
|---|---|---|---|---|---|---|---|---|---|
| large-v3-turbo | 31.0 % | 28.8 % | 14.3 % | 37.2 % | 31.2 % | 22.8 % | 42.7 % | 1.19 / 1.49 s | 0.57 GB |
| **large-v3** | **23.0 %** | **20.6 %** | 14.8 % | **27.1 %** | **25.5 %** | 11.4 % | **32.3 %** | 1.77 / 2.41 s | 1.08 GB |
| PhoWhisper-large (VinAI, BSD-3, tự chuyển + lượng tử) | 34.4 % | 24.8 % | **8.8 %** | 31.4 % | 30.6 % | **10.6 %** | 54.7 % | 2.13 / 3.15 s | 1.08 GB |

Thử thêm (large-v3): chuẩn hóa âm lượng về đỉnh 0.9 → **kém hơn** (20.6 % → 22.2 %) → không dùng.

## Quyết định
- Model: **`ggml-large-v3-q5_0.bin`** (ggerganov/whisper.cpp, MIT, sha256 `d75795ec…ad1`) ở `E:\Mori Data\models\stt\`.
- Giữ turbo trên đĩa làm phương án dự phòng (nhanh hơn ~0.6 s) nếu e2e vượt 4 s ở Phase 3.
- Giữ bản PhoWhisper đã chuyển (`ggml-phowhisper-large-q5_0.bin`, sha256 `1ecff4db…109c`) để đo lại sau khi thu lại; tốt nhất cho tiếng Việt thuần nhưng kém rõ với từ tiếng Anh — điểm yếu nặng với trợ lý lập trình.
- Gate Phase 2 **chưa đạt** ngưỡng ADR-0007 (≤ 15 %). Trước khi tính tới đổi ngưỡng: người dùng thu lại bộ 100 câu với micro đã sửa (chờ tín hiệu thật, lọc chống alias) và xem "Mori nghe được" để thu lại câu đọc lệch → đo lại cả 3 model.

**Đo lại sau khi thu lại (2026-11-13):** large-v3 **WER 9.3 %** (số → chữ 7.7 %), nhóm cao nhất thuật ngữ 16.9 %, 1.67 / 2.08 s → **đạt ngưỡng ADR-0007**. Turbo 12.0 % (miền Nam 15.5 %), 1.11 / 1.29 s — cũng đạt, giữ làm dự phòng nếu e2e > 4 s.

## Phương án đã cân nhắc
- Turbo — nhanh nhất nhưng WER cao nhất ở giọng miền Nam và thuật ngữ.
- PhoWhisper — tổng thể kém large-v3 vì từ tiếng Anh; chậm nhất (P95 3.15 s).
- Kết hợp 2 model (chọn theo độ tin cậy) — phức tạp, gấp đôi RAM/độ trễ; xem lại nếu cần.

## Hệ quả
- RAM/VRAM STT ~1.1 GB (gấp đôi turbo) khi đang nạp; whisper-server vẫn tắt sau 5 phút rảnh.
- Độ trễ STT +0.6 s TB → e2e ước tính ~3.2 s TB (Phase 0: 2.63 s với turbo); **đo lại ở Phase 3**, vượt 4 s → quay về turbo hoặc xem lại.
- Công cụ chuyển đổi: venv `E:\Mori Data\runtime\python\convert-venv` (torch 2.15 CPU, transformers 5.22), `whisper-quantize` build từ `src-v1.9.5`. Nguồn PhoWhisper fp32 ở `models\stt\src\` (~9 GB) — xóa được khi không cần đo lại.
