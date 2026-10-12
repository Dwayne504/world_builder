import type { TypeRetirementPreview } from "./types";
export const typeRetirementPreview = (
  changes: Partial<TypeRetirementPreview> = {},
): TypeRetirementPreview => ({
  globalRevision: 4,
  typeId: "sword",
  categoryId: "weapons",
  name: "Sword",
  retiredAt: null,
  assignedEntries: [
    {
      id: "blade",
      label: "Old Blade",
      typeId: "sword",
      typeName: "Sword",
      workspaceState: "active",
    },
  ],
  descendantTypes: [{ id: "cutlass", name: "Cutlass", parentTypeId: "sword", retiredAt: null }],
  directEntryCount: 1,
  inheritedEntryCount: 0,
  defaultFields: [{ id: "mass", name: "Mass", kind: "number" }],
  ...changes,
});
