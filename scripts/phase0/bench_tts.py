"""Benchmark TTS: first-audio, RTF, RAM đỉnh; lưu wav để nghe chấm điểm.

    <venv>/python scripts/phase0/bench_tts.py <korva|zerotts|vieneu|dummy> [--voice V] [--model PATH] [--mode v3turbo|v3nano]

- Chạy với HF_HUB_OFFLINE=1 sau khi đã tải weights (không tự tải ngầm).
- `dummy` = sóng sin, không cần model: dùng để kiểm tra script.
- API theo README chính thức (2026-11): korvatts 0.1.3, zerotts 0.1.5, vieneu 3.8.x.
"""
import argparse
import ctypes
import csv
import math
import os
import statistics
import sys
import time
import wave
from array import array
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SENTENCES = ROOT / "tests" / "fixtures" / "tts_sentences.txt"
OUT = Path(__file__).resolve().parent / "out" / "tts"


# ---- Engines: mỗi hàm trả về (sample_rate, synth(text) -> iterator các chunk audio float) ----

def load_korva(a):
    from korvatts import TTS  # không có streaming: một chunk = cả câu
    tts = TTS(assets_dir=a.model) if a.model else TTS()
    voice = a.voice or "khanh_vy"
    sr = getattr(tts, "sample_rate", 44100)
    return sr, lambda text: iter([tts.synthesize(text, voice=voice, lang="vi")[0]])


def load_zerotts(a):
    from zerotts import ZeroTTS
    tts = ZeroTTS.from_pretrained(a.model or "zeroweight-ai/ZeroTTS")
    voice = a.voice or "maichi"
    return tts.sample_rate, lambda text: tts.synthesize_stream(text, voice=voice)


def load_vieneu(a):
    from vieneu import Vieneu
    mode = a.mode or "v3turbo"
    tts = Vieneu(mode=mode)
    voice = a.voice or "Hải Đăng"
    sr = 24000 if mode == "v3nano" else 48000  # theo README; API không có thuộc tính sample rate
    return sr, lambda text: tts.infer_stream(text, voice=voice)


def load_dummy(a):
    sr = 24000

    def synth(text):
        n = int(sr * 0.06 * len(text.split()))  # ~60 ms/từ
        for start in range(0, n, sr // 10):
            time.sleep(0.005)
            yield [0.3 * math.sin(2 * math.pi * 220 * i / sr) for i in range(start, min(n, start + sr // 10))]

    return sr, synth


ENGINES = {"korva": load_korva, "zerotts": load_zerotts, "vieneu": load_vieneu, "dummy": load_dummy}


# ---- Tiện ích ----

def flat(chunk):
    return chunk.reshape(-1).tolist() if hasattr(chunk, "reshape") else list(chunk)


def write_wav(path, samples, sr):
    pcm = array("h", (int(max(-1.0, min(1.0, x)) * 32767) for x in samples))
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())


def peak_rss_mb():
    if os.name != "nt":
        return float("nan")

    class PMC(ctypes.Structure):
        _fields_ = [("cb", ctypes.c_ulong), ("PageFaultCount", ctypes.c_ulong)] + [
            (n, ctypes.c_size_t) for n in ("PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                                           "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
                                           "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage")]

    k32 = ctypes.windll.kernel32
    k32.GetCurrentProcess.restype = ctypes.c_void_p  # pseudo-handle 64-bit, mặc định ctypes cắt còn 32-bit
    k32.K32GetProcessMemoryInfo.argtypes = [ctypes.c_void_p, ctypes.POINTER(PMC), ctypes.c_ulong]
    pmc = PMC(cb=ctypes.sizeof(PMC))
    if not k32.K32GetProcessMemoryInfo(k32.GetCurrentProcess(), ctypes.byref(pmc), pmc.cb):
        return float("nan")
    return pmc.PeakWorkingSetSize / 2**20


def p95(values):
    values = sorted(values)
    return values[min(len(values) - 1, round(0.95 * (len(values) - 1)))]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("engine", choices=ENGINES)
    ap.add_argument("--voice")
    ap.add_argument("--model", help="đường dẫn weights local")
    ap.add_argument("--mode", help="vieneu: v3turbo | v3nano")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # console Windows mặc định cp1252

    if a.engine != "dummy" and os.environ.get("HF_HUB_OFFLINE") != "1":
        print("CANH BAO: HF_HUB_OFFLINE != 1 -> engine co the tu tai weights.", file=sys.stderr)

    texts = [l.strip() for l in SENTENCES.read_text(encoding="utf-8").splitlines() if l.strip() and not l.startswith("#")]
    out = OUT / (a.engine + (f"-{a.mode}" if a.mode else ""))
    out.mkdir(parents=True, exist_ok=True)

    t0 = time.perf_counter()
    sr, synth = ENGINES[a.engine](a)
    load_s = time.perf_counter() - t0
    list(synth("Xin chào."))  # warm-up, không tính

    rows = []
    for i, text in enumerate(texts, 1):
        start = time.perf_counter()
        first = None
        samples = []
        for chunk in synth(text):
            if first is None:
                first = time.perf_counter() - start
            samples += flat(chunk)
        total = time.perf_counter() - start
        dur = len(samples) / sr
        write_wav(out / f"{i:02d}.wav", samples, sr)
        rows.append({"id": i, "first_audio_s": round(first or total, 3), "total_s": round(total, 3),
                     "audio_s": round(dur, 3), "rtf": round(total / dur, 3) if dur else None, "text": text})
        print(f"{i:02d} first={rows[-1]['first_audio_s']:.2f}s rtf={rows[-1]['rtf']} | {text[:60]}")

    with open(out / "results.csv", "w", encoding="utf-8-sig", newline="") as f:
        w = csv.DictWriter(f, fieldnames=rows[0].keys())
        w.writeheader()
        w.writerows(rows)

    fa = [r["first_audio_s"] for r in rows]
    rtf = [r["rtf"] for r in rows if r["rtf"]]
    print(f"\n== {a.engine} | sr={sr} | load={load_s:.1f}s | {len(rows)} cau")
    print(f"first_audio TB={statistics.mean(fa):.2f}s P95={p95(fa):.2f}s | RTF TB={statistics.mean(rtf):.2f} | RAM dinh={peak_rss_mb():.0f} MB")
    print(f"Nghe va cham diem: {out}")


if __name__ == "__main__":
    main()
