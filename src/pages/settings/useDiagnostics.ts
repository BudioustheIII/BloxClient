import { useEffect, useState } from "react";
import type { Diagnostics } from "@/lib/bindings/Diagnostics";
import { api, IpcFailure } from "@/lib/ipc";

export function useDiagnostics() {
  const [diagnostics, setDiagnostics] = useState<Diagnostics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    api
      .diagnostics()
      .then((value) => {
        if (active) setDiagnostics(value);
      })
      .catch((reason: unknown) => {
        if (active) {
          setError(reason instanceof IpcFailure ? reason.message : "Diagnostics couldn't be loaded.");
        }
      });
    return () => {
      active = false;
    };
  }, []);

  return { diagnostics, error };
}
