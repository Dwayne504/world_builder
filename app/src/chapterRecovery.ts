import type { ChapterDraft, ChapterSnapshot } from "./storyTypes";
import { canEditRecoveredDocument } from "./writingSchema";
export interface RecoveryDraft {
  version: 1;
  revision: number;
  savedAt: string;
  draft: ChapterDraft;
  readOnlyReason?: string;
  originalRaw?: string;
}
const key = (project: string, chapter: string) =>
  `worldcrafter:chapter-recovery:${project}:${chapter}`;
/** Best-effort local WebView recovery outside the Project; never a Saved acknowledgement. */
export function storeChapterDraft(
  project: string,
  chapter: string,
  revision: number,
  draft: ChapterDraft,
): boolean {
  try {
    if (draft.title === undefined && !Object.keys(draft.documents).length)
      localStorage.removeItem(key(project, chapter));
    else
      localStorage.setItem(
        key(project, chapter),
        JSON.stringify({ version: 1, revision, savedAt: new Date().toISOString(), draft }),
      );
    return true;
  } catch {
    return false;
  }
}
export function readChapterDraft(project: string, snapshot: ChapterSnapshot): RecoveryDraft | null {
  let raw: string | null = null;
  const unsupported = (): RecoveryDraft => ({
    version: 1,
    revision: 0,
    savedAt: "",
    draft: { documents: {} },
    readOnlyReason:
      "This local recovery copy is damaged or uses an unsupported format. Its original data is preserved below.",
    originalRaw: raw ?? "",
  });
  try {
    raw = localStorage.getItem(key(project, snapshot.chapter.id));
    if (!raw) return null;
    const value = JSON.parse(raw) as RecoveryDraft;
    if (
      value.version !== 1 ||
      !Number.isInteger(value.revision) ||
      typeof value.savedAt !== "string" ||
      !value.draft ||
      typeof value.draft.documents !== "object" ||
      value.draft.documents === null ||
      Array.isArray(value.draft.documents) ||
      (value.draft.title !== undefined && typeof value.draft.title !== "string")
    )
      return unsupported();
    for (const [area, content] of Object.entries(value.draft.documents)) {
      if (!["manuscript", "plan", "notes"].includes(area) || !canEditRecoveredDocument(content))
        return unsupported();
    }
    // A crash after acknowledgement may leave the already-saved emergency copy.
    const documents = Object.fromEntries(
      Object.entries(value.draft.documents).filter(
        ([area, content]) =>
          JSON.stringify(content) !==
          JSON.stringify(snapshot.documents.find((d) => d.area === area)?.content),
      ),
    );
    const title = value.draft.title === snapshot.chapter.title ? undefined : value.draft.title;
    return title === undefined && !Object.keys(documents).length
      ? null
      : { ...value, draft: { title, documents } };
  } catch {
    return raw ? unsupported() : null;
  }
}
export function plainDocument(content: import("@tiptap/react").JSONContent): string {
  if (content.type === "text") return content.text ?? "";
  return (
    (content.content ?? []).map(plainDocument).join("") +
    (["paragraph", "heading", "hardBreak", "horizontalRule"].includes(content.type ?? "")
      ? "\n"
      : "")
  );
}
