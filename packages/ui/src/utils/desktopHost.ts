import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type ProjectSummary = {
  id: string;
  name: string;
  status: string;
  phase: string;
};

export type DesktopHostState =
  | { status: "browser" }
  | { status: "loading" }
  | { status: "connected"; project: ProjectSummary }
  | { status: "error"; message: string };

export function useDesktopHostProject(): DesktopHostState {
  const [state, setState] = useState<DesktopHostState>(() => {
    if (!isTauriHost()) {
      return { status: "browser" };
    }

    return { status: "loading" };
  });

  useEffect(() => {
    if (!isTauriHost()) {
      return;
    }

    let mounted = true;

    invoke<ProjectSummary>("get_project")
      .then((project) => {
        if (mounted) {
          setState({ status: "connected", project });
        }
      })
      .catch((error: unknown) => {
        if (mounted) {
          setState({ status: "error", message: String(error) });
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  return state;
}

function isTauriHost(): boolean {
  return "__TAURI_INTERNALS__" in window;
}
