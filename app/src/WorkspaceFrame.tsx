import { useEffect, useState, type ReactNode } from "react";
import type { Category, Entry } from "./types";

export function WorkspaceFrame({
  children,
  categories,
  entries,
  page,
  categoryId,
  collapsed,
  onToggle,
  onBrowse,
  onRelationships,
  onAddEntry,
  onBack,
  onForward,
  canBack,
  canForward,
  busy,
  browsingDisabled,
}: {
  children: ReactNode;
  categories: Category[];
  entries: Entry[];
  page: "entries" | "relationships";
  categoryId: string;
  collapsed: boolean;
  onToggle: () => void;
  onBrowse: (id: string) => void;
  onRelationships: () => void;
  onAddEntry: (categoryId: string) => void;
  onBack: () => void;
  onForward: () => void;
  canBack: boolean;
  canForward: boolean;
  busy: boolean;
  browsingDisabled: boolean;
}) {
  const [categoriesOpen, setCategoriesOpen] = useState(true);
  const counts = new Map<string, number>();
  for (const entry of entries)
    counts.set(entry.categoryId, (counts.get(entry.categoryId) ?? 0) + 1);
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
          <nav aria-label="Project navigation">
            <button
              className="sidebar-destination"
              aria-label="All Entries"
              aria-describedby="all-entries-count"
              aria-current={page === "entries" && !categoryId ? "page" : undefined}
              disabled={busy || browsingDisabled}
              onClick={() => onBrowse("")}
            >
              <span>All Entries</span>
              <small id="all-entries-count" className="sidebar-count">
                {entries.length}
                <span className="sr-only"> Entries</span>
              </small>
            </button>
            <button
              className="sidebar-destination"
              aria-current={page === "relationships" ? "page" : undefined}
              disabled={busy || browsingDisabled}
              onClick={onRelationships}
            >
              Relationships
            </button>
            <button
              className="sidebar-section-toggle"
              aria-expanded={categoriesOpen}
              aria-controls="sidebar-categories"
              onClick={() => setCategoriesOpen((value) => !value)}
            >
              Categories <span aria-hidden="true">{categoriesOpen ? "▾" : "▸"}</span>
            </button>
            <div id="sidebar-categories" hidden={!categoriesOpen}>
              {categories.map((category) => (
                <div className="sidebar-category" key={category.id}>
                  <button
                    className="sidebar-destination"
                    aria-label={category.name}
                    aria-describedby={`category-count-${category.id}`}
                    aria-current={
                      page === "entries" && categoryId === category.id ? "page" : undefined
                    }
                    disabled={busy || browsingDisabled}
                    onClick={() => onBrowse(category.id)}
                  >
                    <span>{category.name}</span>
                    <small id={`category-count-${category.id}`} className="sidebar-count">
                      {counts.get(category.id) ?? 0}
                      <span className="sr-only"> Entries</span>
                    </small>
                  </button>
                  <button
                    className="sidebar-add quiet-button"
                    aria-label={`Add Entry to ${category.name}`}
                    title={`Add Entry to ${category.name}`}
                    disabled={busy || browsingDisabled}
                    onClick={() => onAddEntry(category.id)}
                  >
                    <span aria-hidden="true">+</span>
                  </button>
                </div>
              ))}
            </div>
          </nav>
        </aside>
        <fieldset className="workspace-content navigation-guard" disabled={busy}>
          {children}
        </fieldset>
      </div>
    </>
  );
}
