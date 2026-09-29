import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { AppearanceButton, AppearanceProvider } from "./AppearanceProvider";
import { getAppearance, setAppearance } from "./api";

vi.mock("./api", () => ({ getAppearance: vi.fn(), setAppearance: vi.fn() }));
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getAppearance).mockResolvedValue("storybook");
  vi.mocked(setAppearance).mockImplementation(async (a) => a);
});
async function show() {
  const view = render(
    <AppearanceProvider>
      <AppearanceButton />
      <input aria-label="Draft" defaultValue="Unfinished story" />
    </AppearanceProvider>,
  );
  await waitFor(() =>
    expect(screen.queryByRole("status", { hidden: true })).not.toBeInTheDocument(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Appearance" }));
  await waitFor(() => expect(screen.getByRole("button", { name: /Starship/ })).toBeEnabled());
  return view;
}
it("switches only after a saved acknowledgement and keeps the current writing draft mounted", async () => {
  let finish!: (value: "starship") => void;
  vi.mocked(setAppearance).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  await show();
  const input = screen.getByLabelText("Draft");
  fireEvent.change(input, { target: { value: "Still writing" } });
  fireEvent.click(screen.getByRole("button", { name: /Starship/ }));
  expect(document.documentElement.dataset.appearance).toBe("storybook");
  expect(screen.getByRole("button", { name: /Storybook/ })).toBeDisabled();
  await act(async () => finish("starship"));
  expect(document.documentElement.dataset.appearance).toBe("starship");
  expect(screen.getByLabelText("Draft")).toBe(input);
  expect(input).toHaveValue("Still writing");
});
it("loads a saved style on restart and preserves it when saving fails", async () => {
  vi.mocked(getAppearance).mockResolvedValue("starship");
  vi.mocked(setAppearance).mockRejectedValueOnce(new Error("Preferences are read-only"));
  await show();
  expect(document.documentElement.dataset.appearance).toBe("starship");
  fireEvent.click(screen.getByRole("button", { name: /Storybook/ }));
  await screen.findByText(/Preferences are read-only/);
  expect(document.documentElement.dataset.appearance).toBe("starship");
  expect(screen.getByRole("button", { name: /Storybook/ })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Retry loading appearance" }));
  await waitFor(() => expect(screen.getByRole("button", { name: /Storybook/ })).toBeEnabled());
});
it("fails closed for corrupt or newer preference files", async () => {
  vi.mocked(getAppearance).mockRejectedValue(new Error("Unsupported preferences version"));
  render(
    <AppearanceProvider>
      <AppearanceButton />
    </AppearanceProvider>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Appearance" }));
  await screen.findByText(/Unsupported preferences version/);
  expect(screen.getByRole("button", { name: /Starship/ })).toBeDisabled();
  expect(setAppearance).not.toHaveBeenCalled();
});
