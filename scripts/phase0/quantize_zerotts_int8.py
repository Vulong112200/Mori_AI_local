"""Tạo bản int8 của ZeroTTS (lượng tử hóa động, chỉ MatMul) — Phase 0, thử tăng tốc.

    <tts-venv>/python -I -X utf8 scripts/phase0/quantize_zerotts_int8.py <thư mục fp32> <thư mục int8 đích>

Kỹ thuật theo mô tả của tác giả ZeroTTS (tools/quantize_onnx_int8.py trong repo gốc):
graph fp32 bị nghẽn băng thông RAM; weights đi qua node Identity nên quantize_dynamic
bỏ sót phần lớn MatMul → cho ORT gộp graph trước (ORT_ENABLE_BASIC), rồi mới lượng tử hóa.
Chỉ 3 graph nóng được thay; codec giữ fp32. Không ghi vào thư mục nguồn.
Cần gói `onnx` trong venv.
"""
import collections
import shutil
import sys
from pathlib import Path

import onnx
import onnxruntime as ort
from onnxruntime.quantization import QuantType, quantize_dynamic

GRAPHS = ("text_encoder.onnx", "prefix_step.onnx", "local_frame_decode.onnx")


def quantize_one(src: Path, dst: Path) -> collections.Counter:
    folded = dst.with_name(dst.name + ".opt")
    so = ort.SessionOptions()
    so.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_BASIC
    so.optimized_model_filepath = str(folded)
    ort.InferenceSession(str(src), so, providers=["CPUExecutionProvider"])  # chỉ để ghi graph đã gộp
    try:
        quantize_dynamic(str(folded), str(dst), weight_type=QuantType.QInt8, op_types_to_quantize=["MatMul"])
    finally:
        folded.unlink(missing_ok=True)
    return collections.Counter(n.op_type for n in onnx.load(str(dst), load_external_data=False).graph.node)


def main(src: Path, dst: Path):
    if not (src / "config.json").is_file():
        sys.exit(f"{src} không phải thư mục model ZeroTTS (thiếu config.json)")
    if dst.resolve() == src.resolve():
        sys.exit("thư mục đích phải khác thư mục nguồn")
    shutil.copytree(src, dst, dirs_exist_ok=True, symlinks=False)
    for name in GRAPHS:
        c = quantize_one(src / "onnx" / name, dst / "onnx" / name)
        mb_in, mb_out = ((p / "onnx" / name).stat().st_size / 2**20 for p in (src, dst))
        print(f"{name:26s} {mb_in:6.0f} MB -> {mb_out:6.0f} MB | MatMul={c['MatMul']} MatMulInteger={c['MatMulInteger']}")
        assert c["MatMulInteger"] > 0, f"{name}: không lượng tử hóa được MatMul nào"
    print("Đã ghi:", dst)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(Path(sys.argv[1]), Path(sys.argv[2]))
