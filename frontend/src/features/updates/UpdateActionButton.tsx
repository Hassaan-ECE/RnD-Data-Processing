import { LoaderCircle, RefreshCw } from "lucide-react";

import { updateButtonLabel, type UpdateState } from "./useDesktopUpdates";

interface UpdateActionButtonProps {
  state: UpdateState;
  onClick: () => void;
}

export function UpdateActionButton({ state, onClick }: UpdateActionButtonProps) {
  const busy = state.status === "checking" || state.status === "downloading" || state.status === "installing";

  return (
    <button className="update-button" type="button" onClick={onClick} disabled={busy}>
      {busy ? <LoaderCircle className="spin" /> : <RefreshCw />}
      {updateButtonLabel(state)}
    </button>
  );
}
