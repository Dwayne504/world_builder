import { fireEvent, render, screen, type RenderOptions } from "@testing-library/react";
import type { ReactNode } from "react";
import { flushSync } from "react-dom";
import { DesktopMenu } from "./DesktopMenu";
import { DesktopMenuProvider } from "./desktopMenuContext";

// This test-only wrapper is intentionally exported together with interaction helpers.
// eslint-disable-next-line react-refresh/only-export-components
function MenuHarness({ children }: { children: ReactNode }) {
  return (
    <DesktopMenuProvider>
      <DesktopMenu />
      {children}
    </DesktopMenuProvider>
  );
}

export function renderWithMenu(ui: ReactNode, options?: RenderOptions) {
  return render(ui, { wrapper: MenuHarness, ...options });
}

/** Exercise the same visible menu path as a desktop user, including submenus. */
export function menuItem(menu: string, ...path: string[]) {
  const trigger = screen.getByRole("menuitem", { name: menu });
  if (trigger.getAttribute("aria-expanded") !== "true") flushSync(() => fireEvent.click(trigger));
  let item = trigger;
  path.forEach((name, index) => {
    item =
      screen.queryByRole("menuitem", { name }) ?? screen.getByRole("menuitemcheckbox", { name });
    if (index < path.length - 1 && item.getAttribute("aria-expanded") !== "true")
      flushSync(() => fireEvent.click(item));
  });
  return item;
}
