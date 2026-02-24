import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";

export interface Filters {
  statusGroups: Set<string>;
  method: string;
  pathSearch: string;
}

interface FilterBarProps {
  filters: Filters;
  onFiltersChange: (filters: Filters) => void;
  onClear: () => void;
}

const STATUS_GROUPS = ["2xx", "3xx", "4xx", "5xx"] as const;
const METHODS = ["ALL", "GET", "POST", "PUT", "PATCH", "DELETE"] as const;

export function FilterBar({
  filters,
  onFiltersChange,
  onClear,
}: FilterBarProps) {
  return (
    <div className="flex items-center gap-3 flex-wrap">
      <div className="flex items-center gap-1.5">
        {STATUS_GROUPS.map((group) => (
          <button
            key={group}
            type="button"
            onClick={() => {
              const next = new Set(filters.statusGroups);
              if (next.has(group)) {
                next.delete(group);
              } else {
                next.add(group);
              }
              onFiltersChange({ ...filters, statusGroups: next });
            }}
            className={`px-2 py-0.5 text-xs rounded font-mono transition-colors ${
              filters.statusGroups.has(group)
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:text-foreground"
            }`}
          >
            {group}
          </button>
        ))}
      </div>

      <select
        value={filters.method}
        onChange={(e) =>
          onFiltersChange({ ...filters, method: e.target.value })
        }
        className="h-8 text-xs bg-muted border border-border rounded px-2 text-foreground"
      >
        {METHODS.map((m) => (
          <option key={m} value={m}>
            {m}
          </option>
        ))}
      </select>

      <Input
        placeholder="Filter path..."
        value={filters.pathSearch}
        onChange={(e) =>
          onFiltersChange({ ...filters, pathSearch: e.target.value })
        }
        className="h-8 w-48 text-xs"
      />

      <div className="ml-auto">
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button variant="outline" size="sm" className="text-xs h-8">
              Clear
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Clear all requests?</AlertDialogTitle>
              <AlertDialogDescription>
                This will permanently clear all captured requests from the
                buffer.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={onClear}>Clear</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  );
}

export function createDefaultFilters(): Filters {
  return {
    statusGroups: new Set(["2xx", "3xx", "4xx", "5xx"]),
    method: "ALL",
    pathSearch: "",
  };
}

export function matchesFilters(
  event: { request: { method: string; path: string }; response: { status: number } | null },
  filters: Filters,
): boolean {
  // Method filter
  if (filters.method !== "ALL" && event.request.method !== filters.method) {
    return false;
  }

  // Path filter
  if (
    filters.pathSearch &&
    !event.request.path.toLowerCase().includes(filters.pathSearch.toLowerCase())
  ) {
    return false;
  }

  // Status group filter
  if (event.response) {
    const status = event.response.status;
    const group = `${Math.floor(status / 100)}xx`;
    if (!filters.statusGroups.has(group)) {
      return false;
    }
  }

  return true;
}
