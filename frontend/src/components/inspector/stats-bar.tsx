import { Card, CardContent } from "@/components/ui/card";
import type { InspectorStats } from "@/lib/types";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

interface StatsBarProps {
  stats: InspectorStats;
  connected: boolean;
}

export function StatsBar({ stats, connected }: StatsBarProps) {
  return (
    <div className="grid grid-cols-4 gap-3">
      <StatCard label="Requests" value={stats.total.toString()} />
      <StatCard
        label="Error Rate"
        value={`${stats.errorRate.toFixed(1)}%`}
        highlight={stats.errorRate > 10}
      />
      <StatCard
        label="Avg Latency"
        value={formatDuration(stats.avgLatency || null)}
      />
      <Card size="sm">
        <CardContent className="flex items-center gap-2 py-0">
          <div
            className={cn(
              "h-2 w-2 rounded-full",
              connected ? "bg-green-500" : "bg-red-500",
            )}
          />
          <span className="text-xs text-muted-foreground">
            {connected ? "Connected" : "Disconnected"}
          </span>
        </CardContent>
      </Card>
    </div>
  );
}

function StatCard({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: boolean;
}) {
  return (
    <Card size="sm">
      <CardContent className="py-0">
        <p className="text-xs text-muted-foreground">{label}</p>
        <p
          className={cn(
            "text-lg font-semibold font-mono",
            highlight && "text-destructive",
          )}
        >
          {value}
        </p>
      </CardContent>
    </Card>
  );
}
