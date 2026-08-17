import { describe, expect, it } from "vitest";

import { ApiError } from "./api";
import { translateApiError } from "./errors";

describe("translateApiError", () => {
  it("translates known backend messages to French", () => {
    expect(
      translateApiError(
        new ApiError(422, "validation", "password must be between 8 and 128 characters"),
      ),
    ).toBe("Le mot de passe doit contenir entre 8 et 128 caractères.");
    expect(
      translateApiError(
        new ApiError(422, "validation", "cannot remove the club's last administrator"),
      ),
    ).toBe("Impossible de retirer le dernier administrateur du club.");
  });

  it("maps 429 to the rate-limit message", () => {
    expect(translateApiError(new ApiError(429, "too_many_requests", "too many requests"))).toBe(
      "Trop de tentatives, réessayez dans quelques minutes.",
    );
  });

  it("shows unknown 4xx messages as-is (they are written for humans)", () => {
    expect(translateApiError(new ApiError(409, "conflict", "some new message"))).toBe(
      "some new message",
    );
  });

  it("falls back to a generic message for 5xx and non-API errors", () => {
    expect(
      translateApiError(new ApiError(500, "internal", "internal server error")),
    ).toBe("Une erreur est survenue.");
    expect(translateApiError(new Error("boom"))).toBe("Une erreur est survenue.");
    expect(translateApiError(undefined)).toBe("Une erreur est survenue.");
  });
});
