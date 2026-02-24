import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import viteTsConfigPaths from "vite-tsconfig-paths";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [
    viteTsConfigPaths({
      projects: ["./tsconfig.json"],
    }),
    tailwindcss(),
    viteReact(),
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // Target modern browsers — enables more aggressive tree-shaking and smaller output
    target: ["es2022", "chrome100", "firefox100", "safari16"],
    // Raise warning only for truly large chunks
    chunkSizeWarningLimit: 300,
    rollupOptions: {
      output: {
        // Deterministic file naming — chunk hash only changes when content changes
        chunkFileNames: "assets/[name]-[hash].js",
        entryFileNames: "assets/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
        manualChunks(id) {
          // React runtime — most stable, cached longest
          if (
            id.includes("node_modules/react/") ||
            id.includes("node_modules/react-dom/") ||
            id.includes("node_modules/scheduler/") ||
            id.includes("node_modules/use-sync-external-store/")
          ) {
            return "vendor-react";
          }

          // TanStack Router — changes with router upgrades, not app changes
          if (
            id.includes("node_modules/@tanstack/react-router") ||
            id.includes("node_modules/@tanstack/router-core") ||
            id.includes("node_modules/@tanstack/history") ||
            id.includes("node_modules/@tanstack/react-start") ||
            id.includes("node_modules/@tanstack/start-server-core") ||
            id.includes("node_modules/@tanstack/start-client-core")
          ) {
            return "vendor-router";
          }

          // Base UI + styling utilities — grouped because they share internal infra
          if (
            id.includes("node_modules/@base-ui/react") ||
            id.includes("node_modules/@base-ui/utils") ||
            id.includes("node_modules/class-variance-authority") ||
            id.includes("node_modules/clsx") ||
            id.includes("node_modules/tailwind-merge") ||
            // floating-ui is a transitive dep of @base-ui
            id.includes("node_modules/@floating-ui/")
          ) {
            return "vendor-ui";
          }

          // Lucide icons — large, tree-shaken, separate for caching
          if (id.includes("node_modules/lucide-react")) {
            return "vendor-icons";
          }

          // Everything else in node_modules → misc vendor chunk
          if (id.includes("node_modules/")) {
            return "vendor-misc";
          }
        },
      },
    },
  },
});
