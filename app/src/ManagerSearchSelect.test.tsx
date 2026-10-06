import { useState } from "react";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { ManagerSearchSelect } from "./ManagerSearchSelect";

const choices = Array.from({ length: 125 }, (_, index) => ({
  id: `entry-${index}`,
  label: `Entry ${index}`,
  searchText: index === 124 ? "Northern coast" : "People",
}));

it("bounds large pickers and keeps an explicitly selected ID when a search hides it", () => {
  const changed = vi.fn();
  function Harness() {
    const [value, setValue] = useState("entry-123");
    return (
      <ManagerSearchSelect
        label="Entry"
        value={value}
        emptyLabel="Choose an Entry"
        choices={choices}
        onChange={(id) => {
          setValue(id);
          changed(id);
        }}
      />
    );
  }
  render(<Harness />);
  const select = screen.getByRole("combobox", { name: "Entry" });
  expect(within(select).getAllByRole("option")).toHaveLength(22);
  expect(select).toHaveValue("entry-123");
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Northern coast" } });
  expect(within(select).getAllByRole("option")).toHaveLength(3);
  expect(select).toHaveValue("entry-123");
  expect(changed).not.toHaveBeenCalled();
  fireEvent.change(select, { target: { value: "entry-124" } });
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "missing" } });
  expect(select).toHaveValue("entry-124");
  expect(screen.getByText(/No matches/)).toBeVisible();
  expect(changed).toHaveBeenCalledExactlyOnceWith("entry-124");
});

it("keeps a filter clearable when reloading reduces the available choices", () => {
  const props = { label: "Entry", value: "", emptyLabel: "Choose an Entry", onChange: vi.fn() };
  const result = render(<ManagerSearchSelect {...props} choices={choices} />);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "missing" } });
  result.rerender(<ManagerSearchSelect {...props} choices={choices.slice(0, 2)} />);
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "" } });
  expect(screen.getAllByRole("option")).toHaveLength(3);
  expect(props.onChange).not.toHaveBeenCalled();
});

it("preserves a disabled incompatible selection through filtering and omits unrequested empty options", () => {
  const changed = vi.fn();
  render(
    <ManagerSearchSelect
      label="Category"
      value="incompatible"
      choices={[
        ...choices,
        {
          id: "incompatible",
          label: "Incompatible current Type — choose explicitly",
          disabled: true,
        },
      ]}
      onChange={changed}
    />,
  );
  expect(screen.getByRole("combobox")).toHaveValue("incompatible");
  expect(
    screen.getByRole("option", { name: "Incompatible current Type — choose explicitly" }),
  ).toBeDisabled();
  expect(screen.queryByRole("option", { name: "" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Northern coast" } });
  expect(screen.getByRole("combobox")).toHaveValue("incompatible");
  expect(
    screen.getByRole("option", { name: "Incompatible current Type — choose explicitly" }),
  ).toBeDisabled();
  expect(changed).not.toHaveBeenCalled();
});
