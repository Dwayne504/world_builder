import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { applyFields, readFields, deleteEntryField } from "./api";
import type { EntryFields } from "./types";
import { parseFieldDraft, useEntryFields } from "./useEntryFields";
vi.mock("./api", () => ({ applyFields: vi.fn(), readFields: vi.fn(), deleteEntryField: vi.fn() }));
const snapshot: EntryFields = {
  globalRevision: 3,
  definitions: [],
  fields: [
    {
      definition: {
        id: "field",
        name: "Age",
        kind: "number",
        retired: false,
        revision: 1,
        options: [],
        bindings: [],
      },
      available: true,
      value: null,
    },
  ],
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(readFields).mockResolvedValue(snapshot);
});

describe("Field save contract", () => {
  it("preserves a newer edit if an earlier value finishes saving", async () => {
    const pending = deferred<EntryFields>();
    vi.mocked(applyFields).mockReturnValueOnce(pending.promise);
    const { result } = renderHook(() => useEntryFields("project", "entry", 1, vi.fn()));
    await waitFor(() => expect(result.current.snapshot).not.toBeNull());
    act(() => result.current.change("field", "43"));
    let request!: ReturnType<typeof result.current.submit>;
    act(() => {
      request = result.current.submit();
    });
    act(() => result.current.change("field", "44"));
    await act(async () => {
      pending.resolve({ ...snapshot, globalRevision: 4 });
      expect(await request).toEqual({ kind: "committed-stale" });
    });
    expect(result.current.drafts.field).toBe("44");
    expect(result.current.state).toBe("dirty");
  });
  it("keeps zero and false distinct from missing values", () => {
    expect(parseFieldDraft(snapshot.fields[0], "0")).toEqual({ kind: "number", value: 0 });
    expect(parseFieldDraft(snapshot.fields[0], "")).toBeNull();
    const boolean = {
      ...snapshot.fields[0],
      definition: { ...snapshot.fields[0].definition, kind: "boolean" as const },
    };
    expect(parseFieldDraft(boolean, "false")).toEqual({ kind: "boolean", value: false });
    expect(() => parseFieldDraft(snapshot.fields[0], "Unknown")).toThrow(/finite number/);
  });
  it("reports Saved only after commit acknowledgement and serializes submissions", async () => {
    const pending = deferred<EntryFields>();
    vi.mocked(applyFields).mockReturnValueOnce(pending.promise);
    const onRevision = vi.fn();
    const { result } = renderHook(() => useEntryFields("project", "entry", 1, onRevision));
    await waitFor(() => expect(result.current.snapshot).not.toBeNull());
    act(() => result.current.change("field", "43"));
    let saving!: ReturnType<typeof result.current.submit>;
    act(() => {
      saving = result.current.submit();
      void result.current.submit();
    });
    expect(result.current.state).toBe("saving");
    expect(applyFields).toHaveBeenCalledTimes(1);
    expect(result.current.drafts.field).toBe("43");
    await act(async () => {
      pending.resolve({ ...snapshot, globalRevision: 4 });
      await saving;
    });
    expect(result.current.state).toBe("saved");
    expect(result.current.drafts).toEqual({});
    expect(onRevision).toHaveBeenLastCalledWith(4);
  });
  it("keeps failed drafts and permits explicit reload and retry after a revision conflict", async () => {
    vi.mocked(applyFields).mockRejectedValueOnce(new Error("Revision conflict"));
    const { result } = renderHook(() => useEntryFields("project", "entry", 1, vi.fn()));
    await waitFor(() => expect(result.current.snapshot).not.toBeNull());
    act(() => result.current.change("field", "44"));
    await act(async () => {
      await result.current.submit();
    });
    expect(result.current.state).toBe("failed");
    expect(result.current.drafts.field).toBe("44");
    vi.mocked(readFields).mockResolvedValueOnce({ ...snapshot, globalRevision: 5 });
    await act(async () => {
      await result.current.reload();
    });
    expect(result.current.drafts.field).toBe("44");
    vi.mocked(applyFields).mockResolvedValueOnce({ ...snapshot, globalRevision: 6 });
    await act(async () => {
      await result.current.submit();
    });
    expect(applyFields).toHaveBeenLastCalledWith("project", "entry", 5, {
      kind: "set_values",
      edits: [{ fieldId: "field", value: { kind: "number", value: 44 } }],
    });
    expect(result.current.state).toBe("saved");
  });
  it("does not send invalid numeric drafts or discard them on failure", async () => {
    const { result } = renderHook(() => useEntryFields("project", "entry", 1, vi.fn()));
    await waitFor(() => expect(result.current.snapshot).not.toBeNull());
    act(() => result.current.change("field", "-"));
    await act(async () => {
      await result.current.submit();
    });
    expect(applyFields).not.toHaveBeenCalled();
    expect(result.current.state).toBe("failed");
    expect(result.current.drafts.field).toBe("-");
  });
});

it("never rebases a deletion review and navigation waits for its pending acknowledgement", async () => {
  const pending = deferred<Awaited<ReturnType<typeof deleteEntryField>>>();
  vi.mocked(deleteEntryField).mockReturnValueOnce(pending.promise);
  const onBackup = vi.fn();
  const { result } = renderHook(() => useEntryFields("project", "entry", 1, vi.fn(), () => 9));
  await waitFor(() => expect(result.current.snapshot).not.toBeNull());
  let saving!: ReturnType<typeof result.current.submit>;
  act(() => {
    saving = result.current.deleteLocal("field", 3, "/Backups", onBackup);
  });
  expect(deleteEntryField).toHaveBeenCalledWith("project", "entry", "field", 3, "/Backups");
  expect(result.current.submit()).toBe(saving);
  await act(async () => {
    pending.reject(new Error("Revision conflict"));
    await saving;
  });
  expect(result.current.snapshot).toEqual(snapshot);
  expect(result.current.state).toBe("failed");
  expect(onBackup).not.toHaveBeenCalled();
});
