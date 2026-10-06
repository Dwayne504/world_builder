import {
  Fragment,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import {
  useDesktopMenuItems,
  type DesktopMenuId,
  type DesktopMenuItem,
} from "./desktopMenuContext";
import "./DesktopMenu.css";

const menus: { id: DesktopMenuId; label: string }[] = [
  { id: "file", label: "File" },
  { id: "edit", label: "Edit" },
  { id: "view", label: "View" },
  { id: "help", label: "Help" },
];
const pathKey = (menu: DesktopMenuId, path: string[]) => JSON.stringify([menu, ...path]);
const pathId = (menu: DesktopMenuId, path: string[]) =>
  encodeURIComponent(pathKey(menu, path)).replace(/'/g, "%27");

export function DesktopMenu() {
  const commands = useDesktopMenuItems();
  const prefix = useId();
  const container = useRef<HTMLElement>(null);
  const triggers = useRef(new Map<DesktopMenuId, HTMLButtonElement>());
  const rows = useRef(new Map<string, HTMLButtonElement>());
  const panels = useRef(new Map<string, HTMLUListElement>());
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingFocus = useRef<{ menu: DesktopMenuId; path: string[]; last: boolean } | null>(null);
  const [activeRoot, setActiveRoot] = useState<DesktopMenuId>("file");
  const [openMenu, setOpenMenu] = useState<DesktopMenuId | null>(null);
  const [branch, setBranch] = useState<string[]>([]);

  function cancelHover() {
    if (hoverTimer.current !== null) clearTimeout(hoverTimer.current);
    hoverTimer.current = null;
  }

  useEffect(() => cancelHover, []);

  function itemsAt(menu: DesktopMenuId, path: string[]) {
    let items = commands[menu];
    for (const id of path) items = items.find((item) => item.id === id)?.children ?? [];
    return items;
  }

  function close(restoreFocus = true) {
    cancelHover();
    if (restoreFocus && openMenu) triggers.current.get(openMenu)?.focus();
    pendingFocus.current = null;
    setOpenMenu(null);
    setBranch([]);
  }

  function open(menu: DesktopMenuId, last = false) {
    cancelHover();
    pendingFocus.current = { menu, path: [], last };
    setActiveRoot(menu);
    setBranch([]);
    setOpenMenu(menu);
  }

  function openChild(menu: DesktopMenuId, path: string[], item: DesktopMenuItem) {
    cancelHover();
    const next = [...path, item.id];
    pendingFocus.current = { menu, path: next, last: false };
    setBranch(next);
  }

  function switchRoot(menu: DesktopMenuId, step: number, expand: boolean) {
    const index = menus.findIndex((candidate) => candidate.id === menu);
    const next = menus[(index + step + menus.length) % menus.length].id;
    setActiveRoot(next);
    if (expand) open(next);
    else triggers.current.get(next)?.focus();
  }

  useLayoutEffect(() => {
    for (const [key, panel] of panels.current) {
      const [menu, ...path] = JSON.parse(key) as [DesktopMenuId, ...string[]];
      const anchor = path.length
        ? rows.current.get(pathKey(menu, path))
        : triggers.current.get(menu);
      if (!anchor) continue;
      const rect = anchor.getBoundingClientRect();
      const width = panel.getBoundingClientRect().width;
      const height = panel.getBoundingClientRect().height;
      const inset = 8;
      let left = path.length ? rect.right - 2 : rect.left;
      if (left + width > window.innerWidth - inset) {
        left = path.length ? rect.left - width + 2 : window.innerWidth - width - inset;
      }
      const top = path.length ? rect.top : rect.bottom + 3;
      panel.style.left = `${Math.max(inset, left)}px`;
      panel.style.top = `${Math.max(inset, Math.min(top, window.innerHeight - height - inset))}px`;
    }
    const pending = pendingFocus.current;
    if (!pending || pending.menu !== openMenu) return;
    const items = itemsAt(pending.menu, pending.path).filter(
      (item) => !item.separator && !item.disabled,
    );
    const target = pending.last ? items[items.length - 1] : items[0];
    const element = target
      ? rows.current.get(pathKey(pending.menu, [...pending.path, target.id]))
      : panels.current.get(pathKey(pending.menu, pending.path));
    element?.focus();
    pendingFocus.current = null;
  });

  useEffect(() => {
    if (!openMenu) return;
    const dismissOutside = (event: Event) => {
      if (event.target instanceof Node && !container.current?.contains(event.target)) {
        cancelHover();
        setOpenMenu(null);
        setBranch([]);
        pendingFocus.current = null;
      }
    };
    const dismissOnResize = () => {
      cancelHover();
      triggers.current.get(openMenu)?.focus();
      setOpenMenu(null);
      setBranch([]);
    };
    document.addEventListener("pointerdown", dismissOutside);
    document.addEventListener("focusin", dismissOutside);
    window.addEventListener("resize", dismissOnResize);
    return () => {
      document.removeEventListener("pointerdown", dismissOutside);
      document.removeEventListener("focusin", dismissOutside);
      window.removeEventListener("resize", dismissOnResize);
    };
  }, [openMenu]);

  function onItemKey(
    event: KeyboardEvent,
    menu: DesktopMenuId,
    path: string[],
    item?: DesktopMenuItem,
  ) {
    if (event.nativeEvent.isComposing || event.altKey || event.ctrlKey || event.metaKey) return;
    cancelHover();
    event.stopPropagation();
    const siblings = itemsAt(menu, path).filter(
      (candidate) => !candidate.disabled && !candidate.separator,
    );
    const index = siblings.findIndex((candidate) => candidate.id === item?.id);
    const focus = (next: DesktopMenuItem | undefined) => {
      if (next) {
        rows.current.get(pathKey(menu, [...path, next.id]))?.focus();
        setBranch(path);
      }
    };
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        focus(siblings[(index + step + siblings.length) % siblings.length]);
        break;
      }
      case "Home":
      case "End":
        event.preventDefault();
        focus(event.key === "Home" ? siblings[0] : siblings[siblings.length - 1]);
        break;
      case "ArrowRight":
        event.preventDefault();
        if (item?.children?.length) openChild(menu, path, item);
        else switchRoot(menu, 1, true);
        break;
      case "ArrowLeft":
      case "Escape":
        event.preventDefault();
        if (path.length) {
          rows.current.get(pathKey(menu, path))?.focus();
          setBranch(path.slice(0, -1));
        } else if (event.key === "ArrowLeft") switchRoot(menu, -1, true);
        else close();
        break;
      case "Tab":
        close();
        break;
      default:
        if (
          event.key.length === 1 &&
          event.key !== " " &&
          !event.ctrlKey &&
          !event.metaKey &&
          !event.altKey
        ) {
          const ordered = [...siblings.slice(index + 1), ...siblings.slice(0, index + 1)];
          const match = ordered.find((candidate) =>
            candidate.label.toLocaleLowerCase().startsWith(event.key.toLocaleLowerCase()),
          );
          if (match) {
            event.preventDefault();
            focus(match);
          }
        }
    }
  }

  function renderPanel(menu: DesktopMenuId, path: string[], labelledBy: string) {
    const key = pathKey(menu, path);
    const items = itemsAt(menu, path);
    return (
      <ul
        role="menu"
        aria-labelledby={labelledBy}
        id={`${prefix}-${pathId(menu, path)}`}
        className="desktop-menu-panel"
        tabIndex={-1}
        ref={(element) => {
          if (element) panels.current.set(key, element);
          else panels.current.delete(key);
        }}
        onKeyDown={(event) => onItemKey(event, menu, path)}
        onPointerEnter={cancelHover}
      >
        {items.length === 0 && (
          <li role="presentation" className="desktop-menu-empty">
            No commands available
          </li>
        )}
        {items.map((item) => {
          if (item.separator)
            return <li role="separator" key={item.id} className="desktop-menu-separator" />;
          const itemPath = [...path, item.id];
          const itemKey = pathKey(menu, itemPath);
          const itemId = `${prefix}-${pathId(menu, itemPath)}-item`;
          const hasChildren = Boolean(item.children?.length);
          const expanded = hasChildren && branch[path.length] === item.id && !item.disabled;
          return (
            <Fragment key={item.id}>
              {item.separatorBefore && <li role="separator" className="desktop-menu-separator" />}
              <li role="none">
                <button
                  type="button"
                  id={itemId}
                  role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
                  aria-checked={item.checked}
                  aria-haspopup={hasChildren ? "menu" : undefined}
                  aria-expanded={hasChildren ? expanded : undefined}
                  aria-controls={expanded ? `${prefix}-${pathId(menu, itemPath)}` : undefined}
                  aria-disabled={item.disabled || undefined}
                  disabled={item.disabled}
                  tabIndex={-1}
                  className="desktop-menu-command"
                  ref={(element) => {
                    if (element) rows.current.set(itemKey, element);
                    else rows.current.delete(itemKey);
                  }}
                  onPointerEnter={() => {
                    if (item.disabled) return;
                    cancelHover();
                    rows.current.get(itemKey)?.focus();
                    const next = hasChildren ? itemPath : path;
                    // A short grace period lets the pointer cross another row
                    // diagonally on its way into the currently open submenu.
                    if (branch.length > path.length && branch[path.length] !== item.id) {
                      hoverTimer.current = setTimeout(() => setBranch(next), 160);
                    } else setBranch(next);
                  }}
                  onKeyDown={(event) => onItemKey(event, menu, path, item)}
                  onClick={() => {
                    if (item.disabled) return;
                    if (hasChildren) openChild(menu, path, item);
                    else {
                      close();
                      item.action?.();
                    }
                  }}
                >
                  <span className="desktop-menu-check" aria-hidden="true">
                    {item.checked ? "✓" : ""}
                  </span>
                  <span className="desktop-menu-label">{item.label}</span>
                  {item.shortcut && (
                    <span className="desktop-menu-shortcut" aria-hidden="true">
                      {item.shortcut}
                    </span>
                  )}
                  <span className="desktop-menu-arrow" aria-hidden="true">
                    {hasChildren ? "›" : ""}
                  </span>
                </button>
                {expanded && renderPanel(menu, itemPath, itemId)}
              </li>
            </Fragment>
          );
        })}
      </ul>
    );
  }

  return (
    <nav className="desktop-menu" aria-label="Application menu" ref={container}>
      <div role="menubar" aria-label="Application">
        {menus.map(({ id, label }) => (
          <div role="none" key={id}>
            <button
              type="button"
              role="menuitem"
              id={`${prefix}-${id}-trigger`}
              className="desktop-menu-trigger"
              aria-haspopup="menu"
              aria-expanded={openMenu === id}
              aria-controls={openMenu === id ? `${prefix}-${pathId(id, [])}` : undefined}
              tabIndex={activeRoot === id ? 0 : -1}
              ref={(element) => {
                if (element) triggers.current.set(id, element);
                else triggers.current.delete(id);
              }}
              onFocus={() => setActiveRoot(id)}
              onClick={() => (openMenu === id ? close() : open(id))}
              onPointerEnter={() => {
                if (openMenu && openMenu !== id) open(id);
              }}
              onKeyDown={(event) => {
                if (event.nativeEvent.isComposing || event.altKey || event.ctrlKey || event.metaKey)
                  return;
                cancelHover();
                if (
                  [
                    "ArrowDown",
                    "ArrowUp",
                    "ArrowLeft",
                    "ArrowRight",
                    "Home",
                    "End",
                    "Escape",
                  ].includes(event.key)
                )
                  event.preventDefault();
                if (event.key === "ArrowDown" || event.key === "ArrowUp")
                  open(id, event.key === "ArrowUp");
                else if (event.key === "ArrowLeft" || event.key === "ArrowRight")
                  switchRoot(id, event.key === "ArrowRight" ? 1 : -1, Boolean(openMenu));
                else if (event.key === "Home" || event.key === "End") {
                  const next = event.key === "Home" ? "file" : "help";
                  setActiveRoot(next);
                  if (openMenu) open(next);
                  else triggers.current.get(next)?.focus();
                } else if (event.key === "Escape") close();
                else if (event.key === "Tab") close(false);
              }}
            >
              {label}
            </button>
            {openMenu === id && renderPanel(id, [], `${prefix}-${id}-trigger`)}
          </div>
        ))}
      </div>
    </nav>
  );
}

export default DesktopMenu;
