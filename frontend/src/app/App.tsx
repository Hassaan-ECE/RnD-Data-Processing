import { useCallback, useEffect, useState, type ReactNode } from "react";

import { HubPage } from "../features/hub/HubPage";
import { cloneComparisonGradients } from "../features/processor/gradientConfig";
import { ProcessorPage } from "../features/processor/ProcessorPage";
import { processorTest, type ProcessorTestId } from "../features/processor/testCatalog";
import { UpdateActionButton } from "../features/updates/UpdateActionButton";
import { useDesktopUpdates } from "../features/updates/useDesktopUpdates";
import {
  getAppVersion,
  isTauriRuntime,
  type ComparisonGradientOptions,
} from "../integrations/tauri/backend";
import { ScrollRegion } from "../shared/ui/ScrollRegion";

const SETUP_STORAGE_KEY = "rnd-data-processing.setup-path";

export function App() {
  const [activeTestId, setActiveTestId] = useState<ProcessorTestId | null>(null);
  const [gradientClipboard, setGradientClipboard] = useState<ComparisonGradientOptions | null>(null);
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

  const updateControl: ReactNode = updates.visible ? (
    <UpdateActionButton state={updates.state} onClick={() => void updates.runAction()} />
  ) : null;

  return (
    <div className="app-shell">
      <span className="sr-only" aria-live="polite">
        {announcement}
      </span>
      <main className={`app-main ${activeTestId ? "app-main-fill" : ""}`}>
        {!activeTestId ? (
          <ScrollRegion className="app-scroll" aria-label="Application content">
            <HubPage
              setupPath={setupPath}
              onSetupPathChange={setSetupPath}
              onOpenTest={setActiveTestId}
              announce={announce}
              updateControl={updateControl}
            />
          </ScrollRegion>
        ) : (
          // Processor owns two independent scroll panes (main + load-range rail).
          <ProcessorPage
            key={activeTestId}
            test={processorTest(activeTestId)}
            setupPath={setupPath}
            onSetupPathChange={setSetupPath}
            gradientClipboard={gradientClipboard}
            onCopyGradients={(nextGradients) =>
              setGradientClipboard(cloneComparisonGradients(nextGradients))
            }
            onBack={() => setActiveTestId(null)}
            announce={announce}
          />
        )}
      </main>
    </div>
  );
}
