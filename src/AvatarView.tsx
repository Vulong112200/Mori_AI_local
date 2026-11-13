// Cửa sổ avatar (label "avatar"): trong suốt, luôn trên, kéo để di chuyển, nhấp đúp mở khung chat, giữ F2 để nói.
// Phase 7 (ADR-0013): hoạt cảnh theo giờ máy + đạo cụ emoji + bong bóng chữ (câu soạn sẵn, không gọi model, không tiếng).
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
// Chỉ import kiểu: mã Framework dùng biến toàn cục Live2DCubismCore ngay khi nạp → import động sau loadCore().
import type { Activity, Avatar, AvatarState, Emotion } from "./live2d/avatar";

/** FPS khi đang nghe/nghĩ/nói (gate Phase 5: 30), khi rảnh, khi ngủ (đỡ tốn GPU cho LLM). */
const FPS_ACTIVE = 30;
const FPS_IDLE = 15;
const FPS_SLEEP = 10;
/** Đo gate Phase 5 (chỉ bản dev): `VITE_AVATAR_BENCH=1 npm run tauri dev` → vẽ liên tục 30 FPS như lúc hoạt động. */
const BENCH = import.meta.env.DEV && import.meta.env.VITE_AVATAR_BENCH === "1";
/** Bản dev: `VITE_AVATAR_DEMO=1` → lần lượt từng biểu cảm/hoạt cảnh, chụp `avatar_snapshot_<tên>.png` để kiểm tra. */
const DEMO = import.meta.env.DEV && import.meta.env.VITE_AVATAR_DEMO === "1";

/** Bong bóng bâng quơ theo hoạt cảnh. */
const LINES: Record<Activity, string[]> = {
  free: ["Mori ở đây nè~", "Hôm nay của bạn thế nào?", "Nhớ uống nước nha!", "Có chuyện gì vui kể Mori nghe với."],
  sleep: ["Zzz…", "Ưm… năm phút nữa thôi…"],
  wake: ["Oáp~ chào buổi sáng!", "Để Mori gấp chăn cái đã…"],
  eat: ["Măm măm~", "Bạn ăn gì chưa đó?", "Ăn xong rồi làm tiếp nha."],
  work: ["Mori làm việc cùng bạn nè.", "Tập trung nào!", "Mệt thì nghỉ chút nhé."],
  study: ["Đọc sách chút…", "Hôm nay học được gì hay không?"],
  music: ["La la la~", "Bài này hay ghê!"],
  play: ["Cổ vũ bạn nè!", "Chơi một mình cũng vui hihi."],
  drowsy: ["Khuya rồi, bạn ngủ sớm nha…", "Mori buồn ngủ quá…"],
};
/** Đạo cụ vẽ chồng (bản xấp xỉ — nhân vật riêng sẽ có part thật, xem docs/CHARACTER_SPEC.md). */
const PROPS: Partial<Record<Activity, string>> = {
  sleep: "🛏️",
  wake: "🛏️",
  eat: "🍙",
  work: "💻",
  study: "📚",
  music: "🎧",
  drowsy: "🌙",
};
const FREE: Activity[] = ["free", "free", "work", "study", "music", "play"];
const pick = <T,>(a: T[]): T => a[Math.floor(Math.random() * a.length)];

/** Lịch theo giờ máy; `null` = giờ tự do (chọn ngẫu nhiên). */
function scheduled(now: Date, wokeToday: boolean): Activity | null {
  const h = now.getHours() + now.getMinutes() / 60;
  if (h < 6) return "sleep";
  if (h < 10 && !wokeToday) return "wake"; // lần mở đầu tiên buổi sáng: thức dậy trong chăn
  if ((h >= 7 && h < 7.5) || (h >= 12 && h < 12.75) || (h >= 19 && h < 19.75)) return "eat";
  if (h >= 22) return "drowsy";
  return null;
}

// Ngày đã "thức dậy" — chỉ là tiện ích hiển thị; không lưu được thì mỗi lần mở buổi sáng lại thức dậy, không sao.
const today = (d: Date) => d.toDateString();
function wokeOn(): string | null {
  try {
    return localStorage.getItem("mori.wokeOn");
  } catch {
    return null;
  }
}
function setWokeOn(d: string) {
  try {
    localStorage.setItem("mori.wokeOn", d);
  } catch {
    /* bỏ qua */
  }
}

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
  const [activity, setActivity] = useState<Activity>("free");
  const [bubble, setBubble] = useState<string | null>(null);
  const [idle, setIdle] = useState(true);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    let alive = true; // StrictMode (dev) chạy effect 2 lần → lần nạp cũ không được khởi động vòng vẽ
    let avatar: Avatar | null = null;
    let raf = 0;
    let state: AvatarState = "idle";
    let speaking = false;
    let level = 0;
    let act: Activity = "free";
    let proactive = true;
    let snapshot = import.meta.env.DEV && !DEMO; // bản dev: chụp 1 khung hình để kiểm tra hiển thị
    const timers: number[] = [];
    const unlisten: (() => void)[] = [];
    const sync = () => setIdle(!speaking && state === "idle");
    let bubbleTimer = 0;
    const say = (text: string, secs: number) => {
      setBubble(text);
      clearTimeout(bubbleTimer);
      bubbleTimer = window.setTimeout(() => setBubble(null), secs * 1000);
    };
    api.onAvatarState((s) => ((state = s), sync())).then((u) => unlisten.push(u));
    api.onTtsPlaying((p) => ((speaking = p), sync())).then((u) => unlisten.push(u));
    api.onTtsLevel((l) => (level = l)).then((u) => unlisten.push(u));
    api.onAvatarEmotion((e) => avatar?.setEmotion(e)).then((u) => unlisten.push(u));
    api.onAvatarSay((t) => say(t, Math.min(12, Math.max(6, t.length / 12)))).then((u) => unlisten.push(u));
    api.onProactiveChanged((p) => (proactive = p)).then((u) => unlisten.push(u));
    api.getProactive().then((p) => (proactive = p), () => {});

    // Hoạt cảnh theo giờ máy: xét lại mỗi 20 s; giờ tự do đổi ngẫu nhiên mỗi 15–25 phút.
    let freePick: Activity = "free";
    let freeUntil = 0;
    let wakeUntil = 0;
    const schedule = () => {
      if (DEMO) return;
      const now = new Date();
      const ms = now.getTime();
      let next: Activity;
      if (ms < wakeUntil) next = "wake";
      else {
        const s = scheduled(now, wokeOn() === today(now));
        if (s === "wake") {
          setWokeOn(today(now));
          wakeUntil = ms + 12_000;
          timers.push(window.setTimeout(schedule, 12_500));
        }
        if (s) next = s;
        else {
          if (ms >= freeUntil) {
            freePick = pick(FREE);
            freeUntil = ms + (15 + Math.random() * 10) * 60_000;
          }
          next = freePick;
        }
      }
      if (next !== act) {
        act = next;
        setActivity(next);
      }
    };
    schedule();
    timers.push(window.setInterval(schedule, 20_000));
    // Bong bóng bâng quơ mỗi 3–8 phút, chỉ khi rảnh và bật "Mori chủ động".
    const chatter = () => {
      if (proactive && !speaking && state === "idle") say(pick(LINES[act]), 7);
      timers.push(window.setTimeout(chatter, (3 + Math.random() * 5) * 60_000));
    };
    timers.push(window.setTimeout(chatter, (1 + Math.random() * 2) * 60_000));

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

    // Bản dev: lần lượt từng cảnh, giữ 4 s rồi chụp.
    const demo: [string, () => void][] = DEMO
      ? [
          ...(["neutral", "happy", "love", "sad", "surprised", "shy", "worried"] as Emotion[]).map(
            (e): [string, () => void] => [`emo_${e}`, () => avatar?.setEmotion(e)],
          ),
          ...(["sleep", "wake", "eat", "work", "study", "music", "play", "drowsy"] as Activity[]).map(
            (a): [string, () => void] => [`act_${a}`, () => ((act = a), setActivity(a))],
          ),
        ]
      : [];
    let demoAt = 3000;
    let demoShot: string | null = null;

    let last = performance.now();
    let frames = 0;
    let fpsSince = last;
    const frame = (now: number) => {
      raf = requestAnimationFrame(frame);
      const active = BENCH || speaking || state !== "idle";
      const fps = active ? FPS_ACTIVE : act === "sleep" ? FPS_SLEEP : FPS_IDLE;
      const dt = (now - last) / 1000;
      if (!avatar || dt < 1 / fps - 0.002) return;
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
      avatar.activity = act;
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
      if (DEMO && now > demoAt) {
        if (demoShot) api.avatarSnapshot(c.toDataURL("image/png"), demoShot).catch(() => {});
        const next = demo.shift();
        demoShot = next?.[0] ?? null;
        next?.[1]();
        demoAt = next ? now + 4000 : Infinity;
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
      clearTimeout(bubbleTimer);
      timers.forEach((t) => (clearTimeout(t), clearInterval(t)));
      unlisten.forEach((u) => u());
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

  const prop = idle ? PROPS[activity] : undefined;
  return (
    <div className="group relative h-screen w-screen select-none overflow-hidden">
      <canvas
        ref={canvas}
        className="h-full w-full cursor-grab"
        onPointerDown={(e) => e.button === 0 && api.avatarDrag()}
        onDoubleClick={() => api.setAvatarMode("chat")}
        title="Kéo để di chuyển · nhấp đúp mở khung chat · giữ F2 hoặc Ctrl+Space+Q để nói"
      />
      {prop && (
        <span aria-hidden className="pointer-events-none absolute bottom-6 left-2 text-3xl drop-shadow">
          {prop}
        </span>
      )}
      {idle && (activity === "sleep" || activity === "drowsy") && (
        <span aria-hidden className="pointer-events-none absolute right-8 top-10 animate-pulse text-xl text-violet-400">
          💤
        </span>
      )}
      {idle && activity === "music" && (
        <span aria-hidden className="pointer-events-none absolute right-6 top-14 animate-bounce text-lg text-pink-400">
          ♪
        </span>
      )}
      {bubble && (
        <p
          role="status"
          className="pointer-events-none absolute inset-x-3 top-2 rounded-2xl bg-white/90 px-3 py-1.5 text-center text-xs text-slate-700 shadow"
        >
          {bubble}
        </p>
      )}
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
