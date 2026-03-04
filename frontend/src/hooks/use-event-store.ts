import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";
import type {
  InspectorStats,
  RequestEvent,
  WsServerMessage,
} from "@/lib/types";

export function useEventStore() {
  const eventsRef = useRef(new Map<number, RequestEvent>());
  const [tick, bump] = useReducer((n: number) => n + 1, 0);
  const rafRef = useRef<ReturnType<typeof requestAnimationFrame> | null>(null);

  // Cancel any pending rAF on unmount to avoid calling dispatch after unmount.
  useEffect(() => {
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  // Rate-limited notify: coalesces rapid WebSocket messages into one render
  // per animation frame (~16 ms), preventing the useSyncExternalStore tearing
  // loop that froze the browser when events arrived faster than React could render.
  const notify = useCallback(() => {
    if (rafRef.current !== null) return;
    rafRef.current = requestAnimationFrame(() => {
      rafRef.current = null;
      bump();
    });
  }, [bump]);

  const handleMessage = useCallback(
    (msg: WsServerMessage) => {
      switch (msg.type) {
        case "snapshot":
          eventsRef.current.clear();
          for (const event of msg.events) {
            eventsRef.current.set(event.id, event);
          }
          notify();
          break;

        case "request_started":
          eventsRef.current.set(msg.event.id, msg.event);
          notify();
          break;

        case "request_completed": {
          const existing = eventsRef.current.get(msg.id);
          if (existing) {
            eventsRef.current.set(msg.id, {
              ...existing,
              response: msg.response,
              duration_ms: msg.duration_ms,
            });
            notify();
          }
          break;
        }

        case "cleared":
          eventsRef.current.clear();
          notify();
          break;
      }
    },
    [notify],
  );

  // Clear is called directly by the user action: skip the rAF and render
  // immediately so the table empties without a visible delay.
  const clear = useCallback(() => {
    eventsRef.current.clear();
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    bump();
  }, [bump]);

  const events = useMemo((): RequestEvent[] => {
    void tick;
    return Array.from(eventsRef.current.values()).sort(
      (a, b) => b.id - a.id,
    );
  }, [tick]);

  const stats = useMemo((): InspectorStats => {
    void tick;
    const all = Array.from(eventsRef.current.values());
    const total = all.length;
    const errors = all.filter(
      (e) => e.response && e.response.status >= 400,
    ).length;
    const withDuration = all.filter((e) => e.duration_ms !== null);
    const avgLatency =
      withDuration.length > 0
        ? withDuration.reduce((s, e) => s + e.duration_ms!, 0) /
          withDuration.length
        : 0;
    return {
      total,
      errorRate: total > 0 ? (errors / total) * 100 : 0,
      avgLatency,
    };
  }, [tick]);

  return { events, stats, handleMessage, clear };
}
