/**
 * Auto-update integration. On startup we quietly check for a newer
 * release. If one is available, the status bar surfaces an "Update
 * available" affordance; clicking it downloads, installs and restarts.
 */

import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type OnStatus = (message: string) => void;

export async function initUpdater(
  banner: HTMLElement,
  onStatus: OnStatus
): Promise<void> {
  banner.classList.add("hidden");
  banner.innerHTML = "";

  let update: Update | null = null;
  try {
    update = await check();
  } catch (err) {
    // Offline, rate-limited, or no release yet — silent failure is fine.
    console.warn("Update check failed:", err);
    return;
  }
  if (!update) return;

  banner.classList.remove("hidden");
  banner.innerHTML = `
    <span class="update-banner-msg">Version <strong>${escapeHtml(update.version)}</strong> is available.</span>
    <button type="button" class="btn btn-primary btn-small" data-action="install">Install &amp; restart</button>
    <button type="button" class="icon-btn small" data-action="dismiss" aria-label="Dismiss">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
  `;

  banner.addEventListener("click", async (event) => {
    const target = event.target as HTMLElement;
    const action = target.closest("[data-action]")?.getAttribute("data-action");
    if (action === "dismiss") {
      banner.classList.add("hidden");
      return;
    }
    if (action === "install" && update) {
      onStatus(`Downloading v${update.version}…`);
      try {
        await update.downloadAndInstall((event) => {
          if (event.event === "Progress") {
            const mb = (event.data.chunkLength / 1024 / 1024).toFixed(1);
            onStatus(`Downloading update… (${mb} MB chunk)`);
          } else if (event.event === "Finished") {
            onStatus("Update downloaded — restarting");
          }
        });
        await relaunch();
      } catch (err) {
        console.error("Update install failed:", err);
        onStatus(`Update failed: ${String(err)}`);
      }
    }
  });
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
