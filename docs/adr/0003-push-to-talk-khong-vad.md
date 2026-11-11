# ADR-0003: MVP dùng push-to-talk, không có VAD

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-11

## Bối cảnh
Spec v0 đưa VAD (Silero/WebRTC) và một VAD worker vào MVP, đồng thời yêu cầu MVP bắt đầu bằng push-to-talk.

## Quyết định
MVP chỉ dùng push-to-talk: nhấn giữ = thu, thả = kết thúc câu. Không có VAD, không có VAD worker.

## Phương án đã cân nhắc
- PTT + VAD cắt im lặng đầu/cuối — lợi ích nhỏ (Whisper chịu được im lặng ngắn), thêm một model và một worker.
- VAD rảnh tay — cần xử lý echo và mic mở liên tục, mâu thuẫn "không ghi âm nền mặc định".

## Hệ quả
- Ít thành phần hơn, ít rủi ro riêng tư hơn.
- Rủi ro echo giảm: khi nhấn PTT lúc Mori đang nói → dừng TTS trước khi mở micro.
- VAD + wake-word sẽ thiết kế cùng nhau sau MVP (ADR mới).
