import { act, fireEvent, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AppHelp } from "./AppHelp";
import { getBuildInfo } from "./api";
import { menuItem, renderWithMenu } from "./desktopMenuTestUtils";

vi.mock("./api", () => ({ getBuildInfo: vi.fn() }));
afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  vi.resetAllMocks();
});

it("shows the running desktop build's actual supported versions", async () => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
  vi.mocked(getBuildInfo).mockResolvedValue({
    version: "0.2.7",
    supportedSchemaVersion: 10,
    supportedFormatVersion: 3,
  });
  renderWithMenu(<AppHelp />);
  fireEvent.click(menuItem("Help", "About Worldcrafter"));
  expect(await screen.findByText("0.2.7")).toBeVisible();
  expect(screen.getByText("10")).toBeVisible();
  expect(screen.getByText("3")).toBeVisible();
});

it("keeps About useful when an older desktop build lacks the information command", async () => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {} });
  vi.mocked(getBuildInfo).mockRejectedValue(new Error("Unknown command"));
  renderWithMenu(<AppHelp />);
  fireEvent.click(menuItem("Help", "About Worldcrafter"));
  expect(
    await screen.findByText("Build information is unavailable in this app build."),
  ).toBeVisible();
  expect(screen.queryByText("Supported Project schema")).not.toBeInTheDocument();
});

it("provides an offline reference and identifies browser previews without inventing build data", async () => {
  renderWithMenu(<AppHelp />);
  fireEvent.click(menuItem("Help", "Using Worldcrafter"));
  expect(screen.getByRole("dialog", { name: "Using Worldcrafter" })).toHaveTextContent(
    "Finish or cancel unfinished forms",
  );
  fireEvent.click(screen.getByRole("button", { name: "Close Using Worldcrafter" }));
  await act(async () => fireEvent.click(menuItem("Help", "About Worldcrafter")));
  expect(screen.getByText(/Browser preview/)).toBeVisible();
  expect(getBuildInfo).not.toHaveBeenCalled();
});
