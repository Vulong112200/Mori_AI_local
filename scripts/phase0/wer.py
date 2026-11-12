"""WER/CER cho tiếng Việt, chỉ dùng stdlib.

    python wer.py                      # self-check
    python wer.py "<ref>" "<hyp>"      # một cặp câu
    python wer.py --csv results.csv    # tổng hợp (cột: file,ref,hyp,latency_s)

CER_khong_dau: CER sau khi bỏ dấu. CER - CER_khong_dau ~ phần lỗi do sai dấu thanh.
"""
import csv
import re
import statistics
import sys
import unicodedata


def normalize(text: str) -> str:
    text = unicodedata.normalize("NFC", text.lower())
    text = "".join(" " if unicodedata.category(c).startswith("P") else c for c in text)
    return " ".join(text.split())


DIGITS = "không một hai ba bốn năm sáu bảy tám chín".split()


def _under_1000(n: int, full: bool) -> list[str]:
    """Đọc 0..999. `full`: nhóm không đứng đầu → đọc đủ "không trăm", "linh"."""
    h, t, u = n // 100, n // 10 % 10, n % 10
    w = []
    if h or full:
        w += [DIGITS[h], "trăm"]
    if t == 0:
        if u and (h or full):
            w.append("linh")
    elif t == 1:
        w.append("mười")
    else:
        w += [DIGITS[t], "mươi"]
    if u:
        w.append("mốt" if u == 1 and t > 1 else "lăm" if u == 5 and t > 0 else DIGITS[u])
    return w


def spell(n: int) -> str:
    """Số nguyên → chữ tiếng Việt (cách đọc phổ thông), để so model viết số bằng chữ với câu gốc viết bằng số."""
    if n == 0:
        return "không"
    groups, words = [], []
    while n:
        groups.append(n % 1000)
        n //= 1000
    for i in range(len(groups) - 1, -1, -1):
        g = groups[i]
        if g:
            words += _under_1000(g, full=i < len(groups) - 1)
            words += [["", "nghìn", "triệu", "tỷ"][i]] if i else []
    return " ".join(w for w in words if w)


def spell_numbers(text: str) -> str:
    text = re.sub(r"(?<=\d)[.,](?=\d{3})", "", text)  # 850.000 → 850000
    text = re.sub(r"(\d+)h(\d*)", lambda m: f"{m[1]} giờ {m[2]}", text)  # 7h45 → 7 giờ 45
    return re.sub(r"\d+", lambda m: f" {spell(int(m[0]))} ", text)


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


def score(pairs, numbers=False):
    """Corpus-level: tổng số lỗi / tổng độ dài ref. `numbers`: đổi số → chữ ở cả hai phía trước khi so."""
    w_err = w_len = c_err = c_len = t_err = 0
    for ref, hyp in pairs:
        if numbers:
            ref, hyp = spell_numbers(ref), spell_numbers(hyp)
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
    for n, w in [(2024, "hai nghìn không trăm hai mươi bốn"), (15, "mười lăm"), (21, "hai mươi mốt"),
                 (105, "một trăm linh năm"), (3500000, "ba triệu năm trăm nghìn"), (45, "bốn mươi lăm"),
                 (1024, "một nghìn không trăm hai mươi bốn"), (850000, "tám trăm năm mươi nghìn"), (10, "mười")]:
        assert spell(n) == w, (n, spell(n))
    assert score([("lúc 7 giờ 45", "lúc 7h45")], numbers=True)["WER"] == 0
    assert score([("năm 2024", "năm hai nghìn không trăm hai mươi bốn")], numbers=True)["WER"] == 0
    print("wer.py self-check OK")


def main(argv):
    if not argv:
        return self_check()
    if argv[0] == "--csv":
        with open(argv[1], encoding="utf-8-sig", newline="") as f:
            rows = list(csv.DictReader(f))
        s = score((r["ref"], r["hyp"]) for r in rows)
        n = score(((r["ref"], r["hyp"]) for r in rows), numbers=True)
        lat = [float(r["latency_s"]) for r in rows if r.get("latency_s")]
        print(f"so_cau={len(rows)} WER={s['WER']:.3f} CER={s['CER']:.3f} CER_khong_dau={s['CER_khong_dau']:.3f}")
        print(f"  so->chu: WER={n['WER']:.3f} CER={n['CER']:.3f}")
        if lat:
            print(f"latency_tb={statistics.mean(lat):.2f}s latency_p95={p95(lat):.2f}s")
        for cat in sorted({r.get("category") for r in rows} - {None, ""}):
            c = score((r["ref"], r["hyp"]) for r in rows if r["category"] == cat)
            cn = score(((r["ref"], r["hyp"]) for r in rows if r["category"] == cat), numbers=True)
            print(f"  {cat}: WER={c['WER']:.3f} CER={c['CER']:.3f} | so->chu WER={cn['WER']:.3f}")
        return
    s = score([(argv[0], argv[1])])
    print(" ".join(f"{k}={v:.3f}" for k, v in s.items()))


if __name__ == "__main__":
    main(sys.argv[1:])
