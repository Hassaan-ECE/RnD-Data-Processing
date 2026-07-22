import { useCallback, useEffect, useState } from "react";
import { LoaderCircle, RefreshCw } from "lucide-react";

import { HubPage } from "../features/hub/HubPage";
import { ProcessorPage } from "../features/processor/ProcessorPage";
import { updateButtonLabel, useDesktopUpdates } from "../features/updates/useDesktopUpdates";
import { getAppVersion, isTauriRuntime } from "../integrations/tauri/backend";

const SETUP_STORAGE_KEY = "rnd-data-processing.setup-path";

export function App() {
  const [page, setPage] = useState<"hub" | "system_208v">("hub");
  const [setupPath, setSetupPath] = useState(() => window.localStorage.getItem(SETUP_STORAGE_KEY) ?? "");
  const [announcement, setAnnouncement] = useState("Ready");
  const announce = useCallback((message: string) => setAnnouncement(message), []);
  const updates = useDesktopUpdates(announce);

  useEffect(() => {
    let active = true;
    getAppVersion()
      .then(async (version) => {
        if (!active) {
          return;
        }
        const title = `Data Processing v${version}`;
        if (isTauriRuntime()) {
          try {
            const { getCurrentWindow } = await import("@tauri-apps/api/window");
            await getCurrentWindow().setTitle(title);
          } catch {
            // Browser-only preview keeps the static HTML title.
          }
        } else {
          document.title = title;
        }
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (setupPath) {
      window.localStorage.setItem(SETUP_STORAGE_KEY, setupPath);
    } else {
      window.localStorage.removeItem(SETUP_STORAGE_KEY);
    }
  }, [setupPath]);

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="announcement" aria-live="polite" title={announcement}>
          {announcement}
        </span>
        <button
          className="update-button"
          type="button"
          onClick={updates.runAction}
          disabled={["checking", "downloading", "installing"].includes(updates.state.status)}
        >
          {updates.state.status === "checking" ||
          updates.state.status === "downloading" ||
          updates.state.status === "installing" ? (
            <LoaderCircle className="spin" />
          ) : (
            <RefreshCw />
          )}
          {updateButtonLabel(updates.state)}
        </button>
      </header>
      <main className="app-main">
        {page === "hub" ? (
          <HubPage
            setupPath={setupPath}
            onSetupPathChange={setSetupPath}
            onOpenSystem208v={() => setPage("system_208v")}
            announce={announce}
          />
        ) : (
          <ProcessorPage
            setupPath={setupPath}
            onSetupPathChange={setSetupPath}
            onBack={() => setPage("hub")}
            announce={announce}
          />
        )}
      </main>
    </div>
  );
}
