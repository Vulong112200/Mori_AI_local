// Chép Live2D (đã tải về data root) vào các thư mục bị gitignore để Vite đóng gói (ADR-0006):
// Cubism Core + shader → public/live2d/, Framework (TS) → src/live2d/framework/, model Koharu → public/avatar/koharu/.
// Chạy lại sau khi đổi bản SDK/model:  node scripts/setup_avatar.mjs
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const data = process.env.MORI_DATA_DIR ?? "E:\\Mori Data";
const avatar = join(data, "avatar");
const sdk = join(avatar, "sdk", readdirSync(join(avatar, "sdk")).find((d) => d.startsWith("CubismSdkForWeb-")) ?? "");
const model = join(avatar, "models", "koharu_haruto", "koharu", "runtime");
const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");

const copies = [
  [join(sdk, "Core", "live2dcubismcore.min.js"), join(root, "public", "live2d", "live2dcubismcore.min.js")],
  [join(sdk, "Core", "live2dcubismcore.d.ts"), join(root, "src", "live2d", "live2dcubismcore.d.ts")],
  [join(sdk, "Framework", "Shaders", "WebGL"), join(root, "public", "live2d", "shaders")],
  [join(sdk, "Framework", "src"), join(root, "src", "live2d", "framework")],
  [model, join(root, "public", "avatar", "koharu")],
];
for (const [from, to] of copies) {
  if (!existsSync(from)) throw new Error(`Thiếu ${from} — tải Cubism SDK for Web + mẫu Koharu vào ${avatar} trước`);
  rmSync(to, { recursive: true, force: true });
  mkdirSync(join(to, ".."), { recursive: true });
  cpSync(from, to, { recursive: true });
  console.log(`${from} → ${to}`);
}

// Framework của Live2D không viết theo `strict` (597 lỗi) → bỏ kiểm kiểu riêng mã bên thứ ba này;
// code của Mori vẫn kiểm strict đầy đủ.
const walk = (dir) =>
  readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(dir, e.name)) : e.name.endsWith(".ts") ? [join(dir, e.name)] : [],
  );
for (const f of walk(join(root, "src", "live2d", "framework"))) {
  writeFileSync(f, "// @ts-nocheck — mã Live2D, xem scripts/setup_avatar.mjs\n" + readFileSync(f, "utf8"));
}
