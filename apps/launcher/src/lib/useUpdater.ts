import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

interface UpdaterDeps {
  setStatus: (message: string) => void;
  pushLog: (entry: string) => void;
}

export interface ReleaseInfo {
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
}

interface CheckOptions {
  userInitiated?: boolean;
}

export function useUpdater({ setStatus, pushLog }: UpdaterDeps) {
  const checking = ref(false);
  const installing = ref(false);
  const installComplete = ref(false);
  const dialogOpen = ref(false);
  const bannerDismissed = ref(false);
  const errorMessage = ref<string | null>(null);
  const lastCheckedAt = ref<string | null>(null);
  const downloadedBytes = ref(0);
  const totalBytes = ref<number | null>(null);
  const updateHandle = ref<Update | null>(null);

  const updateInfo = computed<ReleaseInfo | null>(() => {
    if (!updateHandle.value) {
      return null;
    }
    return {
      currentVersion: updateHandle.value.currentVersion,
      version: updateHandle.value.version,
      date: updateHandle.value.date,
      body: updateHandle.value.body
    };
  });

  const hasUpdate = computed(() => !!updateHandle.value);
  const progressPercent = computed(() => {
    if (!installing.value) {
      return installComplete.value ? 100 : 0;
    }
    if (!totalBytes.value || totalBytes.value <= 0) {
      return 0;
    }
    const ratio = downloadedBytes.value / totalBytes.value;
    return Math.max(0, Math.min(100, Math.round(ratio * 100)));
  });
  const showBanner = computed(() => hasUpdate.value && !bannerDismissed.value);
  const updaterBusy = computed(() => checking.value || installing.value);
  const statusText = computed(() => {
    if (installComplete.value) {
      return "Update installed. Restart required.";
    }
    if (installing.value) {
      if (!totalBytes.value) {
        return "Downloading update...";
      }
      return `Downloading update... ${progressPercent.value}%`;
    }
    if (checking.value) {
      return "Checking for updates...";
    }
    if (hasUpdate.value && updateInfo.value) {
      return `Update ${updateInfo.value.version} is available.`;
    }
    if (errorMessage.value) {
      return errorMessage.value;
    }
    if (lastCheckedAt.value) {
      return `Last checked ${new Date(lastCheckedAt.value).toLocaleString()}.`;
    }
    return "No update check has run yet.";
  });

  function handleDownloadEvent(event: DownloadEvent) {
    if (event.event === "Started") {
      downloadedBytes.value = 0;
      totalBytes.value = event.data.contentLength ?? null;
      return;
    }
    if (event.event === "Progress") {
      downloadedBytes.value += event.data.chunkLength;
      return;
    }
    if (event.event === "Finished") {
      if (totalBytes.value) {
        downloadedBytes.value = totalBytes.value;
      }
    }
  }

  async function releaseCurrentHandle() {
    if (!updateHandle.value) {
      return;
    }
    try {
      await updateHandle.value.close();
    } catch {
      // Ignore resource cleanup errors.
    }
    updateHandle.value = null;
  }

  async function checkForUpdates(options?: CheckOptions) {
    if (updaterBusy.value) {
      return false;
    }
    checking.value = true;
    errorMessage.value = null;
    try {
      const result = await check();
      lastCheckedAt.value = new Date().toISOString();
      if (!result) {
        installComplete.value = false;
        await releaseCurrentHandle();
        if (options?.userInitiated) {
          setStatus("Atlas Launcher is up to date.");
        }
        pushLog("No launcher update available.");
        return false;
      }

      await releaseCurrentHandle();
      updateHandle.value = result;
      installComplete.value = false;
      bannerDismissed.value = false;
      if (options?.userInitiated) {
        dialogOpen.value = true;
      }
      setStatus(`Launcher update ${result.version} is available.`);
      pushLog(`Launcher update available: ${result.currentVersion} -> ${result.version}`);
      return true;
    } catch (err) {
      const rawError = String(err);
      const isReleaseJsonError = rawError.includes("Could not fetch a valid release JSON");
      const isNetworkError =
        rawError.includes("Network") ||
        rawError.includes("fetch") ||
        rawError.includes("timed out") ||
        rawError.includes("dns");
      const message = isReleaseJsonError || isNetworkError
        ? "Update service is temporarily unavailable. Try again later."
        : `Failed to check for updates: ${rawError}`;
      errorMessage.value = message;
      setStatus(message);
      pushLog(`Updater check failed: ${rawError}`);
      return false;
    } finally {
      checking.value = false;
    }
  }

  async function installUpdate() {
    if (!updateHandle.value || updaterBusy.value) {
      return false;
    }
    installing.value = true;
    errorMessage.value = null;
    downloadedBytes.value = 0;
    totalBytes.value = null;
    try {
      await updateHandle.value.downloadAndInstall(handleDownloadEvent);
      installComplete.value = true;
      bannerDismissed.value = false;
      dialogOpen.value = true;
      setStatus("Update installed. Restart Atlas Launcher to finish.");
      pushLog(`Launcher update installed: ${updateHandle.value.version}`);
      return true;
    } catch (err) {
      const message = `Failed to install update: ${String(err)}`;
      errorMessage.value = message;
      setStatus(message);
      pushLog(message);
      return false;
    } finally {
      installing.value = false;
    }
  }

  async function restartNow() {
    try {
      setStatus("Restarting Atlas Launcher...");
      await invoke("restart_app");
      return true;
    } catch (err) {
      const message = `Failed to restart launcher: ${String(err)}`;
      errorMessage.value = message;
      setStatus(message);
      pushLog(message);
      return false;
    }
  }

  function openDialog() {
    dialogOpen.value = true;
  }

  function closeDialog() {
    dialogOpen.value = false;
  }

  function dismissBanner() {
    bannerDismissed.value = true;
  }

  return {
    checking,
    installing,
    installComplete,
    dialogOpen,
    showBanner,
    updaterBusy,
    hasUpdate,
    updateInfo,
    statusText,
    errorMessage,
    lastCheckedAt,
    downloadedBytes,
    totalBytes,
    progressPercent,
    checkForUpdates,
    installUpdate,
    restartNow,
    openDialog,
    closeDialog,
    dismissBanner
  };
}
