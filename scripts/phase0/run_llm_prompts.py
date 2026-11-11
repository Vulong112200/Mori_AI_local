"""Chạy bộ prompt tests/fixtures/llm_prompts.md (AI-01..AI-06) qua llama-server, ghi câu trả lời để chấm điểm.

    python -I -X utf8 scripts/phase0/run_llm_prompts.py <llama_dir> <model.gguf> [--ngl 99]

Tự khởi động llama-server trên 127.0.0.1 (cổng ngẫu nhiên), tắt khi xong. Chỉ dùng stdlib.
AI-07 (dừng sinh) và AI-08 (context dài) không chạy ở đây.
"""
import argparse
import json
import random
import re
import subprocess
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "fixtures"
OUT = Path(__file__).resolve().parent / "out"
SKIP = {"AI-07", "AI-08"}


def parse_cases(md: str):
    """Mỗi mục '## AI-xx — tiêu đề' -> list lượt user (dòng '> ...' hoặc '1. > ...')."""
    cases = []
    for block in re.split(r"^## ", md, flags=re.M)[1:]:
        title = block.splitlines()[0].strip()
        turns = [m.group(1).strip() for m in re.finditer(r"^(?:\d+\.\s*)?>\s*(.+)$", block, flags=re.M)]
        if turns and title.split()[0] not in SKIP:
            cases.append((title, turns))
    return cases


def post(url, body):
    req = urllib.request.Request(url, json.dumps(body).encode("utf-8"), {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return json.load(r)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("llama_dir")
    ap.add_argument("model")
    ap.add_argument("--ngl", default="99")
    a = ap.parse_args()

    cases = parse_cases((FIXTURES / "llm_prompts.md").read_text(encoding="utf-8"))
    assert cases and cases[0][0].startswith("AI-01"), "không parse được llm_prompts.md"
    system = (FIXTURES / "system_prompt_sample.md").read_text(encoding="utf-8")

    port = random.randint(20000, 40000)
    base = f"http://127.0.0.1:{port}"
    srv = subprocess.Popen([str(Path(a.llama_dir) / "llama-server.exe"), "-m", a.model, "--host", "127.0.0.1",
                            "--port", str(port), "-c", "8192", "-ngl", a.ngl, "--jinja"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(360):
            try:
                urllib.request.urlopen(base + "/health", timeout=2)
                break
            except Exception:
                if srv.poll() is not None:
                    raise SystemExit(f"llama-server thoát sớm (exit {srv.returncode})")
                time.sleep(0.5)
        name = Path(a.model).stem
        lines = [f"# Câu trả lời LLM — {name}", "", f"System prompt: `tests/fixtures/system_prompt_sample.md` · temperature 0.7 · non-thinking", ""]
        for title, turns in cases:
            lines += [f"## {title}", ""]
            messages = [{"role": "system", "content": system}]
            for turn in turns:
                messages.append({"role": "user", "content": turn})
                t0 = time.perf_counter()
                r = post(base + "/v1/chat/completions", {
                    "messages": messages, "max_tokens": 500, "temperature": 0.7,
                    "chat_template_kwargs": {"enable_thinking": False},
                })
                reply = r["choices"][0]["message"]["content"].strip()
                messages.append({"role": "assistant", "content": reply})
                lines += [f"**Người dùng:** {turn}", "", f"**Mori** ({time.perf_counter() - t0:.1f}s):", "", reply, ""]
            print("xong", title)
        out = OUT / f"llm_answers_{name}.md"
        OUT.mkdir(exist_ok=True)
        out.write_text("\n".join(lines), encoding="utf-8")
        print("Đã lưu:", out)
    finally:
        srv.terminate()


if __name__ == "__main__":
    main()
