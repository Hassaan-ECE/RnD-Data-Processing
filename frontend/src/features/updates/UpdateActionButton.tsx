import { Download, LoaderCircle, RefreshCw, RotateCw } from "lucide-react";

import { updateButtonLabel, type UpdateState } from "./useDesktopUpdates";

interface UpdateActionButtonProps {
  state: UpdateState;
  onClick: () => void;
}

export function UpdateActionButton({ state, onClick }: UpdateActionButtonProps) {
  const busy = state.status === "checking" || state.status === "downloading" || state.status === "installing";
  const emphasis =
    state.status === "available" ||
    state.status === "ready" ||
    state.status === "downloading" ||
    state.status === "installing";

  let icon = <RefreshCw />;
  if (busy) {
    icon = <LoaderCircle className="spin" />;
  } else if (state.status === "available") {
    icon = <Download />;
  } else if (state.status === "ready") {
    icon = <RotateCw />;
  }

  return (
    <button
      className={`update-button${emphasis ? " update-button-emphasis" : ""}`}
      type="button"
      onClick={onClick}
      disabled={busy}
      title={state.message}
    >
      {icon}
      {updateButtonLabel(state)}
    </button>
  );
}
