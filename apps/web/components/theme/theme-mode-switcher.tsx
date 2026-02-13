"use client";

import { useCallback, useEffect, useState } from "react";
import { LaptopMinimal, Moon, Sun } from "lucide-react";

import { cn } from "@/lib/utils";
import {
  DEFAULT_THEME_MODE,
  resolveThemeIsDark,
  sanitizeThemeMode,
  THEME_MODE_STORAGE_KEY,
  type ThemeMode,
} from "@/lib/theme/theme-mode";

const themeOptions: Array<{ mode: ThemeMode; label: string; icon: typeof Sun }> = [
  { mode: "light", label: "Light", icon: Sun },
  { mode: "dark", label: "Dark", icon: Moon },
  { mode: "system", label: "System", icon: LaptopMinimal },
];

type ThemeModeSwitcherProps = {
  className?: string;
};

export default function ThemeModeSwitcher({ className }: ThemeModeSwitcherProps) {
  const [mode, setMode] = useState<ThemeMode>(() => {
    if (typeof window === "undefined") {
      return DEFAULT_THEME_MODE;
    }

    return sanitizeThemeMode(window.localStorage.getItem(THEME_MODE_STORAGE_KEY));
  });

  const applyTheme = useCallback((nextMode: ThemeMode) => {
    const root = document.documentElement;
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const isDark = resolveThemeIsDark(nextMode, prefersDark);

    root.classList.toggle("dark", isDark);
    root.dataset.themeMode = nextMode;
  }, []);

  useEffect(() => {
    applyTheme(mode);
  }, [applyTheme, mode]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");

    const onChange = () => {
      if (mode === "system") {
        applyTheme("system");
      }
    };

    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [applyTheme, mode]);

  const updateMode = (nextMode: ThemeMode) => {
    setMode(nextMode);
    window.localStorage.setItem(THEME_MODE_STORAGE_KEY, nextMode);
    applyTheme(nextMode);
  };

  return (
    <div
      className={cn(
        "atlas-glass inline-flex items-center gap-1 rounded-full p-1",
        className
      )}
      role="group"
      aria-label="Theme mode"
    >
      {themeOptions.map((option) => {
        const Icon = option.icon;
        const isActive = mode === option.mode;

        return (
          <button
            key={option.mode}
            type="button"
            onClick={() => updateMode(option.mode)}
            aria-pressed={isActive}
            title={option.label}
            className={cn(
              "inline-flex h-8 w-8 items-center justify-center rounded-full border text-[var(--atlas-ink-muted)] transition",
              isActive
                ? "border-[hsl(var(--primary)/0.25)] bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))] shadow-[var(--atlas-shadow-button)]"
                : "border-transparent hover:border-[hsl(var(--border)/0.7)] hover:bg-[var(--atlas-surface-strong)] hover:text-[var(--atlas-ink)]"
            )}
          >
            <Icon className="h-4 w-4" />
            <span className="sr-only">{option.label} theme</span>
          </button>
        );
      })}
    </div>
  );
}
