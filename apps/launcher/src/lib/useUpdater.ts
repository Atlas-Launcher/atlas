import { computed, markRaw, ref, shallowRef } from "vue";
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
  const bannerDismissed = ref(false);
  const errorMessage = ref<string | null>(null);
  const lastCheckedAt = ref<string | null>(null);
  const downloadedBytes = ref(0);
  const totalBytes = ref<number | null>(null);
  const speedBytesPerSecond = ref<number | null>(null);
  const etaSeconds = ref<number | null>(null);
  const lastProgressAt = ref<number | null>(null);
  const lastProgressBytes = ref(0);
  const updateHandle = shallowRef<Update | null>(null);

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
    const now = Date.now();
    if (event.event === "Started") {
      downloadedBytes.value = 0;
      totalBytes.value = event.data.contentLength ?? null;
      speedBytesPerSecond.value = null;
      etaSeconds.value = null;
      lastProgressAt.value = now;
      lastProgressBytes.value = 0;
      return;
    }
    if (event.event === "Progress") {
      downloadedBytes.value += event.data.chunkLength;
      if (lastProgressAt.value) {
        const elapsedMs = Math.max(1, now - lastProgressAt.value);
        const deltaBytes = Math.max(0, downloadedBytes.value - lastProgressBytes.value);
        const instantBps = (deltaBytes * 1000) / elapsedMs;
        speedBytesPerSecond.value =
          speedBytesPerSecond.value == null
            ? instantBps
            : speedBytesPerSecond.value * 0.7 + instantBps * 0.3;
      }
      if (totalBytes.value && speedBytesPerSecond.value && speedBytesPerSecond.value > 0) {
        const remainingBytes = Math.max(0, totalBytes.value - downloadedBytes.value);
        etaSeconds.value = Math.ceil(remainingBytes / speedBytesPerSecond.value);
      } else {
        etaSeconds.value = null;
      }
      lastProgressAt.value = now;
      lastProgressBytes.value = downloadedBytes.value;
      return;
    }
    if (event.event === "Finished") {
      if (totalBytes.value) {
        downloadedBytes.value = totalBytes.value;
      }
      etaSeconds.value = 0;
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
          setStatus("App is up to date.");
        }
        pushLog("No update available.");
        return false;
      }

      await releaseCurrentHandle();
      updateHandle.value = markRaw(result);
      installComplete.value = false;
      bannerDismissed.value = false;
      if (options?.userInitiated) {
        setStatus(`Update ${result.version} is available.`);
      }
      setStatus(`Update ${result.version} is available.`);
      pushLog(`Update available: ${result.currentVersion} -> ${result.version}`);
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
    if (!updateHandle.value) {
      const message = "No pending update is available to install.";
      errorMessage.value = message;
      setStatus(message);
      pushLog(message);
      return false;
    }
    if (updaterBusy.value) {
      const message = "Updater is already busy. Please wait for the current task to finish.";
      errorMessage.value = message;
      setStatus(message);
      pushLog(message);
      return false;
    }
    installing.value = true;
    errorMessage.value = null;
    downloadedBytes.value = 0;
    totalBytes.value = null;
    speedBytesPerSecond.value = null;
    etaSeconds.value = null;
    lastProgressAt.value = null;
    lastProgressBytes.value = 0;
    setStatus(`Downloading update ${updateHandle.value.version}...`);
    try {
      await updateHandle.value.downloadAndInstall(handleDownloadEvent);
      installComplete.value = true;
      bannerDismissed.value = false;
      setStatus("Update installed. Relaunch to finish.");
      pushLog(`Update installed: ${updateHandle.value.version}`);
      return true;
    } catch (err) {
      const rawError = String(err);
      const hasSignatureError =
        rawError.toLowerCase().includes("signature") || rawError.toLowerCase().includes("verify");
      const message = hasSignatureError
        ? "Failed to verify update signature. Ensure release signing keys match the app public key."
        : `Failed to install update: ${rawError}`;
      errorMessage.value = message;
      setStatus(message);
      pushLog(`Updater install failed: ${rawError}`);
      return false;
    } finally {
      installing.value = false;
    }
  }

  async function restartNow() {
    try {
      setStatus("Relaunching...");
      await invoke("restart_app");
      return true;
    } catch (err) {
      const message = `Failed to relaunch app: ${String(err)}`;
      errorMessage.value = message;
      setStatus(message);
      pushLog(message);
      return false;
    }
  }

  function dismissBanner() {
    bannerDismissed.value = true;
  }

  return {
    checking,
    installing,
    installComplete,
    showBanner,
    updaterBusy,
    hasUpdate,
    updateInfo,
    statusText,
    errorMessage,
    lastCheckedAt,
    downloadedBytes,
    totalBytes,
    speedBytesPerSecond,
    etaSeconds,
    progressPercent,
    checkForUpdates,
    installUpdate,
    restartNow,
    dismissBanner
  };
}
