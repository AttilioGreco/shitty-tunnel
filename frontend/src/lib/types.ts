export interface RequestData {
  method: string;
  path: string;
  headers: [string, string][];
  body: string; // base64
  body_size: number;
  truncated: boolean;
}

export interface ResponseData {
  status: number;
  headers: [string, string][];
  body: string; // base64
  body_size: number;
  truncated: boolean;
}

export interface RequestEvent {
  id: number;
  timestamp: string;
  offset_ms: number;
  request: RequestData;
  response: ResponseData | null;
  duration_ms: number | null;
}

// WebSocket messages from server
export type WsServerMessage =
  | { type: "snapshot"; events: RequestEvent[]; epoch_ms: number }
  | { type: "request_started"; event: RequestEvent }
  | {
      type: "request_completed";
      id: number;
      response: ResponseData;
      duration_ms: number;
    }
  | { type: "cleared" };

// WebSocket messages from client
export type WsClientMessage = { type: "clear" };

export interface InspectorStats {
  total: number;
  errorRate: number;
  avgLatency: number;
}
