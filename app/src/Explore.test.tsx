import { useState } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { Explore } from "./Explore";
import { exploreProject } from "./api";
import { initialExploreView, type ExploreResults, type ExploreView } from "./exploreTypes";
vi.mock("./api", () => ({ exploreProject: vi.fn() }));
const result: ExploreResults = {
  globalRevision: 4,
  categories: [{ id: "people", name: "People", categoryId: null }],
  types: [{ id: "human", name: "Human", categoryId: "people" }],
  capabilities: ["base", "spatial", "event"],
  definitions: [
    {
      id: "location",
      name: "Location",
      forwardLabel: "lives on",
      inverseLabel: "hosts",
      directed: true,
      retired: false,
    },
  ],
  selectedOther: null,
  issues: [],
  total: 1,
  page: 0,
  pageSize: 20,
  entries: [
    {
      id: "captain",
      name: "Captain",
      category: "People",
      typeName: "Human",
      workspaceState: "active",
      spatial: false,
      relationshipMatch: null,
    },
  ],
};
const open = vi.fn();
function Harness({ initial = initialExploreView }: { initial?: ExploreView }) {
  const [view, setView] = useState(initial);
  return <Explore projectId="p" view={view} onViewChange={setView} onOpen={open} />;
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(exploreProject).mockResolvedValue(result);
});

it("combines independent filters, resets pagination, and opens the stable Entry identity", async () => {
  render(<Harness initial={{ ...initialExploreView, page: 2 }} />);
  await screen.findByRole("button", { name: "Captain People · Human" });
  fireEvent.change(screen.getByLabelText("Category"), { target: { value: "people" } });
  fireEvent.change(screen.getByLabelText("Exact Type"), { target: { value: "human" } });
  fireEvent.change(screen.getByLabelText("Capability"), { target: { value: "spatial" } });
  fireEvent.change(screen.getByLabelText("Name or alias"), { target: { value: "Captain" } });
  fireEvent.change(screen.getByLabelText("Entries to include"), { target: { value: "archived" } });
  await waitFor(() =>
    expect(exploreProject).toHaveBeenLastCalledWith("p", {
      ...initialExploreView,
      query: "Captain",
      categoryId: "people",
      typeId: "human",
      capability: "spatial",
      workspaceState: "archived",
    }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Captain People · Human" }));
  expect(open).toHaveBeenCalledWith("captain");
});

it("uses a bounded name/alias picker and an explicitly selected relationship perspective and Spatial target", async () => {
  const place = { ...result.entries[0], id: "island", name: "Island", spatial: true };
  vi.mocked(exploreProject).mockImplementation(async (_p, request) =>
    request.workspaceState === "all"
      ? { ...result, entries: [place], total: 3000 }
      : { ...result, selectedOther: request.relationship?.otherEntryId ? place : null },
  );
  render(<Harness />);
  await screen.findByText("1 matching Entries · Showing 1–1");
  fireEvent.change(screen.getByLabelText("Relationship"), { target: { value: "location" } });
  fireEvent.change(screen.getByLabelText("Result Entry’s side"), { target: { value: "target" } });
  fireEvent.click(screen.getByRole("button", { name: "Choose Entry…" }));
  fireEvent.change(screen.getByLabelText("Find the other Entry"), { target: { value: "Island" } });
  expect(await screen.findByText("First 1 of 3000. Search to narrow the list.")).toBeVisible();
  expect(exploreProject).toHaveBeenCalledWith("p", {
    ...initialExploreView,
    query: "Island",
    workspaceState: "all",
  });
  fireEvent.click(screen.getByRole("button", { name: "Island People · Human" }));
  await waitFor(() =>
    expect(
      screen.getByRole("option", { name: "This place and anywhere within it" }),
    ).not.toBeDisabled(),
  );
  fireEvent.change(screen.getByLabelText("Match the other Entry"), {
    target: { value: "contained" },
  });
  await waitFor(() =>
    expect(exploreProject).toHaveBeenLastCalledWith("p", {
      ...initialExploreView,
      relationship: {
        definitionId: "location",
        perspective: "target",
        otherEntryId: "island",
        includeContained: true,
      },
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Clear Entry" }));
  await waitFor(() =>
    expect(exploreProject).toHaveBeenLastCalledWith(
      "p",
      expect.objectContaining({
        relationship: expect.objectContaining({ otherEntryId: null, includeContained: false }),
      }),
    ),
  );
});

it("retains unresolved identities and errors instead of displaying unfiltered results", async () => {
  vi.mocked(exploreProject).mockResolvedValue({
    ...result,
    entries: [],
    total: 0,
    issues: ["The selected Category is unavailable."],
  });
  render(<Harness initial={{ ...initialExploreView, categoryId: "missing" }} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("selected Category is unavailable");
  expect(screen.getByLabelText("Category")).toHaveValue("missing");
  expect(screen.queryByRole("button", { name: /Captain/ })).not.toBeInTheDocument();
  vi.mocked(exploreProject).mockResolvedValue(result);
  fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  expect(await screen.findByRole("button", { name: "Captain People · Human" })).toBeVisible();
  expect(exploreProject).toHaveBeenLastCalledWith("p", initialExploreView);
});

it("bounds large structure pickers while preserving selections outside the visible choices", async () => {
  vi.mocked(exploreProject).mockResolvedValue({
    ...result,
    categories: Array.from({ length: 80 }, (_, n) => ({
      id: `c${n}`,
      name: `Category ${n}`,
      categoryId: null,
    })),
  });
  render(<Harness initial={{ ...initialExploreView, categoryId: "c79" }} />);
  const search = await screen.findByLabelText("Search Category");
  expect(screen.getByLabelText("Category")).toHaveValue("c79");
  expect(screen.getByRole("option", { name: "Category 79" })).toBeInTheDocument();
  expect(screen.queryByRole("option", { name: "Category 50" })).not.toBeInTheDocument();
  fireEvent.change(search, { target: { value: "Category 50" } });
  expect(screen.getByRole("option", { name: "Category 50" })).toBeInTheDocument();
  expect(screen.getByLabelText("Category")).toHaveValue("c79");
});

it("ignores stale responses, retries failures, and refreshes current filters after shared edits", async () => {
  let finish!: (value: ExploreResults) => void;
  vi.mocked(exploreProject).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const view = { ...initialExploreView, query: "Old" };
  const rendered = render(
    <Explore projectId="p" view={view} onViewChange={vi.fn()} onOpen={open} />,
  );
  await waitFor(() => expect(exploreProject).toHaveBeenCalledOnce());
  vi.mocked(exploreProject).mockRejectedValueOnce(new Error("Read failed"));
  rendered.rerender(
    <Explore projectId="p" view={{ ...view, query: "New" }} onViewChange={vi.fn()} onOpen={open} />,
  );
  expect(await screen.findByRole("alert")).toHaveTextContent("Read failed");
  await act(async () => {
    finish(result);
  });
  expect(screen.queryByRole("button", { name: /Captain/ })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Try again" }));
  await screen.findByRole("button", { name: "Captain People · Human" });
  rendered.rerender(
    <Explore
      projectId="p"
      view={{ ...view, query: "New" }}
      onViewChange={vi.fn()}
      onOpen={open}
      refreshKey={1}
    />,
  );
  await waitFor(() => expect(exploreProject).toHaveBeenCalledTimes(4));
});

it("paginates the backend results and distinguishes direct and containment-derived matches", async () => {
  vi.mocked(exploreProject).mockImplementation(async (_p, request) => ({
    ...result,
    page: request.page,
    pageSize: request.pageSize,
    total: 3000,
    entries: [{ ...result.entries[0], relationshipMatch: "contained" }],
  }));
  render(<Harness />);
  expect(await screen.findByText("Via contained place")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  await waitFor(() =>
    expect(exploreProject).toHaveBeenLastCalledWith("p", { ...initialExploreView, page: 1 }),
  );
  fireEvent.change(await screen.findByLabelText("Per page"), { target: { value: "100" } });
  await waitFor(() =>
    expect(exploreProject).toHaveBeenLastCalledWith("p", { ...initialExploreView, pageSize: 100 }),
  );
  expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();
});
