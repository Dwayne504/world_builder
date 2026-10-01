import { useState } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { ProjectSearch } from "./ProjectSearch";
import { EntryAliasesEditor } from "./EntryAliasesEditor";
import { useMutationCoordinator } from "./useMutationCoordinator";
import { searchProject, readAliases, applyAlias } from "./api";
import type { SearchResults, SearchView } from "./searchTypes";
vi.mock("./api", () => ({ searchProject: vi.fn(), readAliases: vi.fn(), applyAlias: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
});
const view: SearchView = { query: "", includeInactive: false, limitPerGroup: 10 };
const result: SearchResults = {
  globalRevision: 3,
  groups: [
    {
      kind: "entries",
      total: 1,
      hits: [
        {
          key: "e",
          title: "Wanderer",
          context: "Character · Human",
          workspaceState: "active",
          reason: "Alias: Captain",
          excerpt: "",
          target: { kind: "entry", entryId: "e" },
        },
      ],
    },
    {
      kind: "text",
      total: 1,
      hits: [
        {
          key: "d",
          title: "Journey",
          context: "notes · Text match",
          workspaceState: "active",
          reason: "Plain text · not a structural link",
          excerpt: '<img src="x" onerror="alert(1)"> Captain in prose',
          target: { kind: "chapter", chapterId: "c", area: "notes" },
        },
      ],
    },
  ],
};
const open = vi.fn();
function Search({ initial = view }: { initial?: SearchView }) {
  const [current, setCurrent] = useState(initial);
  return <ProjectSearch projectId="p" view={current} onViewChange={setCurrent} onOpen={open} />;
}
it("groups identities ahead of prose and opens the actual matching document area", async () => {
  vi.mocked(searchProject).mockResolvedValue(result);
  render(<Search />);
  expect(searchProject).not.toHaveBeenCalled();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Captain" } });
  expect(await screen.findByText("2 matching results")).toBeVisible();
  expect(screen.getByRole("region", { name: "Entries" })).toHaveTextContent("Alias: Captain");
  expect(document.querySelector("img")).toBeNull();
  expect(screen.queryByText(result.groups[1].hits[0].excerpt)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^Journey/ }));
  expect(screen.getByText(result.groups[1].hits[0].excerpt)).toBeVisible();
  expect(document.querySelector("img")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Open Chapter · Notes" }));
  expect(open).toHaveBeenCalledWith({ kind: "chapter", chapterId: "c", area: "notes" });
  expect(searchProject).toHaveBeenCalledWith("p", {
    query: "Captain",
    includeInactive: false,
    limitPerGroup: 10,
  });
});
it("ignores a late response to an earlier query", async () => {
  let resolve!: (value: SearchResults) => void;
  vi.mocked(searchProject)
    .mockReturnValueOnce(
      new Promise((r) => {
        resolve = r;
      }),
    )
    .mockResolvedValueOnce({ globalRevision: 3, groups: [] });
  render(<Search initial={{ ...view, query: "Captain" }} />);
  await waitFor(() => expect(searchProject).toHaveBeenCalledTimes(1));
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Nobody" } });
  await screen.findByText(/No matches/);
  await act(async () => resolve(result));
  expect(screen.queryByRole("button", { name: "Wanderer" })).not.toBeInTheDocument();
  expect(screen.getByText(/No matches/)).toBeVisible();
});
it("retries visible errors, includes inactive records only explicitly, and bounds more results", async () => {
  vi.mocked(searchProject)
    .mockRejectedValueOnce(new Error("Read failed"))
    .mockResolvedValue({ ...result, groups: [{ ...result.groups[0], total: 50 }] });
  render(<Search initial={{ ...view, query: "Captain" }} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Read failed");
  fireEvent.click(screen.getByRole("button", { name: "Retry search" }));
  await screen.findByText("50 matching results");
  fireEvent.click(screen.getByRole("checkbox"));
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Captain",
      includeInactive: true,
      limitPerGroup: 10,
    }),
  );
  fireEvent.click(await screen.findByRole("button", { name: "Show more results" }));
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Captain",
      includeInactive: true,
      limitPerGroup: 30,
    }),
  );
});
const onRevision = vi.fn();
const onDraft = vi.fn();
const getRevision = () => 7;
function Aliases() {
  const mutations = useMutationCoordinator();
  return (
    <EntryAliasesEditor
      projectId="p"
      entryId="e"
      disabled={mutations.state === "saving"}
      mutations={mutations}
      getRevision={getRevision}
      onRevision={onRevision}
      onDraftChange={onDraft}
    />
  );
}
it("keeps failed alias drafts, retries explicitly, and removes only the selected alias", async () => {
  vi.mocked(readAliases).mockResolvedValue({ globalRevision: 7, aliases: [] });
  vi.mocked(applyAlias)
    .mockRejectedValueOnce(new Error("Disk full"))
    .mockResolvedValueOnce({ globalRevision: 8, aliases: [{ id: "alias-id", text: "Captain" }] })
    .mockResolvedValueOnce({ globalRevision: 9, aliases: [] });
  render(<Aliases />);
  await waitFor(() => expect(screen.getByLabelText("New alias")).toBeEnabled());
  fireEvent.change(screen.getByLabelText("New alias"), { target: { value: "Captain" } });
  expect(onDraft).toHaveBeenLastCalledWith(true);
  fireEvent.click(screen.getByRole("button", { name: "Add alias" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
  expect(screen.getByLabelText("New alias")).toHaveValue("Captain");
  fireEvent.click(screen.getByRole("button", { name: "Add alias" }));
  await screen.findByText("Captain");
  expect(screen.getByLabelText("New alias")).toHaveValue("");
  fireEvent.click(screen.getByRole("button", { name: "Remove alias Captain" }));
  expect(applyAlias).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole("button", { name: "Keep alias" }));
  expect(applyAlias).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole("button", { name: "Remove alias Captain" }));
  fireEvent.click(screen.getByRole("button", { name: "Remove alias" }));
  await waitFor(() => expect(screen.queryByText("Captain")).not.toBeInTheDocument());
  expect(applyAlias).toHaveBeenLastCalledWith("p", "e", 7, { kind: "delete", aliasId: "alias-id" });
  expect(onRevision).toHaveBeenLastCalledWith(9);
  expect(onDraft).toHaveBeenLastCalledWith(false);
});
it("keeps alias load errors visible and enables editing after reload", async () => {
  vi.mocked(readAliases)
    .mockRejectedValueOnce(new Error("Unavailable"))
    .mockResolvedValueOnce({ globalRevision: 7, aliases: [] });
  render(<Aliases />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Unavailable");
  expect(screen.getByLabelText("New alias")).toBeDisabled();
  fireEvent.click(
    within(screen.getByRole("alert")).getByRole("button", { name: "Reload aliases" }),
  );
  await waitFor(() => expect(screen.getByLabelText("New alias")).toBeEnabled());
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("keeps relationship wording compact and opens the chosen perspective with a separate note", async () => {
  const target = {
    kind: "relationship" as const,
    relationshipId: "r",
    perspectiveEntryId: "leopold",
  };
  vi.mocked(searchProject).mockResolvedValue({
    globalRevision: 3,
    groups: [
      {
        kind: "structured",
        total: 1,
        hits: [
          {
            key: "r",
            title: "Leopold opposes Thron",
            context: "Adversary · Current",
            workspaceState: "active",
            reason: "Structured context",
            excerpt: "Their authored note.",
            preview: "Their authored note.",
            target,
          },
        ],
      },
    ],
  });
  render(<Search initial={{ ...view, query: "Leopold" }} />);
  const title = await screen.findByRole("button", { name: "Leopold opposes Thron" });
  expect(screen.queryByText("Structured context")).not.toBeInTheDocument();
  expect(screen.queryByText("Their authored note.")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Note/ }));
  expect(screen.getByText("Their authored note.")).toBeVisible();
  fireEvent.click(title);
  expect(open).toHaveBeenCalledWith(target);
});
it("collapses Chapter excerpts, expands bounded context, and remembers open rows", async () => {
  const hit = {
    ...result.groups[1].hits[0],
    preview: "A longer excerpt with surrounding context.",
  };
  vi.mocked(searchProject).mockResolvedValue({
    ...result,
    groups: [{ kind: "text", total: 1, hits: [hit] }],
  });
  render(<Search initial={{ ...view, query: "Captain" }} />);
  const row = await screen.findByRole("button", { name: /^Journey/ });
  expect(row).toHaveAttribute("aria-expanded", "false");
  fireEvent.click(row);
  expect(screen.getByText(hit.excerpt)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Longer preview" }));
  expect(screen.getByText(hit.preview)).toBeVisible();
  expect(screen.queryByText(hit.excerpt)).not.toBeInTheDocument();
  fireEvent.click(row);
  expect(screen.queryByText(hit.preview)).not.toBeInTheDocument();
  fireEvent.click(row);
  expect(screen.getByText(hit.preview)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Shorter preview" }));
  expect(screen.getByText(hit.excerpt)).toBeVisible();
  expect(searchProject).toHaveBeenCalledTimes(1);
});
it("filters before fetching and accepts a chosen count without resetting it on a new query", async () => {
  vi.mocked(searchProject).mockResolvedValue(result);
  render(<Search initial={{ ...view, query: "Captain" }} />);
  await screen.findByText("2 matching results");
  fireEvent.change(screen.getByLabelText("Fields & connections filter"), {
    target: { value: "relationships" },
  });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith(
      "p",
      expect.objectContaining({ structuredKind: "relationships" }),
    ),
  );
  const count = screen.getByRole("spinbutton", { name: "Results per section" });
  fireEvent.change(count, { target: { value: "150" } });
  fireEvent.keyDown(count, { key: "Enter" });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith(
      "p",
      expect.objectContaining({ limitPerGroup: 150 }),
    ),
  );
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Journey" } });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith(
      "p",
      expect.objectContaining({
        query: "Journey",
        limitPerGroup: 150,
        structuredKind: "relationships",
      }),
    ),
  );
  const calls = vi.mocked(searchProject).mock.calls.length;
  fireEvent.change(count, { target: { value: "0" } });
  fireEvent.blur(count);
  expect(screen.getByRole("alert")).toHaveTextContent("Choose a whole number");
  expect(searchProject).toHaveBeenCalledTimes(calls);
});
it("loads an Entry scope without a query and can return to the whole Project", async () => {
  vi.mocked(searchProject).mockResolvedValue(result);
  render(<Search initial={{ ...view, entryId: "e", entryName: "Wanderer" }} />);
  expect(screen.getByRole("heading", { name: "Search within Wanderer" })).toBeVisible();
  await waitFor(() =>
    expect(searchProject).toHaveBeenCalledWith("p", {
      query: "",
      includeInactive: false,
      limitPerGroup: 10,
      entryId: "e",
    }),
  );
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Captain" } });
  await screen.findByText("2 matching results");
  fireEvent.click(screen.getByRole("button", { name: "Search whole Project" }));
  expect(screen.getByRole("heading", { name: "Search your Project" })).toBeVisible();
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Captain",
      includeInactive: false,
      limitPerGroup: 10,
    }),
  );
});

it("labels each preview source and changes the text area without filtering out Entries", async () => {
  vi.mocked(searchProject).mockImplementation(async (_project, request) => ({
    ...result,
    groups: [
      result.groups[0],
      {
        ...result.groups[1],
        total: request.textArea === "plan" ? 0 : 1,
        hits: request.textArea === "plan" ? [] : result.groups[1].hits,
      },
    ],
  }));
  render(<Search initial={{ ...view, query: "Captain" }} />);
  expect(await screen.findByRole("button", { name: "Journey Preview from Notes" })).toBeVisible();
  expect(
    screen.getByText(/Previews come from the writing area matching your search/),
  ).toBeVisible();
  const source = screen.getByRole("combobox", { name: "Text previews" });
  expect(source).toHaveValue("all");
  fireEvent.change(source, { target: { value: "plan" } });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Captain",
      includeInactive: false,
      limitPerGroup: 10,
      textArea: "plan",
    }),
  );
  await screen.findByText("1 matching result");
  expect(screen.getByRole("button", { name: "Wanderer" })).toBeVisible();
  expect(screen.queryByRole("button", { name: /Journey/ })).not.toBeInTheDocument();
  expect(source).toBeVisible(); // Still reachable when the chosen area has no matches.
  fireEvent.change(source, { target: { value: "notes" } });
  const row = await screen.findByRole("button", { name: "Journey Preview from Notes" });
  fireEvent.click(row);
  fireEvent.click(screen.getByRole("button", { name: "Open Chapter · Notes" }));
  expect(open).toHaveBeenCalledWith({ kind: "chapter", chapterId: "c", area: "notes" });
  fireEvent.change(source, { target: { value: "all" } });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Captain",
      includeInactive: false,
      limitPerGroup: 10,
    }),
  );
});

it("finds unused Role definitions and offers exact usage navigation", async () => {
  vi.mocked(searchProject).mockResolvedValue({
    globalRevision: 3,
    groups: [
      {
        kind: "roles",
        total: 1,
        hits: [
          {
            key: "role:intro",
            title: "Intro",
            context: "Story Role · View Chapter uses",
            workspaceState: "active",
            reason: "Exact name",
            excerpt: "",
            target: { kind: "story_role", roleId: "intro", name: "Intro" },
          },
        ],
      },
    ],
  });
  render(<Search initial={{ ...view, query: "Intro" }} />);
  const role = await screen.findByRole("button", { name: "Intro" });
  expect(screen.getByRole("region", { name: "Story Roles" })).toHaveTextContent(
    "Creating a Role makes it available",
  );
  fireEvent.click(role);
  expect(open).toHaveBeenCalledWith({ kind: "story_role", roleId: "intro", name: "Intro" });
});
it("loads Role usage without a text query and explains how to assign an unused Role", async () => {
  vi.mocked(searchProject).mockResolvedValue({ globalRevision: 3, groups: [] });
  render(
    <Search
      initial={{
        ...view,
        storyRoleId: "intro",
        storyRoleName: "Intro",
        structuredKind: "chapters",
      }}
    />,
  );
  expect(screen.getByRole("heading", { name: "Chapters using Intro" })).toBeVisible();
  expect(await screen.findByText(/No matching Chapter links/)).toHaveTextContent(
    "choose Roles beside its name",
  );
  expect(searchProject).toHaveBeenCalledWith("p", {
    query: "",
    includeInactive: false,
    limitPerGroup: 10,
    storyRoleId: "intro",
    structuredKind: "chapters",
  });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Traveller" } });
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith(
      "p",
      expect.objectContaining({ storyRoleId: "intro", query: "Traveller" }),
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "Clear Role filter" }));
  await waitFor(() =>
    expect(searchProject).toHaveBeenLastCalledWith("p", {
      query: "Traveller",
      includeInactive: false,
      limitPerGroup: 10,
      structuredKind: "chapters",
    }),
  );
});
