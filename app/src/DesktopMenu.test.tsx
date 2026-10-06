import { StrictMode, useState } from "react";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DesktopMenu } from "./DesktopMenu";
import {
  DesktopMenuProvider,
  useDesktopCommands,
  type DesktopMenuContribution,
} from "./desktopMenuContext";

function Contribution({
  commands,
  scope = "test",
  order = 0,
}: {
  commands: DesktopMenuContribution;
  scope?: string;
  order?: number;
}) {
  useDesktopCommands(scope, commands, order);
  return null;
}

function show(commands: DesktopMenuContribution) {
  return render(
    <DesktopMenuProvider>
      <DesktopMenu />
      <Contribution commands={commands} />
      <button>Workspace action</button>
    </DesktopMenuProvider>,
  );
}

describe("DesktopMenu", () => {
  it("preserves label lookup and ARIA references while a submenu is open", async () => {
    const user = userEvent.setup();
    render(
      <DesktopMenuProvider>
        <DesktopMenu />
        <Contribution
          commands={{
            edit: [
              {
                id: "entry \"name\" 'panel'",
                label: "Entry",
                children: [{ id: "rename", label: "Rename entry", action: vi.fn() }],
              },
            ],
          }}
        />
        <label htmlFor="project-name">Project name</label>
        <input id="project-name" defaultValue="Tortuga" />
      </DesktopMenuProvider>,
    );
    await user.click(screen.getByRole("menuitem", { name: "Edit" }));
    await user.keyboard("{ArrowRight}");
    expect(screen.getByLabelText("Project name")).toHaveValue("Tortuga");
    const panel = screen.getByRole("menu", { name: "Entry" });
    const trigger = screen.getByRole("menuitem", { name: "Entry" });
    expect(panel).toHaveAttribute("aria-labelledby", trigger.id);
    expect(trigger).toHaveAttribute("aria-controls", panel.id);
    for (const element of screen
      .getByRole("navigation", { name: "Application menu" })
      .querySelectorAll("[id]")) {
      expect(element.id).not.toMatch(/[\s"']/);
    }
  });

  it("lets modified navigation reach the workspace without also moving through menus", async () => {
    const user = userEvent.setup();
    const workspaceKey = vi.fn();
    render(
      <div onKeyDown={(event) => workspaceKey(event.key)}>
        <DesktopMenuProvider>
          <DesktopMenu />
          <Contribution
            commands={{ file: [{ id: "new", label: "New project", action: vi.fn() }] }}
          />
        </DesktopMenuProvider>
      </div>,
    );
    await user.tab();
    const file = screen.getByRole("menuitem", { name: "File" });
    workspaceKey.mockClear();
    fireEvent.keyDown(file, { key: "ArrowRight", altKey: true });
    expect(file).toHaveFocus();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(workspaceKey).toHaveBeenCalledExactlyOnceWith("ArrowRight");
    await user.keyboard("{ArrowDown}");
    const command = screen.getByRole("menuitem", { name: "New project" });
    workspaceKey.mockClear();
    fireEvent.keyDown(command, { key: "ArrowLeft", altKey: true });
    expect(command).toHaveFocus();
    expect(screen.getByRole("menu", { name: "File" })).toBeInTheDocument();
    expect(workspaceKey).toHaveBeenCalledExactlyOnceWith("ArrowLeft");
  });

  it("navigates the menu bar and skips disabled commands and separators", async () => {
    const user = userEvent.setup();
    const disabled = vi.fn();
    show({
      file: [
        { id: "new", label: "New project", action: vi.fn() },
        { id: "save", label: "Save", action: disabled, disabled: true },
        { id: "separator", label: "", separator: true },
        { id: "close", label: "Close project", action: vi.fn() },
      ],
      edit: [{ id: "undo", label: "Undo", action: vi.fn() }],
      help: [{ id: "about", label: "About", action: vi.fn() }],
    });
    await user.tab();
    expect(screen.getByRole("menuitem", { name: "File" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "File" }), {
      key: "ArrowDown",
      isComposing: true,
    });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "New project" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "New project" }), {
      key: "Escape",
      isComposing: true,
    });
    expect(screen.getByRole("menu", { name: "File" })).toBeInTheDocument();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Close project" })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "New project" })).toHaveFocus();
    await user.keyboard("{End}");
    expect(screen.getByRole("menuitem", { name: "Close project" })).toHaveFocus();
    await user.keyboard("{Home}{ArrowUp}");
    expect(screen.getByRole("menuitem", { name: "Close project" })).toHaveFocus();
    fireEvent.click(screen.getByRole("menuitem", { name: "Save" }));
    expect(disabled).not.toHaveBeenCalled();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("menuitem", { name: "Undo" })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus();
    await user.keyboard("{End}{ArrowUp}");
    expect(screen.getByRole("menuitem", { name: "About" })).toHaveFocus();
  });

  it("opens nested menus, returns to their parent, and restores the trigger before dispatch", async () => {
    const user = userEvent.setup();
    const action = vi.fn(() =>
      expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus(),
    );
    show({
      edit: [
        {
          id: "entry",
          label: "Entry",
          children: [
            { id: "name", label: "Rename entry", action },
            {
              id: "fields",
              label: "Fields",
              children: [{ id: "add", label: "Add field", action }],
            },
          ],
        },
      ],
    });
    await user.click(screen.getByRole("menuitem", { name: "Edit" }));
    expect(screen.getByRole("menuitem", { name: "Entry" })).toHaveFocus();
    await user.keyboard("{ArrowRight}{End}{ArrowRight}");
    expect(screen.getByRole("menuitem", { name: "Add field" })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.getByRole("menuitem", { name: "Fields" })).toHaveFocus();
    expect(screen.queryByRole("menuitem", { name: "Add field" })).not.toBeInTheDocument();
    await user.keyboard("{ArrowLeft}");
    expect(screen.getByRole("menuitem", { name: "Entry" })).toHaveFocus();
    await user.keyboard("{Enter}{Enter}");
    expect(action).toHaveBeenCalledOnce();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("dismisses on outside interaction and lets Tab leave the menu", async () => {
    const user = userEvent.setup();
    show({ file: [{ id: "new", label: "New project", action: vi.fn() }] });
    await user.click(screen.getByRole("menuitem", { name: "File" }));
    await user.click(screen.getByRole("button", { name: "Workspace action" }));
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Workspace action" })).toHaveFocus();
    await user.click(screen.getByRole("menuitem", { name: "File" }));
    await user.tab();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Workspace action" })).toHaveFocus();
  });

  it("supports checked items, pointer submenus, and letter navigation", async () => {
    const user = userEvent.setup();
    show({
      view: [
        {
          id: "style",
          label: "Appearance",
          children: [{ id: "storybook", label: "Storybook", checked: true, action: vi.fn() }],
        },
        { id: "timeline", label: "Timeline", action: vi.fn() },
      ],
    });
    await user.click(screen.getByRole("menuitem", { name: "View" }));
    await user.keyboard("t");
    expect(screen.getByRole("menuitem", { name: "Timeline" })).toHaveFocus();
    await user.hover(screen.getByRole("menuitem", { name: "Appearance" }));
    expect(screen.getByRole("menuitemcheckbox", { name: "Storybook" })).toBeChecked();
    await user.hover(screen.getByRole("menuitem", { name: "Timeline" }));
    expect(screen.getByRole("menuitemcheckbox", { name: "Storybook" })).toBeInTheDocument();
    await user.hover(screen.getByRole("menuitemcheckbox", { name: "Storybook" }));
    await new Promise((resolve) => setTimeout(resolve, 180));
    expect(screen.getByRole("menuitemcheckbox", { name: "Storybook" })).toHaveFocus();
    await user.hover(screen.getByRole("menuitem", { name: "File" }));
    expect(screen.queryByRole("menuitemcheckbox")).not.toBeInTheDocument();
    expect(screen.getByRole("menu", { name: "File" })).toHaveFocus();
  });

  it("uses current inline actions without repeated registration or stale closures", async () => {
    const user = userEvent.setup();
    const action = vi.fn();
    function Editor() {
      const [revision, setRevision] = useState(0);
      useDesktopCommands("editor", {
        file: [{ id: "save", label: "Save", action: () => action(revision) }],
      });
      return <button onClick={() => setRevision((current) => current + 1)}>Change draft</button>;
    }
    render(
      <StrictMode>
        <DesktopMenuProvider>
          <DesktopMenu />
          <Editor />
        </DesktopMenuProvider>
      </StrictMode>,
    );
    await user.click(screen.getByRole("button", { name: "Change draft" }));
    await user.click(screen.getByRole("menuitem", { name: "File" }));
    await user.click(screen.getByRole("menuitem", { name: "Save" }));
    expect(action).toHaveBeenCalledExactlyOnceWith(1);
  });

  it("merges contextual children by priority and removes only the unmounted registration", async () => {
    const user = userEvent.setup();
    const rename = vi.fn();
    function App({ editor }: { editor: boolean }) {
      return (
        <DesktopMenuProvider>
          <DesktopMenu />
          <Contribution
            scope="context"
            commands={{
              edit: [
                {
                  id: "entry",
                  label: "Entry",
                  disabled: true,
                  children: [{ id: "new", label: "New entry", disabled: true }],
                },
              ],
            }}
          />
          <Contribution
            scope="context"
            order={10}
            commands={{
              edit: [
                {
                  id: "entry",
                  label: "Entry",
                  children: [{ id: "new", label: "New entry", action: vi.fn() }],
                },
              ],
            }}
          />
          {editor && (
            <Contribution
              scope="context"
              order={20}
              commands={{
                edit: [
                  {
                    id: "entry",
                    label: "Entry",
                    children: [{ id: "rename", label: "Rename entry", action: rename }],
                  },
                ],
              }}
            />
          )}
        </DesktopMenuProvider>
      );
    }
    const view = render(<App editor />);
    await user.click(screen.getByRole("menuitem", { name: "Edit" }));
    await user.keyboard("{ArrowRight}");
    const submenu = screen.getByRole("menu", { name: "Entry" });
    expect(
      within(submenu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent),
    ).toEqual(["New entry", "Rename entry"]);
    expect(screen.getByRole("menuitem", { name: "New entry" })).toBeEnabled();
    view.rerender(<App editor={false} />);
    expect(screen.queryByRole("menuitem", { name: "Rename entry" })).not.toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "New entry" })).toBeEnabled();
    expect(screen.getAllByRole("menuitem", { name: "Entry" })).toHaveLength(1);
  });

  it("updates disabled metadata and tolerates standalone consumers", async () => {
    const user = userEvent.setup();
    const action = vi.fn();
    function Editor() {
      const [disabled, setDisabled] = useState(true);
      useDesktopCommands("editor", { file: [{ id: "save", label: "Save", disabled, action }] });
      return <button onClick={() => setDisabled(false)}>Allow saving</button>;
    }
    const view = render(
      <DesktopMenuProvider>
        <DesktopMenu />
        <Editor />
      </DesktopMenuProvider>,
    );
    await user.click(screen.getByRole("menuitem", { name: "File" }));
    expect(screen.getByRole("menuitem", { name: "Save" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Allow saving" }));
    await user.click(screen.getByRole("menuitem", { name: "File" }));
    expect(screen.getByRole("menuitem", { name: "Save" })).toBeEnabled();
    view.unmount();
    expect(() => render(<Editor />)).not.toThrow();
  });
});
