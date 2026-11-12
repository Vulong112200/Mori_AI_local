"""TTS worker cho Mori (SPEC §13.1): JSON Lines qua stdin/stdout, ZeroTTS int8 (ADR-0005).

Chạy: python -I -X utf8 -c <file này> <model_dir> <threads>
stdout CHỈ chứa protocol; không in nội dung văn bản ra stderr (có thể là hội thoại).
"""
import base64
import json
import queue
import sys
import threading

import numpy as np
import onnxruntime as ort

ort.disable_telemetry_events()  # tắt ETW telemetry của ONNX Runtime (SPEC §14.1), trước khi tạo session
from zerotts import ZeroTTS  # noqa: E402

MAX_TEXT = 2000
model_dir, threads = sys.argv[1], int(sys.argv[2])
out_lock = threading.Lock()


def send(**m):
    with out_lock:
        sys.stdout.write(json.dumps({"v": 1, **m}, ensure_ascii=False) + "\n")
        sys.stdout.flush()


tts = ZeroTTS.from_pretrained(model_dir, local_files_only=True, intra_op_num_threads=threads)
voices = tts.list_voices()
for _ in tts.synthesize_stream("Xin chào.", voice="maichi"):  # nạp + warm-up ONNX
    pass
send(type="ready", sample_rate=tts.sample_rate, voices=voices)

jobs: queue.Queue = queue.Queue()
cancelled: set = set()


def reader():
    for line in sys.stdin:
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if not isinstance(m, dict):
            continue
        t = m.get("type")
        if t == "cancel":
            cancelled.add(m.get("id"))
        elif t == "health":
            send(type="health", id=m.get("id"))
        elif t == "synthesize":
            jobs.put(m)
        elif t == "shutdown":
            break
    jobs.put(None)  # stdin đóng (Mori thoát) hoặc shutdown → dừng êm


threading.Thread(target=reader, daemon=True).start()

while (m := jobs.get()) is not None:
    rid = m.get("id")
    try:
        p = m.get("payload") if isinstance(m.get("payload"), dict) else {}
        text, voice = str(p.get("text", ""))[:MAX_TEXT], p.get("voice") or "maichi"
        if rid in cancelled:
            pass
        elif voice not in voices:
            raise ValueError("voice")
        else:
            for seq, chunk in enumerate(tts.synthesize_stream(text, voice=voice)):
                if rid in cancelled:
                    break
                pcm = (np.clip(np.asarray(chunk, dtype=np.float32).reshape(-1), -1, 1) * 32767).astype("<i2")
                send(type="audio", id=rid, seq=seq, sample_rate=tts.sample_rate,
                     pcm_b64=base64.b64encode(pcm.tobytes()).decode("ascii"))
        send(type="done", id=rid)
    except Exception as e:  # chỉ gửi tên lỗi, không gửi nội dung
        send(type="error", id=rid, message=type(e).__name__)
    cancelled.discard(rid)
