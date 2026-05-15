// Manages the queue of outstanding MCP prompts. Subscribes to the Tauri
// channel event `teraxlyst:mcp-prompt` and exposes the next prompt to the
// dialog component. On submit, invokes the `mcp_prompt_response` command.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { EVENT_PROMPT, type PromptRequest } from "./types";

export function useMcpPrompts() {
  const [queue, setQueue] = useState<PromptRequest[]>([]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let active = true;

    (async () => {
      // Rehydrate anything still outstanding from before the renderer
      // reloaded. The Rust side keeps state in the pipeline; we ask for it.
      try {
        const pending = (await invoke("mcp_list_pending_prompts")) as PromptRequest[];
        if (active && pending.length > 0) setQueue(pending);
      } catch {
        // No host yet (early bootstrap) — silently ignore.
      }

      unlisten = await listen<PromptRequest>(EVENT_PROMPT, (event) => {
        if (!active) return;
        setQueue((prev) => [...prev, event.payload]);
      });
    })();

    return () => {
      active = false;
      if (unlisten) unlisten();
    };
  }, []);

  const current = queue[0] ?? null;

  const submit = async (id: string, value: unknown) => {
    try {
      await invoke("mcp_prompt_response", { id, value });
    } finally {
      setQueue((prev) => prev.filter((p) => p.id !== id));
    }
  };

  return { current, submit };
}
