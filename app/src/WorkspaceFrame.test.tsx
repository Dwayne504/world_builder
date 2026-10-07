import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { WorkspaceFrame } from "./WorkspaceFrame";
import type { Category, Entry } from "./types";

const categories: Category[] = Array.from({ length: 125 }, (_, index) => ({
  id: `category-${index}`,
  name: `Category ${String(index + 1).padStart(3, "0")}`,
  isUncategorized: false,
  revision: 1,
  globalRevision: 1,
}));
const entries: Entry[] = Array.from({ length: 3 }, (_, index) => ({
  id: `entry-${index}`,
  categoryId: index < 2 ? "category-124" : "category-0",
  typeId: null,
  authoredName: `Entry ${index}`,
  displayName: `Entry ${index}`,
  workspaceState: "active",
  revision: 1,
  globalRevision: 1,
}));
function props() {
  return {
    categories,
    entries,
    page: "entries" as const,
    categoryId: "category-124",
    collapsed: false,
    onBrowse: vi.fn(),
    onRelationships: vi.fn(),
    onChapters: vi.fn(),
    onTimeline: vi.fn(),
    onSearch: vi.fn(),
    onAddEntry: vi.fn(),
    onBack: vi.fn(),
    onForward: vi.fn(),
    canBack: true,
    canForward: true,
    busy: false,
    browsingDisabled: false,
    children: <p>Current workspace content</p>,
  };
}

it("searches a large Category catalog without navigating or changing Entry counts and uses stable IDs for actions", () => {
  const actions = props();
  render(<WorkspaceFrame {...actions} />);
  const nav = screen.getByRole("navigation", { name: "Project navigation" });
  fireEvent.change(within(nav).getByRole("searchbox", { name: "Find a Category" }), {
    target: { value: "  CATEGORY 125  " },
  });
  expect(within(nav).queryByRole("button", { name: "Category 001" })).not.toBeInTheDocument();
  const category = within(nav).getByRole("button", { name: "Category 125" });
  expect(category).toHaveAttribute("aria-current", "page");
  expect(category).toHaveAccessibleDescription("2 Entries");
  expect(within(nav).getByRole("button", { name: "All Entries" })).toHaveAccessibleDescription(
    "3 Entries",
  );
  expect(actions.onBrowse).not.toHaveBeenCalled();
  expect(actions.onAddEntry).not.toHaveBeenCalled();
  expect(screen.getByText("Current workspace content")).toBeVisible();
  fireEvent.click(category);
  expect(actions.onBrowse).toHaveBeenCalledExactlyOnceWith("category-124");
  fireEvent.click(within(nav).getByRole("button", { name: "Add Entry to Category 125" }));
  expect(actions.onAddEntry).toHaveBeenCalledExactlyOnceWith("category-124");
});

it("keeps the search and chosen Category when the Category section or sidebar is collapsed", () => {
  const actions = props();
  const view = render(<WorkspaceFrame {...actions} />);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "125" } });
  fireEvent.click(screen.getByRole("button", { name: "Categories" }));
  expect(screen.queryByRole("searchbox")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Categories" }));
  expect(screen.getByRole("searchbox")).toHaveValue("125");
  view.rerender(<WorkspaceFrame {...actions} collapsed />);
  expect(screen.queryByRole("navigation", { name: "Project navigation" })).not.toBeInTheDocument();
  view.rerender(<WorkspaceFrame {...actions} />);
  expect(screen.getByRole("searchbox")).toHaveValue("125");
  expect(screen.getByRole("button", { name: "Category 125" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(actions.onBrowse).not.toHaveBeenCalled();
});

it("exposes the full Category label as a tooltip when the sidebar truncates it", () => {
  const actions = props();
  const name = "The Northern Kingdoms and Their Outlying Settlements";
  render(<WorkspaceFrame {...actions} categories={[{ ...categories[0], name }]} />);
  const category = screen.getByRole("button", { name });
  expect(category).toHaveAttribute("title", name);
  fireEvent.click(category);
  expect(actions.onBrowse).toHaveBeenCalledExactlyOnceWith("category-0");
});

it("keeps an unmatched filter clearable when the available catalog shrinks", () => {
  const actions = props();
  const view = render(<WorkspaceFrame {...actions} />);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Missing" } });
  expect(screen.getByText("No matching Categories.")).toBeVisible();
  view.rerender(<WorkspaceFrame {...actions} categories={categories.slice(0, 2)} />);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect(screen.getByRole("button", { name: "Category 001" })).toBeVisible();
  expect(screen.getByRole("button", { name: "Category 002" })).toBeVisible();
  expect(screen.queryByText("No matching Categories.")).not.toBeInTheDocument();
  expect(screen.queryByRole("searchbox")).not.toBeInTheDocument();
  expect(actions.onBrowse).not.toHaveBeenCalled();
});

it.each(["busy", "browsingDisabled"] as const)(
  "keeps filtered navigation and creation disabled while %s",
  (flag) => {
    const actions = props();
    render(<WorkspaceFrame {...actions} {...{ [flag]: true }} />);
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "125" } });
    const category = screen.getByRole("button", { name: "Category 125" });
    const add = screen.getByRole("button", { name: "Add Entry to Category 125" });
    expect(category).toBeDisabled();
    expect(add).toBeDisabled();
    fireEvent.click(category);
    fireEvent.click(add);
    expect(actions.onBrowse).not.toHaveBeenCalled();
    expect(actions.onAddEntry).not.toHaveBeenCalled();
  },
);
