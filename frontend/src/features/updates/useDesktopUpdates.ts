import { useCallback, useEffect, useRef, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

import { isTauriRuntime } from "../../integrations/tauri/backend";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "error";

export interface UpdateState {
  status: UpdateStatus;
  latestVersion?: string;
  progress?: number;
  message?: string;
}

/** Visible only while an update needs attention (not idle / up-to-date). */
export function isUpdateButtonVisible(state: UpdateState): boolean {
  return (
    state.status === "available" ||
    state.status === "downloading" ||
    state.status === "ready" ||
    state.status === "installing" ||
    state.status === "error"
  );
}

export function useDesktopUpdates(announce: (message: string) => void) {
  const [state, setState] = useState<UpdateState>({ status: "idle" });
  const pendingUpdate = useRef<Update | null>(null);

  useEffect(
    () => () => {
      pendingUpdate.current?.close().catch(() => undefined);
    },
    [],
  );

  // Check once on launch — button appears only if a newer version is published.
  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    let active = true;
    void (async () => {
      try {
        const update = await check();
        if (!active) {
          return;
        }
        pendingUpdate.current?.close().catch(() => undefined);
        pendingUpdate.current = update;
        if (!update) {
          setState({ status: "current", message: "You are up to date." });
          return;
        }
        setState({
          status: "available",
          latestVersion: update.version,
          message: `Version ${update.version} is available.`,
        });
        announce(`Update available: ${update.version}`);
      } catch {
        // Silent on launch — app stays usable if check fails (offline, no release, etc.).
        if (active) {
          setState({ status: "idle" });
        }
      }
    })();
    return () => {
      active = false;
    };
  }, [announce]);

  const runAction = useCallback(async () => {
    if (!isTauriRuntime()) {
      announce("Updates are available from the installed desktop app.");
      return;
    }
    if (["checking", "downloading", "installing"].includes(state.status)) {
      return;
    }

    try {
      // Install → restart so the new version is running.
      if (state.status === "ready" && pendingUpdate.current) {
        setState((current) => ({ ...current, status: "installing" }));
        announce("Installing update… the app will restart.");
        await pendingUpdate.current.install();
        await relaunch();
        return;
      }

      // Available → download; next click installs.
      if (state.status === "available" && pendingUpdate.current) {
        let totalBytes = 0;
        let downloadedBytes = 0;
        setState((current) => ({ ...current, status: "downloading", progress: 0 }));
        await pendingUpdate.current.download((event: DownloadEvent) => {
          if (event.event === "Started") {
            totalBytes = event.data.contentLength ?? 0;
            downloadedBytes = 0;
          } else if (event.event === "Progress") {
            downloadedBytes += event.data.chunkLength;
          }
          const progress = totalBytes > 0 ? Math.min(99, Math.round((downloadedBytes / totalBytes) * 100)) : undefined;
          setState((current) => ({ ...current, status: "downloading", progress }));
        });
        setState((current) => ({
          ...current,
          status: "ready",
          progress: 100,
          message: "Update downloaded. Click to install and restart.",
        }));
        announce("Update ready to install. The app will restart after install.");
        return;
      }

      // Error / retry: re-check for an update.
      setState({ status: "checking" });
      const update = await check();
      pendingUpdate.current?.close().catch(() => undefined);
      pendingUpdate.current = update;
      if (!update) {
        setState({ status: "current", message: "You are up to date." });
        announce("Data Processing is up to date.");
        return;
      }
      setState({
        status: "available",
        latestVersion: update.version,
        message: `Version ${update.version} is available.`,
      });
      announce(`Update available: ${update.version}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Update check failed.";
      setState({ status: "error", message });
      announce(message);
    }
  }, [announce, state.status]);

  return {
    runAction,
    state,
    visible: isUpdateButtonVisible(state),
  };
}

export function updateButtonLabel(state: UpdateState): string {
  switch (state.status) {
    case "checking":
      return "Checking...";
    case "current":
      return "Up to date";
    case "available":
      return "Update available";
    case "downloading":
      return typeof state.progress === "number" ? `Downloading ${state.progress}%` : "Downloading...";
    case "ready":
      return "Install update";
    case "installing":
      return "Installing...";
    case "error":
      return "Retry update";
    default:
      return "Check for updates";
  }
}
