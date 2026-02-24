import { Suspense } from "react";
import { HeadContent, Outlet, Scripts, createRootRoute } from "@tanstack/react-router";

import appCss from "../styles.css?url";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "shittyTunnel Inspector" },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
    ],
  }),

  // component is used in SPA mode (spa-entry.tsx)
  component: RootComponent,
  // shellComponent is used in SSR mode (TanStack Start)
  shellComponent: RootDocument,
});

function RootComponent() {
  return (
    <Suspense>
      <Outlet />
    </Suspense>
  );
}

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}
