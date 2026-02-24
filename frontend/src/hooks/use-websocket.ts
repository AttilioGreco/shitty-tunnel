import { useCallback, useEffect, useRef, useState } from "react";

interface UseWebSocketOptions {
  url: string;
  onMessage: (data: unknown) => void;
}

export function useWebSocket({ url, onMessage }: UseWebSocketOptions) {
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const onMessageRef = useRef(onMessage);
  onMessageRef.current = onMessage;

  const reconnectDelay = useRef(1000);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;

    function connect() {
      if (cancelled) return;
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
        reconnectDelay.current = 1000;
      };

      ws.onclose = () => {
        setConnected(false);
        wsRef.current = null;
        if (!cancelled) {
          timer = setTimeout(() => {
            reconnectDelay.current = Math.min(
              reconnectDelay.current * 2,
              30000,
            );
            connect();
          }, reconnectDelay.current);
        }
      };

      ws.onerror = () => {
        ws.close();
      };

      ws.onmessage = (ev) => {
        try {
          const data = JSON.parse(ev.data);
          onMessageRef.current(data);
        } catch {
          // ignore non-JSON
        }
      };
    }

    connect();

    return () => {
      cancelled = true;
      clearTimeout(timer);
      wsRef.current?.close();
    };
  }, [url]);

  const send = useCallback((msg: unknown) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  return { connected, send };
}
