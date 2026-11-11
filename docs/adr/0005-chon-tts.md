# ADR-0005: TTS — VieNeu-TTS v3 Turbo (ONNX/CPU)

**Trạng thái:** Đề xuất — người dùng thích giọng ZeroTTS; chờ kết quả tăng tốc ZeroTTS
**Ngày:** 2026-11-12

## Bối cảnh
Phase 0 đo 3 engine trên CPU, chạy offline, 18 câu ([MODEL_EVALUATION.md](../MODEL_EVALUATION.md) §4):

| Engine | First audio TB / P95 | RTF | RAM |
|---|---|---|---|
| ZeroTTS 0.1.5 fp32 | 0.52 / 0.66 s | 1.87 ❌ | 1.37 GB |
| KorvaTTS 0.1.3 (8–32 bước) | 9.4–30.7 s ❌ | 1.87–6.09 ❌ | 0.69 GB |
| VieNeu v3 Turbo fp32 | 0.58 / 0.75 s | 0.89 | 1.35 GB |
| **VieNeu v3 Turbo int8** | **0.39 / 0.50 s** | **0.74** | **0.85 GB** |
| VieNeu v3 Nano | 2.12 / 3.14 s | 0.42 | 0.45 GB |

Mục tiêu SPEC §19.4: first audio P95 < 1.5 s; RTF phải < 1 để phát liên tục.

Nghe thử (2026-11-12): người dùng thấy **giọng ZeroTTS phù hợp nhất**; VieNeu v3 Turbo int8 **không bị méo**.

## Quyết định
- Engine: **VieNeu-TTS v3 Turbo** (`vieneu` 3.8.4, Apache-2.0), ONNX/CPU, streaming `infer_stream`, 48 kHz.
- Độ chính xác: **int8** (người dùng nghe không méo).
- Chạy trong worker Python 3.12 (uv venv), giao thức JSON Lines (SPEC §13), biến môi trường `HF_HOME`, `HF_HUB_DISABLE_SYMLINKS=1`, `HF_HUB_OFFLINE=1`.
- Giọng mặc định: chọn sau khi nghe (25 giọng có sẵn).

Nếu ZeroTTS tăng tốc được tới RTF < 1 (int8 / chỉnh threads) → đổi quyết định sang ZeroTTS (sửa ADR này). Nếu không → giữ VieNeu v3 Turbo int8.

## Phương án đã cân nhắc
- ZeroTTS — ra tiếng nhanh nhưng RTF 1.87 (tạo chậm hơn phát → ngắt quãng). Có thể xem lại nếu bản int8 của họ đạt RTF < 1.
- KorvaTTS — không streaming, quá chậm → loại.
- VieNeu v3 Nano — nhẹ, RTF tốt nhưng first audio 2.1 s.

## Hệ quả
- Cập nhật SPEC §7.1 (engine chọn).
- RTF 0.74–0.89 chỉ dư ít; khi LLM (GPU) và TTS (CPU) chạy đồng thời phải đo lại ở Phase 3. Thử `threads` của VieNeu nếu thiếu.
- Python vẫn cần trong runtime (không có bản GGUF/không-Python cho v3).
