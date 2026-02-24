import { useMemo, useState, Fragment } from "react";
import type { RequestEvent } from "@/lib/types";
import { formatDuration, formatBytes, formatTime } from "@/lib/format";
import { StatusBadge } from "./status-badge";
import { WaterfallBar } from "./waterfall-bar";
import { RequestDetail } from "./request-detail";
import { type Filters, matchesFilters } from "./filter-bar";

interface RequestTableProps {
  events: RequestEvent[];
  filters: Filters;
}

export function RequestTable({ events, filters }: RequestTableProps) {
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const filtered = useMemo(
    () => events.filter((e) => matchesFilters(e, filters)),
    [events, filters],
  );

  const windowMs = useMemo(() => {
    if (filtered.length < 2) return 10000;
    const offsets = filtered.map((e) => e.offset_ms);
    const durations = filtered.map((e) => e.duration_ms ?? 0);
    const maxEnd = Math.max(
      ...offsets.map((o, i) => o + durations[i]),
    );
    const minStart = Math.min(...offsets);
    return Math.max(maxEnd - minStart, 1000);
  }, [filtered]);

  const minOffset = useMemo(() => {
    if (filtered.length === 0) return 0;
    return Math.min(...filtered.map((e) => e.offset_ms));
  }, [filtered]);

  if (filtered.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground text-sm">
        {events.length === 0
          ? "No requests captured yet"
          : "No requests match the current filters"}
      </div>
    );
  }

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <table className="w-full text-xs">
        <thead>
          <tr className="bg-muted/50 border-b border-border">
            <th className="text-left px-3 py-2 w-[4.5rem]">Status</th>
            <th className="text-left px-3 py-2 w-[4.5rem]">Method</th>
            <th className="text-left px-3 py-2">Path</th>
            <th className="text-right px-3 py-2 w-[5rem]">Duration</th>
            <th className="text-right px-3 py-2 w-[4.5rem]">Size</th>
            <th className="text-left px-3 py-2 w-[10rem]">Waterfall</th>
            <th className="text-right px-3 py-2 w-[5.5rem]">Time</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((event) => (
            <Fragment key={event.id}>
              <tr
                onClick={() =>
                  setExpandedId(
                    expandedId === event.id ? null : event.id,
                  )
                }
                className="border-b border-border hover:bg-muted/30 cursor-pointer transition-colors"
              >
                <td className="px-3 py-1.5">
                  <StatusBadge
                    status={event.response?.status ?? null}
                  />
                </td>
                <td className="px-3 py-1.5 font-mono font-medium">
                  {event.request.method}
                </td>
                <td className="px-3 py-1.5 font-mono truncate max-w-[20rem]">
                  {event.request.path}
                </td>
                <td className="px-3 py-1.5 text-right font-mono text-muted-foreground">
                  {formatDuration(event.duration_ms)}
                </td>
                <td className="px-3 py-1.5 text-right font-mono text-muted-foreground">
                  {event.response
                    ? formatBytes(event.response.body_size)
                    : "..."}
                </td>
                <td className="px-3 py-1.5">
                  <WaterfallBar
                    offsetMs={event.offset_ms - minOffset}
                    durationMs={event.duration_ms}
                    windowMs={windowMs}
                  />
                </td>
                <td className="px-3 py-1.5 text-right text-muted-foreground">
                  {formatTime(event.timestamp)}
                </td>
              </tr>
              {expandedId === event.id && (
                <tr>
                  <td colSpan={7}>
                    <RequestDetail event={event} />
                  </td>
                </tr>
              )}
            </Fragment>
          ))}
        </tbody>
      </table>
    </div>
  );
}
