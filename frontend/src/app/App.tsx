import { useCallback, useEffect, useState, type ReactNode } from "react";

import { HubPage } from "../features/hub/HubPage";
import { ProcessorPage } from "../features/processor/ProcessorPage";
import { UpdateActionButton } from "../features/updates/UpdateActionButton";
import { useDesktopUpdates } from "../features/updates/useDesktopUpdates";
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

  const updateControl: ReactNode = (
    <UpdateActionButton state={updates.state} onClick={() => void updates.runAction()} />
  );

  return (
    <div className="app-shell">
      <span className="sr-only" aria-live="polite">
        {announcement}
      </span>
      <main className="app-main">
        {page === "hub" ? (
          <HubPage
            setupPath={setupPath}
            onSetupPathChange={setSetupPath}
            onOpenSystem208v={() => setPage("system_208v")}
            announce={announce}
            updateControl={updateControl}
          />
        ) : (
          <ProcessorPage
            setupPath={setupPath}
            onSetupPathChange={setSetupPath}
            onBack={() => setPage("hub")}
            announce={announce}
            updateControl={updateControl}
          />
        )}
      </main>
    </div>
  );
}
