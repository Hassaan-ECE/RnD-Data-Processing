import { useCallback, useEffect, useRef, useState } from "react";
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

export function useDesktopUpdates(announce: (message: string) => void) {
  const [state, setState] = useState<UpdateState>({ status: "idle" });
  const pendingUpdate = useRef<Update | null>(null);

  useEffect(
    () => () => {
      pendingUpdate.current?.close().catch(() => undefined);
    },
    [],
  );

  const runAction = useCallback(async () => {
    if (!isTauriRuntime()) {
      const message = "Updates are available from the installed desktop app.";
      setState({ status: "idle", message });
      announce(message);
      return;
    }
    if (["checking", "downloading", "installing"].includes(state.status)) {
      return;
    }

    try {
      if (state.status === "ready" && pendingUpdate.current) {
        setState((current) => ({ ...current, status: "installing" }));
        await pendingUpdate.current.install();
        announce("Update installation started. The app may restart or close briefly.");
        return;
      }

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
        setState((current) => ({ ...current, status: "ready", progress: 100 }));
        announce("Update downloaded and ready to install.");
        return;
      }

      setState({ status: "checking" });
      const update = await check();
      pendingUpdate.current?.close().catch(() => undefined);
      pendingUpdate.current = update;
      if (!update) {
        setState({ status: "current", message: "You are up to date." });
        announce("Data Processing is up to date.");
        return;
      }
      setState({ status: "available", latestVersion: update.version });
      announce(`Version ${update.version} is available.`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Update check failed.";
      setState({ status: "error", message });
      announce(message);
    }
  }, [announce, state.status]);

  return { runAction, state };
}

export function updateButtonLabel(state: UpdateState): string {
  switch (state.status) {
    case "checking":
      return "Checking...";
    case "current":
      return "Up to date";
    case "available":
      return state.latestVersion ? `Download ${state.latestVersion}` : "Download update";
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
