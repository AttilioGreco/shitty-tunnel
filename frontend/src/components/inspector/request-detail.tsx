import { useState } from "react";
import type { RequestEvent } from "@/lib/types";
import { decodeBody, tryPrettyJson } from "@/lib/format";
import { cn } from "@/lib/utils";

interface RequestDetailProps {
  event: RequestEvent;
}

export function RequestDetail({ event }: RequestDetailProps) {
  return (
    <div className="border-t border-border bg-muted/30 px-4 py-3 space-y-1">
      <Accordion title="Request Headers" defaultOpen>
        <HeaderList headers={event.request.headers} />
      </Accordion>
      {event.request.body && (
        <Accordion title="Request Body">
          <BodyView
            body={event.request.body}
            truncated={event.request.truncated}
          />
        </Accordion>
      )}
      {event.response && (
        <>
          <Accordion title="Response Headers">
            <HeaderList headers={event.response.headers} />
          </Accordion>
          {event.response.body && (
            <Accordion title="Response Body">
              <BodyView
                body={event.response.body}
                truncated={event.response.truncated}
              />
            </Accordion>
          )}
        </>
      )}
    </div>
  );
}

function Accordion({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground py-1 w-full text-left"
      >
        <span
          className={cn(
            "transition-transform text-[10px]",
            open && "rotate-90",
          )}
        >
          ▶
        </span>
        {title}
      </button>
      {open && <div className="pl-4 pb-2">{children}</div>}
    </div>
  );
}

function HeaderList({ headers }: { headers: [string, string][] }) {
  if (headers.length === 0) {
    return (
      <p className="text-xs text-muted-foreground italic">No headers</p>
    );
  }
  return (
    <div className="font-mono text-xs space-y-0.5">
      {headers.map(([key, value], i) => (
        <div key={`${key}-${i}`}>
          <span className="text-muted-foreground">{key}:</span>{" "}
          <span className="text-foreground">{value}</span>
        </div>
      ))}
    </div>
  );
}

function BodyView({
  body,
  truncated,
}: {
  body: string;
  truncated: boolean;
}) {
  const decoded = decodeBody(body);
  const { text, isJson } = tryPrettyJson(decoded);
  return (
    <div>
      <pre
        className={cn(
          "text-xs font-mono whitespace-pre-wrap break-all bg-background rounded p-2 border max-h-64 overflow-auto",
          isJson && "text-chart-2",
        )}
      >
        {text}
      </pre>
      {truncated && (
        <p className="text-[10px] text-muted-foreground mt-1">
          Body truncated at 64KB
        </p>
      )}
    </div>
  );
}
