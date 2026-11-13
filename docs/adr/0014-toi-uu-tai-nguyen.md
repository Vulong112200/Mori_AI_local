# ADR-0014: Tối ưu RAM llama-server — KV cache q8_0, giới hạn cache prompt

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-13

## Bối cảnh
Người dùng yêu cầu (2026-11-13) Mori "đỡ tốn RAM hơn, phản hồi chính xác và nhanh hơn". Đo Phase 6: llama-server Qwen3-8B 6.3 GB, whisper-server large-v3 ~2.3 GB (tự tắt sau 5 phút rảnh), TTS ~0.8 GB, WebView ~1 GB. Arc 140T là iGPU dùng chung RAM → bộ nhớ "VRAM" của model chính là RAM máy.

Đọc log llama-server b12284 (`-lv 4`): `flash_attn = auto` → **đã bật** (Flash Attention enabled); KV cache f16 1152 MiB; `--cache-ram` mặc định 8192 MiB (cache prompt trên RAM host, dùng khi việc nền — tự ghi nhớ, tóm tắt — chen vào slot duy nhất).

## Số đo (2026-11-13, app tắt, `cargo test e2e_gate -- --ignored`, 20 câu giọng thật; RAM lấy mẫu 3 s)

| Cấu hình | llama-server WS / Private max | Nạp model | TTFT TB / P95 | e2e TB / P95 |
|---|---|---|---|---|
| Hiện tại: FA auto (bật), KV f16 | 6319 / 6382 MB | 3.6 s | 396 / 441 ms | 2.90 / 3.14 s |
| **A: `-fa on -ctk q8_0 -ctv q8_0`** | **5793 / 5845 MB** | 7.8 s* | 393 / 440 ms | 3.03 / 3.69 s |
| A + `--cache-ram 1024 -lm none` (bỏ mmap) | 5783 / 5845 MB | 3.6 s | 373 / 387 ms | 2.88 / 3.23 s |
| D: `-fa off`, KV f16 | 6832 / 6887 MB | 6.6 s* | 324 / 351 ms | 2.70 / 3.08 s |

\* lần nạp đầu sau khi đổi cờ (Vulkan biên dịch lại pipeline); số e2e dao động chủ yếu theo độ dài câu đầu model sinh (nhiệt độ 0.7) — TTFT A bằng hiện tại.

`llama-bench` (Qwen3-8B Q4_K_M, Vulkan, độ sâu context 3000): FA + q8_0 pp64 139 / pp512 184 / tg 12.4 tok/s; FA tắt + f16 pp64 198 / pp512 356 / tg 13.2 tok/s. Ở độ sâu 4096: FA + f16 (= hiện tại) pp512 chỉ 78 tok/s, FA + q8_0 118 tok/s. FA tắt + K q8_0: tg 0.78 tok/s (không dùng được).

## Quyết định
llama-server thêm `-fa on -ctk q8_0 -ctv q8_0 --cache-ram 1024` (`llm.rs`, giữ nguyên mọi biện pháp ADR-0009).
- Không dùng `-lm none`: bỏ mmap không giảm RAM (5783 so với 5793 MB).
- KV q8_0: −526 MB RAM, TTFT không đổi, xử lý prompt dài nhanh hơn hiện tại (118 so với 78 tok/s ở 4k).
- `--cache-ram 1024`: giới hạn cache prompt trên RAM host ở 1 GB (mặc định 8 GB) — đủ ~3 trạng thái hội thoại 4k token (q8_0 ≈ 78 KB/token) để khôi phục sau việc nền.

## Phương án đã cân nhắc
- D (tắt flash-attn, KV f16): TTFT −70 ms nhưng +513 MB so với hiện tại (+1.04 GB so với A). RAM là giới hạn chính của máy (thường còn ~7–8 GB trống) → không chọn; xem lại nếu người dùng ưu tiên tốc độ hơn RAM.
- KV q4_0: tiết kiệm thêm ~0.3 GB nhưng rủi ro chất lượng ngữ cảnh dài (AI-08) — chưa cần.
- Giảm `-c 8192`: không được, AI-08 cần ~4 200 token đầu vào + lịch sử (prompt.rs).
- Chế độ nhanh 4B mặc định: người dùng đã chọn 8B (ADR-0008).

## Hệ quả
- Gate cũ phải giữ: `llm_gate` (AI-06/AI-08, tự ghi nhớ) chạy lại với cờ mới.
- Đổi bản llama.cpp → đo lại (hành vi `auto` và hiệu năng FA trên Vulkan thay đổi theo bản).
- SPEC §19.5 / MODEL_EVALUATION §5.4 ghi số đo.
