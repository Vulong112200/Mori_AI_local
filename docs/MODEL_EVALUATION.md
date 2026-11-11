# Đánh giá model

> Mỗi lần đo dùng `/bench <llm|stt|tts>`. Thêm dòng mới, **không xóa** dòng cũ. Ghi build runtime, ngày, điều kiện (cắm sạc/pin).
> Chỉ ghi số đo trên máy đích. Số tác giả công bố ghi ở cột "Ghi chú" và đánh dấu "(công bố)".

## 1. Danh sách ứng viên

### LLM
| Model | Nguồn | License | Kích thước | sha256 | Trạng thái |
|---|---|---|---|---|---|
| Qwen3-4B Q4_K_M | huggingface.co/Qwen/Qwen3-4B-GGUF | Apache-2.0 | ~2.5 GB | | Chưa tải |
| Qwen3-4B-Instruct-2507 Q4_K_M | Hugging Face (kiểm tra repo chính thức trước khi tải) | Apache-2.0 | ~2.5 GB | | Chưa tải |
| Qwen3.5-4B Q4_K_M | huggingface.co/collections/unsloth/qwen35 | Kiểm tra model card | ~2.7 GB | | Chưa tải |

### STT
| Model | Nguồn | License | sha256 | Trạng thái |
|---|---|---|---|---|
| Whisper small (ggml) | huggingface.co/ggerganov/whisper.cpp | MIT | | Chưa tải |
| Whisper large-v3-turbo q5_0 (ggml) | huggingface.co/ggerganov/whisper.cpp | MIT | | Chưa tải |
| PhoWhisper (small/medium) | huggingface.co/vinai | Kiểm tra model card | | Chưa convert |

### TTS
| Engine | Nguồn | License | sha256 | Trạng thái |
|---|---|---|---|---|
| KorvaTTS 0.1.3 | github.com/dogenthq/KorvaTTS · PyPI `korvatts` | Apache-2.0 | | Chưa cài — không streaming |
| VieNeu-TTS v3 (turbo, nano) | github.com/pnnbao97/VieNeu-TTS · PyPI `vieneu` 3.8.x | **Kiểm tra** (v2 0.3B là CC BY-NC) | | Chưa cài |
| ZeroTTS 0.1.5 | github.com/zeroweight-ai/ZeroTTS · PyPI `zerotts` | MIT | | Chưa cài |

## 2. Kết quả LLM

| Ngày | Model | Backend | Build | pp512 (t/s) | tg128 (t/s) | TTFT lạnh (s) | TTFT ấm (s) | RAM đỉnh | Điểm VN (1–5) | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|---|

## 3. Kết quả STT

| Ngày | Model | Backend | Số câu | WER | CER | Độ trễ TB (s) | P95 (s) | RAM | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|

## 4. Kết quả TTS

| Ngày | Engine | Giọng | First audio (s) | RTF | CPU % | RAM | Tự nhiên | Phát âm | Nhịp | vi-en | Ghi chú |
|---|---|---|---|---|---|---|---|---|---|---|---|

## 5. Kết luận

_Ghi tóm tắt + link ADR sau khi chọn._
