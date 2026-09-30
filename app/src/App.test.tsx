import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Preferences, ProjectSummary } from "./types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

const project: ProjectSummary = {
  projectId: "0198c000-0000-7000-8000-000000000000",
  workingName: "Tortuga",
  revision: 0,
  packagePath: "/tmp/Tortuga.wcproj",
  formatVersion: 1,
  schemaVersion: 1,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-01T00:00:00Z",
};

const closeProjectMock = vi.fn();
const renameProjectMock = vi.fn();
const openProjectMock = vi.fn();
const nativeWindowCloseMock = vi.fn();
const onCloseRequestedMock = vi.fn();
const listCategoriesMock = vi.fn();
const listTypesMock = vi.fn();
const listEntriesMock = vi.fn();
const createCategoryMock = vi.fn();
const createTypeMock = vi.fn();
const createEntryMock = vi.fn();
const getEntryMock = vi.fn();
const updateEntryNameMock = vi.fn();
const changeEntryStructureMock = vi.fn();
const getPreferencesMock = vi.fn();
const pickDirectoryMock = vi.fn();
const setDefaultProjectsDirMock = vi.fn();
const setDefaultBackupsDirMock = vi.fn();
const resetPreferencesMock = vi.fn();
const previewPackagePathMock = vi.fn();
// Preferences are a convenience default; tests that don't care about them
// get a harmless "nothing configured" response so mount effects never throw.
getPreferencesMock.mockResolvedValue({
  defaultProjectsDir: null,
  defaultProjectsDirExists: false,
  defaultBackupsDir: null,
  defaultBackupsDirExists: false,
});
previewPackagePathMock.mockImplementation((baseDir: string, workingName: string) =>
  Promise.resolve(`${baseDir}/${workingName}.wcproj`),
);
let closeRequestedHandler: ((event: { preventDefault: () => void }) => void) | undefined;

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    destroy: (...args: unknown[]) => nativeWindowCloseMock(...args),
    onCloseRequested: (...args: unknown[]) => onCloseRequestedMock(...args),
  }),
}));

vi.mock("./api", () => ({
  listRecentProjects: vi.fn().mockResolvedValue([]),
  openRecentProject: vi.fn(),
  forgetRecentProject: vi.fn(),
  automaticBackupDirectory: vi.fn().mockResolvedValue("/Recovery"),
  deleteEntryField: vi.fn(),
  getAppearance: vi.fn().mockResolvedValue("storybook"),
  setAppearance: vi.fn().mockImplementation((appearance: string) => Promise.resolve(appearance)),
  readRelationships: vi
    .fn()
    .mockResolvedValue({ globalRevision: 1, relationships: [], definitions: [], entries: [] }),
  readProjectRelationships: vi
    .fn()
    .mockResolvedValue({ globalRevision: 1, definitions: [], relationships: [], entries: [] }),
  applyRelationships: vi.fn(),
  readFields: vi.fn().mockResolvedValue({ globalRevision: 1, fields: [], definitions: [] }),
  applyFields: vi.fn(),
  readFieldCatalog: vi.fn().mockResolvedValue({ globalRevision: 1, definitions: [] }),
  applyTemplateFields: vi.fn(),
  AppCommandError: class AppCommandError extends Error {
    kind: string;
    constructor(dto: { kind: string; message: string }) {
      super(dto.message);
      this.kind = dto.kind;
    }
  },
  createProject: vi.fn(),
  openProject: (...args: unknown[]) => openProjectMock(...args),
  restoreBackupAsCopy: vi.fn(),
  createBackup: vi.fn(),
  closeProject: (...args: unknown[]) => closeProjectMock(...args),
  renameProject: (...args: unknown[]) => renameProjectMock(...args),
  listCategories: (...args: unknown[]) => listCategoriesMock(...args),
  listTypes: (...args: unknown[]) => listTypesMock(...args),
  listEntries: (...args: unknown[]) => listEntriesMock(...args),
  createCategory: (...args: unknown[]) => createCategoryMock(...args),
  createType: (...args: unknown[]) => createTypeMock(...args),
  createEntry: (...args: unknown[]) => createEntryMock(...args),
  getEntry: (...args: unknown[]) => getEntryMock(...args),
  updateEntryName: (...args: unknown[]) => updateEntryNameMock(...args),
  changeEntryStructure: (...args: unknown[]) => changeEntryStructureMock(...args),
  getPreferences: (...args: unknown[]) => getPreferencesMock(...args),
  pickDirectory: (...args: unknown[]) => pickDirectoryMock(...args),
  setDefaultProjectsDir: (...args: unknown[]) => setDefaultProjectsDirMock(...args),
  setDefaultBackupsDir: (...args: unknown[]) => setDefaultBackupsDirMock(...args),
  resetPreferences: (...args: unknown[]) => resetPreferencesMock(...args),
  previewPackagePath: (...args: unknown[]) => previewPackagePathMock(...args),
}));

import App from "./App";
import {
  AppCommandError,
  deleteEntryField,
  readFieldCatalog,
  applyTemplateFields,
  createProject,
  readFields,
  applyFields,
  createBackup,
  readRelationships,
  applyRelationships,
  readProjectRelationships,
} from "./api";

function backendError(kind: string, message: string): AppCommandError {
  return new AppCommandError({ kind, message });
}

async function renderHomeAndFailOpen(kind: string) {
  openProjectMock.mockRejectedValueOnce(backendError(kind, `${kind} diagnostic detail`));
  render(<App />);
  fireEvent.change(visibleInput("open-project-path"), {
    target: { value: "/tmp/Tortuga.wcproj" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
  await waitFor(() => expect(screen.getAllByRole("alert").length).toBeGreaterThan(0));
}

async function openTheProjectScreen() {
  openProjectMock.mockResolvedValueOnce(project);
  const view = render(<App />);
  fireEvent.change(visibleInput("open-project-path"), {
    target: { value: "/tmp/Tortuga.wcproj" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
  await waitFor(() => screen.getByTestId("project-id"));
  await act(async () => Promise.resolve());
  return view;
}

function projectAction(name: string) {
  const menu = screen.getByText("Project menu").closest("details")!;
  if (!menu.open) fireEvent.click(menu.querySelector("summary")!);
  fireEvent.click(within(menu).getByRole("button", { name }));
}

function closeFromSettings() {
  if (screen.queryByRole("dialog", { name: "Project settings" }))
    fireEvent.click(screen.getByRole("button", { name: "Close Project settings" }));
  projectAction("Close Project");
}

function visibleInput(label: string) {
  if (label.startsWith("new-field-") && !screen.queryByRole("dialog", { name: "Add field" }))
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
  if (label.startsWith("new-entry-") && !screen.queryByRole("dialog", { name: "Add Entry" }))
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
  if (label === "entry-category" || label === "entry-type")
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
  if (label === "project-working-name") projectAction("Project settings");
  if (label === "backup-destination") projectAction("Backups");
  const input = screen.getByLabelText(label);
  const disclosure = input.closest("details");
  if (disclosure && !disclosure.open) fireEvent.click(disclosure.querySelector("summary")!);
  return input;
}

function enableTauriWindow() {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    value: {},
    configurable: true,
  });
}

function mockEditableEntry() {
  const entry = {
    id: "entry",
    categoryId: "characters",
    typeId: "human",
    authoredName: "Thron",
    displayName: "Thron",
    revision: 1,
    globalRevision: 1,
  };
  listCategoriesMock.mockResolvedValue([
    {
      id: "characters",
      name: "Characters",
      isUncategorized: false,
      revision: 1,
      globalRevision: 1,
    },
    {
      id: "places",
      name: "Places",
      isUncategorized: false,
      revision: 1,
      globalRevision: 1,
    },
  ]);
  listTypesMock.mockImplementation((_projectId: string, categoryId: string) =>
    Promise.resolve(
      categoryId === "characters"
        ? [
            {
              id: "human",
              categoryId: "characters",
              parentTypeId: null,
              name: "Human",
              revision: 1,
              globalRevision: 1,
            },
            {
              id: "mage",
              categoryId: "characters",
              parentTypeId: null,
              name: "Mage",
              revision: 1,
              globalRevision: 1,
            },
          ]
        : [],
    ),
  );
  listEntriesMock.mockResolvedValue([entry]);
  return entry;
}

describe("Project screen Saved contract", () => {
  it("keeps Close Project at the bottom of the menu, hidden until opened", async () => {
    await openTheProjectScreen();
    expect(screen.getByRole("button", { name: "Close Project" })).not.toBeVisible();
    fireEvent.click(screen.getByText("Project menu"));
    const close = screen.getByRole("button", { name: "Close Project" });
    expect(close).toBeVisible();
    expect(close.parentElement?.lastElementChild).toBe(close);
    expect(closeProjectMock).not.toHaveBeenCalled();
  });

  beforeEach(() => {
    vi.mocked(readFields)
      .mockReset()
      .mockResolvedValue({ globalRevision: 1, fields: [], definitions: [] });
    vi.mocked(applyFields).mockReset();
    vi.mocked(readFieldCatalog)
      .mockReset()
      .mockResolvedValue({ globalRevision: 1, definitions: [] });
    vi.mocked(applyTemplateFields).mockReset();
    vi.mocked(readRelationships)
      .mockReset()
      .mockResolvedValue({ globalRevision: 1, definitions: [], relationships: [], entries: [] });
    vi.mocked(applyRelationships).mockReset();
    closeProjectMock.mockReset();
    renameProjectMock.mockReset();
    openProjectMock.mockReset();
    listCategoriesMock.mockReset();
    listTypesMock.mockReset();
    listEntriesMock.mockReset();
    createCategoryMock.mockReset();
    createTypeMock.mockReset();
    createEntryMock.mockReset();
    getEntryMock.mockReset();
    updateEntryNameMock.mockReset();
    changeEntryStructureMock.mockReset();
    listCategoriesMock.mockResolvedValue([
      {
        id: "uncategorized",
        name: "Uncategorized",
        isUncategorized: true,
        revision: 0,
        globalRevision: 0,
      },
    ]);
    listTypesMock.mockResolvedValue([]);
    listEntriesMock.mockResolvedValue([]);
    nativeWindowCloseMock.mockReset();
    closeRequestedHandler = undefined;
    onCloseRequestedMock.mockReset();
    onCloseRequestedMock.mockImplementation(
      (handler: (event: { preventDefault: () => void }) => void) => {
        closeRequestedHandler = handler;
        return Promise.resolve(vi.fn());
      },
    );
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("filters by Category and exact Type, including untyped Entries, without changing their data", async () => {
    const entry = mockEditableEntry();
    listEntriesMock.mockResolvedValue([
      entry,
      { ...entry, id: "untyped", typeId: null, displayName: "Unsorted person" },
      { ...entry, id: "planet", categoryId: "places", typeId: null, displayName: "Planet" },
    ]);
    await openTheProjectScreen();
    await act(async () =>
      fireEvent.click(
        within(screen.getByRole("navigation", { name: "Project navigation" })).getByRole("button", {
          name: "Characters",
        }),
      ),
    );
    expect(screen.queryByRole("button", { name: "Planet" })).not.toBeInTheDocument();
    await act(async () =>
      fireEvent.change(screen.getByLabelText("Filter by Type"), { target: { value: "human" } }),
    );
    expect(screen.getByRole("button", { name: "Thron" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "Unsorted person" })).not.toBeInTheDocument();
    await act(async () =>
      fireEvent.change(screen.getByLabelText("Filter by Type"), { target: { value: "__untyped" } }),
    );
    expect(screen.getByRole("button", { name: "Unsorted person" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "Thron" })).not.toBeInTheDocument();
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
  });

  it("counts Entries across filters, collapses Categories, and creates in the chosen Category", async () => {
    const entry = mockEditableEntry();
    listEntriesMock.mockResolvedValue([
      entry,
      { ...entry, id: "person2" },
      { ...entry, id: "place", categoryId: "places" },
    ]);
    await openTheProjectScreen();
    const nav = within(screen.getByRole("navigation", { name: "Project navigation" }));
    expect(nav.getByRole("button", { name: "All Entries" })).toHaveAccessibleDescription(
      "3 Entries",
    );
    expect(nav.getByRole("button", { name: "Characters" })).toHaveAccessibleDescription(
      "2 Entries",
    );
    expect(nav.getByRole("button", { name: "Places" })).toHaveAccessibleDescription("1 Entries");
    fireEvent.click(nav.getByRole("button", { name: "Categories" }));
    expect(nav.queryByRole("button", { name: "Characters" })).not.toBeInTheDocument();
    expect(nav.getByRole("button", { name: "Relationships" })).toBeVisible();
    fireEvent.click(nav.getByRole("button", { name: "Categories" }));
    await act(async () =>
      fireEvent.click(nav.getByRole("button", { name: "Add Entry to Places" })),
    );
    expect(screen.getByRole("dialog", { name: "Add Entry" })).toBeVisible();
    expect(screen.getByLabelText("new-entry-category")).toHaveValue("places");
    fireEvent.change(screen.getByLabelText("new-entry-name"), { target: { value: "Harbor" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Add Entry" }));
    expect(nav.getByRole("button", { name: "Relationships" })).toBeDisabled();
    expect(nav.getByRole("button", { name: "Add Entry to Characters" })).toBeDisabled();
    expect(nav.getByRole("button", { name: "All Entries" })).toHaveAccessibleDescription(
      "3 Entries",
    );
  });

  it("protects Entry drafts on category plus and Relationships navigation", async () => {
    mockEditableEntry();
    await openTheProjectScreen();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Thron" })));
    await act(async () =>
      fireEvent.change(visibleInput("entry-category"), { target: { value: "places" } }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Close Entry settings" }));
    const nav = within(screen.getByRole("navigation", { name: "Project navigation" }));
    await act(async () => fireEvent.click(nav.getByRole("button", { name: "Relationships" })));
    expect(screen.getByRole("button", { name: "Discard and continue" })).toBeVisible();
    expect(screen.getByLabelText("entry-name")).toHaveValue("Thron");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await act(async () =>
      fireEvent.click(nav.getByRole("button", { name: "Add Entry to Places" })),
    );
    expect(screen.queryByRole("dialog", { name: "Add Entry" })).not.toBeInTheDocument();
    await act(async () =>
      fireEvent.click(screen.getByRole("button", { name: "Discard and continue" })),
    );
    expect(screen.getByLabelText("new-entry-category")).toHaveValue("places");
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
  });

  it("restores Relationships filters after visiting an Entry and reloads committed connections", async () => {
    const entry = mockEditableEntry();
    getEntryMock.mockResolvedValue(entry);
    vi.mocked(readProjectRelationships).mockResolvedValue({
      globalRevision: 1,
      definitions: [
        {
          id: "allies",
          name: "Alliance",
          forwardLabel: "allied with",
          inverseLabel: "allied with",
          directed: false,
          expectedTargetsPerSource: null,
          expectedSourcesPerTarget: null,
          retired: false,
          revision: 1,
        },
      ],
      relationships: [
        {
          id: "link",
          definitionId: "allies",
          source: { id: "entry", label: "Navigator", workspaceState: "active" },
          target: { id: "friend", label: "Captain", workspaceState: "active" },
          note: "",
          ended: false,
          workspaceState: "active",
          revision: 1,
          warnings: [],
        },
      ],
      entries: [],
    });
    await openTheProjectScreen();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Relationships" })));
    fireEvent.change(screen.getByLabelText("Relationship", { selector: "select" }), {
      target: { value: "allies" },
    });
    fireEvent.change(screen.getByLabelText("State"), { target: { value: "current" } });
    const participant = screen.getByRole("button", { name: "Navigator" });
    participant.focus();
    await act(async () => fireEvent.click(participant));
    expect(getEntryMock).toHaveBeenCalledWith(project.projectId, "entry");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "← Back" })));
    expect(screen.getByLabelText("Relationship", { selector: "select" })).toHaveValue("allies");
    expect(screen.getByLabelText("State")).toHaveValue("current");
    await waitFor(() => expect(screen.getByRole("button", { name: "Navigator" })).toHaveFocus());
    expect(vi.mocked(readProjectRelationships)).toHaveBeenCalledTimes(2);
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "All Entries" })));
    expect(screen.getByRole("heading", { name: "Entries" })).toBeVisible();
  });

  it("returns through Back and Forward with the Type filter, scroll and fresh Entry content", async () => {
    const entry = mockEditableEntry();
    getEntryMock.mockResolvedValue({
      ...entry,
      displayName: "Renamed",
      authoredName: "Renamed",
      revision: 2,
    });
    await openTheProjectScreen();
    await act(async () =>
      fireEvent.click(
        within(screen.getByRole("navigation", { name: "Project navigation" })).getByRole("button", {
          name: "Characters",
        }),
      ),
    );
    await act(async () =>
      fireEvent.change(screen.getByLabelText("Filter by Type"), { target: { value: "human" } }),
    );
    Object.defineProperty(window, "scrollY", { value: 430, configurable: true });
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Thron" })));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "← Back" })));
    expect(screen.getByLabelText("Filter by Type")).toHaveValue("human");
    await waitFor(() =>
      expect(window.scrollTo).toHaveBeenCalledWith({ top: 430, behavior: "instant" }),
    );
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Forward →" })));
    expect(screen.getByLabelText("entry-name")).toHaveValue("Renamed");
    expect(getEntryMock).toHaveBeenCalledWith(project.projectId, "entry");
    Object.defineProperty(window, "scrollY", { value: 0, configurable: true });
  });

  it("keeps history unchanged on failed navigation and protects an unapplied draft from sidebar navigation", async () => {
    const entry = mockEditableEntry();
    await openTheProjectScreen();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Thron" })));
    await act(async () =>
      fireEvent.change(visibleInput("entry-category"), { target: { value: "places" } }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Close Entry settings" }));
    await act(async () =>
      fireEvent.click(
        within(screen.getByRole("navigation", { name: "Project navigation" })).getByRole("button", {
          name: "Places",
        }),
      ),
    );
    expect(screen.getByLabelText("entry-name")).toHaveValue("Thron");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByLabelText("entry-name")).toHaveValue("Thron");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Back to Entries" })));
    await act(async () =>
      fireEvent.click(screen.getByRole("button", { name: "Discard and continue" })),
    );
    getEntryMock.mockRejectedValueOnce(new Error("Entry unavailable"));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "← Back" })));
    expect(screen.getByRole("heading", { name: "Entries" })).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent("Entry unavailable");
    getEntryMock.mockResolvedValueOnce(entry);
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "← Back" })));
    expect(screen.getByLabelText("entry-name")).toHaveValue("Thron");
  });

  it("follows a relationship then returns through Entry history without losing the browser filter", async () => {
    const entry = mockEditableEntry();
    const blade = {
      ...entry,
      id: "blade",
      categoryId: "places",
      typeId: null,
      displayName: "Survey Ship",
      authoredName: "Survey Ship",
    };
    getEntryMock.mockImplementation((_project: string, id: string) =>
      Promise.resolve(id === "blade" ? blade : entry),
    );
    vi.mocked(readRelationships).mockResolvedValue({
      globalRevision: 1,
      definitions: [
        {
          id: "command",
          name: "Command",
          forwardLabel: "commands",
          inverseLabel: "commanded by",
          directed: true,
          retired: false,
          revision: 1,
          expectedTargetsPerSource: null,
          expectedSourcesPerTarget: null,
        },
      ],
      relationships: [
        {
          id: "relation",
          definitionId: "command",
          source: { id: "entry", label: "Thron", workspaceState: "active" },
          target: { id: "blade", label: "Survey Ship", workspaceState: "active" },
          note: "",
          ended: false,
          workspaceState: "active",
          revision: 1,
          warnings: [],
        },
      ],
      entries: [],
    });
    await openTheProjectScreen();
    await act(async () =>
      fireEvent.click(
        within(screen.getByRole("navigation", { name: "Project navigation" })).getByRole("button", {
          name: "Characters",
        }),
      ),
    );
    await act(async () =>
      fireEvent.change(screen.getByLabelText("Filter by Type"), { target: { value: "human" } }),
    );
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Thron" })));
    await act(async () =>
      fireEvent.click(await screen.findByRole("button", { name: "Survey Ship" })),
    );
    expect(screen.getByLabelText("entry-name")).toHaveValue("Survey Ship");
    await act(async () => fireEvent.keyDown(window, { key: "ArrowLeft", altKey: true }));
    expect(screen.getByLabelText("entry-name")).toHaveValue("Thron");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "← Back" })));
    expect(screen.getByLabelText("Filter by Type")).toHaveValue("human");
    expect(screen.getByRole("button", { name: "Forward →" })).toBeEnabled();
  });

  it("waits for a name save before going Back and retains the draft when that save fails", async () => {
    const entry = mockEditableEntry();
    const pending = deferred<typeof entry>();
    updateEntryNameMock.mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Thron" })));
    fireEvent.change(screen.getByLabelText("entry-name"), { target: { value: "New name" } });
    fireEvent.keyDown(screen.getByLabelText("entry-name"), { key: "Enter" });
    await waitFor(() => expect(updateEntryNameMock).toHaveBeenCalledOnce());
    fireEvent.click(screen.getByRole("button", { name: "← Back" }));
    expect(screen.getByLabelText("entry-name")).toBeDisabled();
    await act(async () => pending.reject(new Error("disk full")));
    expect(screen.getByLabelText("entry-name")).toHaveValue("New name");
    expect(screen.getByLabelText("entry-name")).toBeEnabled();
    expect(screen.getByRole("button", { name: "Forward →" })).toBeDisabled();
    updateEntryNameMock.mockResolvedValueOnce({
      ...entry,
      authoredName: "New name",
      displayName: "New name",
      revision: 2,
      globalRevision: 2,
    });
    await act(async () =>
      fireEvent.click(screen.getByRole("button", { name: "Save and continue" })),
    );
    expect(screen.getByRole("heading", { name: "Entries" })).toBeVisible();
  });

  it("edits the title in place with committed Category and Type context", async () => {
    const entry = mockEditableEntry();
    const pending = deferred<typeof entry>();
    updateEntryNameMock.mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    const title = screen.getByLabelText("entry-name");
    expect(title.closest("h2")).toBe(screen.getByRole("heading", { name: "Thron" }));
    expect(screen.queryByText("Name (optional)")).not.toBeInTheDocument();
    expect(
      screen.getByText("Characters", { selector: ".entry-title-block .eyebrow" }),
    ).toBeVisible();
    expect(await screen.findByText("Human", { selector: ".entry-type" })).toBeVisible();
    title.focus();
    fireEvent.change(title, { target: { value: "Thron II" } });
    fireEvent.keyDown(title, { key: "Enter" });
    await waitFor(() =>
      expect(updateEntryNameMock).toHaveBeenCalledWith(project.projectId, "entry", "Thron II", 1),
    );
    expect(title).toBeEnabled();
    expect(title).toHaveFocus();
    fireEvent.change(title, { target: { value: "Thron III" } });
    await act(async () =>
      pending.resolve({
        ...entry,
        authoredName: "Thron II",
        displayName: "Thron II",
        revision: 2,
        globalRevision: 2,
      }),
    );
    expect(title).toHaveValue("Thron III");
    expect(title).toHaveFocus();
    updateEntryNameMock.mockResolvedValueOnce({
      ...entry,
      authoredName: "Thron III",
      displayName: "Thron III",
      revision: 3,
      globalRevision: 3,
    });
    fireEvent.keyDown(title, { key: "Enter" });
    await waitFor(() => expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saved"));
    await act(async () =>
      fireEvent.change(visibleInput("entry-category"), { target: { value: "places" } }),
    );
    expect(
      screen.getByText("Characters", { selector: ".entry-title-block .eyebrow" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Human", { selector: ".entry-type" })).toBeInTheDocument();
  });

  it("preserves a failed title draft and supports an unnamed title without a second name row", async () => {
    const entry = mockEditableEntry();
    updateEntryNameMock.mockRejectedValueOnce(new Error("Title save failed"));
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    const title = screen.getByLabelText("entry-name");
    fireEvent.change(title, { target: { value: "" } });
    await act(async () => fireEvent.keyDown(title, { key: "Enter" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Title save failed");
    expect(title).toHaveValue("");
    expect(title).toHaveAttribute("placeholder", "[Unnamed Entry]");
    updateEntryNameMock.mockResolvedValueOnce({
      ...entry,
      authoredName: null,
      displayName: "[Unnamed Entry]",
      revision: 2,
    });
    fireEvent.keyDown(title, { key: "Enter" });
    await waitFor(() => expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saved"));
    expect(screen.getByRole("heading", { name: "[Unnamed Entry]" })).toContainElement(title);
  });

  it("retains a dismissed Entry creation draft and protects native close", async () => {
    enableTauriWindow();
    await openTheProjectScreen();
    expect(screen.getByLabelText("new-entry-name")).not.toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    fireEvent.change(screen.getByLabelText("new-entry-name"), {
      target: { value: "A new world idea" },
    });
    fireEvent(
      screen.getByRole("dialog", { name: "Add Entry" }),
      new Event("cancel", { cancelable: true }),
    );
    expect(screen.getByRole("button", { name: "Continue Entry draft" })).toBeVisible();
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Save and close" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close Before you leave" }));
    fireEvent.click(screen.getByRole("button", { name: "Continue Entry draft" }));
    expect(screen.getByLabelText("new-entry-name")).toHaveValue("A new world idea");
    fireEvent.click(screen.getByRole("button", { name: "Cancel new Entry" }));
    expect(screen.queryByRole("dialog", { name: "Add Entry" })).not.toBeInTheDocument();
    expect(createEntryMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("save-state")).toHaveTextContent("Saved");
  });

  it("groups Entries under Category headings without repeating Category names on rows", async () => {
    const entry = mockEditableEntry();
    listEntriesMock.mockResolvedValue([
      entry,
      {
        ...entry,
        id: "city",
        displayName: "Uthlavik",
        authoredName: "Uthlavik",
        categoryId: "places",
        typeId: null,
      },
    ]);
    await openTheProjectScreen();
    const characters = await screen.findByRole("heading", { name: "Characters" });
    const places = screen.getByRole("heading", { name: "Places" });
    expect(characters.closest("details")).toContainElement(
      screen.getByRole("button", { name: "Thron" }),
    );
    expect(places.closest("details")).toContainElement(
      screen.getByRole("button", { name: "Uthlavik" }),
    );
    expect(screen.getByRole("button", { name: "Thron" })).toHaveTextContent(/^Thron$/);
    fireEvent.click(characters.closest("summary")!);
    expect(screen.getByText("Thron")).not.toBeVisible();
  });

  it("creates and selects a Type while editing an existing untyped Entry", async () => {
    const entry = { ...mockEditableEntry(), typeId: null };
    listEntriesMock.mockResolvedValue([entry]);
    createTypeMock.mockResolvedValue({
      id: "warrior",
      name: "Warrior",
      categoryId: "characters",
      parentTypeId: null,
      revision: 1,
      globalRevision: 2,
    });
    changeEntryStructureMock.mockResolvedValue({
      ...entry,
      typeId: "warrior",
      revision: 2,
      globalRevision: 3,
    });
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Create a Type in this Category" }));
    fireEvent.change(screen.getByLabelText("new-editor-type-name"), {
      target: { value: "Warrior" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Type and select" }));
    await waitFor(() => expect(screen.getByLabelText("entry-type")).toHaveValue("warrior"));
    fireEvent.click(screen.getByRole("button", { name: "Apply Category / Type" }));
    await waitFor(() =>
      expect(changeEntryStructureMock).toHaveBeenCalledWith(
        project.projectId,
        "entry",
        "characters",
        "warrior",
        1,
      ),
    );
  });

  it("adopts Category defaults on an open Entry immediately", async () => {
    mockEditableEntry();
    const def = {
      id: "mass",
      name: "Mass",
      kind: "number" as const,
      unit: "tons",
      retired: false,
      revision: 1,
      options: [],
      bindings: [],
    };
    vi.mocked(applyTemplateFields).mockResolvedValue({ globalRevision: 2, definitions: [def] });
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await waitFor(() =>
      expect(
        within(screen.getByText("Project menu").closest("details")!).getByRole("button", {
          name: "Categories",
          hidden: true,
        }),
      ).toBeEnabled(),
    );
    projectAction("Categories");
    fireEvent.click(await screen.findByRole("button", { name: "Add default field" }));
    fireEvent.change(screen.getByLabelText("Default field name"), { target: { value: "Mass" } });
    fireEvent.change(screen.getByLabelText("Default field kind"), { target: { value: "number" } });
    fireEvent.change(screen.getByLabelText("Default field unit"), { target: { value: "tons" } });
    vi.mocked(readFields).mockResolvedValue({
      globalRevision: 2,
      definitions: [def],
      fields: [{ definition: def, available: true, value: null }],
    });
    fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
    await waitFor(() => expect(applyTemplateFields).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Close Categories and defaults" }));
    expect(await screen.findByLabelText("Value: Mass")).toBeVisible();
    expect(screen.getByLabelText("Value: Mass")).toBeVisible();
  });

  it("native close waits for a Category default commit and blocks unfinished manager drafts", async () => {
    enableTauriWindow();
    mockEditableEntry();
    const pending = deferred<{ globalRevision: number; definitions: [] }>();
    vi.mocked(applyTemplateFields).mockReturnValue(pending.promise);
    closeProjectMock.mockResolvedValue(undefined);
    await openTheProjectScreen();
    projectAction("Categories");
    fireEvent.click(await screen.findByRole("button", { name: "Add default field" }));
    fireEvent.change(screen.getByLabelText("Default field name"), { target: { value: "Height" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Add default field" }));
    fireEvent.click(screen.getByRole("button", { name: "Close Categories and defaults" }));
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Save and close" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close Before you leave" }));
    projectAction("Categories");
    fireEvent.click(screen.getByRole("button", { name: "Continue manager draft" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Create default field" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create default field" }));
    await waitFor(() => expect(applyTemplateFields).toHaveBeenCalledOnce());
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    await act(async () => pending.resolve({ globalRevision: 2, definitions: [] }));
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    expect(nativeWindowCloseMock).toHaveBeenCalled();
  });

  it("native close waits for relationship creation to commit before exiting", async () => {
    enableTauriWindow();
    mockEditableEntry();
    closeProjectMock.mockResolvedValue(undefined);
    const updated = { globalRevision: 2, definitions: [], relationships: [], entries: [] };
    const pending = deferred<typeof updated>();
    vi.mocked(applyRelationships).mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Manage relationships" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Manage relationships" }));
    for (const [label, value] of [
      ["Definition name", "Ownership"],
      ["Forward label", "owns"],
      ["Inverse label", "is owned by"],
    ])
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Create definition" }));
    await waitFor(() => expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saving"));
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    await act(async () => pending.resolve(updated));
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    expect(nativeWindowCloseMock).toHaveBeenCalled();
  });

  it("uses acknowledged relationship revisions for the next Field write", async () => {
    mockEditableEntry();
    vi.mocked(applyRelationships).mockResolvedValue({
      globalRevision: 7,
      definitions: [],
      relationships: [],
      entries: [],
    });
    vi.mocked(applyFields).mockResolvedValue({ globalRevision: 8, definitions: [], fields: [] });
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Manage relationships" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Manage relationships" }));
    for (const [label, value] of [
      ["Definition name", "Ownership"],
      ["Forward label", "owns"],
      ["Inverse label", "is owned by"],
    ])
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Create definition" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    fireEvent.change(visibleInput("new-field-name"), { target: { value: "Color" } });
    fireEvent.click(screen.getByRole("button", { name: "Create field" }));
    await waitFor(() =>
      expect(applyFields).toHaveBeenCalledWith(
        project.projectId,
        "entry",
        7,
        expect.objectContaining({ kind: "create" }),
      ),
    );
  });

  it("native close waits for field-value acknowledgement and retains a failed draft", async () => {
    enableTauriWindow();
    mockEditableEntry();
    const snapshot = {
      globalRevision: 1,
      definitions: [],
      fields: [
        {
          definition: {
            id: "field",
            name: "Age",
            kind: "number" as const,
            revision: 1,
            retired: false,
            options: [],
            bindings: [],
          },
          available: true,
          value: null,
        },
      ],
    };
    vi.mocked(readFields).mockResolvedValue(snapshot);
    const pending = deferred<typeof snapshot>();
    vi.mocked(applyFields).mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.change(await screen.findByLabelText("Value: Age"), { target: { value: "43" } });
    fireEvent.blur(screen.getByLabelText("Value: Age"));
    await waitFor(() => expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saving"));
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    await act(async () => pending.reject(new Error("Disk full")));
    expect(screen.getByLabelText("Value: Age")).toHaveValue("43");
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Failed");
  });

  it("guards navigation with an unfinished field definition until explicit discard", async () => {
    mockEditableEntry();
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Add field" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "Add field" }));
    fireEvent.change(visibleInput("new-field-name"), {
      target: { value: "Unsaved local field" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Close Add field" }));
    fireEvent.click(screen.getByRole("button", { name: "Back to Entries" }));
    expect(screen.getByLabelText("new-field-name")).toHaveValue("Unsaved local field");
    fireEvent.click(screen.getByRole("button", { name: "Discard and continue" }));
    await waitFor(() => expect(screen.queryByLabelText("new-field-name")).not.toBeInTheDocument());
    expect(applyFields).not.toHaveBeenCalled();
  });

  it("creates a missing Category and Type inline without losing the Entry draft", async () => {
    createCategoryMock.mockResolvedValue({
      id: "characters",
      name: "Characters",
      isUncategorized: false,
      revision: 1,
      globalRevision: 1,
    });
    createTypeMock.mockResolvedValue({
      id: "human",
      categoryId: "characters",
      parentTypeId: null,
      name: "Human",
      revision: 1,
      globalRevision: 2,
    });
    createEntryMock.mockResolvedValue({
      id: "entry-thron",
      categoryId: "characters",
      typeId: "human",
      authoredName: "Thron",
      displayName: "Thron",
      revision: 1,
      globalRevision: 3,
    });
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => expect(visibleInput("new-entry-name")).toBeInTheDocument());
    fireEvent.change(visibleInput("new-entry-name"), { target: { value: "Thron" } });

    fireEvent.click(screen.getByRole("button", { name: "Create Category inline" }));
    fireEvent.change(screen.getByLabelText("inline-category-name"), {
      target: { value: "Characters" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add Category" }));
    await waitFor(() => expect(createCategoryMock).toHaveBeenCalled());
    expect(visibleInput("new-entry-name")).toHaveValue("Thron");

    fireEvent.click(screen.getByRole("button", { name: "Create Type inline" }));
    fireEvent.change(screen.getByLabelText("inline-type-name"), { target: { value: "Human" } });
    fireEvent.click(screen.getByRole("button", { name: "Add Type" }));
    await waitFor(() =>
      expect(createTypeMock).toHaveBeenCalledWith(project.projectId, "characters", "Human"),
    );
    expect(visibleInput("new-entry-name")).toHaveValue("Thron");

    fireEvent.click(screen.getByRole("button", { name: "Create Entry" }));
    await waitFor(() =>
      expect(createEntryMock).toHaveBeenCalledWith(
        project.projectId,
        "Thron",
        "characters",
        "human",
      ),
    );
    expect(await screen.findByText("Entry ID: entry-thron")).toBeInTheDocument();
  });

  it("creates an incomplete unnamed Entry", async () => {
    createEntryMock.mockResolvedValue({
      id: "entry-unnamed",
      categoryId: "uncategorized",
      typeId: null,
      authoredName: null,
      displayName: "[Unnamed Entry]",
      revision: 1,
      globalRevision: 1,
    });
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => screen.getByRole("button", { name: "Create Entry" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Entry" }));
    await waitFor(() =>
      expect(createEntryMock).toHaveBeenCalledWith(
        project.projectId,
        undefined,
        "uncategorized",
        undefined,
      ),
    );
    expect(await screen.findByPlaceholderText("[Unnamed Entry]")).toHaveValue("");
  });

  it("native close waits for submitted Category creation", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValue(undefined);
    const categorySave = deferred<{
      id: string;
      name: string;
      isUncategorized: boolean;
      revision: number;
      globalRevision: number;
    }>();
    createCategoryMock.mockReturnValue(categorySave.promise);
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => screen.getByRole("button", { name: "Create Category inline" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Category inline" }));
    fireEvent.change(screen.getByLabelText("inline-category-name"), {
      target: { value: "Characters" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add Category" }));
    await waitFor(() => expect(createCategoryMock).toHaveBeenCalledTimes(1));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(closeProjectMock).not.toHaveBeenCalled();
    categorySave.resolve({
      id: "characters",
      name: "Characters",
      isUncategorized: false,
      revision: 1,
      globalRevision: 1,
    });
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("in-app close waits for submitted Entry creation", async () => {
    closeProjectMock.mockResolvedValue(undefined);
    const entrySave = deferred<{
      id: string;
      categoryId: string;
      typeId: null;
      authoredName: null;
      displayName: string;
      revision: number;
      globalRevision: number;
    }>();
    createEntryMock.mockReturnValue(entrySave.promise);
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => screen.getByRole("button", { name: "Create Entry" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Entry" }));
    closeFromSettings();
    expect(closeProjectMock).not.toHaveBeenCalled();
    entrySave.resolve({
      id: "entry",
      categoryId: "uncategorized",
      typeId: null,
      authoredName: null,
      displayName: "[Unnamed Entry]",
      revision: 1,
      globalRevision: 1,
    });
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
  });

  it("Entry navigation waits for an in-flight Category and Type change", async () => {
    const entry = {
      id: "entry",
      categoryId: "uncategorized",
      typeId: null,
      authoredName: "Thron",
      displayName: "Thron",
      revision: 1,
      globalRevision: 1,
    };
    listCategoriesMock.mockResolvedValue([
      {
        id: "uncategorized",
        name: "Uncategorized",
        isUncategorized: true,
        revision: 0,
        globalRevision: 0,
      },
      {
        id: "characters",
        name: "Characters",
        isUncategorized: false,
        revision: 1,
        globalRevision: 1,
      },
    ]);
    listEntriesMock.mockResolvedValue([entry]);
    const structureSave = deferred<typeof entry>();
    changeEntryStructureMock.mockReturnValue(structureSave.promise);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.change(visibleInput("entry-category"), {
      target: { value: "characters" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply Category / Type" }));
    await waitFor(() => expect(changeEntryStructureMock).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByRole("button", { name: "Back to Entries" }));
    expect(screen.getByText("Entry ID: entry")).toBeInTheDocument();
    structureSave.resolve({
      ...entry,
      categoryId: "characters",
      revision: 2,
      globalRevision: 2,
    });
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Entries" })).toBeInTheDocument(),
    );
  });

  it("structural failure stays open, is visible, and cannot be described as discarded", async () => {
    createEntryMock.mockRejectedValue(new Error("disk full"));
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Create Entry" })));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("disk full"));
    fireEvent.click(screen.getByRole("button", { name: "Close Add Entry" }));
    closeFromSettings();
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "Close Project anyway (discard changes)" }),
    ).toBeInTheDocument();
  });

  it("guards repeated Category, Type, and Entry submissions", async () => {
    const categorySave = deferred<{
      id: string;
      name: string;
      isUncategorized: boolean;
      revision: number;
      globalRevision: number;
    }>();
    createCategoryMock.mockReturnValue(categorySave.promise);
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    fireEvent.click(await screen.findByRole("button", { name: "Create Category inline" }));
    fireEvent.change(screen.getByLabelText("inline-category-name"), { target: { value: "A" } });
    const addCategory = screen.getByRole("button", { name: "Add Category" });
    fireEvent.click(addCategory);
    fireEvent.click(addCategory);
    expect(createCategoryMock).toHaveBeenCalledTimes(1);
    categorySave.resolve({
      id: "characters",
      name: "A",
      isUncategorized: false,
      revision: 1,
      globalRevision: 1,
    });
    await waitFor(() =>
      expect(screen.queryByLabelText("inline-category-name")).not.toBeInTheDocument(),
    );

    const typeSave = deferred<{
      id: string;
      categoryId: string;
      parentTypeId: null;
      name: string;
      revision: number;
      globalRevision: number;
    }>();
    createTypeMock.mockReturnValue(typeSave.promise);
    fireEvent.click(screen.getByRole("button", { name: "Create Type inline" }));
    fireEvent.change(screen.getByLabelText("inline-type-name"), { target: { value: "Human" } });
    const addType = screen.getByRole("button", { name: "Add Type" });
    fireEvent.click(addType);
    fireEvent.click(addType);
    expect(createTypeMock).toHaveBeenCalledTimes(1);
    typeSave.resolve({
      id: "human",
      categoryId: "characters",
      parentTypeId: null,
      name: "Human",
      revision: 1,
      globalRevision: 2,
    });
    await waitFor(() =>
      expect(screen.queryByLabelText("inline-type-name")).not.toBeInTheDocument(),
    );

    const entrySave = deferred<{
      id: string;
      categoryId: string;
      typeId: string;
      authoredName: string;
      displayName: string;
      revision: number;
      globalRevision: number;
    }>();
    createEntryMock.mockReturnValue(entrySave.promise);
    const addEntry = screen.getByRole("button", { name: "Create Entry" });
    fireEvent.click(addEntry);
    fireEvent.click(addEntry);
    expect(createEntryMock).toHaveBeenCalledTimes(1);
    entrySave.resolve({
      id: "thron",
      categoryId: "characters",
      typeId: "human",
      authoredName: "Thron",
      displayName: "Thron",
      revision: 1,
      globalRevision: 3,
    });
    expect(await screen.findByText("Entry ID: thron")).toBeInTheDocument();
  });

  it("does not allow an old Type-list response to replace the current Category options", async () => {
    listCategoriesMock.mockResolvedValue([
      {
        id: "uncategorized",
        name: "Uncategorized",
        isUncategorized: true,
        revision: 0,
        globalRevision: 0,
      },
      {
        id: "characters",
        name: "Characters",
        isUncategorized: false,
        revision: 1,
        globalRevision: 1,
      },
    ]);
    const oldTypes = deferred<
      Array<{
        id: string;
        categoryId: string;
        parentTypeId: null;
        name: string;
        revision: number;
        globalRevision: number;
      }>
    >();
    const currentTypes = deferred<
      Array<{
        id: string;
        categoryId: string;
        parentTypeId: null;
        name: string;
        revision: number;
        globalRevision: number;
      }>
    >();
    listTypesMock.mockImplementation((_projectId: string, categoryId: string) =>
      categoryId === "characters" ? currentTypes.promise : oldTypes.promise,
    );
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    const categorySelect = await screen.findByLabelText("new-entry-category");
    await screen.findByRole("option", { name: "Characters" });
    fireEvent.change(categorySelect, { target: { value: "characters" } });
    currentTypes.resolve([
      {
        id: "human",
        categoryId: "characters",
        parentTypeId: null,
        name: "Human",
        revision: 1,
        globalRevision: 1,
      },
    ]);
    expect(await screen.findByRole("option", { name: "Human" })).toBeInTheDocument();
    oldTypes.resolve([
      {
        id: "old",
        categoryId: "uncategorized",
        parentTypeId: null,
        name: "Old Type",
        revision: 1,
        globalRevision: 1,
      },
    ]);
    await act(async () => Promise.resolve());
    expect(screen.getByRole("option", { name: "Human" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Old Type" })).not.toBeInTheDocument();
  });

  it("requires explicit discard when navigating Back with an unapplied Category", async () => {
    mockEditableEntry();
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await act(async () =>
      fireEvent.change(visibleInput("entry-category"), { target: { value: "places" } }),
    );
    await act(async () => Promise.resolve());
    expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Pending");
    expect(screen.getByTestId("save-state")).toHaveTextContent("Pending");

    fireEvent.click(screen.getByRole("button", { name: "Back to Entries" }));
    expect(screen.getByRole("button", { name: "Discard and continue" })).toBeInTheDocument();
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Discard and continue" }));
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    await screen.findByRole("option", { name: "Human" });
    expect(visibleInput("entry-category")).toHaveValue("characters");
    expect(visibleInput("entry-type")).toHaveValue("human");
  });

  it("requires explicit discard on native close with an unapplied Type", async () => {
    enableTauriWindow();
    mockEditableEntry();
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    await screen.findByRole("option", { name: "Mage" });
    fireEvent.change(visibleInput("entry-type"), { target: { value: "mage" } });

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "Close app anyway (discard changes)" }),
    ).toBeInTheDocument();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
  });

  it("clears structural dirty state when selectors return to persisted values", async () => {
    mockEditableEntry();
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    await screen.findByRole("option", { name: "Mage" });
    fireEvent.change(visibleInput("entry-type"), { target: { value: "mage" } });
    expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Pending");
    fireEvent.change(visibleInput("entry-type"), { target: { value: "human" } });
    expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saved");
    expect(screen.getByTestId("save-state")).toHaveTextContent("Saved");
  });

  it("locks selectors during Apply and synchronizes the committed structure", async () => {
    const entry = mockEditableEntry();
    const pending = deferred<typeof entry>();
    changeEntryStructureMock.mockReturnValue(pending.promise);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    await screen.findByRole("option", { name: "Mage" });
    fireEvent.change(visibleInput("entry-type"), { target: { value: "mage" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply Category / Type" }));

    await waitFor(() => expect(visibleInput("entry-category")).toBeDisabled());
    expect(visibleInput("entry-type")).toBeDisabled();
    expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saving");

    pending.resolve({ ...entry, typeId: "mage", revision: 2, globalRevision: 2 });
    await waitFor(() => expect(screen.getByTestId("entry-save-state")).toHaveTextContent("Saved"));
    expect(visibleInput("entry-category")).toHaveValue("characters");
    expect(visibleInput("entry-type")).toHaveValue("mage");
    expect(screen.getByTestId("save-state")).toHaveTextContent("Saved");
  });

  it("allows explicit discard-close after structural failure and preserves failure on cancel", async () => {
    mockEditableEntry();
    changeEntryStructureMock.mockRejectedValue(new Error("structure write failed"));
    closeProjectMock.mockResolvedValue(undefined);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.click(screen.getByRole("button", { name: "Entry settings" }));
    await screen.findByRole("option", { name: "Mage" });
    fireEvent.change(visibleInput("entry-type"), { target: { value: "mage" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply Category / Type" }));
    await waitFor(() => expect(screen.getByText("structure write failed")).toBeInTheDocument());
    await waitFor(() =>
      expect(screen.getByTestId("save-state")).toHaveTextContent("Failed to save"),
    );

    closeFromSettings();
    const discard = screen.getByRole("button", {
      name: "Close Project anyway (discard changes)",
    });
    expect(discard).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByText("structure write failed")).toBeInTheDocument();
    expect(screen.getByTestId("save-state")).toHaveTextContent("Failed to save");
    expect(closeProjectMock).not.toHaveBeenCalled();

    closeFromSettings();
    fireEvent.click(screen.getByRole("button", { name: "Close Project anyway (discard changes)" }));
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
  });

  it("keeps a structural draft dirty after an overlapping name save settles", async () => {
    const entry = mockEditableEntry();
    const pendingName = deferred<typeof entry>();
    updateEntryNameMock.mockReturnValue(pendingName.promise);
    closeProjectMock.mockResolvedValue(undefined);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.change(screen.getByLabelText("entry-name"), { target: { value: "Thron II" } });
    await act(async () =>
      fireEvent.change(visibleInput("entry-category"), { target: { value: "places" } }),
    );
    closeFromSettings();
    expect(closeProjectMock).not.toHaveBeenCalled();

    pendingName.resolve({
      ...entry,
      authoredName: "Thron II",
      displayName: "Thron II",
      revision: 2,
      globalRevision: 2,
    });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Close Project anyway (discard changes)" }),
      ).toBeInTheDocument(),
    );
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.getByTestId("save-state")).toHaveTextContent("Pending");
  });

  it("handles overlapping name and structural saves without claiming either was discarded", async () => {
    const entry = {
      id: "entry",
      categoryId: "uncategorized",
      typeId: null,
      authoredName: "Thron",
      displayName: "Thron",
      revision: 1,
      globalRevision: 1,
    };
    listEntriesMock.mockResolvedValue([entry]);
    listCategoriesMock.mockResolvedValue([
      {
        id: "uncategorized",
        name: "Uncategorized",
        isUncategorized: true,
        revision: 0,
        globalRevision: 0,
      },
      {
        id: "characters",
        name: "Characters",
        isUncategorized: false,
        revision: 1,
        globalRevision: 1,
      },
    ]);
    const nameSave = deferred<typeof entry>();
    const structureSave = deferred<typeof entry>();
    updateEntryNameMock.mockReturnValue(nameSave.promise);
    changeEntryStructureMock.mockReturnValue(structureSave.promise);
    closeProjectMock.mockResolvedValue(undefined);
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    fireEvent.change(screen.getByLabelText("entry-name"), { target: { value: "Thron II" } });
    fireEvent.change(visibleInput("entry-category"), {
      target: { value: "characters" },
    });
    fireEvent.change(visibleInput("entry-type"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply Category / Type" }));
    await waitFor(() => expect(updateEntryNameMock).toHaveBeenCalledTimes(1));
    expect(changeEntryStructureMock).not.toHaveBeenCalled();
    expect(visibleInput("entry-category")).toBeDisabled();
    expect(visibleInput("entry-type")).toBeDisabled();
    expect(screen.getByLabelText("entry-name")).toBeDisabled();
    nameSave.resolve({
      ...entry,
      authoredName: "Thron II",
      displayName: "Thron II",
      revision: 2,
      globalRevision: 2,
    });
    await waitFor(() => expect(changeEntryStructureMock).toHaveBeenCalledTimes(1));
    closeFromSettings();
    expect(closeProjectMock).not.toHaveBeenCalled();
    structureSave.resolve({
      ...entry,
      authoredName: "Thron II",
      displayName: "Thron II",
      revision: 3,
      globalRevision: 3,
    });
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
  });

  it("shows the Project ID unchanged after a successful rename", async () => {
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });

    await openTheProjectScreen();
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saved"));
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);
  });

  it("refuses to silently close when a rename has failed and is still pending", async () => {
    renameProjectMock.mockRejectedValueOnce(new Error("disk full"));

    await openTheProjectScreen();

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Broken Rename" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(screen.getByTestId("save-state").textContent).toBe("Failed to save"),
    );

    closeFromSettings();

    // Close must be blocked/warned, not silently succeed.
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the Project/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Close Project anyway (discard changes)" }),
    ).toBeInTheDocument();
  });

  it("shows a window-close failure on Home after safely closing the Project", async () => {
    enableTauriWindow();
    nativeWindowCloseMock.mockRejectedValueOnce(new Error("window close denied"));
    await openTheProjectScreen();
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Open Project" })).toBeVisible(),
    );
    expect(closeProjectMock).toHaveBeenCalledWith(project.projectId);
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Your Project is closed, but the app window could not close",
    );
    expect(screen.getByRole("alert")).toHaveTextContent("window close denied");
    expect(screen.queryByTestId("save-state")).not.toBeInTheDocument();
  });

  it("keeps native close intent when the app close request is confirmed", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);

    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Unsaved Rename" },
    });

    const preventDefault = vi.fn();
    await act(async () => {
      closeRequestedHandler?.({ preventDefault });
    });

    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the app/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));

    await waitFor(() =>
      expect(closeProjectMock).toHaveBeenCalledWith("0198c000-0000-7000-8000-000000000000"),
    );
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not downgrade a pending native close when the in-app button is clicked", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));
    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Unsaved Rename" },
    });
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));
    closeFromSettings();
    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not call the rename API merely because a dirty draft blurred during native close", async () => {
    enableTauriWindow();
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = visibleInput("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    // Native window closing blurs the focused input before/while handling
    // the close: simulate that here. This must not trigger a save.
    fireEvent.blur(input);
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    expect(renameProjectMock).not.toHaveBeenCalled();
    expect(screen.getByText(/before closing the app/i)).toBeInTheDocument();
  });

  it("confirming discard from a dirty draft closes without ever submitting it", async () => {
    enableTauriWindow();
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = visibleInput("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.blur(input);
    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    fireEvent.click(screen.getByRole("button", { name: "Close app anyway (discard changes)" }));

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
    expect(renameProjectMock).not.toHaveBeenCalled();
  });

  it("in-app Close Project discards a dirty draft without submitting it", async () => {
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();

    const input = visibleInput("project-working-name");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.blur(input);

    closeFromSettings();
    fireEvent.click(screen.getByRole("button", { name: "Close Project anyway (discard changes)" }));

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    expect(renameProjectMock).not.toHaveBeenCalled();
  });

  it("waits for an explicit save already in flight instead of pretending to cancel it", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    const preventDefault = vi.fn();
    await act(async () => closeRequestedHandler?.({ preventDefault }));

    // While the save is in flight, close must not offer/claim a discard.
    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
    expect(
      screen.queryByRole("button", { name: "Close app anyway (discard changes)" }),
    ).not.toBeInTheDocument();

    closeProjectMock.mockResolvedValueOnce(undefined);
    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
      await Promise.resolve();
    });

    // A successful in-flight save completes the pending close afterwards.
    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("keeps the app open with an honest failed state when the in-flight save fails", async () => {
    enableTauriWindow();
    let rejectRename!: (reason: Error) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectRename = reject;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    await act(async () => {
      rejectRename(new Error("disk full"));
      await Promise.resolve();
    });

    // Must not close, and must not claim the failed save was "discarded".
    await waitFor(() =>
      expect(screen.getByTestId("save-state").textContent).toBe("Failed to save"),
    );
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
  });

  it("completes a successful in-flight save's pending close from the explicit outcome, not a rerender", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    closeProjectMock.mockResolvedValueOnce(undefined);
    // Resolve the rename and flush its `.then` continuation, but do not
    // flush any subsequent scheduler/render pass first: if the close
    // continuation depended on `saveState` having already re-rendered
    // rather than on the explicit resolved outcome, this would still show
    // the race. It must still complete the close correctly.
    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
    });

    await waitFor(() => expect(closeProjectMock).toHaveBeenCalledWith(project.projectId));
    await waitFor(() => expect(nativeWindowCloseMock).toHaveBeenCalledTimes(1));
  });

  it("does not close when a newer draft was typed while an older save was in flight", async () => {
    enableTauriWindow();
    let resolveRename!: (value: typeof project) => void;
    renameProjectMock.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveRename = resolve;
      }),
    );
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    const input = visibleInput("project-working-name");
    fireEvent.change(input, { target: { value: "Tortuga Prime" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saving…"));

    await act(async () => closeRequestedHandler?.({ preventDefault: vi.fn() }));

    // A newer draft appears while the older submit is still in flight.
    fireEvent.change(input, { target: { value: "Tortuga Prime Newer" } });

    await act(async () => {
      resolveRename({ ...project, workingName: "Tortuga Prime", revision: 1 });
    });

    // The older save committed, but the currently displayed draft never
    // did: closing now would silently discard "Tortuga Prime Newer".
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Pending"));
    expect(closeProjectMock).not.toHaveBeenCalled();
    expect(nativeWindowCloseMock).not.toHaveBeenCalled();
  });

  it("does not re-register the native close listener merely because the draft or save state changes", async () => {
    enableTauriWindow();
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });
    await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    fireEvent.change(visibleInput("project-working-name"), {
      target: { value: "Tortuga Prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByTestId("save-state").textContent).toBe("Saved"));

    // Typing (dirty), saving, and settling back to saved must not churn the
    // native close listener effect: it should register exactly once.
    expect(onCloseRequestedMock).toHaveBeenCalledTimes(1);
  });

  it("cleans up a native close listener even when registration resolves after unmount", async () => {
    enableTauriWindow();
    let resolveListener!: (listener: () => void) => void;
    onCloseRequestedMock.mockImplementationOnce(
      (handler: (event: { preventDefault: () => void }) => void) => {
        closeRequestedHandler = handler;
        return new Promise<() => void>((resolve) => {
          resolveListener = resolve;
        });
      },
    );

    const view = await openTheProjectScreen();
    await waitFor(() => expect(onCloseRequestedMock).toHaveBeenCalledTimes(1));

    view.unmount();

    const lateUnlisten = vi.fn();
    resolveListener(lateUnlisten);

    await waitFor(() => expect(lateUnlisten).toHaveBeenCalledTimes(1));
  });
});

describe("Home screen stale-lock recovery", () => {
  beforeEach(() => {
    openProjectMock.mockReset();
  });

  it("offers explicit recovery only when the backend reports lock_recovery_required", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");

    // The entered path is retained after the failed open.
    expect(visibleInput("open-project-path")).toHaveValue("/tmp/Tortuga.wcproj");

    // Understandable wording leads; jargon stays out of the primary text.
    expect(screen.getByText(/not closed properly/i)).toBeInTheDocument();
    expect(screen.getByText(/lock_recovery_required diagnostic detail/)).toBeInTheDocument();

    const recover = screen.getByRole("button", { name: "Recover lock and open Project" });
    expect(recover).toBeInTheDocument();
    // The safety caveat is shown next to the action.
    expect(screen.getByText(/no other Worldcrafter instance/i)).toBeInTheDocument();

    // The recovery action opens the same path with forceStaleLockRecovery=true.
    openProjectMock.mockResolvedValueOnce(project);
    fireEvent.click(recover);
    await waitFor(() => screen.getByTestId("project-id"));
    expect(openProjectMock).toHaveBeenLastCalledWith("/tmp/Tortuga.wcproj", true);
    expect(screen.getByTestId("project-id").textContent).toBe(project.projectId);
  });

  it("invalidates recovery immediately when the Package path is edited", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");
    const path = visibleInput("open-project-path");

    fireEvent.change(path, { target: { value: "/tmp/Other.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();

    fireEvent.change(path, { target: { value: "/tmp/Tortuga.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
  });

  it("never authorizes recovery for an old path whose request finishes after an edit", async () => {
    let rejectOpen: ((reason: AppCommandError) => void) | undefined;
    openProjectMock.mockReturnValueOnce(
      new Promise((_resolve, reject) => {
        rejectOpen = reject;
      }),
    );
    render(<App />);
    const path = visibleInput("open-project-path");
    fireEvent.change(path, { target: { value: "/tmp/Old.wcproj" } });
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));

    fireEvent.change(path, { target: { value: "/tmp/New.wcproj" } });
    rejectOpen?.(backendError("lock_recovery_required", "old path diagnostic"));
    await waitFor(() => expect(screen.getByText(/not closed properly/i)).toBeInTheDocument());

    fireEvent.change(path, { target: { value: "/tmp/Old.wcproj" } });
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    expect(openProjectMock).toHaveBeenCalledWith("/tmp/Old.wcproj");
  });

  it.each([
    ["lock_held", /currently open in another Worldcrafter instance/i],
    ["lock_metadata_corrupt", /lock information is unreadable/i],
  ])("offers no unsafe recovery action for %s", async (kind, wording) => {
    await renderHomeAndFailOpen(kind);

    expect(screen.getByText(wording)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    // The path is retained so the user can retry or edit it.
    expect(visibleInput("open-project-path")).toHaveValue("/tmp/Tortuga.wcproj");
  });

  it.each([
    ["lock_held", "held by pid 42"],
    ["lock_metadata_corrupt", "invalid lock metadata"],
    ["invalid_package", "unrelated open failure"],
  ])("clears recovery when the latest recovery attempt fails with %s", async (kind, diagnostic) => {
    openProjectMock.mockRejectedValueOnce(
      backendError("lock_recovery_required", "stale lock diagnostic"),
    );
    render(<App />);
    fireEvent.change(visibleInput("open-project-path"), {
      target: { value: "/tmp/Tortuga.wcproj" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
    await waitFor(() => screen.getByRole("button", { name: "Recover lock and open Project" }));

    openProjectMock.mockRejectedValueOnce(backendError(kind, diagnostic));
    fireEvent.click(screen.getByRole("button", { name: "Recover lock and open Project" }));

    await waitFor(() => expect(screen.getByText(new RegExp(diagnostic))).toBeInTheDocument());
    expect(openProjectMock).toHaveBeenLastCalledWith("/tmp/Tortuga.wcproj", true);
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId("project-id")).not.toBeInTheDocument();
  });

  it("retains recovery only when the latest failure still requires it for the unchanged path", async () => {
    await renderHomeAndFailOpen("lock_recovery_required");
    openProjectMock.mockRejectedValueOnce(
      backendError("lock_recovery_required", "still stale diagnostic"),
    );

    fireEvent.click(screen.getByRole("button", { name: "Recover lock and open Project" }));

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Recover lock and open Project" }),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText(/still stale diagnostic/)).toBeInTheDocument();
  });

  it("does not offer recovery for non-lock open failures", async () => {
    await renderHomeAndFailOpen("invalid_package");
    expect(screen.getByText(/invalid_package/)).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Recover lock and open Project" }),
    ).not.toBeInTheDocument();
  });
});

describe("Home screen preferences and native pickers", () => {
  const defaults: Preferences = {
    defaultProjectsDir: null,
    defaultProjectsDirExists: false,
    defaultBackupsDir: null,
    defaultBackupsDirExists: false,
  };
  beforeEach(() => {
    getPreferencesMock.mockReset();
    pickDirectoryMock.mockReset();
    setDefaultProjectsDirMock.mockReset();
    setDefaultBackupsDirMock.mockReset();
    resetPreferencesMock.mockReset();
    (createProject as ReturnType<typeof vi.fn>).mockReset();
    getPreferencesMock.mockResolvedValue({
      defaultProjectsDir: null,
      defaultProjectsDirExists: false,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
  });

  it("preserves manual input when the initial preferences request resolves late", async () => {
    const pending = deferred<Preferences>();
    getPreferencesMock.mockReturnValueOnce(pending.promise);
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/manual" },
    });
    await act(async () =>
      pending.resolve({
        ...defaults,
        defaultProjectsDir: "/default",
        defaultProjectsDirExists: true,
      }),
    );
    expect(visibleInput("new-project-location")).toHaveValue("/manual");
  });

  it("does not let an old initial load overwrite a newly selected default", async () => {
    const pending = deferred<Preferences>();
    getPreferencesMock.mockReturnValueOnce(pending.promise);
    pickDirectoryMock.mockResolvedValueOnce("/new");
    setDefaultProjectsDirMock.mockResolvedValueOnce({
      ...defaults,
      defaultProjectsDir: "/new",
      defaultProjectsDirExists: true,
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Choose…" })[0]);
    await waitFor(() => expect(visibleInput("new-project-location")).toHaveValue("/new"));
    await act(async () =>
      pending.resolve({ ...defaults, defaultProjectsDir: "/old", defaultProjectsDirExists: true }),
    );
    expect(visibleInput("new-project-location")).toHaveValue("/new");
  });

  it("reports native chooser failures without changing the selected location", async () => {
    pickDirectoryMock.mockRejectedValueOnce(new Error("Picker unavailable"));
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/manual" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Choose location…" }));
    await screen.findByText("Picker unavailable");
    expect(visibleInput("new-project-location")).toHaveValue("/manual");
  });

  it("uses a newly selected backup default when opening a Project in the same session", async () => {
    const updated = { ...defaults, defaultBackupsDir: "/backups", defaultBackupsDirExists: true };
    pickDirectoryMock.mockResolvedValueOnce("/backups");
    setDefaultBackupsDirMock.mockResolvedValueOnce(updated);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Choose…" })[1]);
    await screen.findByText("/backups");
    getPreferencesMock.mockResolvedValue(updated);
    openProjectMock.mockResolvedValueOnce(project);
    fireEvent.change(visibleInput("open-project-path"), {
      target: { value: project.packagePath },
    });
    fireEvent.click(screen.getByRole("button", { name: "Open Project" }));
    await waitFor(() => expect(visibleInput("backup-destination")).toHaveValue("/backups"));
  });

  it("preserves a manual backup location against a delayed preference load", async () => {
    const pending = deferred<Preferences>();
    getPreferencesMock.mockResolvedValueOnce(defaults).mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    fireEvent.change(visibleInput("backup-destination"), { target: { value: "/manual" } });
    await act(async () =>
      pending.resolve({
        ...defaults,
        defaultBackupsDir: "/default",
        defaultBackupsDirExists: true,
      }),
    );
    expect(visibleInput("backup-destination")).toHaveValue("/manual");
  });

  it("warns about a missing configured Projects folder without using it", async () => {
    getPreferencesMock.mockResolvedValueOnce({ ...defaults, defaultProjectsDir: "/missing" });
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(/missing or inaccessible/);
    expect(visibleInput("new-project-location")).toHaveValue("");
  });

  it("prefills the New Project location from a configured default Projects directory", async () => {
    getPreferencesMock.mockResolvedValue({
      defaultProjectsDir: "/home/writer/Projects",
      defaultProjectsDirExists: true,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
    render(<App />);
    await waitFor(() =>
      expect(visibleInput("new-project-location")).toHaveValue("/home/writer/Projects"),
    );
  });

  it("lets the native chooser cancellation leave the location unchanged", async () => {
    pickDirectoryMock.mockResolvedValueOnce(null);
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/kept/as/is" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Choose location…" }));
    await waitFor(() => expect(pickDirectoryMock).toHaveBeenCalled());
    expect(visibleInput("new-project-location")).toHaveValue("/kept/as/is");
  });

  it("reports a package path collision clearly and lets the user retry with a different name", async () => {
    (createProject as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      backendError("already_exists", "a Project package already exists at '/p/Tortuga.wcproj'"),
    );
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/p" },
    });
    fireEvent.change(screen.getByLabelText("new-project-name"), {
      target: { value: "Tortuga" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Project" }));

    await waitFor(() =>
      expect(screen.getByText(/already exists at that location/i)).toBeInTheDocument(),
    );
  });

  it("shows the resulting package location immediately after creation", async () => {
    (createProject as ReturnType<typeof vi.fn>).mockResolvedValueOnce(project);
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/p" },
    });
    fireEvent.change(screen.getByLabelText("new-project-name"), {
      target: { value: "Tortuga" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Project" }));
    await waitFor(() => screen.getByTestId("project-id"));
    // The Project screen displays the package location right away.
    expect(screen.getByText(project.packagePath)).toBeInTheDocument();
  });

  it("lets the user set and clear a default Projects directory", async () => {
    pickDirectoryMock.mockResolvedValueOnce("/chosen/Projects");
    setDefaultProjectsDirMock.mockResolvedValueOnce({
      defaultProjectsDir: "/chosen/Projects",
      defaultProjectsDirExists: true,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
    render(<App />);
    await waitFor(() => expect(getPreferencesMock).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    const chooseButtons = await screen.findAllByRole("button", { name: "Choose…" });
    fireEvent.click(chooseButtons[0]);

    await waitFor(() => expect(screen.getAllByText("/chosen/Projects").length).toBeGreaterThan(0));
    expect(setDefaultProjectsDirMock).toHaveBeenCalledWith("/chosen/Projects");
    // Immediately usable for New Project creation in this session, since the
    // per-operation location was never manually changed.
    expect(visibleInput("new-project-location")).toHaveValue("/chosen/Projects");

    setDefaultProjectsDirMock.mockResolvedValueOnce({
      defaultProjectsDir: null,
      defaultProjectsDirExists: false,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    await waitFor(() => expect(setDefaultProjectsDirMock).toHaveBeenCalledWith(null));
    await waitFor(() => expect(visibleInput("new-project-location")).toHaveValue(""));
  });

  it("shows a plain-language warning for corrupt preferences and offers an explicit reset", async () => {
    getPreferencesMock.mockReset();
    getPreferencesMock.mockRejectedValueOnce(
      backendError("preferences_corrupt", "invalid JSON at line 1"),
    );
    render(<App />);

    await waitFor(() =>
      expect(screen.getByText(/preferences could not be read/i)).toBeInTheDocument(),
    );
    expect(screen.getByText(/Project files are unaffected/i)).toBeInTheDocument();

    resetPreferencesMock.mockResolvedValueOnce({
      defaultProjectsDir: null,
      defaultProjectsDirExists: false,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
    fireEvent.click(screen.getByRole("button", { name: "Review settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Reset application preferences" }));

    await waitFor(() => expect(resetPreferencesMock).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.queryByText(/preferences could not be read/i)).not.toBeInTheDocument(),
    );
  });

  it("shows an unsupported-version warning without offering to reset a version mismatch silently", async () => {
    getPreferencesMock.mockReset();
    getPreferencesMock.mockRejectedValueOnce(
      backendError("unsupported_preferences_version", "found 99, supported 1"),
    );
    render(<App />);

    await waitFor(() =>
      expect(screen.getByText(/different version of Worldcrafter/i)).toBeInTheDocument(),
    );
    expect(
      screen.queryByRole("button", { name: "Reset application preferences" }),
    ).not.toBeInTheDocument();
  });

  it("reports a reset failure visibly instead of swallowing it", async () => {
    getPreferencesMock.mockReset();
    getPreferencesMock.mockRejectedValueOnce(backendError("preferences_corrupt", "bad json"));
    resetPreferencesMock.mockRejectedValueOnce(backendError("io_error", "disk full"));
    render(<App />);

    await screen.findByRole("button", { name: "Review settings" });
    fireEvent.click(screen.getByRole("button", { name: "Review settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Reset application preferences" }));

    await waitFor(() => expect(screen.getByText(/Reset failed/i)).toBeInTheDocument());
  });

  it("catches and displays a failure when choosing a default directory instead of an unhandled rejection", async () => {
    pickDirectoryMock.mockResolvedValueOnce("/blocked/Projects");
    setDefaultProjectsDirMock.mockRejectedValueOnce(
      backendError("invalid_directory", "'/blocked/Projects' does not exist or is not a directory"),
    );
    render(<App />);
    await waitFor(() => expect(getPreferencesMock).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    const chooseButtons = await screen.findAllByRole("button", { name: "Choose…" });
    fireEvent.click(chooseButtons[0]);

    await waitFor(() =>
      expect(screen.getByText(/does not exist or is not a directory/i)).toBeInTheDocument(),
    );
  });

  it("previews the package path using the backend's authoritative sanitizer", async () => {
    render(<App />);
    fireEvent.change(visibleInput("new-project-location"), {
      target: { value: "/p" },
    });
    fireEvent.change(screen.getByLabelText("new-project-name"), {
      target: { value: "Tortuga" },
    });

    await waitFor(() => expect(previewPackagePathMock).toHaveBeenCalledWith("/p", "Tortuga"));
    await waitFor(() =>
      expect(screen.getByText(/Will be created as: \/p\/Tortuga\.wcproj/)).toBeInTheDocument(),
    );
  });
});

describe("Inline Category/Type creation forms", () => {
  beforeEach(() => {
    listCategoriesMock.mockReset();
    listTypesMock.mockReset();
    listEntriesMock.mockReset();
    createCategoryMock.mockReset();
    listCategoriesMock.mockResolvedValue([
      {
        id: "uncategorized",
        name: "Uncategorized",
        isUncategorized: true,
        revision: 0,
        globalRevision: 0,
      },
    ]);
    listTypesMock.mockResolvedValue([]);
    listEntriesMock.mockResolvedValue([]);
  });

  it("shows a labelled container and lets Cancel discard the draft without submitting", async () => {
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => screen.getByRole("heading", { name: "Entries" }));
    await act(async () => {});

    fireEvent.click(screen.getByRole("button", { name: "Create Category inline" }));
    expect(screen.getByText("New Category")).toBeInTheDocument();
    const input = screen.getByLabelText("inline-category-name");
    fireEvent.change(input, { target: { value: "Places" } });

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByLabelText("inline-category-name")).not.toBeInTheDocument();
    expect(createCategoryMock).not.toHaveBeenCalled();
  });

  it("never submits an empty Category name", async () => {
    await openTheProjectScreen();
    fireEvent.click(screen.getByRole("button", { name: "Add Entry" }));
    await waitFor(() => screen.getByRole("heading", { name: "Entries" }));
    await act(async () => {});

    fireEvent.click(screen.getByRole("button", { name: "Create Category inline" }));
    expect(screen.getByRole("button", { name: "Add Category" })).toBeDisabled();

    fireEvent.change(screen.getByLabelText("inline-category-name"), {
      target: { value: "   " },
    });
    expect(screen.getByRole("button", { name: "Add Category" })).toBeDisabled();
    expect(createCategoryMock).not.toHaveBeenCalled();
  });
});

describe("Focused workspace", () => {
  beforeEach(() => {
    getPreferencesMock.mockReset().mockResolvedValue({
      defaultProjectsDir: null,
      defaultProjectsDirExists: false,
      defaultBackupsDir: null,
      defaultBackupsDirExists: false,
    });
    listCategoriesMock.mockResolvedValue([]);
    listTypesMock.mockResolvedValue([]);
    listEntriesMock.mockResolvedValue([]);
    renameProjectMock.mockReset();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("keeps setup out of Home and preserves a Project draft across Settings", async () => {
    render(<App />);
    await waitFor(() => expect(getPreferencesMock).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByLabelText("new-project-location")).not.toBeVisible();
    expect(screen.getByLabelText("restore-backup-path")).not.toBeVisible();
    fireEvent.change(screen.getByLabelText("new-project-name"), {
      target: { value: "A new world" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("dialog", { name: "Application settings" })).toBeVisible();
    fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByLabelText("new-project-name")).toHaveValue("A new world");
  });

  it("shows a preference update failure even if Settings was dismissed while choosing", async () => {
    const picked = deferred<string | null>();
    pickDirectoryMock.mockReturnValueOnce(picked.promise);
    setDefaultProjectsDirMock.mockRejectedValueOnce(new Error("Folder unavailable"));
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Choose…" })[0]);
    fireEvent.click(screen.getByRole("button", { name: "Close Application settings" }));
    await act(async () => picked.resolve("/test/projects"));
    expect(await screen.findByRole("alert")).toHaveTextContent("Folder unavailable");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows a backup failure after the backup dialog was dismissed", async () => {
    const pending = deferred<string>();
    vi.mocked(createBackup).mockReturnValueOnce(pending.promise);
    await openTheProjectScreen();
    fireEvent.change(visibleInput("backup-destination"), { target: { value: "/test/backups" } });
    fireEvent.click(screen.getByRole("button", { name: "Create Manual Backup" }));
    fireEvent.click(screen.getByRole("button", { name: "Close Backups" }));
    await act(async () => pending.reject(new Error("Disk full")));
    expect(await screen.findByText("Backup failed: Disk full")).toBeVisible();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Review backup" })));
    expect(screen.getByLabelText("backup-destination")).toHaveValue("/test/backups");
  });

  it("preserves the restore form when dismissed and reopened", async () => {
    render(<App />);
    await waitFor(() => expect(getPreferencesMock).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Restore a backup…" }));
    fireEvent.change(screen.getByLabelText("restore-backup-path"), {
      target: { value: "/test/backup" },
    });
    fireEvent.change(screen.getByLabelText("restore-new-name"), {
      target: { value: "Recovered world" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Close Restore Backup as Copy" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Restore a backup…" }));
    expect(screen.getByLabelText("restore-backup-path")).toHaveValue("/test/backup");
    expect(screen.getByLabelText("restore-new-name")).toHaveValue("Recovered world");
  });

  it("shows an editing workspace with Project details and backups out of the way", async () => {
    mockEditableEntry();
    await openTheProjectScreen();
    expect(screen.getByTestId("project-id")).not.toBeVisible();
    expect(screen.getByLabelText("project-working-name")).not.toBeVisible();
    expect(screen.getByLabelText("backup-destination")).not.toBeVisible();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    expect(await screen.findByLabelText("entry-name")).toBeVisible();
    expect(screen.getByLabelText("entry-category")).not.toBeVisible();
    expect(screen.getByLabelText("new-field-name")).not.toBeVisible();
    expect(screen.getByRole("button", { name: "Manage fields" })).toBeVisible();
  });

  it("keeps deletion simple and puts the automatic recovery receipt under Backups", async () => {
    mockEditableEntry();
    const definition = {
      id: "age",
      name: "Age",
      kind: "number" as const,
      unit: "years",
      options: [],
      bindings: [],
      revision: 1,
      retired: false,
    };
    vi.mocked(readFields).mockResolvedValueOnce({
      globalRevision: 1,
      definitions: [definition],
      fields: [{ definition, available: true, value: { kind: "number", value: 48 } }],
    });
    vi.mocked(deleteEntryField).mockResolvedValueOnce({
      snapshot: { globalRevision: 2, definitions: [definition], fields: [] },
      backupPath: "/Recovery/entry-copy.wcbackup",
    });
    await openTheProjectScreen();
    fireEvent.click(await screen.findByRole("button", { name: "Thron" }));
    await screen.findByLabelText("Value: Age");
    fireEvent.click(screen.getByRole("button", { name: "Manage fields" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete from Entry: Age" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await screen.findByRole("button", { name: "Add to Entry: Age" });
    expect(deleteEntryField).toHaveBeenCalledWith(project.projectId, "entry", "age", 1);
    expect(screen.queryByText(/Field deleted · recovery backup/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close Manage fields" }));
    projectAction("Backups");
    fireEvent.click(await screen.findByText("Automatic recovery copies"));
    expect(await screen.findByText("Folder: /Recovery")).toBeVisible();
    expect(
      screen.getByText("Latest copy this session: /Recovery/entry-copy.wcbackup"),
    ).toBeVisible();
  });

  it("keeps a failed Project rename visible and editable after closing Settings", async () => {
    renameProjectMock.mockRejectedValueOnce(new Error("Disk full"));
    await openTheProjectScreen();
    fireEvent.change(visibleInput("project-working-name"), { target: { value: "Tortuga Prime" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
    fireEvent.click(screen.getByRole("button", { name: "Close Project settings" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Disk full");
    expect(screen.getByTestId("save-state")).toHaveTextContent("Failed to save");
    fireEvent.click(screen.getByRole("button", { name: "Review Project name" }));
    expect(screen.getByLabelText("project-working-name")).toHaveValue("Tortuga Prime");
  });

  it("offers a save-and-close action from the unsaved dialog", async () => {
    renameProjectMock.mockResolvedValueOnce({
      ...project,
      workingName: "Tortuga Prime",
      revision: 1,
    });
    closeProjectMock.mockResolvedValueOnce(undefined);
    await openTheProjectScreen();
    fireEvent.change(visibleInput("project-working-name"), { target: { value: "Tortuga Prime" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Project settings" }));
    closeFromSettings();
    expect(screen.getByRole("dialog", { name: "Before you leave" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Save and close" }));
    await waitFor(() => expect(screen.queryByTestId("project-id")).not.toBeInTheDocument());
    expect(renameProjectMock).toHaveBeenCalledWith(project.projectId, "Tortuga Prime", 0);
  });
});
