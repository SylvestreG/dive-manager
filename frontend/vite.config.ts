import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    // Web dev: same-origin /api/v1 proxied to the backend.
    proxy: {
      "/api": "http://localhost:8081",
    },
  },
  test: {
    environment: "jsdom",
  },
});
