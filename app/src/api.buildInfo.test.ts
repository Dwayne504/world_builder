import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { AppCommandError, getBuildInfo } from "./api";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

it("reads the running backend's supported versions rather than inferring them from the UI", async () => {
  const backend = { version: "0.1.0", supportedSchemaVersion: 10, supportedFormatVersion: 1 };
  vi.mocked(invoke).mockResolvedValue(backend);
  expect(await getBuildInfo()).toEqual(backend);
  expect(invoke).toHaveBeenCalledWith("get_build_info", {});
});

it("reports unavailable build information without fabricating a supported version", async () => {
  vi.mocked(invoke).mockRejectedValue({ kind: "unavailable", message: "Older backend" });
  await expect(getBuildInfo()).rejects.toBeInstanceOf(AppCommandError);
});
