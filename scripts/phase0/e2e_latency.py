"""Đo độ trễ end-to-end thật: wav → whisper-cli (Vulkan) → LLM (llama-server, stream) → ZeroTTS (chunk đầu).

    <tts-venv>/python -I -X utf8 scripts/phase0/e2e_latency.py <llama_url> <model_id> <wav16k> [<wav16k> ...]

- llama_url: llama-server đang chạy (vd http://127.0.0.1:8765, router mode → cần model_id).
- Đo: STT (không tính thời gian nạp model, như whisper-server giữ sẵn) + TTFT + tới hết câu đầu + TTS chunk đầu.
- Cần env: HF_HOME, HF_HUB_DISABLE_SYMLINKS=1, HF_HUB_OFFLINE=1 (ZeroTTS int8 ở E:/Mori Data/models/tts/zerotts-int8).
"""
import json
import re
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WHISPER = r"E:\Mori Data\runtime\whisper.cpp\v1.9.5-vulkan\whisper-cli.exe"
STT_MODEL = r"E:\Mori Data\models\stt\ggml-large-v3-turbo-q5_0.bin"
TTS_MODEL = r"E:\Mori Data\models\tts\zerotts-int8"
SENTENCE_END = re.compile(r"[.!?…\n]")


def stt(wav):
    p = subprocess.run([WHISPER, "-m", STT_MODEL, "-f", wav, "-l", "vi", "-nt", "-t", "8"],
                       capture_output=True, text=True, encoding="utf-8", errors="replace", check=True)
    ms = {k: float(m.group(1)) for k in ("load", "total")
          if (m := re.search(rf"{k} time\s*=\s*([\d.]+)\s*ms", p.stderr))}
    return " ".join(p.stdout.split()), (ms["total"] - ms["load"]) / 1000


def first_sentence(url, model, system, text):
    """Stream chat; trả (TTFT, thời điểm hết câu đầu, câu đầu)."""
    body = {"model": model, "stream": True, "max_tokens": 120,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": text}]}
    req = urllib.request.Request(url + "/v1/chat/completions", json.dumps(body).encode(), {"Content-Type": "application/json"})
    t0 = time.perf_counter()
    ttft, buf = None, ""
    with urllib.request.urlopen(req, timeout=120) as r:
        for line in r:
            line = line.decode("utf-8").strip()
            if not line.startswith("data: ") or line == "data: [DONE]":
                continue
            delta = json.loads(line[6:])["choices"][0]["delta"].get("content") or ""
            if delta and ttft is None:
                ttft = time.perf_counter() - t0
            buf += delta
            if (m := SENTENCE_END.search(buf)) and len(buf[:m.end()].strip()) > 3:
                return ttft, time.perf_counter() - t0, buf[:m.end()].strip()
    return ttft, time.perf_counter() - t0, buf.strip()


def main(url, model, wavs):
    from zerotts import ZeroTTS
    tts = ZeroTTS.from_pretrained(TTS_MODEL, intra_op_num_threads=4)
    system = (ROOT / "tests/fixtures/system_prompt_sample.md").read_text(encoding="utf-8")
    first_sentence(url, model, system, "Xin chào.")  # warm-up: nạp model, shader, prompt cache
    list(tts.synthesize_stream("Xin chào.", voice="maichi"))
    totals = []
    for wav in wavs:
        text, t_stt = stt(wav)
        ttft, t_sent, sent = first_sentence(url, model, system, text)
        t0 = time.perf_counter()
        next(iter(tts.synthesize_stream(sent, voice="maichi")))
        t_tts = time.perf_counter() - t0
        total = t_stt + t_sent + t_tts
        totals.append(total)
        print(f"{Path(wav).name}: STT {t_stt:.2f}s | TTFT {ttft:.2f}s | câu đầu {t_sent:.2f}s | TTS {t_tts:.2f}s | TỔNG {total:.2f}s")
        print(f"    nghe: {text}\n    đáp : {sent}")
    totals.sort()
    print(f"\n== {model}: {len(totals)} lượt | TB {sum(totals) / len(totals):.2f}s | max {totals[-1]:.2f}s")


if __name__ == "__main__":
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    main(sys.argv[1].rstrip("/"), sys.argv[2], sys.argv[3:])
