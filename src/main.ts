import "./styles.css";

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import { initTheme } from "./theme";
import { initUpdater } from "./updater";
import { queue, formatBytes, type QueueItem } from "./queue";
import { FORMATS, commonTargets } from "./formats";

interface ConversionResult {
  ok: boolean;
  output_path?: string;
  error?: string;
}

interface FileMeta {
  path: string;
  name: string;
  size_bytes: number;
}

const $ = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing element #${id}`);
  return el as T;
};

function setStatus(text: string): void {
  $("status-text").textContent = text;
}

async function addPaths(paths: string[]): Promise<void> {
  if (paths.length === 0) return;
  try {
    const metas = await invoke<FileMeta[]>("get_file_meta", { paths });
    queue.add(
      metas.map((m) => ({ path: m.path, name: m.name, sizeBytes: m.size_bytes }))
    );
    setStatus(`${metas.length} file${metas.length === 1 ? "" : "s"} added`);
  } catch (err) {
    console.error(err);
    setStatus(`Error reading files: ${String(err)}`);
  }
}

async function promptForFiles(): Promise<void> {
  const selected = await openDialog({
    multiple: true,
    directory: false,
    title: "Select files to convert",
  });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  await addPaths(paths);
}

function updateFormatPicker(): void {
  const select = $<HTMLSelectElement>("format-select");
  const convertBtn = $<HTMLButtonElement>("convert-btn");

  const items = queue.getAll();
  const exts = items.map((i) => i.ext);
  const targets = commonTargets(exts);

  const previous = select.value;
  select.innerHTML = "";

  if (items.length === 0) {
    select.innerHTML = '<option value="">—</option>';
    select.disabled = true;
    convertBtn.disabled = true;
    return;
  }

  if (targets.length === 0) {
    select.innerHTML = '<option value="">No common target</option>';
    select.disabled = true;
    convertBtn.disabled = true;
    setStatus("Files in the queue don't share a common target format.");
    return;
  }

  select.disabled = false;
  for (const t of targets) {
    const opt = document.createElement("option");
    opt.value = t;
    opt.textContent = FORMATS[t]?.label ?? t.toUpperCase();
    select.appendChild(opt);
  }
  if (targets.includes(previous)) select.value = previous;
  convertBtn.disabled = items.every((i) => i.status === "converting");
}

function renderQueue(): void {
  const section = $("queue-section");
  const list = $<HTMLUListElement>("file-list");
  const count = $("queue-count");
  const items = queue.getAll();

  count.textContent = String(items.length);
  section.classList.toggle("hidden", items.length === 0);

  list.innerHTML = "";
  for (const item of items) {
    list.appendChild(renderRow(item));
  }

  updateFormatPicker();
}

function renderRow(item: QueueItem): HTMLLIElement {
  const li = document.createElement("li");
  li.className = `file-row status-${item.status}`;

  const icon = document.createElement("div");
  icon.className = "file-icon";
  icon.textContent = (item.ext || "?").toUpperCase().slice(0, 4);

  const meta = document.createElement("div");
  meta.className = "file-meta";
  const nameEl = document.createElement("div");
  nameEl.className = "file-name";
  nameEl.textContent = item.name;
  const subEl = document.createElement("div");
  subEl.className = "file-sub";
  subEl.textContent = statusText(item);
  meta.appendChild(nameEl);
  meta.appendChild(subEl);

  const actions = document.createElement("div");
  actions.className = "file-actions";
  if (item.status === "done" && item.outputPath) {
    const reveal = document.createElement("button");
    reveal.className = "btn btn-ghost btn-small";
    reveal.textContent = "Show";
    reveal.addEventListener("click", () => {
      void invoke("reveal_in_file_manager", { path: item.outputPath });
    });
    actions.appendChild(reveal);
  }
  const removeBtn = document.createElement("button");
  removeBtn.className = "icon-btn small";
  removeBtn.title = "Remove";
  removeBtn.ariaLabel = "Remove";
  removeBtn.innerHTML =
    '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';
  removeBtn.addEventListener("click", () => queue.remove(item.id));
  actions.appendChild(removeBtn);

  li.appendChild(icon);
  li.appendChild(meta);
  li.appendChild(actions);
  return li;
}

function statusText(item: QueueItem): string {
  const size = formatBytes(item.sizeBytes);
  switch (item.status) {
    case "queued":
      return `${size} · ready`;
    case "converting":
      return `${size} · converting…`;
    case "done":
      return `${size} · done → ${item.outputPath ?? ""}`;
    case "error":
      return `${size} · error: ${item.error ?? "unknown"}`;
  }
}

async function convertAll(): Promise<void> {
  const select = $<HTMLSelectElement>("format-select");
  const target = select.value;
  if (!target) return;

  const convertBtn = $<HTMLButtonElement>("convert-btn");
  convertBtn.disabled = true;

  const items = queue.getAll().filter((i) => i.status !== "converting");
  let okCount = 0;
  let errCount = 0;

  for (const item of items) {
    queue.update(item.id, { status: "converting", error: undefined });
    setStatus(`Converting ${item.name} → ${target.toUpperCase()}…`);
    try {
      const res = await invoke<ConversionResult>("convert_file", {
        inputPath: item.path,
        targetExt: target,
      });
      if (res.ok && res.output_path) {
        queue.update(item.id, { status: "done", outputPath: res.output_path });
        okCount++;
      } else {
        queue.update(item.id, { status: "error", error: res.error ?? "failed" });
        errCount++;
      }
    } catch (err) {
      queue.update(item.id, { status: "error", error: String(err) });
      errCount++;
    }
  }

  convertBtn.disabled = false;
  setStatus(
    `Finished · ${okCount} ok${errCount ? ` · ${errCount} failed` : ""}`
  );
}

function wireDragAndDrop(): void {
  const dz = $("dropzone");

  // Prevent browser default so files don't replace the page when dropped
  // outside the dropzone (e.g. if the user misses by a pixel).
  const prevent = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
  };
  for (const evt of ["dragenter", "dragover", "dragleave", "drop"] as const) {
    window.addEventListener(evt, prevent);
  }

  // Tauri v2: native drag/drop events on the webview give us real paths.
  const webview = getCurrentWebview();
  void webview.onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === "over" || payload.type === "enter") {
      dz.classList.add("drag-over");
    } else if (payload.type === "leave") {
      dz.classList.remove("drag-over");
    } else if (payload.type === "drop") {
      dz.classList.remove("drag-over");
      if (Array.isArray(payload.paths) && payload.paths.length > 0) {
        void addPaths(payload.paths);
      }
    }
  });

  dz.addEventListener("click", () => void promptForFiles());
  dz.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      void promptForFiles();
    }
  });
}

function main(): void {
  initTheme($("theme-toggle"));
  wireDragAndDrop();

  $("browse-btn").addEventListener("click", (e) => {
    e.stopPropagation();
    void promptForFiles();
  });
  $("clear-btn").addEventListener("click", () => {
    queue.clear();
    setStatus("Queue cleared");
  });
  $("convert-btn").addEventListener("click", () => void convertAll());

  queue.subscribe(renderQueue);
  renderQueue();
  setStatus("Ready");

  void initUpdater($("update-banner"), setStatus);
}

document.addEventListener("DOMContentLoaded", main);
