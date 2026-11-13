// Nhân vật Live2D của Mori (dev: mẫu Koharu — ADR-0006) trên Cubism Web Framework R5.
// Không dùng nhóm EyeBlink/LipSync trong model3.json (Koharu không khai báo) → tự đặt tham số PARAM_* mỗi khung hình.
// Phase 7: biểu cảm theo cảm xúc câu trả lời, động tác ngẫu nhiên từ mọi nhóm motion, hoạt cảnh theo giờ (ADR-0013).
import { CubismFramework, LogLevel, Option } from "./framework/live2dcubismframework";
import { CubismUserModel } from "./framework/model/cubismusermodel";
import { CubismModelSettingJson } from "./framework/cubismmodelsettingjson";
import { CubismMatrix44 } from "./framework/math/cubismmatrix44";
import { BreathParameterData, CubismBreath } from "./framework/effect/cubismbreath";
import { CubismUpdateScheduler } from "./framework/motion/cubismupdatescheduler";
import { CubismBreathUpdater } from "./framework/motion/cubismbreathupdater";
import { CubismPhysicsUpdater } from "./framework/motion/cubismphysicsupdater";
import type { CubismMotion } from "./framework/motion/cubismmotion";
import type { CubismIdHandle } from "./framework/id/cubismid";
import type { CubismModel } from "./framework/model/cubismmodel";

export type AvatarState = "idle" | "listening" | "thinking" | "speaking";
export type Emotion = "neutral" | "happy" | "love" | "sad" | "surprised" | "shy" | "worried";
export type Activity = "free" | "sleep" | "wake" | "eat" | "work" | "study" | "music" | "play" | "drowsy";

export const SHADERS = "/live2d/shaders/";
const PRIORITY_IDLE = 1;
const PRIORITY_ACT = 2;
/// Mức RMS giọng TTS (~0.02–0.25) → độ mở miệng 0–1.
const MOUTH_GAIN = 5;
/// Biểu cảm giữ thêm ngần này giây sau khi hết nói.
const EMOTION_HOLD = 8;

// Motion của Koharu theo tên file (đọc curve: 04 cười híp mắt, 05 khóc, 07/08 pompom cổ vũ, 09 cầm đồ, 02/03 trầm ngâm).
const IDLE = ["idle", "idle_02", "06"];
const FIDGET = ["01", "03", "04", "06", "09"];
const EMOTION_MOTION: Record<Emotion, string[]> = {
  neutral: [],
  happy: ["01", "04", "09"],
  love: ["04", "01"],
  sad: ["05"],
  surprised: ["03"],
  shy: ["01"],
  worried: ["02", "03"],
};
/// Hoạt cảnh dùng bộ motion riêng thay cho Idle. Ngủ: chỉ Idle (tay buông) — dừng hẳn motion thì tham số giữ nguyên
/// tư thế cuối (vd. đang giơ tay), ảnh chụp kiểm tra cho thấy vậy.
const ACTIVITY_MOTION: Partial<Record<Activity, string[]>> = { sleep: ["idle", "idle_02"], play: ["07", "08", "09"] };

// Preset biểu cảm: giá trị chuẩn hóa −1…1 (dương → phần max của tham số, âm → phần min), áp sau motion theo trọng số.
type Param = "smileL" | "smileR" | "mouthForm" | "cheek" | "sparkle" | "tearL" | "tearR" | "browLY" | "browRY" | "browLForm" | "browRForm" | "eyeSize";
const PRESET: Record<Emotion, Partial<Record<Param, number>>> = {
  neutral: {},
  happy: { smileL: 1, smileR: 1, mouthForm: 1, cheek: 0.4 },
  love: { smileL: 1, smileR: 1, mouthForm: 1, cheek: 1, sparkle: 1 },
  sad: { mouthForm: -1, tearL: 0.4, tearR: 0.4, browLY: -0.6, browRY: -0.6, browLForm: -0.8, browRForm: -0.8 },
  surprised: { browLY: 1, browRY: 1, mouthForm: 0, eyeSize: 0.8 },
  shy: { cheek: 1, smileL: 0.5, smileR: 0.5, mouthForm: 0.3 },
  worried: { browLForm: -1, browRForm: -1, browLY: -0.3, browRY: -0.3, mouthForm: -0.6 },
};

let started = false;
function startFramework() {
  if (started) return;
  const option = new Option();
  option.logFunction = (m: string) => console.warn(m);
  option.loggingLevel = LogLevel.LogLevel_Warning;
  CubismFramework.startUp(option);
  CubismFramework.initialize();
  started = true;
}

async function bytes(url: string): Promise<ArrayBuffer> {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`không tải được ${url} (${r.status})`);
  return r.arrayBuffer();
}

async function texture(gl: WebGL2RenderingContext, url: string): Promise<WebGLTexture> {
  const img = new Image();
  img.src = url;
  await img.decode();
  const tex = gl.createTexture();
  if (!tex) throw new Error("không tạo được texture");
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1); // Framework vẽ theo premultiplied alpha
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
  gl.generateMipmap(gl.TEXTURE_2D);
  gl.bindTexture(gl.TEXTURE_2D, null);
  return tex;
}

/** Tiến dần tới đích (làm mượt, không phụ thuộc FPS). */
const approach = (v: number, target: number, rate: number, dt: number) => v + (target - v) * Math.min(1, rate * dt);
const pick = <T>(a: T[]): T | undefined => a[Math.floor(Math.random() * a.length)];

/** Tham số có tên + khoảng giá trị (đọc từ model, không đoán min/max). */
class P {
  constructor(
    readonly id: CubismIdHandle,
    readonly min: number,
    readonly max: number,
  ) {}
  /** Giá trị chuẩn hóa −1…1 → giá trị thật. */
  of(n: number) {
    return n >= 0 ? n * this.max : -n * this.min;
  }
}

export class Avatar extends CubismUserModel {
  state: AvatarState = "idle";
  /** RMS âm thanh Mori đang nói (sự kiện `tts-level`). */
  level = 0;
  activity: Activity = "free";
  private scheduler = new CubismUpdateScheduler();
  private motions = new Map<string, CubismMotion>();
  private p!: Record<Param | "mouth" | "eyeL" | "eyeR" | "ballX" | "ballY" | "angleY" | "angleZ", P>;
  private t = 0;
  private blinkAt = 2;
  private fidgetAt = 30;
  private mouth = 0;
  private look = 0;
  private sparkle = 0;
  private emotion: Emotion = "neutral";
  private emotionUntil = 0;
  private emotionW = 0;
  /** Trọng số hoạt cảnh: 0 khi đang nghe/nghĩ/nói (hoạt cảnh tạm dừng), 1 khi rảnh. */
  private actW = 1;
  private actSince = 0;
  private lastActivity: Activity = "free";

  static async load(gl: WebGL2RenderingContext, dir: string, width: number, height: number): Promise<Avatar> {
    startFramework();
    const a = new Avatar();
    const json = await bytes(dir + "koharu.model3.json");
    const setting = new CubismModelSettingJson(json, json.byteLength);
    a.loadModel(await bytes(dir + setting.getModelFileName()));
    if (setting.getPhysicsFileName()) {
      const b = await bytes(dir + setting.getPhysicsFileName());
      a.loadPhysics(b, b.byteLength);
      a.scheduler.addUpdatableList(new CubismPhysicsUpdater(a._physics));
    }
    const model = a.getModel();
    const param = (s: string) => {
      const id = CubismFramework.getIdManager().getId(s);
      const i = model.getParameterIndex(id);
      return new P(id, model.getParameterMinimumValue(i), model.getParameterMaximumValue(i));
    };
    a.p = {
      mouth: param("PARAM_MOUTH_OPEN_Y"),
      eyeL: param("PARAM_EYE_L_OPEN"),
      eyeR: param("PARAM_EYE_R_OPEN"),
      ballX: param("PARAM_EYE_BALL_X"),
      ballY: param("PARAM_EYE_BALL_Y"),
      sparkle: param("PARAM_EYE_01"),
      cheek: param("PARAM_CHEEK"),
      smileL: param("PARAM_EYE_L_SMILE"),
      smileR: param("PARAM_EYE_R_SMILE"),
      mouthForm: param("PARAM_MOUTH_FORM"),
      tearL: param("PARAM_TEAR_L"),
      tearR: param("PARAM_TEAR_R"),
      browLY: param("PARAM_BROW_L_Y"),
      browRY: param("PARAM_BROW_R_Y"),
      browLForm: param("PARAM_BROW_L_FORM"),
      browRForm: param("PARAM_BROW_R_FORM"),
      eyeSize: param("PARAM_EYE_SIZE"),
      angleY: param("PARAM_ANGLE_Y"),
      angleZ: param("PARAM_ANGLE_Z"),
    };
    const id = (s: string) => CubismFramework.getIdManager().getId(s);
    const breath = CubismBreath.create();
    breath.setParameters([
      new BreathParameterData(id("PARAM_ANGLE_X"), 0, 15, 6.5345, 0.5),
      new BreathParameterData(id("PARAM_ANGLE_Y"), 0, 8, 3.5345, 0.5),
      new BreathParameterData(id("PARAM_ANGLE_Z"), 0, 10, 5.5345, 0.5),
      new BreathParameterData(id("PARAM_BODY_ANGLE_X"), 0, 4, 15.5345, 0.5),
      new BreathParameterData(id("PARAM_BREATH"), 0.5, 0.5, 3.2345, 1),
    ]);
    a.scheduler.addUpdatableList(new CubismBreathUpdater(breath));
    // Mọi nhóm motion (Idle, Tap, Flick…), đặt tên theo file: "motion/04.motion3.json" → "04".
    for (let g = 0; g < setting.getMotionGroupCount(); g++) {
      const group = setting.getMotionGroupName(g);
      for (let i = 0; i < setting.getMotionCount(group); i++) {
        const file = setting.getMotionFileName(group, i);
        const b = await bytes(dir + file);
        const name = file.replace(/^.*\//, "").replace(/\.motion3\.json$/, "");
        const m = a.loadMotion(b, b.byteLength, name);
        if (m) {
          m.setEffectIds([], []); // mắt/miệng do update() tự đặt; bỏ trống thì motion đọc null → lỗi
          a.motions.set(name, m);
        }
      }
    }
    a.createRenderer(width, height);
    const r = a.getRenderer();
    for (let i = 0; i < setting.getTextureCount(); i++) {
      r.bindTexture(i, await texture(gl, dir + setting.getTextureFileName(i)));
    }
    r.setIsPremultipliedAlpha(true);
    r.startUp(gl);
    r.loadShaders(SHADERS);
    return a;
  }

  private play(names: string[] | undefined, priority: number) {
    const m = this.motions.get(pick(names ?? []) ?? "");
    if (m) this._motionManager.startMotionPriority(m, false, priority);
  }

  /** Cảm xúc của câu trả lời (sự kiện `avatar-emotion`): một động tác hợp + giữ biểu cảm tới khi nói xong ~8 s. */
  setEmotion(e: Emotion) {
    if (e === this.emotion && this.t < this.emotionUntil) return;
    this.emotion = e;
    this.emotionUntil = this.t + EMOTION_HOLD;
    this.play(EMOTION_MOTION[e], PRIORITY_ACT);
  }

  update(dt: number) {
    this.t += dt;
    const model = this.getModel();
    const busy = this.state !== "idle";
    if (this.activity !== this.lastActivity) {
      this.lastActivity = this.activity;
      this.actSince = this.t;
      const pool = ACTIVITY_MOTION[this.activity];
      if (pool && !busy) this.play(pool, PRIORITY_ACT); // vào cảnh ngay, không chờ động tác cũ hết
      if (this.activity === "wake") this.blinkAt = this.t + 99; // tự điều khiển mắt trong cảnh thức dậy
    }
    this.actW = approach(this.actW, busy ? 0 : 1, 2, dt);
    const act = this.actW > 0.5 ? this.activity : "free";
    const since = this.t - this.actSince;

    model.loadParameters();
    if (this._motionManager.isFinished()) {
      this.play(ACTIVITY_MOTION[act] ?? IDLE, PRIORITY_IDLE); // động tác của hoạt cảnh nối tiếp nhau
    } else {
      this._motionManager.updateMotion(model, dt);
    }
    // Rảnh mà đứng không thì thỉnh thoảng làm một động tác ngẫu nhiên (khỏi lặp mãi 3 động tác Idle).
    if (this.t >= this.fidgetAt) {
      this.fidgetAt = this.t + 20 + Math.random() * 40;
      if (!busy && act === "free") this.play(FIDGET, PRIORITY_ACT);
    }
    model.saveParameters();

    // Chớp mắt 0.15 s, cách nhau 2–6 s; nhân với độ mở mắt của motion (04/05 nhắm mắt cười/khóc).
    let blink = 1;
    if (this.t >= this.blinkAt) {
      const p = (this.t - this.blinkAt) / 0.15;
      blink = p < 1 ? Math.abs(1 - 2 * p) : 1;
      if (p >= 1) this.blinkAt = this.t + 2 + Math.random() * 4;
    }
    let eye = blink * this.actEye(act, since);
    const speaking = this.state === "speaking";
    if (speaking) this.emotionUntil = Math.max(this.emotionUntil, this.t + EMOTION_HOLD);
    this.emotionW = approach(this.emotionW, this.t < this.emotionUntil && this.emotion !== "neutral" ? 1 : 0, 3, dt);
    let mouth = speaking ? Math.min(1, this.level * MOUTH_GAIN) : 0;
    if (!speaking && act === "eat") mouth = 0.25 * (1 + Math.sin(this.t * 15)); // nhai
    if (!speaking && act === "wake" && since > 3 && since < 5) mouth = 0.9; // ngáp
    if (!speaking && this.emotion === "surprised") mouth = Math.max(mouth, 0.4 * this.emotionW);
    this.mouth = approach(this.mouth, mouth, 18, dt);
    this.look = approach(this.look, this.state === "thinking" ? 0.8 : 0, 4, dt); // nghĩ: liếc sang bên
    this.sparkle = approach(this.sparkle, this.state === "listening" ? 1 : 0, 6, dt); // nghe: mắt long lanh
    if (act === "sleep") eye = 0;

    const set = (p: P, v: number, w = 1) => model.setParameterValueById(p.id, v, w);
    set(this.p.eyeL, model.getParameterValueById(this.p.eyeL.id) * eye);
    set(this.p.eyeR, model.getParameterValueById(this.p.eyeR.id) * eye);
    set(this.p.mouth, this.mouth);
    set(this.p.ballX, this.look);
    set(this.p.ballY, this.look * 0.5);
    set(this.p.sparkle, this.sparkle);
    set(this.p.cheek, this.sparkle * 0.6, this.sparkle);
    // Biểu cảm theo cảm xúc, trộn lên trên motion.
    if (this.emotionW > 0.01) {
      for (const [k, v] of Object.entries(PRESET[this.emotion]) as [Param, number][]) {
        set(this.p[k], this.p[k].of(v), this.emotionW);
      }
    }
    this.applyActivity(model, act, since);

    this.scheduler.onLateUpdate(model, dt);
    model.update();
  }

  /** Độ mở mắt theo hoạt cảnh (1 = bình thường). */
  private actEye(act: Activity, since: number) {
    if (act === "drowsy") return 0.35 + 0.15 * Math.sin(this.t * 0.7); // lim dim
    if (act === "wake") {
      if (since < 1.5) return 0; // còn ngủ
      if (since < 3) return 0.3; // hé mắt
      if (since < 5) return 0.15; // ngáp, nheo mắt
      if (since < 5.5) this.blinkAt = this.t; // trả lại chớp mắt
    }
    return 1;
  }

  /** Tư thế đầu/mắt theo hoạt cảnh, trọng số actW (tự tắt khi đang hội thoại). */
  private applyActivity(model: CubismModel, act: Activity, since: number) {
    const w = this.actW;
    const add = (p: P, v: number) => model.addParameterValueById(p.id, v * w);
    switch (act) {
      case "sleep":
        add(this.p.angleY, -15);
        add(this.p.angleZ, 8 * Math.sin(this.t * 0.4));
        break;
      case "drowsy":
        add(this.p.angleY, -8 + 6 * Math.sin(this.t * 0.9)); // gật gù
        break;
      case "music":
        add(this.p.angleZ, 10 * Math.sin(this.t * Math.PI * 1.6)); // ~96 nhịp/phút
        model.setParameterValueById(this.p.smileL.id, this.p.smileL.of(0.7), w);
        model.setParameterValueById(this.p.smileR.id, this.p.smileR.of(0.7), w);
        break;
      case "work":
      case "study":
        add(this.p.angleY, -10);
        model.setParameterValueById(this.p.ballY.id, this.p.ballY.of(-0.7), w); // nhìn xuống màn hình/sách
        break;
      case "eat":
        add(this.p.angleY, -5 + 3 * Math.sin(this.t * 7.5));
        break;
      case "wake":
        if (since >= 5 && since < 5.1) this.play(["03"], PRIORITY_ACT); // vươn vai
        if (since < 3) add(this.p.angleY, -15);
        break;
    }
  }

  draw(gl: WebGL2RenderingContext, width: number, height: number) {
    const projection = new CubismMatrix44();
    if (this.getModel().getCanvasWidth() > 1 && width < height) {
      this.getModelMatrix().setWidth(2);
      projection.scale(1, width / height);
    } else {
      projection.scale(height / width, 1);
    }
    projection.multiplyByMatrix(this.getModelMatrix());
    const r = this.getRenderer();
    r.setMvpMatrix(projection);
    r.setRenderState(gl.getParameter(gl.FRAMEBUFFER_BINDING), [0, 0, width, height]);
    r.drawModel(SHADERS);
  }

  /** Bản dev: khoảng giá trị tham số (kiểm preset). */
  ranges() {
    return Object.fromEntries(Object.entries(this.p).map(([k, p]) => [k, [p.min, p.max]]));
  }
}
