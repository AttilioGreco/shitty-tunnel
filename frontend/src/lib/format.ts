export function formatDuration(ms: number | null): string {
  if (ms === null) return "...";
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

export function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString("en-GB", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

export function decodeBody(base64: string): string {
  try {
    return atob(base64);
  } catch {
    return base64;
  }
}

export function tryPrettyJson(raw: string): { text: string; isJson: boolean } {
  try {
    const parsed = JSON.parse(raw);
    return { text: JSON.stringify(parsed, null, 2), isJson: true };
  } catch {
    return { text: raw, isJson: false };
  }
}
