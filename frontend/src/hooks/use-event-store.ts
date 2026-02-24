import { useCallback, useMemo, useRef, useSyncExternalStore } from "react";
import type {
  InspectorStats,
  RequestEvent,
  WsServerMessage,
} from "@/lib/types";

export function useEventStore() {
  const eventsRef = useRef(new Map<number, RequestEvent>());
  const versionRef = useRef(0);
  const listenersRef = useRef(new Set<() => void>());

  const subscribe = useCallback((listener: () => void) => {
    listenersRef.current.add(listener);
    return () => {
      listenersRef.current.delete(listener);
    };
  }, []);

  const getVersion = useCallback(() => versionRef.current, []);

  const version = useSyncExternalStore(subscribe, getVersion, getVersion);

  const notify = useCallback(() => {
    versionRef.current++;
    for (const listener of listenersRef.current) {
      listener();
    }
  }, []);

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

  const clear = useCallback(() => {
    eventsRef.current.clear();
    notify();
  }, [notify]);

  const events = useMemo((): RequestEvent[] => {
    // Force re-compute when version changes
    void version;
    return Array.from(eventsRef.current.values()).sort(
      (a, b) => b.id - a.id,
    );
  }, [version]);

  const stats = useMemo((): InspectorStats => {
    void version;
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
  }, [version]);

  return { events, stats, handleMessage, clear };
}
