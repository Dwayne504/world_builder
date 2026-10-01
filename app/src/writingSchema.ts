import { getSchema } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";

export const writingExtensions = () => [
  StarterKit.configure({
    link: false,
    codeBlock: false,
    heading: { levels: [1, 2, 3] },
    trailingNode: false,
  }),
];
const schema = getSchema(writingExtensions());

/** Recovery must never feed an AST to the editor that it would silently discard. */
export function canEditRecoveredDocument(value: unknown): boolean {
  let nodes = 0;
  function bounded(item: unknown, depth: number): boolean {
    if (depth > 64 || ++nodes > 100_000) return false;
    return (
      !item ||
      typeof item !== "object" ||
      Object.values(item).every((child) => bounded(child, depth + 1))
    );
  }
  function preserves(source: unknown, encoded: unknown): boolean {
    if (source === null || typeof source !== "object") return source === encoded;
    if (Array.isArray(source))
      return (
        (!source.length && encoded === undefined) ||
        (Array.isArray(encoded) &&
          source.length === encoded.length &&
          source.every((child, i) => preserves(child, encoded[i])))
      );
    return (
      !!encoded &&
      typeof encoded === "object" &&
      Object.entries(source).every(([key, child]) =>
        preserves(child, (encoded as Record<string, unknown>)[key]),
      )
    );
  }
  try {
    if (!bounded(value, 0) || JSON.stringify(value).length > 16 * 1024 * 1024) return false;
    const node = schema.nodeFromJSON(value);
    node.check();
    return node.type.name === "doc" && preserves(value, node.toJSON());
  } catch {
    return false;
  }
}
