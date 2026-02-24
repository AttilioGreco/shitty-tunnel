interface WaterfallBarProps {
  offsetMs: number;
  durationMs: number | null;
  windowMs: number;
}

export function WaterfallBar({
  offsetMs,
  durationMs,
  windowMs,
}: WaterfallBarProps) {
  if (windowMs <= 0) return null;

  const left = Math.min((offsetMs / windowMs) * 100, 100);
  const width =
    durationMs !== null
      ? Math.max((durationMs / windowMs) * 100, 0.5)
      : 0.5;

  return (
    <div className="relative h-4 w-full bg-muted/50 rounded-sm overflow-hidden">
      <div
        className="absolute top-0.5 bottom-0.5 rounded-sm bg-chart-1"
        style={{
          left: `${left}%`,
          width: `${Math.min(width, 100 - left)}%`,
          minWidth: "2px",
        }}
      />
      {durationMs !== null && (
        <span className="absolute right-1 top-0 text-[10px] text-muted-foreground leading-4">
          {Math.round(durationMs)}
        </span>
      )}
    </div>
  );
}
