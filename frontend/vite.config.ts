import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig(({ mode }) => ({
  // Web production is served under /v2/ next to the legacy app (same domain).
  // Every other build (dev, tests, Capacitor/iOS) stays at the root.
  base: mode === "web-prod" ? "/v2/" : "/",
  plugins: [react(), tailwindcss()],
  server: {
    // Web dev: same-origin /api/v1 proxied to the backend.
    proxy: {
      "/api": "http://localhost:8081",
    },
  },
  test: {
    environment: "jsdom",
    // Node ≥ 25's experimental localStorage global shadows jsdom's — the
    // setup file restores a working Storage (see vitest.setup.ts).
    setupFiles: ["./vitest.setup.ts"],
  },
}));
