const WIDTH_KEY = "danxi.sidebar.width";
const COLLAPSED_KEY = "danxi.sidebar.collapsed";

export const SIDEBAR_MIN_WIDTH = 160;
export const SIDEBAR_MAX_WIDTH = 340;
export const SIDEBAR_DEFAULT_WIDTH = 232;
export const SIDEBAR_RAIL_WIDTH = 56;

function loadNumber(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    const value = raw == null ? Number.NaN : Number(raw);
    return Number.isFinite(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

function loadBool(key: string): boolean {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}

function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Persistence is best-effort; the sidebar still works without it.
  }
}

/** Sidebar geometry state: drag-to-resize width and collapsed rail mode. */
export class SidebarStore {
  width = $state(
    Math.min(
      SIDEBAR_MAX_WIDTH,
      Math.max(SIDEBAR_MIN_WIDTH, loadNumber(WIDTH_KEY, SIDEBAR_DEFAULT_WIDTH)),
    ),
  );
  collapsed = $state(loadBool(COLLAPSED_KEY));
  /** True while the user drags the resize handle; suspends width transitions. */
  resizing = $state(false);

  setWidth(value: number) {
    this.width = Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, value));
  }

  commitWidth() {
    persist(WIDTH_KEY, String(Math.round(this.width)));
  }

  resetWidth() {
    this.setWidth(SIDEBAR_DEFAULT_WIDTH);
    this.commitWidth();
  }

  toggle() {
    this.collapsed = !this.collapsed;
    persist(COLLAPSED_KEY, this.collapsed ? "1" : "0");
  }
}

export const sidebar = new SidebarStore();
