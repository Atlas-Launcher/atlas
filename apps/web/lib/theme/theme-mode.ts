export const THEME_MODE_STORAGE_KEY = "atlas-theme-mode";

export const THEME_MODES = ["light", "dark", "system"] as const;

export type ThemeMode = (typeof THEME_MODES)[number];

export const DEFAULT_THEME_MODE: ThemeMode = "system";

export function sanitizeThemeMode(value: string | null | undefined): ThemeMode {
  if (value === "light" || value === "dark" || value === "system") {
    return value;
  }

  return DEFAULT_THEME_MODE;
}

export function resolveThemeIsDark(mode: ThemeMode, systemPrefersDark: boolean) {
  return mode === "dark" || (mode === "system" && systemPrefersDark);
}
