// Cửa sổ avatar (label "avatar"): trong suốt, luôn trên, kéo để di chuyển, nhấp đúp mở khung chat, giữ F2 để nói.
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
// Chỉ import kiểu: mã Framework dùng biến toàn cục Live2DCubismCore ngay khi nạp → import động sau loadCore().
import type { Avatar, AvatarState } from "./live2d/avatar";

/** FPS khi đang nghe/nghĩ/nói (gate Phase 5: 30) và khi rảnh (đỡ tốn GPU cho LLM). */
const FPS_ACTIVE = 30;
const FPS_IDLE = 15;
/** Đo gate Phase 5 (chỉ bản dev): `VITE_AVATAR_BENCH=1 npm run tauri dev` → vẽ liên tục 30 FPS như lúc hoạt động. */
const BENCH = import.meta.env.DEV && import.meta.env.VITE_AVATAR_BENCH === "1";

async function loadCore() {
  if (!("Live2DCubismCore" in window)) {
    await new Promise<void>((ok, fail) => {
      const s = document.createElement("script");
      s.src = "/live2d/live2dcubismcore.min.js";
      s.onload = () => ok();
      s.onerror = () => fail(new Error("không tải được Cubism Core (chạy node scripts/setup_avatar.mjs)"));
      document.head.append(s);
    });
  }
  // Core biên dịch WASM bất đồng bộ → đợi tới khi gọi được.
  for (let i = 0; i < 100; i++) {
    try {
      Live2DCubismCore.Version.csmGetVersion();
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 50));
    }
  }
  throw new Error("Cubism Core không khởi động được");
}

export default function AvatarView() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    let alive = true; // StrictMode (dev) chạy effect 2 lần → lần nạp cũ không được khởi động vòng vẽ
    let avatar: Avatar | null = null;
    let raf = 0;
    let state: AvatarState = "idle";
    let speaking = false;
    let level = 0;
    let snapshot = import.meta.env.DEV; // bản dev: chụp 1 khung hình để kiểm tra hiển thị
    const unlisten: (() => void)[] = [];
    api.onAvatarState((s) => (state = s)).then((u) => unlisten.push(u));
    api.onTtsPlaying((p) => (speaking = p)).then((u) => unlisten.push(u));
    api.onTtsLevel((l) => (level = l)).then((u) => unlisten.push(u));

    const c = canvas.current!;
    const gl = c.getContext("webgl2", { alpha: true, premultipliedAlpha: true });
    const fail = (e: unknown) => {
      const msg = e instanceof Error ? `${e.message} @ ${e.stack?.split("\n").slice(1, 4).join(" | ")}` : String(e);
      setError(msg);
      api.avatarReport({ error: msg });
    };
    if (!gl) {
      fail("WebGL2 không dùng được");
      return;
    }

    let last = performance.now();
    let frames = 0;
    let fpsSince = last;
    const frame = (now: number) => {
      raf = requestAnimationFrame(frame);
      const active = BENCH || speaking || state !== "idle";
      const dt = (now - last) / 1000;
      if (!avatar || dt < 1 / (active ? FPS_ACTIVE : FPS_IDLE) - 0.002) return;
      last = now;
      const dpr = window.devicePixelRatio || 1;
      const w = Math.round(c.clientWidth * dpr);
      const h = Math.round(c.clientHeight * dpr);
      if (c.width !== w || c.height !== h) {
        c.width = w;
        c.height = h;
      }
      gl.viewport(0, 0, w, h);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      avatar.state = speaking ? "speaking" : state;
      avatar.level = level;
      try {
        avatar.update(Math.min(dt, 0.1));
        avatar.draw(gl, w, h);
      } catch (e) {
        avatar = null; // dừng vẽ, báo lỗi một lần
        fail(e);
        return;
      }
      if (snapshot && now > 3000) {
        snapshot = false;
        api.avatarSnapshot(c.toDataURL("image/png")).catch(() => {});
      }
      frames++;
      if (now - fpsSince >= 5000) {
        api.avatarReport({ fps: (frames * 1000) / (now - fpsSince), active });
        frames = 0;
        fpsSince = now;
      }
    };

    (async () => {
      await loadCore();
      const { Avatar } = await import("./live2d/avatar");
      const a = await Avatar.load(gl, "/avatar/koharu/", c.clientWidth, c.clientHeight);
      if (!alive) return;
      avatar = a;
      raf = requestAnimationFrame(frame);
    })().catch(fail);

    // Giữ F2 khi cửa sổ avatar đang focus = nói (chuyển cho cửa sổ chat xử lý, kể cả khi nó đang ẩn).
    const key = (down: boolean) => (e: KeyboardEvent) => {
      if (e.key === "F2" && !e.repeat) {
        e.preventDefault();
        api.avatarPtt(down);
      }
    };
    const down = key(true);
    const up = key(false);
    const blur = () => api.avatarPtt(false); // giữ F2 rồi chuyển cửa sổ → không có keyup
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      alive = false;
      cancelAnimationFrame(raf);
      unlisten.forEach((u) => u());
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

  return (
    <div className="group relative h-screen w-screen select-none overflow-hidden">
      <canvas
        ref={canvas}
        className="h-full w-full cursor-grab"
        onPointerDown={(e) => e.button === 0 && api.avatarDrag()}
        onDoubleClick={() => api.setAvatarMode("chat")}
        title="Kéo để di chuyển · nhấp đúp mở khung chat · giữ F2 để nói"
      />
      <button
        onClick={() => api.setAvatarMode("chat")}
        className="absolute right-1 top-1 rounded-full bg-white/80 px-2 py-0.5 text-sm opacity-0 shadow transition group-hover:opacity-100"
        aria-label="Mở khung chat"
      >
        💬
      </button>
      <p className="pointer-events-none absolute bottom-0 w-full text-center text-[9px] text-slate-500 opacity-0 transition group-hover:opacity-70">
        This content uses sample data owned and copyrighted by Live2D Inc.
      </p>
      {error && <p className="absolute inset-x-1 bottom-4 rounded bg-rose-50/90 p-1 text-[10px] text-rose-700">{error}</p>}
    </div>
  );
}
