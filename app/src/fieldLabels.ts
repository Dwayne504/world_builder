import type { FieldDefinition } from "./types";

export const fieldKinds = {
  short_text: "Short text",
  number: "Number",
  boolean: "Yes / no",
  choice: "Choice",
  multi_choice: "Multi-choice",
};
export function matchingFields(definitions: FieldDefinition[], name: string) {
  return definitions.filter(
    (d) => !d.retired && d.name.trim().toLowerCase() === name.trim().toLowerCase(),
  );
}
export function fieldContext(field: FieldDefinition) {
  const scopes = field.bindings.map((b) => b.label).join("; ");
  return `${fieldKinds[field.kind]}${field.unit ? ` · ${field.unit}` : ""} · ${scopes || "No current defaults"}${field.retired ? " · retired" : ""}`;
}
export function fieldLabel(field: FieldDefinition, definitions: FieldDefinition[]) {
  const label = `${field.name} · ${fieldContext(field)}`;
  const identical = definitions.filter((d) => `${d.name} · ${fieldContext(d)}` === label);
  return identical.length > 1
    ? `${label} · copy ${identical.findIndex((d) => d.id === field.id) + 1}`
    : label;
}
