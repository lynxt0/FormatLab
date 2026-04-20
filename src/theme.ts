/**
 * Light/dark theme handling. Stored in localStorage, falls back to system.
 */

type Theme = "light" | "dark";

const STORAGE_KEY = "formatlab.theme";

function systemTheme(): Theme {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function currentTheme(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return systemTheme();
}

function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
}

export function initTheme(toggleBtn: HTMLElement): void {
  applyTheme(currentTheme());

  toggleBtn.addEventListener("click", () => {
    const next: Theme = currentTheme() === "dark" ? "light" : "dark";
    localStorage.setItem(STORAGE_KEY, next);
    applyTheme(next);
  });

  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => {
      if (!localStorage.getItem(STORAGE_KEY)) applyTheme(systemTheme());
    });
}
