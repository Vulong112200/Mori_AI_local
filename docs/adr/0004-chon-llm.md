# ADR-0004: LLM — Qwen3-4B-Instruct-2507 Q4_K_M trên llama.cpp Vulkan

**Trạng thái:** Thay thế bởi [ADR-0008](0008-llm-8b-mac-dinh.md) (2026-11-12)
**Ngày:** 2026-11-12

## Bối cảnh
Phase 0 đã đo 3 model 4B trên Arc 140T (llama.cpp b12284), số liệu trong [MODEL_EVALUATION.md](../MODEL_EVALUATION.md) §2:

| Model | Backend | pp512 | tg128 | TTFT lạnh / ấm | Điểm VN* |
|---|---|---|---|---|---|
| Qwen3-4B | CPU | 108–133 | 8–16 (dao động) | 4.38 / 0.26 s | — |
| Qwen3-4B | Vulkan | 888 | 23.2 | 0.60 / 0.24 s | 3 |
| **Qwen3-4B-Instruct-2507** | **Vulkan** | **983** | **23.7** | **0.55 / 0.25 s** | **3.5** |
| Qwen3.5-4B | Vulkan | 784 | 20.5 | 2.25 / 1.7 s | 2.5 |

\* chấm sơ bộ của Claude trên AI-01..06, chờ người dùng chấm.

## Quyết định
- Model mặc định: **Qwen3-4B-Instruct-2507 Q4_K_M** (bản GGUF của unsloth, sha256 `3605803b…c67e597`).
- Backend: **Vulkan**, `-ngl 99`, ctx 8192, `cache_prompt` bật. CPU là fallback khi Vulkan lỗi.
- Không cần SYCL.
- App phải gửi một request warm-up ngay sau khi `llama-server` khởi động (lượt đầu Vulkan mất ~4–5 s để biên dịch shader).
- Lọc emoji và ký hiệu markdown khỏi câu trả lời trước khi đưa vào TTS (model hay chèn emoji).

## Phương án đã cân nhắc
- Qwen3-4B (hybrid thinking) — cùng tốc độ, nhưng lẫn chữ tiếng Nga vào câu tiếng Việt.
- Qwen3.5-4B — chậm hơn rõ (TTFT ấm 1.7 s), dài dòng, sai kiến thức nhiều nhất.
- CPU-only — tốc độ sinh dao động mạnh theo nhiệt/điện năng, TTFT lạnh 4.4 s.

## Hệ quả
- Cập nhật SPEC §5.1 (model chọn), §5.2 (backend Vulkan), §15.4 (LLM chạy GPU).
- Cả 3 model 4B còn sai kiến thức phổ thông tiếng Việt. Vulkan dư tốc độ (tiếng nói chỉ cần ~6 tok/s) → **đề xuất thử thêm một model ~8B** trước khi chốt hẳn. Nếu 8B tốt hơn rõ và TTFT ấm < 1 s → ADR mới thay thế.
- Xem lại khi: Vulkan gặp lỗi driver, hoặc có model nhỏ mới tốt hơn rõ về tiếng Việt.
