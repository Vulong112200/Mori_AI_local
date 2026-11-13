// Nhân vật Live2D của Mori (dev: mẫu Koharu — ADR-0006) trên Cubism Web Framework R5.
// Không dùng nhóm EyeBlink/LipSync trong model3.json (Koharu không khai báo) → tự đặt tham số PARAM_* mỗi khung hình.
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

export type AvatarState = "idle" | "listening" | "thinking" | "speaking";

export const SHADERS = "/live2d/shaders/";
const PRIORITY_IDLE = 1;
/// Mức RMS giọng TTS (~0.02–0.25) → độ mở miệng 0–1.
const MOUTH_GAIN = 5;

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

export class Avatar extends CubismUserModel {
  state: AvatarState = "idle";
  /** RMS âm thanh Mori đang nói (sự kiện `tts-level`). */
  level = 0;
  private scheduler = new CubismUpdateScheduler();
  private idle: CubismMotion[] = [];
  private id!: Record<"mouth" | "eyeL" | "eyeR" | "ballX" | "ballY" | "sparkle" | "cheek", CubismIdHandle>;
  private t = 0;
  private blinkAt = 2;
  private mouth = 0;
  private look = 0;
  private sparkle = 0;

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
    const id = (s: string) => CubismFramework.getIdManager().getId(s);
    a.id = {
      mouth: id("PARAM_MOUTH_OPEN_Y"),
      eyeL: id("PARAM_EYE_L_OPEN"),
      eyeR: id("PARAM_EYE_R_OPEN"),
      ballX: id("PARAM_EYE_BALL_X"),
      ballY: id("PARAM_EYE_BALL_Y"),
      sparkle: id("PARAM_EYE_01"),
      cheek: id("PARAM_CHEEK"),
    };
    const breath = CubismBreath.create();
    breath.setParameters([
      new BreathParameterData(id("PARAM_ANGLE_X"), 0, 15, 6.5345, 0.5),
      new BreathParameterData(id("PARAM_ANGLE_Y"), 0, 8, 3.5345, 0.5),
      new BreathParameterData(id("PARAM_ANGLE_Z"), 0, 10, 5.5345, 0.5),
      new BreathParameterData(id("PARAM_BODY_ANGLE_X"), 0, 4, 15.5345, 0.5),
      new BreathParameterData(id("PARAM_BREATH"), 0.5, 0.5, 3.2345, 1),
    ]);
    a.scheduler.addUpdatableList(new CubismBreathUpdater(breath));
    for (let i = 0; i < setting.getMotionCount("Idle"); i++) {
      const b = await bytes(dir + setting.getMotionFileName("Idle", i));
      const m = a.loadMotion(b, b.byteLength, `Idle_${i}`);
      if (m) {
        m.setEffectIds([], []); // mắt/miệng do update() tự đặt; bỏ trống thì motion đọc null → lỗi
        a.idle.push(m);
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

  update(dt: number) {
    this.t += dt;
    const model = this.getModel();
    model.loadParameters();
    if (this._motionManager.isFinished() && this.idle.length > 0) {
      this._motionManager.startMotionPriority(this.idle[Math.floor(Math.random() * this.idle.length)], false, PRIORITY_IDLE);
    } else {
      this._motionManager.updateMotion(model, dt);
    }
    model.saveParameters();

    // Chớp mắt 0.15 s, cách nhau 2–6 s.
    let eye = 1;
    if (this.t >= this.blinkAt) {
      const p = (this.t - this.blinkAt) / 0.15;
      eye = p < 1 ? Math.abs(1 - 2 * p) : 1;
      if (p >= 1) this.blinkAt = this.t + 2 + Math.random() * 4;
    }
    const speaking = this.state === "speaking";
    this.mouth = approach(this.mouth, speaking ? Math.min(1, this.level * MOUTH_GAIN) : 0, 18, dt);
    this.look = approach(this.look, this.state === "thinking" ? 0.8 : 0, 4, dt); // nghĩ: liếc sang bên
    this.sparkle = approach(this.sparkle, this.state === "listening" ? 1 : 0, 6, dt); // nghe: mắt long lanh
    model.setParameterValueById(this.id.eyeL, eye);
    model.setParameterValueById(this.id.eyeR, eye);
    model.setParameterValueById(this.id.mouth, this.mouth);
    model.setParameterValueById(this.id.ballX, this.look);
    model.setParameterValueById(this.id.ballY, this.look * 0.5);
    model.setParameterValueById(this.id.sparkle, this.sparkle);
    model.setParameterValueById(this.id.cheek, this.sparkle * 0.6);

    this.scheduler.onLateUpdate(model, dt);
    model.update();
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
}
