import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

class DesktopWindowController {
  readonly available = isTauri();

  async minimize() {
    if (!this.available) return;
    await getCurrentWindow().minimize();
  }

  async toggleMaximize() {
    if (!this.available) return;
    await getCurrentWindow().toggleMaximize();
  }

  async close() {
    if (!this.available) return;
    await getCurrentWindow().close();
  }

  /** Open an external URL with the system browser, never inside the app window. */
  async openExternal(url: string) {
    if (!this.available) {
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  }
}

export const desktopWindow = new DesktopWindowController();

/**
 * Whether the UI is running on macOS, where the native traffic-light
 * controls overlay the content and custom window buttons must be hidden.
 */
export const isMacOS =
  typeof navigator !== "undefined" && /Mac/i.test(navigator.platform ?? navigator.userAgent);
