export const playerCopy = {
  auth: {
    signInAtlas: "Sign in to your Atlas account to continue.",
    signInMicrosoft: "Sign in with Microsoft to continue.",
    linkAccounts: "Link your Atlas and Microsoft accounts to launch safely."
  },
  statuses: {
    preparing: "Preparing your game setup...",
    ready: "Ready to play.",
    noTasks: "No active tasks right now."
  },
  actions: {
    openLauncher: "Open Atlas Launcher",
    openAssist: "Open Launch Assist",
    retry: "Try again"
  },
  fallback: {
    noProfiles: "No profiles available yet. Sync packs or create a local profile.",
    noMods: "No mods added yet.",
    launcherNotDetected:
      "Atlas Launcher did not open automatically. Install it, then try again."
  }
} as const;

export type PlayerCopy = typeof playerCopy;
