import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

type Kind = "obj" | "mtl" | "png";
const KINDS: Kind[] = ["obj", "mtl", "png"];

interface TypeInfo { key: string; path: string }
interface AppInfo { mod_id: string; model_types: TypeInfo[] }
interface ExportResult { obj_target: string; mtl_target: string; png_target: string }

const NAME_RE = /^[a-z](?:[a-z_]*[a-z])?$/;

const files: Record<Kind, string | null> = { obj: null, mtl: null, png: null };
let busy = false;
let hasTypes = false;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (className) e.className = className;
  if (text !== undefined) e.textContent = text;
  return e;
}

async function copyText(text: string, btn?: HTMLButtonElement) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    /* Fallback for webviews that block the async clipboard API */
    const ta = el("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  if (btn) {
    const old = btn.textContent;
    btn.textContent = "Copied";
    setTimeout(() => (btn.textContent = old), 1200);
  }
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function setFile(kind: Kind, path: string | null) {
  files[kind] = path;
  const span = $<HTMLSpanElement>(`path-${kind}`);
  span.textContent = path ?? "No file selected";
  span.title = path ?? "";
  span.classList.toggle("empty", path === null);
  refresh();
}

function nameIsValid(): boolean {
  return NAME_RE.test($<HTMLInputElement>("model-name").value);
}

function refresh() {
  const name = $<HTMLInputElement>("model-name").value;
  const hint = $<HTMLParagraphElement>("name-hint");
  hint.classList.toggle("bad", name.length > 0 && !nameIsValid());

  const ready = KINDS.every((k) => files[k]) && nameIsValid() && hasTypes && !busy;
  $<HTMLButtonElement>("export").disabled = !ready;
  document.querySelectorAll<HTMLButtonElement>("[data-browse]").forEach((b) => (b.disabled = busy));
}

function showNotice(text: string | null) {
  const n = $<HTMLParagraphElement>("notice");
  n.hidden = text === null;
  n.textContent = text ?? "";
}

async function browse(kind: Kind) {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: kind.toUpperCase(), extensions: [kind] }],
  });
  if (typeof selected === "string") {
    showNotice(null);
    setFile(kind, selected);
  }
}

function handleDrop(paths: string[]) {
  const ignored: string[] = [];
  const seen = new Set<Kind>();
  const dup: string[] = [];
  for (const p of paths) {
    const ext = p.split(".").pop()?.toLowerCase() ?? "";
    if ((KINDS as string[]).includes(ext)) {
      const kind = ext as Kind;
      if (seen.has(kind)) dup.push(fileName(p));
      else setFile(kind, p);
      seen.add(kind);
    } else {
      ignored.push(fileName(p));
    }
  }
  const msgs: string[] = [];
  if (ignored.length) msgs.push(`Ignored (not obj/mtl/png): ${ignored.join(", ")}`);
  if (dup.length) msgs.push(`Ignored (only one file per type): ${dup.join(", ")}`);
  showNotice(msgs.length ? msgs.join(". ") : null);
}

function clearResult() {
  const r = $<HTMLElement>("result");
  r.hidden = true;
  r.replaceChildren();
}

function showError(message: string, context: string) {
  const r = $<HTMLElement>("result");
  r.className = "result error";
  r.replaceChildren();
  r.append(el("h2", undefined, "Export failed"));
  if (message.includes("WARNING")) {
    r.append(el("p", "warn", "Some files may have been written. Do not retry until the maintainer has checked the target folders."));
  }
  r.append(el("p", "hint", "Copy this report and send it to the tool maintainer:"));

  const report = `OBJMF error report\nError: ${message}\n${context}`;
  const ta = el("textarea");
  ta.readOnly = true;
  ta.rows = 9;
  ta.value = report;
  const btn = el("button", undefined, "Copy report");
  btn.type = "button";
  btn.addEventListener("click", () => copyText(report, btn));
  r.append(ta, btn);
  r.hidden = false;
}

function showSuccess(res: ExportResult) {
  const r = $<HTMLElement>("result");
  r.className = "result ok";
  r.replaceChildren();
  r.append(el("h2", undefined, "Export complete"));
  r.append(el("p", "hint", "Files were written to:"));

  const rows: [string, string][] = [
    ["OBJ", res.obj_target],
    ["MTL", res.mtl_target],
    ["PNG", res.png_target],
  ];
  for (const [label, path] of rows) {
    const row = el("div", "target");
    row.append(el("span", "label", label), el("code", undefined, path));
    const copy = el("button", undefined, "Copy");
    copy.type = "button";
    copy.addEventListener("click", () => copyText(path, copy));
    const show = el("button", undefined, "Show in folder");
    show.type = "button";
    show.addEventListener("click", async () => {
      try { await revealItemInDir(path); } catch { /* the path is still shown */ }
    });
    row.append(copy, show);
    r.append(row);
  }

  const again = el("button", "primary", "Export another");
  again.type = "button";
  again.addEventListener("click", () => {
    KINDS.forEach((k) => setFile(k, null));
    $<HTMLInputElement>("model-name").value = "";
    showNotice(null);
    clearResult();
    refresh();
  });
  r.append(again);
  r.hidden = false;
}

async function runExport() {
  const modelName = $<HTMLInputElement>("model-name").value;
  const modelType = $<HTMLSelectElement>("model-type").value;
  const context = [
    `Model name: ${modelName}`,
    `Model type: ${modelType}`,
    `OBJ: ${files.obj}`,
    `MTL: ${files.mtl}`,
    `PNG: ${files.png}`,
  ].join("\n");

  busy = true;
  clearResult();
  refresh();
  try {
    const res = await invoke<ExportResult>("export_model", {
      modelName,
      modelType,
      obj: files.obj,
      mtl: files.mtl,
      png: files.png,
    });
    showSuccess(res);
  } catch (e) {
    showError(String(e), context);
  } finally {
    busy = false;
    refresh();
  }
}

function showFatal(message: string) {
  $<HTMLElement>("app").hidden = true;
  $<HTMLElement>("fatal").hidden = false;
  const text = `OBJMF startup error\nError: ${message}`;
  $<HTMLTextAreaElement>("fatal-text").value = text;
  const btn = $<HTMLButtonElement>("fatal-copy");
  btn.addEventListener("click", () => copyText(text, btn));
}

window.addEventListener("DOMContentLoaded", async () => {
  let info: AppInfo;
  try {
    info = await invoke<AppInfo>("get_app_info");
  } catch (e) {
    showFatal(String(e));
    return;
  }

  $<HTMLElement>("mod-id").textContent = info.mod_id;
  const select = $<HTMLSelectElement>("model-type");
  for (const t of info.model_types) {
    const opt = el("option", undefined, `${t.key}  (${t.path})`);
    opt.value = t.key;
    select.append(opt);
  }
  hasTypes = info.model_types.length > 0;
  if (!hasTypes) {
    showNotice("The config defines no model types, so nothing can be exported");
  }

  document.querySelectorAll<HTMLButtonElement>("[data-browse]").forEach((b) =>
    b.addEventListener("click", () => browse(b.dataset.browse as Kind)),
  );
  $<HTMLInputElement>("model-name").addEventListener("input", refresh);
  $<HTMLButtonElement>("export").addEventListener("click", runExport);

  try {
    const drop = $<HTMLElement>("drop");
    await getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === "enter" || p.type === "over") drop.classList.add("active");
      else drop.classList.remove("active");
      if (p.type === "drop") handleDrop(p.paths);
    });
  } catch {
    /* Drag and drop is a convenience, Browse still works */
  }

  $<HTMLElement>("app").hidden = false;
  refresh();
});