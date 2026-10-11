import type { TypeDef } from "./types";

export const typeStatusLabel = (type: Pick<TypeDef, "name" | "retiredAt">) =>
  `${type.name}${type.retiredAt ? " · retired" : ""}`;

/** Saved assignments may remain; a newly retired draft stays visible but cannot be applied. */
export function assignmentTypeChoices(types: TypeDef[], currentId = "", selectedId = "") {
  return types
    .filter((type) => !type.retiredAt || type.id === currentId || type.id === selectedId)
    .map((type) => ({
      id: type.id,
      label: typeStatusLabel(type),
      disabled: !!type.retiredAt && type.id !== currentId,
    }));
}
