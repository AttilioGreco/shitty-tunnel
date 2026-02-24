import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface StatusBadgeProps {
  status: number | null;
}

function statusColor(status: number | null): string {
  if (status === null) return "bg-muted text-muted-foreground";
  if (status < 300) return "bg-[var(--status-2xx)] text-white";
  if (status < 400) return "bg-[var(--status-3xx)] text-white";
  if (status < 500) return "bg-[var(--status-4xx)] text-white";
  return "bg-[var(--status-5xx)] text-white";
}

export function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <Badge
      className={cn(
        "font-mono text-[11px] min-w-[3rem] justify-center",
        statusColor(status),
      )}
    >
      {status ?? "..."}
    </Badge>
  );
}
