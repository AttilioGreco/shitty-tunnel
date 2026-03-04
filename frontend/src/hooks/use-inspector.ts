import { useCallback } from "react";
import { useEventStore } from "./use-event-store";
import { useWebSocket } from "./use-websocket";
import type { WsServerMessage } from "@/lib/types";

export function useInspector() {
  const { events, stats, handleMessage, clear: clearStore } = useEventStore();

  const onMessage = useCallback(
    (data: unknown) => {
      handleMessage(data as WsServerMessage);
    },
    [handleMessage],
  );

  const wsUrl = `ws://${typeof window !== "undefined" ? window.location.host : "localhost:3001"}/api/ws`;

  const { connected, send } = useWebSocket({
    url: wsUrl,
    onMessage,
  });

  const clear = useCallback(() => {
    clearStore(); // immediate local clear (skips rAF, renders instantly)
    send({ type: "clear" }); // server clears its buffer and confirms via "cleared"
  }, [send, clearStore]);

  return { events, stats, connected, clear };
}
