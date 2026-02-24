import { useState } from "react";
import { useInspector } from "@/hooks/use-inspector";
import { StatsBar } from "./stats-bar";
import { FilterBar, createDefaultFilters, type Filters } from "./filter-bar";
import { RequestTable } from "./request-table";

export function InspectorPage() {
  const { events, stats, connected, clear } = useInspector();
  const [filters, setFilters] = useState<Filters>(createDefaultFilters);

  return (
    <div className="min-h-screen bg-background p-4 space-y-4 max-w-[90rem] mx-auto">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">shittyTunnel Inspector</h1>
      </div>

      <StatsBar stats={stats} connected={connected} />
      <FilterBar
        filters={filters}
        onFiltersChange={setFilters}
        onClear={clear}
      />
      <RequestTable events={events} filters={filters} />
    </div>
  );
}
