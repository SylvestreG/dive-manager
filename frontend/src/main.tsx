import {
  MutationCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";

import App from "./App";
import { pushErrorToast, ToastViewport } from "./components/toasts";
import { translateApiError } from "./lib/errors";
import "./index.css";

const queryClient = new QueryClient({
  // Frontend rule: NO silent mutation failure. Every failed write surfaces a
  // toast, unless the calling form displays the error inline and opts out
  // with `meta: { inlineError: true }`.
  mutationCache: new MutationCache({
    onError: (error, _variables, _context, mutation) => {
      if (mutation.meta?.inlineError) return;
      pushErrorToast(translateApiError(error));
    },
  }),
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      {/* BASE_URL is "/v2/" in web production, "/" otherwise (vite.config.ts). */}
      <BrowserRouter basename={import.meta.env.BASE_URL.replace(/\/$/, "")}>
        <ToastViewport />
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
