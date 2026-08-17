// Session state = the bearer token (localStorage) + the /me query.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { useNavigate } from "react-router";

import { api, getToken } from "./api";

export function useMe() {
  return useQuery({
    queryKey: ["me"],
    queryFn: api.me,
    enabled: getToken() !== null,
    staleTime: 30_000,
    retry: false,
  });
}

/** Redirects to /login whenever the API reports an invalid session. */
export function useUnauthorizedRedirect() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  useEffect(() => {
    const onUnauthorized = () => {
      queryClient.clear();
      navigate("/login", { replace: true });
    };
    window.addEventListener("dm:unauthorized", onUnauthorized);
    return () => window.removeEventListener("dm:unauthorized", onUnauthorized);
  }, [navigate, queryClient]);
}
