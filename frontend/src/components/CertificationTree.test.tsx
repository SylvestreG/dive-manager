import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CatalogEntry, CertificationDto } from "../lib/api";
import { CertificationTree } from "./CertificationTree";

// Without vitest globals, testing-library cannot auto-register its cleanup.
afterEach(cleanup);

// Minimal catalog slice mirroring the backend `covers` semantics.
const CATALOG: CatalogEntry[] = [
  { code: "N1", label: "Niveau 1", covers: ["PE12"] },
  { code: "PE12", label: "PE12", covers: [] },
  { code: "PE40", label: "PE40", covers: ["PE12"] },
  { code: "N2", label: "Niveau 2", covers: ["N1", "PE12", "PE40", "PA20", "PA12"] },
  { code: "PA20", label: "PA20", covers: ["PA12"] },
  { code: "PA12", label: "PA12", covers: [] },
  { code: "RIFAP", label: "RIFA Plongée", covers: [] },
];

function checkbox(label: string): HTMLInputElement {
  return screen.getByRole("checkbox", { name: label }) as HTMLInputElement;
}

describe("CertificationTree", () => {
  it("checking N2 auto-ticks everything it covers, disabled", () => {
    render(
      <CertificationTree
        certifications={[]}
        catalog={CATALOG}
        busy={false}
        onSave={() => {}}
      />,
    );
    fireEvent.click(checkbox("Niveau 2"));

    for (const covered of ["Niveau 1", "PE12", "PE40", "PA20", "PA12"]) {
      const box = checkbox(covered);
      expect(box.checked, `${covered} checked`).toBe(true);
      expect(box.disabled, `${covered} disabled`).toBe(true);
    }
    // Outside the coverage: untouched and still actionable.
    expect(checkbox("RIFA Plongée").checked).toBe(false);
    expect(checkbox("RIFA Plongée").disabled).toBe(false);
  });

  it("saves only the explicitly checked codes and drops absorbed staged additions", () => {
    const onSave = vi.fn();
    render(
      <CertificationTree
        certifications={[]}
        catalog={CATALOG}
        busy={false}
        onSave={onSave}
      />,
    );
    // Stage PE40 first, then N2: PE40 becomes covered and must be dropped.
    fireEvent.click(checkbox("PE40"));
    fireEvent.click(checkbox("Niveau 2"));
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(onSave).toHaveBeenCalledTimes(1);
    const changes = onSave.mock.calls[0][0];
    expect(changes.additions.map((a: { certification: string }) => a.certification)).toEqual([
      "N2",
    ]);
    expect(changes.removals).toEqual([]);
  });

  it("unchecking a held row stages its removal", () => {
    const onSave = vi.fn();
    const held: CertificationDto[] = [
      {
        id: "row-1",
        code: "RIFAP",
        label: "RIFA Plongée",
        obtained_on: null,
        provenance: "validated",
      },
    ];
    render(
      <CertificationTree
        certifications={held}
        catalog={CATALOG}
        busy={false}
        onSave={onSave}
      />,
    );
    const box = checkbox("RIFA Plongée");
    expect(box.checked).toBe(true);
    fireEvent.click(box);
    expect(screen.getByText("Sera retirée")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(onSave.mock.calls[0][0].removals).toEqual(["row-1"]);
  });
});
