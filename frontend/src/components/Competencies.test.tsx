import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { CompetencyProgress, LevelRules } from "../lib/api";
import { ProgressSummary, StageBadge, StageLegend, stageLabel } from "./Competencies";

const n1: LevelRules = {
  certification: "N1",
  label: "Niveau 1",
  sea_validation: false,
  required_validator_grade: "E1",
  stages: ["vu", "a_perfectionner", "valide"],
};
const n2: LevelRules = { ...n1, certification: "N2", label: "Niveau 2", sea_validation: true, required_validator_grade: "E2" };

describe("fixed validation stages", () => {
  it("labels the final stage per the sea rule (PO 2026-08-19)", () => {
    expect(stageLabel("valide", n1)).toBe("Validé");
    expect(stageLabel("valide", n2)).toBe("Validé en mer");
    expect(stageLabel("vu", n2)).toBe("Vu");
    expect(stageLabel("a_perfectionner", n2)).toBe("À perfectionner");
  });

  it("shows the not-started badge when there is no validation", () => {
    render(<StageBadge stage={null} level={n1} />);
    expect(screen.getByText("Non commencé")).toBeTruthy();
  });

  it("legend shows the sea reminder only above N1", () => {
    const { rerender } = render(<StageLegend level={n1} />);
    expect(screen.queryByText(/en mer/)).toBeNull();
    rerender(<StageLegend level={n2} />);
    expect(screen.getAllByText(/en mer/).length).toBeGreaterThan(0);
  });
});

describe("progress summary", () => {
  it("renders the weighted percentage and the detail line", () => {
    const progress: CompetencyProgress = {
      total: 4,
      validated: 1,
      in_progress: 2,
      not_started: 1,
      percentage: 25,
    };
    render(<ProgressSummary progress={progress} />);
    expect(screen.getByText("25%")).toBeTruthy();
    expect(screen.getByText("1/4 validés")).toBeTruthy();
    expect(screen.getByText("1 validés • 2 en cours • 1 à faire")).toBeTruthy();
  });
});
