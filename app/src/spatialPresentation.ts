import type { SpatialEntry } from "./types";

/** Derived display only. Moves and cycle validation always belong to the backend. */
export function spatialPath(entries: SpatialEntry[], id: string): SpatialEntry[] {
  const byId = new Map(entries.map((entry) => [entry.id, entry]));
  const result: SpatialEntry[] = [];
  const seen = new Set<string>();
  let next = byId.get(id);
  while (next?.spatial && !seen.has(next.id)) {
    result.unshift(next);
    seen.add(next.id);
    next = next.parentId ? byId.get(next.parentId) : undefined;
  }
  return result;
}

export function spatialDescendants(entries: SpatialEntry[], id: string): Set<string> {
  const children = new Map<string, string[]>();
  for (const entry of entries) {
    if (entry.parentId)
      children.set(entry.parentId, [...(children.get(entry.parentId) ?? []), entry.id]);
  }
  const result = new Set<string>();
  const pending = [...(children.get(id) ?? [])];
  while (pending.length) {
    const child = pending.pop()!;
    if (child === id || result.has(child)) continue;
    result.add(child);
    pending.push(...(children.get(child) ?? []));
  }
  return result;
}
