# ADR-0006: Điều kiện dùng Live2D Cubism SDK for Web

**Trạng thái:** Chấp nhận (2026-11-12)
**Ngày:** 2026-11-12

## Bối cảnh
SPEC chọn Live2D cho avatar. Rà điều khoản chính thức (đọc 2026-11-12; EULA bản sửa đổi 03/02/2025):

- **Hai phần license khác nhau:**
  - *Cubism Core* — độc quyền (Live2D Proprietary Software License). Chỉ được phân phối lại các file trong `RedistributableFiles.txt`, kèm sản phẩm cho người dùng cuối. Không được đưa lên server công khai → **không commit vào repo GitHub**.
  - *Cubism Web Framework* — "Live2D Open Software License" (không phải license mã nguồn mở chuẩn). Không được phát hành lại code SDK dưới license khác.
- **Giấy phép phát hành (Publication License):** chỉ cần khi **phát hành**, không cần khi phát triển/thử nghiệm. Cá nhân và doanh nghiệp nhỏ (doanh thu < 10 triệu yên/năm) được miễn phí. Với app phân phối miễn phí (gói D), mức "General User / Small-Scale" là **Free**. Việc miễn trừ do Live2D quyết định; phải làm thủ tục **trước ngày phát hành ít nhất 1 tháng**.
- **Expandable Application** (app avatar/livestream cho người dùng nạp model Live2D tùy ý): **không được miễn**, phải xin duyệt và ký thỏa thuận đặc biệt.
- **Model mẫu** (Free Material License v1.6):
  - Hiyori, Haru, Niziiro Mao (nhân vật gốc của Live2D): cá nhân được dùng và phân phối; Hiyori **cấm sửa thiết kế**; phải ghi "This content uses sample data owned and copyrighted by Live2D Inc."
  - Natori (nhân vật hợp tác): **không được** dùng thương mại, sửa hay phân phối → không dùng.
  - Không dùng dữ liệu âm thanh mẫu.

Nguồn: live2d.com/en/sdk/license/, Live2D Proprietary Software License Agreement v2.1, Live2D Open Software License Agreement v1.4, Live2D Free Material License Agreement v1.6, live2d.com/en/learn/sample/model-terms/, github.com/Live2D/CubismWebFramework (LICENSE.md). Đây là cách đọc điều khoản, không phải tư vấn pháp lý.

## Quyết định
- Dùng Live2D Cubism SDK for Web cho MVP (cá nhân, không phát hành) — không cần thủ tục.
- Cubism Core tải thủ công vào `E:\Mori Data\avatar\` hoặc thư mục gitignored; **không commit**.
- Mori dùng **một (hoặc vài) nhân vật cố định đóng gói sẵn**; **không làm tính năng cho người dùng nạp model Live2D tùy ý** — để không thành "Expandable Application".
- **Hướng nhân vật (người dùng chọn 2026-11-12): nữ, phong cách chibi/SD, dễ thương** — hợp với giọng ZeroTTS `maichi`.
- Khi phát triển: dùng **Koharu** (bé gái trong mẫu "Koharu & Haruto" — mẫu duy nhất được Live2D mô tả là SD và là nữ; nhân vật gốc của Live2D, không có điều khoản riêng; có biểu cảm mắt lấp lánh, nước mắt, nhiều chuyển động). Hiển thị dòng ghi công bản quyền mẫu trong màn hình About. Phase 5 phải kiểm tra: Koharu là model riêng hay chung file với Haruto; tương thích Cubism SDK for Web (trang mẫu ghi Cubism 3.x, `.moc3`).
- Trước khi phát hành công khai: dùng nhân vật **nữ chibi riêng** (tự vẽ/thuê vẽ + rig Live2D, có quyền), làm thủ tục Publication License (miễn phí nếu đủ điều kiện) ≥ 1 tháng trước ngày phát hành, đọc lại hướng dẫn logo/thương hiệu.

## Phương án đã cân nhắc
- Cho phép nạp model tùy ý — tiện nhưng thành Expandable Application, cần duyệt riêng, không miễn phí.
- Avatar không dùng Live2D (sprite/Spine/VRM) — tránh license nhưng mất chất lượng chuyển động; người dùng đã chốt Live2D.
- Hiyori / Haru làm model phát triển — không phải phong cách chibi người dùng muốn.

## Hệ quả
- Cập nhật SPEC §9.1, §9.5.
- Thêm `**/live2dcubismcore*` (đã có trong `.gitignore`).
- Chưa đọc: trang Expandable Applications chi tiết, hướng dẫn thương hiệu — đọc trước khi phát hành.
- Phase 5 (2026-11-13): tải Cubism SDK for Web 5-r.5 + mẫu Koharu & Haruto (người dùng duyệt). `RedistributableFiles.txt` chỉ cho phân phối lại `live2dcubismcore.d.ts/.js/.min.js` → đóng gói kèm app được, **không commit**. Framework + code mẫu: Live2D Open Software License. Koharu là **model riêng** (`koharu/runtime/`, Cubism 3, tham số `PARAM_*`), không chung file với Haruto. Mọi file Live2D chép vào thư mục gitignore bằng `scripts/setup_avatar.mjs`. Dòng bản quyền mẫu hiển thị khi rê chuột lên nhân vật.
