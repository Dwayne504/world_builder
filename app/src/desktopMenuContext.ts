import {
  createContext,
  createElement,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";

export type DesktopMenuId = "file" | "edit" | "view" | "help";

export interface DesktopMenuItem {
  id: string;
  label: string;
  action?: () => void;
  disabled?: boolean;
  children?: DesktopMenuItem[];
  shortcut?: string;
  checked?: boolean;
  separator?: boolean;
  separatorBefore?: boolean;
}

export type DesktopMenuContribution = Partial<Record<DesktopMenuId, DesktopMenuItem[]>>;
type MenuSnapshot = Record<DesktopMenuId, DesktopMenuItem[]>;
const menuIds: DesktopMenuId[] = ["file", "edit", "view", "help"];
const emptySnapshot: MenuSnapshot = { file: [], edit: [], view: [], help: [] };

function mergeItems(base: DesktopMenuItem[], incoming: DesktopMenuItem[]): DesktopMenuItem[] {
  const merged = [...base];
  for (const item of incoming) {
    const index = merged.findIndex((existing) => existing.id === item.id);
    const existing = index === -1 ? undefined : merged[index];
    const next = {
      ...existing,
      ...item,
      disabled: Boolean(item.disabled),
      ...(item.children ? { children: mergeItems(existing?.children ?? [], item.children) } : {}),
    };
    if (index === -1) merged.push(next);
    else merged[index] = next;
  }
  return merged;
}

function createRegistry() {
  const entries = new Map<
    object,
    { scope: string; order: number; sequence: number; commands: DesktopMenuContribution }
  >();
  const sequences = new WeakMap<object, number>();
  const listeners = new Set<() => void>();
  let sequence = 0;
  let snapshot = emptySnapshot;
  const publish = () => {
    const next: MenuSnapshot = { file: [], edit: [], view: [], help: [] };
    const ordered = [...entries.values()].sort(
      (a, b) => a.order - b.order || a.sequence - b.sequence,
    );
    for (const entry of ordered) {
      for (const menu of menuIds) next[menu] = mergeItems(next[menu], entry.commands[menu] ?? []);
    }
    snapshot = next;
    listeners.forEach((listener) => listener());
  };
  return {
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    getSnapshot: () => snapshot,
    register(token: object, scope: string, order: number, commands: DesktopMenuContribution) {
      if (!sequences.has(token)) sequences.set(token, sequence++);
      entries.set(token, { scope, order, sequence: sequences.get(token)!, commands });
      publish();
      return () => {
        entries.delete(token);
        publish();
      };
    },
  };
}

const DesktopMenuContext = createContext<ReturnType<typeof createRegistry> | null>(null);

export function DesktopMenuProvider({ children }: { children: ReactNode }) {
  const [registry] = useState(createRegistry);
  return createElement(DesktopMenuContext.Provider, { value: registry }, children);
}

/** Contributions may be inline: actions always call the latest committed closure.
 * Higher orders override matching ids; submenu children merge recursively.
 * Separate registrations with the same scope have independent lifetimes.
 */
export function useDesktopCommands(scope: string, commands: DesktopMenuContribution, order = 0) {
  const registry = useContext(DesktopMenuContext);
  const token = useRef({});
  const latest = useRef(commands);
  const signature = JSON.stringify(commands, (_key, value: unknown) =>
    typeof value === "function" ? true : value,
  );
  useLayoutEffect(() => {
    latest.current = commands;
  });
  useLayoutEffect(() => {
    if (!registry) return;
    const wrap = (
      menu: DesktopMenuId,
      items: DesktopMenuItem[],
      parentPath: string[] = [],
    ): DesktopMenuItem[] =>
      items.map((item) => {
        const path = [...parentPath, item.id];
        return {
          ...item,
          ...(item.action
            ? {
                action: () => {
                  let current: DesktopMenuItem | undefined;
                  let siblings = latest.current[menu];
                  for (const id of path) {
                    current = siblings?.find((candidate) => candidate.id === id);
                    siblings = current?.children;
                  }
                  if (!current?.disabled) current?.action?.();
                },
              }
            : {}),
          ...(item.children ? { children: wrap(menu, item.children, path) } : {}),
        };
      });
    const contribution: DesktopMenuContribution = {};
    for (const menu of menuIds) {
      const items = latest.current[menu];
      if (items) contribution[menu] = wrap(menu, items);
    }
    return registry.register(token.current, scope, order, contribution);
  }, [registry, scope, order, signature]);
}

const emptySubscribe = () => () => {};
const getEmptySnapshot = () => emptySnapshot;

export function useDesktopMenuItems() {
  const registry = useContext(DesktopMenuContext);
  return useSyncExternalStore(
    registry?.subscribe ?? emptySubscribe,
    registry?.getSnapshot ?? getEmptySnapshot,
    getEmptySnapshot,
  );
}
