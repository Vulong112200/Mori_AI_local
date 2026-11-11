"""WER/CER cho tiếng Việt, chỉ dùng stdlib.

    python wer.py                      # self-check
    python wer.py "<ref>" "<hyp>"      # một cặp câu
    python wer.py --csv results.csv    # tổng hợp (cột: file,ref,hyp,latency_s)

CER_khong_dau: CER sau khi bỏ dấu. CER - CER_khong_dau ~ phần lỗi do sai dấu thanh.
"""
import csv
import statistics
import sys
import unicodedata


def normalize(text: str) -> str:
    text = unicodedata.normalize("NFC", text.lower())
    text = "".join(" " if unicodedata.category(c).startswith("P") else c for c in text)
    return " ".join(text.split())


def strip_tones(text: str) -> str:
    text = unicodedata.normalize("NFD", text).replace("đ", "d")
    return unicodedata.normalize("NFC", "".join(c for c in text if not unicodedata.combining(c)))


def edits(ref, hyp) -> int:
    prev = list(range(len(hyp) + 1))
    for i, r in enumerate(ref, 1):
        cur = [i]
        for j, h in enumerate(hyp, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (r != h)))
        prev = cur
    return prev[-1]


def score(pairs):
    """Corpus-level: tổng số lỗi / tổng độ dài ref."""
    w_err = w_len = c_err = c_len = t_err = 0
    for ref, hyp in pairs:
        ref, hyp = normalize(ref), normalize(hyp)
        w_err += edits(ref.split(), hyp.split())
        w_len += len(ref.split())
        c_err += edits(ref, hyp)
        c_len += len(ref)
        t_err += edits(strip_tones(ref), strip_tones(hyp))
    return {
        "WER": w_err / max(w_len, 1),
        "CER": c_err / max(c_len, 1),
        "CER_khong_dau": t_err / max(c_len, 1),
    }


def p95(values):
    values = sorted(values)
    return values[min(len(values) - 1, round(0.95 * (len(values) - 1)))]


def self_check():
    assert normalize("Xin chào, Mori!") == "xin chào mori"
    assert score([("xin chào", "xin chào")])["WER"] == 0
    assert score([("một hai ba bốn", "một hai bốn")])["WER"] == 0.25
    s = score([("tôi đi học", "toi di hoc")])
    assert s["WER"] == 1 and s["CER_khong_dau"] == 0  # chỉ sai dấu
    assert p95([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]) == 10
    print("wer.py self-check OK")


def main(argv):
    if not argv:
        return self_check()
    if argv[0] == "--csv":
        with open(argv[1], encoding="utf-8-sig", newline="") as f:
            rows = list(csv.DictReader(f))
        s = score((r["ref"], r["hyp"]) for r in rows)
        lat = [float(r["latency_s"]) for r in rows if r.get("latency_s")]
        print(f"so_cau={len(rows)} WER={s['WER']:.3f} CER={s['CER']:.3f} CER_khong_dau={s['CER_khong_dau']:.3f}")
        if lat:
            print(f"latency_tb={statistics.mean(lat):.2f}s latency_p95={p95(lat):.2f}s")
        return
    s = score([(argv[0], argv[1])])
    print(" ".join(f"{k}={v:.3f}" for k, v in s.items()))


if __name__ == "__main__":
    main(sys.argv[1:])
