import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { EditorView } from "@tiptap/pm/view";
import { renderWithMenu as render, menuItem } from "./desktopMenuTestUtils";
import { EntryFieldsPanel, type FieldsController } from "./EntryFieldsPanel";
import { applyFields, readFields } from "./api";
import type { Entry, EntryFields } from "./types";
import { deferred, textDocument } from "./chapterTestFixtures";
import { plainDocument } from "./chapterRecovery";
import { storeEntryDescriptionDraft } from "./entryDescriptionRecovery";

vi.mock("./api", () => ({ applyFields: vi.fn(), readFields: vi.fn(), deleteEntryField: vi.fn() }));
const entry: Entry = {
  id: "entry",
  categoryId: "category",
  typeId: "type",
  authoredName: "Invented",
  displayName: "Invented",
  revision: 1,
  globalRevision: 2,
  workspaceState: "active",
};
let saved: EntryFields;
let controller: FieldsController;
let revision: number;
const register = (next: FieldsController) => {
  controller = next;
};
const report = (next: number) => {
  revision = next;
};
function fixture(text: string | null = "Saved writing"): EntryFields {
  const definition = {
    id: "rich",
    name: "Background",
    kind: "rich_text" as const,
    revision: 1,
    retired: false,
    options: [],
    bindings: [],
  };
  const scalar = { ...definition, id: "scalar", name: "Motto", kind: "short_text" as const };
  return {
    globalRevision: 2,
    definitions: [definition, scalar],
    fields: [
      {
        definition,
        available: true,
        value:
          text === null
            ? null
            : {
                kind: "rich_text",
                value: {
                  schemaVersion: 1,
                  revision: 2,
                  content: textDocument(text),
                  plainText: text,
                  readOnlyReason: null,
                  originalJson: null,
                },
              },
      },
      { definition: scalar, available: true, value: { kind: "text", value: "Saved motto" } },
    ],
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  saved = fixture();
  revision = 2;
  vi.mocked(readFields).mockImplementation(async () => saved);
  vi.mocked(applyFields).mockImplementation(async (_project, _entry, expected, command) => {
    if (command.kind !== "set_values") return saved;
    saved = {
      ...saved,
      globalRevision: expected + 1,
      fields: saved.fields.map((field) => {
        const edit = command.edits.find((edit) => edit.fieldId === field.definition.id);
        if (!edit) return field;
        const value = edit.value;
        if (value?.kind !== "rich_text") return { ...field, value };
        const plain = plainDocument(value.value.content!);
        return {
          ...field,
          value: plain.trim()
            ? { ...value, value: { ...value.value, revision: expected + 1, plainText: plain } }
            : null,
        };
      }),
    };
    return saved;
  });
  vi.spyOn(EditorView.prototype, "coordsAtPos").mockReturnValue({
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
  });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});
function show() {
  return render(
    <EntryFieldsPanel
      projectId="project"
      entry={entry}
      disabled={false}
      onController={register}
      onRevision={report}
      getRevision={() => revision}
    />,
  );
}
async function write(prose: HTMLElement, text: string) {
  await act(async () => {
    prose.querySelector("p")!.textContent = text;
    fireEvent.input(prose, { inputType: "insertText", data: text });
  });
}
it("loads saved Rich Text before mounting its editor and retains focus and undo on acknowledgement", async () => {
  const pending = deferred<EntryFields>();
  vi.mocked(applyFields).mockReturnValueOnce(pending.promise);
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  expect(prose).toHaveTextContent("Saved writing");
  act(() => prose.focus());
  await write(prose, "More writing");
  await waitFor(() => expect(applyFields).toHaveBeenCalledTimes(1));
  const next = fixture("More writing");
  next.globalRevision = 3;
  await act(async () => {
    saved = next;
    pending.resolve(next);
  });
  expect(screen.getByRole("textbox", { name: "Value: Background" })).toBe(prose);
  expect(prose).toHaveFocus();
  expect(prose).toHaveTextContent("More writing");
  fireEvent.click(screen.getByRole("button", { name: "Undo writing" }));
  expect(prose).toHaveTextContent("Saved writing");
});
it("keeps empty defaults unwritten and creates Rich Text defaults without an initial scalar value", async () => {
  saved = fixture(null);
  show();
  await screen.findByRole("textbox", { name: "Value: Background" });
  expect(applyFields).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Add field" }));
  fireEvent.change(screen.getByLabelText("new-field-name"), { target: { value: "Culture" } });
  fireEvent.change(screen.getByLabelText("new-field-kind"), { target: { value: "rich_text" } });
  fireEvent.change(screen.getByLabelText("new-field-scope"), { target: { value: "type" } });
  expect(screen.queryByLabelText("new-field-value")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Create field" }));
  await waitFor(() =>
    expect(applyFields).toHaveBeenCalledWith(
      "project",
      "entry",
      2,
      expect.objectContaining({
        kind: "create",
        fieldKind: "rich_text",
        value: null,
        provider: { kind: "type", id: "type" },
      }),
    ),
  );
});
it("passively waits without starting Rich writes, then flushes with the relationship acknowledgement revision", async () => {
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  vi.useFakeTimers();
  await write(prose, "Pending field draft");
  await act(async () => {
    await controller.waitForPending?.();
  });
  expect(applyFields).not.toHaveBeenCalled();
  // The parent saves relationships between its passive wait and explicit Field flush.
  revision = 8;
  await act(async () => {
    await controller.submit();
  });
  expect(applyFields).toHaveBeenCalledWith(
    "project",
    "entry",
    8,
    expect.objectContaining({
      kind: "set_values",
      edits: [
        {
          fieldId: "rich",
          value: expect.objectContaining({
            kind: "rich_text",
            value: expect.objectContaining({ revision: 2 }),
          }),
        },
      ],
    }),
  );
  expect(controller.state).toBe("saved");
});
it("clears with the exact saved revision and uses revision zero for new typing without losing editor focus", async () => {
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  act(() => prose.focus());
  await write(prose, "");
  await act(async () => {
    await controller.submit();
  });
  expect(saved.fields[0].value).toBeNull();
  expect(screen.getByRole("textbox", { name: "Value: Background" })).toBe(prose);
  await write(prose, "A new beginning");
  await act(async () => {
    await controller.submit();
  });
  expect(
    vi.mocked(applyFields).mock.calls[vi.mocked(applyFields).mock.calls.length - 1]?.[3],
  ).toEqual(
    expect.objectContaining({
      edits: [
        {
          fieldId: "rich",
          value: expect.objectContaining({ value: expect.objectContaining({ revision: 0 }) }),
        },
      ],
    }),
  );
  expect(prose).toHaveFocus();
});
it("preserves scalar failures and drafts when a companion Rich save succeeds", async () => {
  const normal = vi.mocked(applyFields).getMockImplementation()!;
  vi.mocked(applyFields).mockImplementation((...args) =>
    args[3].kind === "set_values" && args[3].edits.some((edit) => edit.fieldId === "scalar")
      ? Promise.reject(new Error("Scalar disk failure"))
      : normal(...args),
  );
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  fireEvent.change(screen.getByRole("textbox", { name: "Value: Motto" }), {
    target: { value: "Keep my failed motto" },
  });
  await act(async () => {
    await controller.submit();
  });
  expect(controller.state).toBe("failed");
  const scalarCalls = vi.mocked(applyFields).mock.calls.length;
  await write(prose, "Rich writing can save");
  await waitFor(() =>
    expect(
      saved.fields[0].value?.kind === "rich_text" && saved.fields[0].value.value.plainText,
    ).toContain("Rich writing can save"),
  );
  expect(screen.getByRole("textbox", { name: "Value: Motto" })).toHaveValue("Keep my failed motto");
  expect(screen.getByText("Scalar disk failure")).toBeVisible();
  expect(controller.state).toBe("failed");
  expect(applyFields).toHaveBeenCalledTimes(scalarCalls + 1);
});
it("keeps failed Rich drafts, blocks configuration and navigation, and requires review before rebasing", async () => {
  vi.mocked(applyFields).mockRejectedValue(new Error("Stale document revision"));
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  await write(prose, "My unsaved story");
  await act(async () => {
    await controller.submit();
  });
  expect(controller.state).toBe("failed");
  expect(screen.getByRole("button", { name: "Add field" })).toBeDisabled();
  expect(prose).toHaveTextContent("My unsaved story");
  saved = fixture("Another writer's version");
  saved.globalRevision = 9;
  fireEvent.click(screen.getByRole("button", { name: "Review saved writing" }));
  fireEvent.click(await screen.findByRole("button", { name: "Review writing draft" }));
  expect(screen.getByLabelText("Recovered writing")).toHaveValue("My unsaved story\n");
  expect(screen.getByLabelText("Saved writing")).toHaveValue("Another writer's version");
  expect(controller.canSubmit).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Keep saved writing" }));
  expect(await screen.findByRole("textbox", { name: "Value: Background" })).toHaveTextContent(
    "Another writer's version",
  );
  expect(controller.state).toBe("saved");
});
it("isolates recovery per Field and visibly preserves unsupported writing while allowing Hide", async () => {
  storeEntryDescriptionDraft("project", "entry:field:different", 1, textDocument("Another field"));
  saved.fields[0].value = {
    kind: "rich_text",
    value: {
      schemaVersion: 99,
      revision: 2,
      content: null,
      plainText: "Readable fallback",
      readOnlyReason: "Newer document version",
      originalJson: "original future bytes",
    },
  };
  show();
  expect(await screen.findByRole("alert")).toHaveTextContent("Newer document version");
  expect(screen.queryByRole("textbox", { name: "Value: Background" })).not.toBeInTheDocument();
  expect(screen.queryByText("Another field")).not.toBeInTheDocument();
  fireEvent.click(screen.getByText("Original writing"));
  expect(screen.getByLabelText("Original writing data")).toHaveValue("original future bytes");
  fireEvent.click(menuItem("Edit", "Field", "Manage fields…"));
  expect(screen.getByRole("button", { name: "Delete from Entry: Background" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Hide field: Background" })).toBeEnabled();
  expect(applyFields).not.toHaveBeenCalled();
});
it("requires review of a recovered Field draft and blocks close until the author chooses", async () => {
  saved = fixture(null);
  storeEntryDescriptionDraft(
    "project",
    "entry:field:rich",
    1,
    textDocument("Recovered Field writing"),
  );
  show();
  fireEvent.click(await screen.findByRole("button", { name: "Review writing draft" }));
  expect(controller.canSubmit).toBe(false);
  expect(screen.getByLabelText("Recovered writing")).toHaveValue("Recovered Field writing\n");
  fireEvent.click(screen.getByRole("button", { name: "Use writing draft" }));
  expect(await screen.findByRole("textbox", { name: "Value: Background" })).toHaveTextContent(
    "Recovered Field writing",
  );
  expect(controller.state).toBe("dirty");
});

it("does not refresh over a scalar draft typed while Rich writing is saving", async () => {
  const pending = deferred<EntryFields>();
  vi.mocked(applyFields).mockReturnValueOnce(pending.promise);
  show();
  const prose = await screen.findByRole("textbox", { name: "Value: Background" });
  await write(prose, "Rich in flight");
  await waitFor(() => expect(applyFields).toHaveBeenCalledTimes(1));
  vi.useFakeTimers();
  fireEvent.change(screen.getByRole("textbox", { name: "Value: Motto" }), {
    target: { value: "Still typing" },
  });
  const reads = vi.mocked(readFields).mock.calls.length;
  await act(async () => {
    saved = fixture("Rich in flight");
    saved.globalRevision = 3;
    pending.resolve(saved);
  });
  expect(controller.state).toBe("dirty");
  expect(screen.getByRole("textbox", { name: "Value: Motto" })).toHaveValue("Still typing");
  expect(readFields).toHaveBeenCalledTimes(reads);
});
