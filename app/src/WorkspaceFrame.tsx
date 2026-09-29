import { useEffect, type ReactNode } from "react";
import type { Category } from "./types";

export function WorkspaceFrame({
  children,
  categories,
  categoryId,
  collapsed,
  onToggle,
  onBrowse,
  onBack,
  onForward,
  canBack,
  canForward,
  busy,
  browsingDisabled,
}: {
  children: ReactNode;
  categories: Category[];
  categoryId: string;
  collapsed: boolean;
  onToggle: () => void;
  onBrowse: (id: string) => void;
  onBack: () => void;
  onForward: () => void;
  canBack: boolean;
  canForward: boolean;
  busy: boolean;
  browsingDisabled: boolean;
}) {
  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (
        !event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        event.shiftKey ||
        event.repeat ||
        document.querySelector("dialog[open]")
      )
        return;
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        event.preventDefault();
        if (event.key === "ArrowLeft" && canBack && !busy && !browsingDisabled) onBack();
        if (event.key === "ArrowRight" && canForward && !busy && !browsingDisabled) onForward();
      }
    }
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [onBack, onForward, canBack, canForward, busy, browsingDisabled]);
  return (
    <>
      <nav className="navigation-bar" aria-label="Workspace navigation">
        <button
          className="quiet-button"
          disabled={!canBack || busy || browsingDisabled}
          onClick={onBack}
          title="Back (Alt+Left)"
        >
          ← Back
        </button>
        <button
          className="quiet-button"
          disabled={!canForward || busy || browsingDisabled}
          onClick={onForward}
          title="Forward (Alt+Right)"
        >
          Forward →
        </button>
        <button
          className="quiet-button"
          aria-expanded={!collapsed}
          aria-controls="project-navigation"
          onClick={onToggle}
        >
          {collapsed ? "Show sidebar" : "Hide sidebar"}
        </button>
        {busy && (
          <span role="status" className="muted">
            Opening…
          </span>
        )}
      </nav>
      <div className={`workspace-layout ${collapsed ? "sidebar-collapsed" : ""}`}>
        <aside id="project-navigation" className="project-sidebar" hidden={collapsed}>
          <nav aria-label="Browse Categories">
            <button
              aria-current={!categoryId ? "page" : undefined}
              disabled={busy || browsingDisabled}
              onClick={() => onBrowse("")}
            >
              All Entries
            </button>
            <p className="eyebrow">CATEGORIES</p>
            {categories.map((category) => (
              <button
                key={category.id}
                aria-current={categoryId === category.id ? "page" : undefined}
                disabled={busy || browsingDisabled}
                onClick={() => onBrowse(category.id)}
              >
                {category.name}
              </button>
            ))}
          </nav>
        </aside>
        <fieldset className="workspace-content navigation-guard" disabled={busy}>
          {children}
        </fieldset>
      </div>
    </>
  );
}
