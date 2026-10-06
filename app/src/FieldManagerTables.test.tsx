import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { FieldManagerTables } from "./FieldManagerTables";
import type { Entry, FieldDefinition } from "./types";

it("finds current and reusable fields beyond the first page while actions keep stable identities", () => {
  const definitions: FieldDefinition[] = Array.from({ length: 220 }, (_, index) => ({
    id: `field-${index}`,
    name: `Field ${index}`,
    kind: "short_text",
    revision: 1,
    retired: false,
    options: [],
    bindings: [],
  }));
  const entry: Entry = {
    id: "entry",
    categoryId: "category",
    typeId: null,
    authoredName: "Navigator",
    displayName: "Navigator",
    workspaceState: "active",
    revision: 1,
    globalRevision: 1,
  };
  const onEdit = vi.fn(),
    onHide = vi.fn(),
    onDelete = vi.fn(),
    onAdd = vi.fn();
  const fields = definitions
    .slice(0, 110)
    .map((definition) => ({ definition, available: true, value: null }));
  render(
    <FieldManagerTables
      entry={entry}
      definitions={definitions}
      fields={fields}
      disabled={false}
      onEdit={onEdit}
      onHide={onHide}
      onDelete={onDelete}
      onAdd={onAdd}
    />,
  );
  const [current, reusable] = screen.getAllByRole("table");
  expect(within(current).getAllByRole("row")).toHaveLength(13);
  expect(within(reusable).getAllByRole("row")).toHaveLength(13);
  fireEvent.change(screen.getByRole("searchbox", { name: "Find a field" }), {
    target: { value: "Field 109" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Hide field: Field 109" }));
  expect(onHide).toHaveBeenCalledExactlyOnceWith(fields[109]);
  expect(screen.getByText("No other fields match your search.")).toBeVisible();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Field 219" } });
  fireEvent.click(screen.getByRole("button", { name: "Add to Entry: Field 219" }));
  expect(onAdd).toHaveBeenCalledExactlyOnceWith(definitions[219]);
  expect(onDelete).not.toHaveBeenCalled();
  expect(onEdit).not.toHaveBeenCalled();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Show more Entry fields" }));
  expect(within(current).getAllByRole("row")).toHaveLength(25);
  expect(within(reusable).getAllByRole("row")).toHaveLength(13);
});
