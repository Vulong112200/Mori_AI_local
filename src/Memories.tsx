// Panel "Ghi nhớ": duyệt/sửa/xóa memory (SPEC §11.4) + sao lưu/khôi phục DB.
import { useEffect, useState } from "react";
import * as api from "./api";
import type { Memory, MemoryCategory } from "./api";

const KEEP_NOTE =
  "Xóa ghi nhớ chỉ xóa mục ghi nhớ: tin nhắn gốc trong cuộc trò chuyện, phần tóm tắt và các bản sao lưu vẫn còn nội dung đó.";

export default function Memories({ onClose, onCount }: { onClose: () => void; onCount: (proposed: number) => void }) {
  const [items, setItems] = useState<Memory[]>([]);
  const [backups, setBackups] = useState<string[]>([]);
  const [chosen, setChosen] = useState("");
  const [confirm, setConfirm] = useState<"all" | "restore" | null>(null);
  const [note, setNote] = useState<string | null>(null);

  async function reload() {
    try {
      const list = await api.listMemories();
      setItems(list);
      onCount(list.filter((m) => m.status === "proposed").length);
      const b = await api.listBackups();
      setBackups(b);
      setChosen((c) => (b.includes(c) ? c : (b[0] ?? "")));
    } catch (e) {
      setNote(String(e));
    }
  }

  useEffect(() => {
    reload();
    const esc = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", esc);
    return () => window.removeEventListener("keydown", esc);
  }, []);

  async function run(action: () => Promise<unknown>, done?: string) {
    setNote(null);
    try {
      await action();
      if (done) setNote(done);
      await reload();
    } catch (e) {
      setNote(String(e));
    }
  }

  const proposed = items.filter((m) => m.status === "proposed");
  const approved = items.filter((m) => m.status === "approved");

  return (
    <div className="fixed inset-0 z-20 flex justify-end bg-slate-900/20" onClick={onClose}>
      <aside
        role="dialog"
        aria-label="Ghi nhớ"
        className="flex h-full w-full max-w-md flex-col gap-3 overflow-y-auto bg-white p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="font-semibold">⭐ Ghi nhớ</h2>
          <button onClick={onClose} className="rounded-lg px-2 py-1 text-sm text-slate-500 hover:bg-pink-100" aria-label="Đóng">
            ✕
          </button>
        </div>
        <p className="text-xs text-slate-500">
          Mori tự ghi nhớ điều quan trọng bạn kể (sở thích, công việc, người thân…). Bạn cũng có thể nói "Mori, nhớ là …" hoặc bấm ⭐ trên tin nhắn. Sửa hoặc xóa bất cứ lúc nào ở đây.
        </p>
        <p className="text-xs text-slate-500">{KEEP_NOTE}</p>

        {proposed.length > 0 && (
          <Section title={`Chờ duyệt (${proposed.length})`}>
            {proposed.map((m) => (
              <Item key={m.id} memory={m} run={run} />
            ))}
          </Section>
        )}

        <Section title={`Mori đang nhớ (${approved.length})`}>
          {approved.length === 0 && <Empty text="Mori chưa nhớ gì về bạn." />}
          {approved.map((m) => (
            <Item key={m.id} memory={m} run={run} />
          ))}
          {approved.length + proposed.length > 0 &&
            (confirm === "all" ? (
              <div className="flex items-center gap-2 text-xs">
                <span className="text-rose-700">Xóa vĩnh viễn mọi mục ghi nhớ? ({KEEP_NOTE})</span>
                <button onClick={() => (setConfirm(null), run(api.deleteAllMemories, "Đã xóa toàn bộ ghi nhớ."))} className="rounded-lg bg-rose-600 px-2 py-1 text-white">
                  Xóa hết
                </button>
                <button onClick={() => setConfirm(null)} className="rounded-lg px-2 py-1 hover:bg-slate-100">
                  Thôi
                </button>
              </div>
            ) : (
              <button onClick={() => setConfirm("all")} className="self-start rounded-lg px-2 py-1 text-xs text-rose-600 hover:bg-rose-50">
                Xóa toàn bộ ghi nhớ…
              </button>
            ))}
        </Section>

        <Section title="Sao lưu">
          <p className="text-xs text-slate-500">
            Sao lưu cả hội thoại và ghi nhớ vào <code>E:\Mori Data\backups</code>. Bản sao lưu cũ vẫn giữ những gì bạn xóa sau đó.
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <button onClick={() => run(api.backupDb, "Đã sao lưu.")} className="rounded-lg bg-pink-100 px-2.5 py-1 text-xs text-pink-700 hover:bg-pink-200">
              Sao lưu ngay
            </button>
            {backups.length > 0 && (
              <>
                <label htmlFor="backup" className="sr-only">
                  Bản sao lưu
                </label>
                <select id="backup" value={chosen} onChange={(e) => setChosen(e.target.value)} className="max-w-48 rounded-lg border border-pink-100 px-1.5 py-1 text-xs">
                  {backups.map((b) => (
                    <option key={b} value={b}>
                      {b}
                    </option>
                  ))}
                </select>
                {confirm === "restore" ? (
                  <>
                    <button
                      onClick={async () => {
                        setConfirm(null);
                        try {
                          await api.restoreBackup(chosen);
                          window.location.reload(); // nạp lại hội thoại từ DB đã khôi phục
                        } catch (e) {
                          setNote(String(e));
                        }
                      }}
                      className="rounded-lg bg-amber-500 px-2 py-1 text-xs text-white"
                    >
                      Khôi phục
                    </button>
                    <button onClick={() => setConfirm(null)} className="rounded-lg px-2 py-1 text-xs hover:bg-slate-100">
                      Thôi
                    </button>
                  </>
                ) : (
                  <button onClick={() => setConfirm("restore")} className="rounded-lg px-2 py-1 text-xs text-amber-700 hover:bg-amber-50">
                    Khôi phục…
                  </button>
                )}
              </>
            )}
          </div>
          {confirm === "restore" && (
            <p className="text-xs text-amber-700">Dữ liệu hiện tại sẽ được thay bằng bản đã chọn (Mori tự sao lưu bản hiện tại trước).</p>
          )}
        </Section>

        {note && (
          <p role="status" className="rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-600">
            {note}
          </p>
        )}
      </aside>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2 border-t border-pink-100 pt-3">
      <h3 className="text-sm font-medium text-slate-700">{title}</h3>
      {children}
    </section>
  );
}

function Empty({ text }: { text: string }) {
  return <p className="text-xs text-slate-400">{text}</p>;
}

function Item({ memory, run }: { memory: Memory; run: (a: () => Promise<unknown>, done?: string) => Promise<void> }) {
  const [content, setContent] = useState(memory.content);
  const [category, setCategory] = useState<MemoryCategory>(memory.category);
  const pending = memory.status === "proposed";
  const changed = content !== memory.content || category !== memory.category;
  const tooLong = content.length > api.MAX_MEMORY_CHARS;
  return (
    <div className={`flex flex-col gap-1.5 rounded-xl border p-2 ${pending ? "border-amber-200 bg-amber-50/50" : "border-pink-100"}`}>
      <label htmlFor={`m-${memory.id}`} className="sr-only">
        Nội dung ghi nhớ
      </label>
      <textarea
        id={`m-${memory.id}`}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        rows={2}
        className="resize-y rounded-lg border border-pink-100 bg-white px-2 py-1 text-sm outline-none focus:border-pink-400"
      />
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        <label htmlFor={`c-${memory.id}`} className="sr-only">
          Nhóm
        </label>
        <select
          id={`c-${memory.id}`}
          value={category}
          onChange={(e) => setCategory(e.target.value as MemoryCategory)}
          className="rounded-lg border border-pink-100 bg-white px-1.5 py-0.5"
        >
          {Object.entries(api.MEMORY_CATEGORIES).map(([k, v]) => (
            <option key={k} value={k}>
              {v}
            </option>
          ))}
        </select>
        <span className="flex-1" />
        {tooLong && <span className="text-rose-600">tối đa {api.MAX_MEMORY_CHARS} ký tự</span>}
        {(pending || changed) && (
          <button
            disabled={!content.trim() || tooLong}
            onClick={() => run(() => api.updateMemory(memory.id, content, category, true), pending ? "Đã duyệt." : "Đã lưu.")}
            className="rounded-lg bg-emerald-500 px-2 py-0.5 text-white hover:bg-emerald-600 disabled:opacity-40"
          >
            {pending ? "Duyệt" : "Lưu"}
          </button>
        )}
        <button onClick={() => run(() => api.deleteMemory(memory.id), pending ? "Đã bỏ đề xuất." : "Đã xóa.")} className="rounded-lg px-2 py-0.5 text-rose-600 hover:bg-rose-50">
          {pending ? "Bỏ" : "Xóa"}
        </button>
      </div>
    </div>
  );
}
