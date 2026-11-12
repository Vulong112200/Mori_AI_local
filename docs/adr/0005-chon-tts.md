# ADR-0005: TTS — ZeroTTS int8 (ONNX/CPU)

**Trạng thái:** Chấp nhận (2026-11-12)
**Ngày:** 2026-11-12

## Bối cảnh
Phase 0 đo 3 engine trên CPU, chạy offline, 18 câu ([MODEL_EVALUATION.md](../MODEL_EVALUATION.md) §4):

| Engine | First audio TB / P95 | RTF | RAM |
|---|---|---|---|
| ZeroTTS 0.1.5 fp32 | 0.52 / 0.66 s | 1.87 ❌ | 1.37 GB |
| KorvaTTS 0.1.3 (8–32 bước) | 9.4–30.7 s ❌ | 1.87–6.09 ❌ | 0.69 GB |
| ZeroTTS 0.1.5 fp32, 6 luồng | 0.32 / 0.50 s | 1.05 ⚠️ | 1.37 GB |
| **ZeroTTS 0.1.5 int8, 4 luồng** | **0.06 / 0.07 s** | **0.31** | **0.82 GB** |
| VieNeu v3 Turbo fp32 | 0.58 / 0.75 s | 0.89 | 1.35 GB |
| **VieNeu v3 Turbo int8** | **0.39 / 0.50 s** | **0.74** | **0.85 GB** |
| VieNeu v3 Nano | 2.12 / 3.14 s | 0.42 | 0.45 GB |

Mục tiêu SPEC §19.4: first audio P95 < 1.5 s; RTF phải < 1 để phát liên tục.

Nghe thử (2026-11-12): người dùng thấy **giọng ZeroTTS phù hợp nhất**; VieNeu v3 Turbo int8 **không bị méo**. Bản ZeroTTS int8: "ổn, hơi ngọng và giọng miền Nam vài chỗ nhưng rất dễ thương".

## Quyết định
- Engine: **ZeroTTS 0.1.5** (MIT), ONNX/CPU, streaming `synthesize_stream`, 48 kHz — giọng người dùng thích nhất.
- Model: bản **int8** tạo local từ weights fp32 chính thức bằng `scripts/phase0/quantize_zerotts_int8.py`, đặt ở `E:\Mori Data\models\tts\zerotts-int8\`. Model Manager phải ghi sha256 của bản int8 sau khi tạo.
- `intra_op_num_threads=4` (6 và 8 luồng chậm hơn trên 285H).
- Chạy trong worker Python 3.12 (uv venv), giao thức JSON Lines (SPEC §13), biến môi trường `HF_HOME`, `HF_HUB_DISABLE_SYMLINKS=1`, `HF_HUB_OFFLINE=1`.
- Giọng mặc định: `maichi` (người dùng đã nghe); có 8 giọng.
- Dự phòng: VieNeu v3 Turbo int8 (first audio 0.39 s, RTF 0.74, người dùng nghe không méo).

## Phương án đã cân nhắc
- ZeroTTS fp32 — RTF 1.05–1.87 (tạo chậm hơn phát → ngắt quãng). int8 giải quyết được.
- VieNeu v3 Turbo int8 — đạt chỉ tiêu, nhưng người dùng thích giọng ZeroTTS hơn; giữ làm dự phòng.
- KorvaTTS — không streaming, quá chậm → loại.
- VieNeu v3 Nano — nhẹ, RTF tốt nhưng first audio 2.1 s.

## Hệ quả
- Cập nhật SPEC §7.1 (engine chọn).
- RTF 0.31 dư nhiều; vẫn phải đo lại khi LLM (GPU) và TTS (CPU) chạy đồng thời ở Phase 3.
- Python vẫn cần trong runtime. ZeroTTS có bản GGUF cộng đồng (`thuongvv/ZeroTTS-GGUF`) — có thể xem lại để bỏ Python sau MVP.
