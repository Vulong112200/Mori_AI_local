# Đặc tả nhân vật Mori (Live2D) — cho họa sĩ / rigger

> Mục đích: đặt vẽ + rig nhân vật riêng của Mori thay model mẫu Koharu trước khi phát hành ([ADR-0006](adr/0006-license-live2d.md)).
> Bản xấp xỉ hiện tại (Phase 7) dùng Koharu + đạo cụ emoji vẽ chồng; tài liệu này mô tả thứ cần để làm thật.

## 1. Nhân vật

| Mục | Yêu cầu |
|---|---|
| Giới tính / tỉ lệ | Nữ, **chibi / SD** (đầu : thân ≈ 1 : 1.5–2), dễ thương, thân thiện |
| Tính cách để thể hiện | Ấm áp, tinh nghịch nhẹ, chu đáo; biểu cảm rõ ở mắt, lông mày, má |
| Giọng (tham khảo) | ZeroTTS `maichi` — nữ trẻ, hơi giọng miền Nam |
| Màu chủ đạo | Hồng phấn / tím nhạt (khớp UI: `pink-50…500`, `violet-50`) |
| Trang phục | 2 bộ: **đồ ở nhà** (mặc định, áo len rộng) và **đồ ngủ** (pijama). Tùy chọn: tạp dề (ăn), khăn tắm quấn tóc (tắm) |
| Tóc | Có vật lý (mái, đuôi tóc) — tham số tóc riêng cho physics |
| Cửa sổ hiển thị | 260×360 px CSS (×DPR 1.25–2), nền trong suốt, toàn thân hoặc gối trở lên đều được |

## 2. Kỹ thuật

- **Cubism Editor 4.2+ / 5**, xuất `moc3` + `model3.json` + `physics3.json` + `cdi3.json` + `exp3.json` + `motion3.json`.
- Texture: **1 tấm 2048×2048** (tối đa 2), PNG, premultiplied alpha xử lý ở app (không bake).
- Dùng **ID tham số chuẩn Cubism 4** (`ParamAngleX`, `ParamEyeLOpen`, `ParamMouthOpenY`, `ParamMouthForm`, `ParamBrowLY`…).
- Khai báo nhóm `EyeBlink` (2 mắt) và `LipSync` (`ParamMouthOpenY`) trong `model3.json`.
- Không hit area bắt buộc (app tự xử lý kéo/nhấp đúp).
- Hiệu năng: ≤ 120 ArtMesh, ≤ 2 mask lồng nhau; mục tiêu 30 FPS trên iGPU Intel Arc 140T khi model AI đang chạy.

## 3. Tham số cần rig

| Nhóm | Tham số | Ghi chú |
|---|---|---|
| Đầu / thân | `ParamAngleX/Y/Z`, `ParamBodyAngleX/Y/Z`, `ParamBreath` | Lắc đầu theo nhạc dùng `AngleZ` |
| Mắt | `ParamEyeLOpen/ROpen`, `ParamEyeLSmile/RSmile`, `ParamEyeBallX/Y`, **`ParamEyeSparkle`** (long lanh), **`ParamEyeSleepy`** (lim dim, khác nhắm) | Nhắm hẳn phải đẹp (ngủ) |
| Lông mày | `ParamBrowLY/RY`, `ParamBrowLForm/RForm`, `ParamBrowLAngle/RAngle` | |
| Miệng | `ParamMouthOpenY`, `ParamMouthForm` (−1 buồn … +1 cười), **`ParamMouthYawn`** (ngáp), **`ParamMouthChew`** (nhai) | |
| Má / nước mắt | `ParamCheek` (ửng hồng), `ParamTear` (rưng rưng), `ParamSweat` (giọt mồ hôi — lo lắng) | |
| Tay | `ParamArmLA/RA`, `ParamArmLB/RB` (2 khớp), **`ParamHandProp`** (chọn đồ cầm: 0 không, 1 đũa, 2 bút, 3 cốc, 4 điện thoại, 5 sách) | Đồ cầm là part bật/tắt theo giá trị |
| Đạo cụ | **`ParamPropBed`** (chăn đệm 0 ẩn → 1 đắp kín), **`ParamBlanketFold`** (0 trải → 1 gấp gọn), `ParamPropDesk` (bàn + laptop), `ParamPropHeadphone`, `ParamPropBowl` (bát cơm), `ParamPropBook` | Mỗi đạo cụ là part riêng, ẩn = opacity 0 |
| Trang phục | **`ParamOutfit`** (0 ở nhà, 1 ngủ, 2 tạp dề, 3 khăn tắm) | Đổi bằng opacity part, có motion che (xoay người / hơi nước) |
| Hiệu ứng | `ParamZzz` (bong bóng ngủ), `ParamSteam` (hơi nước tắm), `ParamMusicNote` (nốt nhạc bay), `ParamHeart` | Có thể làm bằng part hoặc để app vẽ HTML |

## 4. Expression (`exp3.json`)

`neutral`, `happy`, `love` (mắt cong + tim), `sad` (rưng rưng), `surprised` (mắt to, miệng tròn), `shy` (má đỏ, liếc), `worried` (mày chau, mồ hôi), `sleepy`, `thinking` (liếc lên, tay chống cằm nếu có), `listening` (mắt long lanh, nghiêng đầu nhẹ).
App chọn expression theo nhãn cảm xúc câu trả lời (`emotion.rs`) và trạng thái nghe/nghĩ/nói.

## 5. Motion (`motion3.json`)

Mỗi motion 2–8 s, có fade-in/out 0.3–0.5 s, **không** khóa tham số mắt/miệng (app tự chớp mắt + nhép miệng).

| Nhóm | Motion | Mô tả |
|---|---|---|
| Idle (≥ 4) | thở, nhìn quanh, đung đưa, ngân nga | Phát ngẫu nhiên, không lặp liên tiếp |
| Reaction | gật đầu, lắc đầu, vẫy tay, nhảy nhẹ vui, che mặt ngại, giật mình, nghiêng đầu thắc mắc | Theo cảm xúc |
| Ngủ / thức dậy | `sleep_loop` (thở chậm trong chăn), `wake_up` (hé mắt → dụi mắt → ngáp → vươn vai → ngồi dậy), `fold_blanket` (gấp chăn) | Sáng mở máy lần đầu trong ngày: wake_up → fold_blanket |
| Ăn | `eat_loop` (gắp, nhai), `drink` | 7h / 12h / 19h |
| Làm việc / học | `type_loop` (gõ laptop), `read_loop` (lật sách), `write` | Giờ hành chính |
| Nghe nhạc | `music_loop` (đeo tai nghe, lắc lư theo nhịp) | |
| Chơi một mình | `play_pompom`, `stretch`, `spin` | |
| Tắm / thay đồ | `bath` (hơi nước che, khăn quấn tóc), `change_clothes` (xoay người / màn hơi che → đổi `ParamOutfit`) | **Không hở hang** — luôn có che chắn, phù hợp mọi lứa tuổi |
| Buồn ngủ | `drowsy_loop` (gật gù) | 22–24h |

## 6. Bàn giao & license

- Bàn giao: file Cubism Editor gốc (`.cmo3`, `.can3`), bản xuất runtime (§2), PSD gốc theo layer, bảng tham số (tên, min/max/default, mô tả).
- License: Mori được **toàn quyền sử dụng, sửa, phân phối kèm phần mềm** (kể cả thương mại), không giới hạn số bản cài; ghi tên tác giả trong màn hình giới thiệu nếu họa sĩ muốn.
- Ứng dụng nhúng model cố định, không cho người dùng nạp model khác (ADR-0006, tránh "Expandable Application").
- Kiểm thu: hiển thị đúng trong app (`npm run tauri dev`), 30 FPS khi AI chạy (MODEL_EVALUATION §5.2), mọi expression/motion chụp `avatar_snapshot` duyệt.
