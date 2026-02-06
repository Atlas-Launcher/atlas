import type { Updater } from "@tanstack/vue-table"
import type { ClassValue } from "clsx"
import type { Ref } from "vue"
import { clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function valueUpdater<T extends Updater<any>>(updaterOrValue: T, ref: Ref) {
  ref.value
    = typeof updaterOrValue === "function"
      ? updaterOrValue(ref.value)
      : updaterOrValue
}

// Add a small utility that prunes a path so the user's home/profile
// portion is replaced with "~" for display purposes. Handles common
// Unix (macOS/Linux) home prefixes (/Users/<name>, /home/<name>) and
// Windows user profiles (C:\\Users\\<name>).
export function pruneHomePath(path?: string | null): string {
  if (!path) return ""

  // Keep the original for separator preservation
  const original = String(path)

  // Windows: drive-letter style e.g. C:\Users\alice\... or D:\Users\bob
  const winRegex = /^([a-zA-Z]:\\Users\\[^\\/]+)([\\/].*)?$/
  const winMatch = original.match(winRegex)
  if (winMatch) {
    const rest = winMatch[2] ?? ""
    // Return ~ + rest (preserve backslashes)
    return `~${rest}`
  }

  // Unix-like: /Users/alice/... or /home/alice/...
  const unixRegex = /^(\/(?:Users|home)\/[^\/]+)(\/.*)?$/
  const unixMatch = original.match(unixRegex)
  if (unixMatch) {
    const rest = unixMatch[2] ?? ""
    return `~${rest}`
  }

  // No recognizable home/profile prefix: return unchanged
  return original
}

export function formatLoaderKind(loader?: string | null): string {
  const normalized = (loader ?? "").trim().toLowerCase()
  if (normalized === "fabric") {
    return "Fabric"
  }
  if (normalized === "neoforge" || normalized === "neo") {
    return "NeoForge"
  }
  return "Vanilla"
}
