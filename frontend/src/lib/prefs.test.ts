import { beforeEach, describe, expect, it } from "vitest";

import { getLastClubSlug, setLastClubSlug } from "./prefs";

describe("last-club preference", () => {
  beforeEach(() => localStorage.clear());

  it("is stored per user so accounts on a shared device never leak", () => {
    setLastClubSlug("user-a", "usi");
    setLastClubSlug("user-b", "club-b");
    expect(getLastClubSlug("user-a")).toBe("usi");
    expect(getLastClubSlug("user-b")).toBe("club-b");
    expect(getLastClubSlug("user-c")).toBeNull();
  });
});
