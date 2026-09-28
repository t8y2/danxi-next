export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = Exclude<ThemePreference, "system">;

const STORAGE_KEY = "danxi-theme";

class ThemeController {
  preference = $state<ThemePreference>("system");
  resolved = $state<ResolvedTheme>("light");
  #mediaQuery: MediaQueryList | null = null;

  init() {
    if (typeof window === "undefined") return;

    this.#mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") {
      this.preference = stored;
    }

    this.#mediaQuery.addEventListener("change", this.#handleSystemThemeChange);
    this.#apply();

    return () => {
      this.#mediaQuery?.removeEventListener("change", this.#handleSystemThemeChange);
      this.#mediaQuery = null;
    };
  }

  set(preference: ThemePreference) {
    this.preference = preference;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, preference);
    }
    this.#apply();
  }

  toggle() {
    this.set(this.resolved === "dark" ? "light" : "dark");
  }

  #handleSystemThemeChange = () => {
    if (this.preference === "system") {
      this.#apply();
    }
  };

  #apply() {
    if (typeof document === "undefined") return;

    const resolved =
      this.preference === "system"
        ? this.#mediaQuery?.matches
          ? "dark"
          : "light"
        : this.preference;

    this.resolved = resolved;
    document.documentElement.classList.toggle("dark", resolved === "dark");
    document.documentElement.dataset.theme = this.preference;
    document.documentElement.style.colorScheme = resolved;
  }
}

export const theme = new ThemeController();
