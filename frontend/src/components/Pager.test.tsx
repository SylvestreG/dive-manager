import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Pager } from "./ui";

// Without vitest globals, testing-library cannot auto-register its cleanup.
afterEach(cleanup);

describe("Pager", () => {
  it("is hidden when everything fits on one page", () => {
    const { container } = render(
      <Pager page={1} perPage={50} total={12} onPage={() => {}} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("pages forward and back, disabling buttons at the bounds", () => {
    const onPage = vi.fn();
    render(<Pager page={1} perPage={50} total={120} onPage={onPage} />);
    expect(screen.getByText("Page 1 / 3")).toBeTruthy();

    const previous = screen.getByRole("button", { name: /Précédent/ });
    const next = screen.getByRole("button", { name: /Suivant/ });
    expect((previous as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(next);
    expect(onPage).toHaveBeenCalledWith(2);
  });

  it("disables next on the last page", () => {
    render(<Pager page={3} perPage={50} total={120} onPage={() => {}} />);
    const next = screen.getByRole("button", { name: /Suivant/ });
    expect((next as HTMLButtonElement).disabled).toBe(true);
  });
});
