# ADR-0008: LLM mặc định Qwen3-8B, Qwen3-4B-Instruct-2507 làm "chế độ nhanh"

**Trạng thái:** Chấp nhận (2026-11-12) — thay thế [ADR-0004](0004-chon-llm.md)
**Ngày:** 2026-11-12

## Bối cảnh
ADR-0004 chọn Qwen3-4B-Instruct-2507 vì nhanh nhất, kèm điều kiện thử model ~8B. Đã đo Qwen3-8B Q4_K_M trên cùng máy, cùng llama.cpp b12284 Vulkan ([MODEL_EVALUATION.md](../MODEL_EVALUATION.md) §2):

| | Qwen3-4B-Instruct-2507 | Qwen3-8B |
|---|---|---|
| TTFT lạnh / ấm | 0.55 / 0.25 s | 1.16 / 0.40 s |
| Tốc độ sinh | 23.7 tok/s | 11.8 tok/s (≈ 2× mức giọng nói cần) |
| RAM llama-server | 4.1 GB | 6.2 GB |
| Chất lượng tiếng Việt (Claude chấm, 6 prompt) | 3.5 | 4 |
| Ước tính thả PTT → nghe tiếng đầu | ≈ 2.5–2.8 s | ≈ 3.3–3.6 s |

8B đúng kiến thức hơn rõ, không lẫn ngôn ngữ, và có chế độ "thinking" bật được khi cần (SPEC §5.2). Người dùng đồng ý chọn 8B mặc định.

## Quyết định
- Model mặc định: **Qwen3-8B Q4_K_M** (Qwen chính thức, sha256 `d98cdcbd…2c5745785`), Vulkan `-ngl 99`, ctx 8192, `--reasoning off` mặc định.
- **Chế độ nhanh** (setting người dùng bật được): **Qwen3-4B-Instruct-2507 Q4_K_M**.
- Đổi model = khởi động lại `llama-server` với file khác; không đổi code.
- Phase 1 phải đo RAM khi chạy đủ bộ; nếu thiếu → hạ mặc định xuống 4B (sửa ADR này).

## Phương án đã cân nhắc
- Giữ 4B mặc định (ADR-0004) — nhanh, nhẹ hơn nhưng sai kiến thức nhiều hơn.
- Qwen3.5-4B — chậm trên Vulkan, chất lượng thấp nhất.

## Hệ quả
- SPEC §5.1–5.2 cập nhật. ADR-0004 → "Thay thế bởi ADR-0008".
- App cần setting `llm_model` + nút "Chế độ nhanh".
- Thinking theo yêu cầu (câu hỏi khó) — để sau Phase 1.
