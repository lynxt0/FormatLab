/**
 * Central state for the conversion queue. Tiny observable store — keeping
 * the dependency footprint at zero means no framework, no bundler surprises.
 */

import { extOf } from "./formats";

export type FileStatus = "queued" | "converting" | "done" | "error";

export interface QueueItem {
  id: string;
  /** Absolute path on disk. Only available when the user drops via Tauri or picks via the native dialog. */
  path: string;
  /** Display name (basename). */
  name: string;
  sizeBytes: number;
  ext: string;
  status: FileStatus;
  /** Populated when conversion finishes. */
  outputPath?: string;
  /** Populated when conversion fails. */
  error?: string;
}

type Listener = () => void;

class QueueStore {
  private items: QueueItem[] = [];
  private listeners = new Set<Listener>();
  private nextId = 1;

  getAll(): ReadonlyArray<QueueItem> {
    return this.items;
  }

  add(paths: Array<{ path: string; name: string; sizeBytes: number }>): void {
    const existing = new Set(this.items.map((i) => i.path));
    let added = 0;
    for (const p of paths) {
      if (existing.has(p.path)) continue;
      this.items.push({
        id: String(this.nextId++),
        path: p.path,
        name: p.name,
        sizeBytes: p.sizeBytes,
        ext: extOf(p.name),
        status: "queued",
      });
      added++;
    }
    if (added > 0) this.notify();
  }

  remove(id: string): void {
    const before = this.items.length;
    this.items = this.items.filter((i) => i.id !== id);
    if (this.items.length !== before) this.notify();
  }

  clear(): void {
    if (this.items.length === 0) return;
    this.items = [];
    this.notify();
  }

  update(id: string, patch: Partial<QueueItem>): void {
    const idx = this.items.findIndex((i) => i.id === id);
    if (idx < 0) return;
    this.items[idx] = { ...this.items[idx], ...patch };
    this.notify();
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private notify(): void {
    for (const fn of this.listeners) fn();
  }
}

export const queue = new QueueStore();

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}
