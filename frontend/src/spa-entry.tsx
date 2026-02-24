import { StrictMode, startTransition, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

import "./styles.css";

const router = createRouter({
  routeTree,
  scrollRestoration: true,
  defaultPreloadStaleTime: 0,
  // Show pending state only if navigation takes longer than 200ms
  // avoids spinner flash on fast navigations
  defaultPendingMs: 200,
  defaultPendingMinMs: 500,
});

const root = createRoot(document.getElementById("root")!);

// startTransition lets React keep the current UI interactive while
// the new route is loading (React 19 concurrent rendering)
startTransition(() => {
  root.render(
    <StrictMode>
      <Suspense>
        <RouterProvider router={router} />
      </Suspense>
    </StrictMode>,
  );
});
