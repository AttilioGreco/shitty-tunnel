import { Suspense } from "react";
import { Outlet, createRootRoute } from "@tanstack/react-router";

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
});

function RootComponent() {
  return (
    <Suspense>
      <Outlet />
    </Suspense>
  );
}
