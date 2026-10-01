import { useEffect, useRef, useState } from "react";
import { getEntry, listEntries, readFields, readRelationships, readSpatial } from "./api";
import { Dialog } from "./Dialog";
import { RichTextEditor, type WritingPosition } from "./RichTextEditor";
import { plainDocument, readChapterDraft, storeChapterDraft } from "./chapterRecovery";
import {
  chapterLabel,
  type ChapterController,
  type ChapterSnapshot,
  type DocumentArea,
  type StoryCommand,
  type StoryLink,
  type StoryRole,
} from "./storyTypes";
import type { Category, Entry, EntryFields, EntryRelationships, SpatialSnapshot } from "./types";
import { spatialPath } from "./spatialPresentation";
import { useChapter } from "./useChapter";

function EntryPreview({
  projectId,
  link,
  onOpen,
}: {
  projectId: string;
  link: StoryLink;
  onOpen: (id: string) => void;
}) {
  const [content, setContent] = useState<{
    entry: Entry;
    fields: EntryFields;
    relationships: EntryRelationships;
    spatial: SpatialSnapshot;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    if (link.entryId)
      void Promise.all([
        getEntry(projectId, link.entryId),
        readFields(projectId, link.entryId),
        readRelationships(projectId, link.entryId),
        readSpatial(projectId),
      ])
        .then(([entry, fields, relationships, spatial]) => {
          if (current) setContent({ entry, fields, relationships, spatial });
        })
        .catch(() => {
          if (current) setError("This preview could not be loaded. Your writing is still here.");
        });
    return () => {
      current = false;
    };
  }, [projectId, link.entryId]);
  const path = content && link.entryId ? spatialPath(content.spatial.entries, link.entryId) : [];
  return (
    <div className="chapter-entry-preview">
      {error && <p role="alert">{error}</p>}
      {!content && !error && <p role="status">Loading preview…</p>}
      {content && (
        <>
          {path.length > 1 && (
            <p className="muted">
              Spatial context · derived
              <br />
              {path.map((e) => e.label).join(" › ")}
            </p>
          )}
          <dl>
            {content.fields.fields
              .filter((f) => f.value && !f.hidden)
              .slice(0, 12)
              .map((f) => (
                <div key={f.definition.id}>
                  <dt>{f.definition.name}</dt>
                  <dd>
                    {f.value?.kind === "choices"
                      ? f.value.value
                          .map(
                            (id) =>
                              f.definition.options.find((o) => o.id === id)?.label ??
                              "Retired choice",
                          )
                          .join(", ")
                      : String(f.value?.value ?? "")}
                    {f.definition.unit ? ` ${f.definition.unit}` : ""}
                  </dd>
                </div>
              ))}
          </dl>
          {content.relationships.relationships
            .filter((r) => !r.ended && r.workspaceState === "active")
            .slice(0, 5)
            .map((r) => {
              const source = r.source.id === link.entryId;
              const definition = content.relationships.definitions.find(
                (d) => d.id === r.definitionId,
              );
              return (
                <p key={r.id}>
                  {source ? definition?.forwardLabel : definition?.inverseLabel}{" "}
                  <strong>{source ? r.target.label : r.source.label}</strong>
                </p>
              );
            })}
          <button className="quiet-button" onClick={() => onOpen(content.entry.id)}>
            Open {content.entry.displayName}
          </button>
        </>
      )}
    </div>
  );
}

export function ChapterEditor({
  locked = false,
  initialArea = "manuscript",
  onAreaChange,
  categories = [],
  projectId,
  initial,
  onController,
  onChanged,
  onBack,
  onEntry,
  positions,
  onFindRole,
}: {
  locked?: boolean;
  initialArea?: DocumentArea;
  onAreaChange?: (area: DocumentArea) => void;
  categories?: Category[];
  projectId: string;
  initial: ChapterSnapshot;
  onController: (controller: ChapterController) => void;
  onChanged: (chapter: ChapterSnapshot) => void;
  onBack: () => void;
  onEntry: (id: string) => void;
  onFindRole?: (role: StoryRole) => void;
  positions: Record<string, WritingPosition>;
}) {
  const chapter = useChapter(projectId, initial, onChanged);
  const [area, setArea] = useState<DocumentArea>(initialArea);
  const [contextOpen, setContextOpen] = useState(true);
  const [linkOpen, setLinkOpen] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [roleLinkId, setRoleLinkId] = useState<string | null>(null);
  const [roleQuery, setRoleQuery] = useState("");
  const [createdRole, setCreatedRole] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const [entryError, setEntryError] = useState<string | null>(null);
  const [expandedLinks, setExpandedLinks] = useState<string[]>([]);
  const [roleDraft, setRoleDraft] = useState("");
  const roleRef = useRef(roleDraft);
  roleRef.current = roleDraft;
  const [recovery, setRecovery] = useState(() => readChapterDraft(projectId, initial));
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const [editorEpoch, setEditorEpoch] = useState(0);
  const { flush } = chapter;
  useEffect(
    () =>
      onController({
        state: chapter.state === "saved" && (roleDraft || recovery) ? "dirty" : chapter.state,
        canSubmit: !roleDraft && !recovery,
        autoFlush: !roleDraft && !recovery,
        submit: () => (roleRef.current || recovery ? Promise.resolve({ kind: "failed" }) : flush()),
      }),
    [chapter.state, roleDraft, recovery, flush, onController],
  );
  useEffect(() => {
    if (!linkOpen) return;
    let current = true;
    void listEntries(projectId)
      .then((items) => {
        if (current) {
          setEntries(items);
          setEntryError(null);
        }
      })
      .catch(() => {
        if (current) setEntryError("Entries could not be loaded.");
      });
    return () => {
      current = false;
    };
  }, [projectId, linkOpen]);
  const readOnly = chapter.snapshot.chapter.workspaceState !== "active";
  const roleLink = chapter.snapshot.links.find((link) => link.id === roleLinkId);
  const visibleRoles = chapter.snapshot.roles.filter((role) =>
    role.name.toLocaleLowerCase().includes(roleQuery.trim().toLocaleLowerCase()),
  );
  const blocked = locked || readOnly || chapter.state !== "saved" || !!recovery;
  const available = entries.filter(
    (e) =>
      !chapter.snapshot.links.some((l) => l.entryId === e.id) &&
      e.displayName.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  function command(value: StoryCommand) {
    void chapter.run(value);
  }
  const id = initial.chapter.id;
  const manuscript = chapter.snapshot.documents.find((d) => d.area === "manuscript");
  const workingText = chapter.draft.documents.manuscript
    ? plainDocument(chapter.draft.documents.manuscript)
    : (manuscript?.plainText ?? "");
  const wordCount = [
    ...new Intl.Segmenter(undefined, { granularity: "word" }).segment(workingText),
  ].filter((s) => s.isWordLike).length;
  function discardRecovery() {
    storeChapterDraft(projectId, id, initial.globalRevision, { documents: {} });
    setRecovery(null);
    setRecoveryOpen(false);
  }
  const canRestore =
    recovery &&
    !recovery.readOnlyReason &&
    Object.keys(recovery.draft.documents).every((area) =>
      chapter.snapshot.documents.some((d) => d.area === area && !d.readOnlyReason),
    );
  return (
    <section
      className={`panel chapter-editor ${contextOpen ? "" : "context-collapsed"}`}
      aria-label="Chapter editor"
    >
      <header className="chapter-heading">
        <div>
          <p className="eyebrow">
            CHAPTER{readOnly ? ` · ${chapter.snapshot.chapter.workspaceState}` : ""}
          </p>
          <input
            className="editable-title"
            aria-label="Chapter title"
            placeholder="Untitled Chapter"
            value={chapter.draft.title ?? chapter.snapshot.chapter.title}
            disabled={locked || readOnly || !!recovery}
            onChange={(e) => chapter.changeTitle(e.target.value)}
            onBlur={() => {
              if (chapter.state !== "failed") void flush();
            }}
          />
        </div>
        <div className="row">
          <button
            className="quiet-button"
            aria-expanded={contextOpen}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => setContextOpen((v) => !v)}
          >
            {contextOpen ? "Hide context" : "Show context"}
          </button>
          <button className="quiet-button" onClick={() => setOptionsOpen(true)}>
            Chapter options
          </button>
          <button className="quiet-button" onClick={onBack}>
            All Chapters
          </button>
        </div>
      </header>
      {chapter.error && (
        <div role="alert" className="writing-error">
          <strong>Changes are not being saved.</strong>
          <p>{chapter.error} Your writing is kept here.</p>
          {(chapter.draft.title !== undefined ||
            Object.keys(chapter.draft.documents).length > 0) && (
            <button onClick={() => void flush()}>Retry saving</button>
          )}
          <button
            onClick={() => {
              void chapter.reloadSaved().then((held) => {
                if (held !== undefined) {
                  setRecovery(held);
                  setRecoveryOpen(!!held);
                  setEditorEpoch((n) => n + 1);
                }
              });
            }}
          >
            Review saved version
          </button>
          <details>
            <summary>Copy unsaved writing</summary>
            <textarea
              readOnly
              aria-label="Unsaved writing"
              value={Object.values(chapter.draft.documents)
                .map((d) => plainDocument(d!))
                .join("\n\n")}
            />
          </details>
        </div>
      )}
      {!chapter.recoveryAvailable && (
        <p role="alert">
          The local recovery copy is unavailable. Keep this window open if saving fails.
        </p>
      )}
      {recovery && (
        <p role="status">
          There is an unsaved writing draft from an earlier session.{" "}
          <button onClick={() => setRecoveryOpen(true)}>Review recovered draft</button>
        </p>
      )}
      <div className="chapter-writing-layout">
        <div className="chapter-manuscript">
          <div className="writing-area-tabs" role="tablist" aria-label="Chapter content">
            {(["manuscript", "plan", "notes"] as const).map((value) => (
              <button
                key={value}
                role="tab"
                id={`tab-${value}`}
                aria-controls={`writing-${value}`}
                aria-selected={area === value}
                onClick={() => {
                  setArea(value);
                  onAreaChange?.(value);
                }}
              >
                {value === "manuscript" ? "Manuscript" : value === "plan" ? "Plan" : "Notes"}
              </button>
            ))}
          </div>
          {chapter.snapshot.documents.map((doc) => (
            <div
              key={doc.id}
              role="tabpanel"
              id={`writing-${doc.area}`}
              aria-labelledby={`tab-${doc.area}`}
              hidden={area !== doc.area}
            >
              {doc.readOnlyReason ? (
                <div role="alert">
                  <p>
                    {doc.readOnlyReason} You can copy the preserved text below; other writing areas
                    remain available.
                  </p>
                  <textarea
                    readOnly
                    aria-label={`${doc.area} recovery text`}
                    value={doc.plainText}
                  />
                  <details>
                    <summary>Original document</summary>
                    <textarea
                      readOnly
                      aria-label={`${doc.area} original document`}
                      value={doc.originalJson ?? ""}
                    />
                  </details>
                </div>
              ) : readOnly || recovery ? (
                <pre className="preserved-manuscript">{doc.plainText || "No writing yet."}</pre>
              ) : (
                <RichTextEditor
                  locked={locked}
                  key={`${doc.id}-${editorEpoch}`}
                  label={
                    doc.area === "manuscript"
                      ? "Manuscript"
                      : doc.area === "plan"
                        ? "Chapter plan"
                        : "Chapter notes"
                  }
                  content={chapter.draft.documents[doc.area] ?? doc.content!}
                  onChange={(content) => chapter.changeDocument(doc.area, content)}
                  onBlur={() => {
                    if (chapter.state !== "failed") void flush();
                  }}
                  position={positions[`${id}:${doc.area}`]}
                  onPosition={(value) => {
                    positions[`${id}:${doc.area}`] = value;
                  }}
                />
              )}
            </div>
          ))}
          <footer className="writing-footer">
            <small>{wordCount.toLocaleString()} words · manuscript</small>
            <span role="status">
              {chapter.state === "saved"
                ? "Saved"
                : chapter.state === "saving"
                  ? "Saving…"
                  : chapter.state === "failed"
                    ? "Not saved"
                    : "Unsaved changes"}
            </span>
          </footer>
        </div>
        <aside className="chapter-context" aria-label="Chapter context" hidden={!contextOpen}>
          <div className="section-heading">
            <h3>In this Chapter</h3>
            <button disabled={blocked} onClick={() => setLinkOpen(true)}>
              Link Entry
            </button>
          </div>
          {!chapter.snapshot.links.length && (
            <p className="muted">
              Keep the people, places, and objects you need beside your writing.
            </p>
          )}
          {chapter.snapshot.links.map((link) => (
            <div key={link.id} className="chapter-world-link">
              <details
                onToggle={(e) => {
                  if (e.target !== e.currentTarget) return;
                  const open = e.currentTarget.open;
                  setExpandedLinks((ids) =>
                    open ? [...new Set([...ids, link.id])] : ids.filter((id) => id !== link.id),
                  );
                }}
              >
                <summary>
                  <strong>{link.label}</strong>
                  <small>
                    {link.roles.map((r) => r.name).join(" · ") || "No Roles assigned"}
                    {link.workspaceState !== "active" ? ` · ${link.workspaceState}` : ""}
                  </small>
                </summary>
                {contextOpen && expandedLinks.includes(link.id) && link.entryId && (
                  <EntryPreview projectId={projectId} link={link} onOpen={onEntry} />
                )}
              </details>
              <button
                className="quiet-button chapter-role-button"
                aria-label={`Roles for ${link.label}`}
                onClick={() => {
                  setRoleLinkId(link.id);
                  setRoleQuery("");
                }}
              >
                Roles
              </button>
            </div>
          ))}
        </aside>
      </div>
      <Dialog
        open={!!roleLink && !optionsOpen}
        title={`Roles for ${roleLink?.label ?? "Entry"}`}
        onClose={() => setRoleLinkId(null)}
      >
        {roleLink && (
          <>
            <p className="muted">
              How this Entry is used in this Chapter: POV for the viewpoint character, Appears for
              someone present, or Setting for a place. Choose none, one, or several. Changes save
              automatically.
            </p>
            {chapter.error && (
              <p role="alert">
                {chapter.error} The Role change was not saved. Close this window to review the saved
                version.
              </p>
            )}
            <label>
              Find a Role
              <input
                type="search"
                value={roleQuery}
                onChange={(e) => setRoleQuery(e.target.value)}
              />
            </label>
            <div className="story-role-options">
              {visibleRoles.map((role) => (
                <label key={role.id}>
                  <input
                    type="checkbox"
                    disabled={blocked || !roleLink.entryId || roleLink.workspaceState !== "active"}
                    checked={roleLink.roles.some((r) => r.id === role.id)}
                    onChange={(e) =>
                      command({
                        kind: "set_link",
                        chapterId: id,
                        entryId: roleLink.entryId!,
                        roleIds: e.target.checked
                          ? [...roleLink.roles.map((r) => r.id), role.id]
                          : roleLink.roles.filter((r) => r.id !== role.id).map((r) => r.id),
                      })
                    }
                  />
                  {role.name}
                </label>
              ))}
            </div>
            {!visibleRoles.length && <p>No matching Roles.</p>}
            <button
              className="quiet-button"
              onClick={() => {
                setRoleQuery("");
                setOptionsOpen(true);
              }}
            >
              Manage available Roles
            </button>
            <p className="field-note">
              Removing every Role keeps the Entry linked. Roles also appear in this Entry’s Story
              usage and in Project Search.
            </p>
            <button
              className="quiet-button"
              disabled={blocked}
              onClick={() => command({ kind: "unlink", chapterId: id, linkId: roleLink.id })}
            >
              Remove link
            </button>
          </>
        )}
      </Dialog>
      <Dialog open={linkOpen} title="Link world material" onClose={() => setLinkOpen(false)}>
        <label>
          Find an Entry
          <input value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
        {entryError && <p role="alert">{entryError}</p>}
        <ul className="chapter-entry-results">
          {available.slice(0, 10).map((entry) => (
            <li key={entry.id}>
              <button
                aria-label={entry.displayName}
                disabled={blocked}
                onClick={() => {
                  void chapter
                    .run({ kind: "set_link", chapterId: id, entryId: entry.id, roleIds: [] })
                    .then((result) => {
                      if (result.kind === "committed") {
                        setLinkOpen(false);
                        setQuery("");
                      }
                    });
                }}
              >
                {entry.displayName}
                <small className="muted">
                  {categories.find((category) => category.id === entry.categoryId)?.name}
                </small>
              </button>
            </li>
          ))}
        </ul>
        {available.length > 10 && <p className="muted">Keep typing to narrow the results.</p>}
        {!available.length && <p>No matching unlinked Entries.</p>}
      </Dialog>
      <Dialog open={optionsOpen} title="Chapter options" onClose={() => setOptionsOpen(false)}>
        <h3>Story Roles</h3>
        <p className="muted">
          Roles describe an Entry’s part in a Chapter, such as POV, Appears, or Setting. Create a
          Role here, then choose Roles beside a linked Entry under In this Chapter to assign it.
        </p>
        {chapter.error && (
          <p role="alert">{chapter.error} Close this window to review the saved version.</p>
        )}
        <label>
          Find an available Role
          <input type="search" value={roleQuery} onChange={(e) => setRoleQuery(e.target.value)} />
        </label>
        <ul className="story-role-catalog">
          {visibleRoles.map((role) => {
            const linked = chapter.snapshot.links.filter((link) =>
              link.roles.some((r) => r.id === role.id),
            );
            return (
              <li key={role.id}>
                <div>
                  <strong>{role.name}</strong>
                  <small className="muted">
                    {linked.length
                      ? `In this Chapter: ${linked.map((link) => link.label).join(", ")}`
                      : "Not assigned in this Chapter"}
                  </small>
                </div>
                {onFindRole && (
                  <button
                    className="quiet-button"
                    aria-label={`Find Chapters using ${role.name}`}
                    onClick={() => {
                      setOptionsOpen(false);
                      setRoleLinkId(null);
                      onFindRole(role);
                    }}
                  >
                    Find uses
                  </button>
                )}
              </li>
            );
          })}
        </ul>
        {!visibleRoles.length && <p>No matching Roles.</p>}
        {createdRole && (
          <p role="status">
            {createdRole} is available. Choose Roles beside a linked Entry to assign it.
          </p>
        )}
        <label>
          New Story Role
          <input
            disabled={blocked}
            value={roleDraft}
            onChange={(e) => setRoleDraft(e.target.value)}
          />
        </label>
        <div className="row">
          <button
            disabled={blocked || !roleDraft.trim()}
            onClick={() => {
              void chapter
                .run({ kind: "create_role", chapterId: id, name: roleDraft })
                .then((result) => {
                  if (result.kind === "committed") {
                    setCreatedRole(roleDraft.trim());
                    setRoleDraft("");
                    setRoleQuery("");
                  }
                });
            }}
          >
            Create Role
          </button>
          <button onClick={() => setRoleDraft("")}>Cancel Role draft</button>
        </div>
        <h3>{readOnly ? "Restore Chapter" : "Put this Chapter aside"}</h3>
        <p>
          Archive and Trash keep the writing and Entry links. Restore the Chapter from the Chapters
          page whenever you need it.
        </p>
        <div className="row">
          {readOnly ? (
            <button
              disabled={chapter.state !== "saved"}
              onClick={() => {
                command({ kind: "set_state", chapterId: id, state: "active" });
                setOptionsOpen(false);
              }}
            >
              Restore Chapter
            </button>
          ) : (
            <>
              <button
                disabled={blocked || !!roleDraft}
                onClick={() => {
                  command({ kind: "set_state", chapterId: id, state: "archived" });
                  setOptionsOpen(false);
                }}
              >
                Archive Chapter
              </button>
              <button
                disabled={blocked || !!roleDraft}
                onClick={() => {
                  command({ kind: "set_state", chapterId: id, state: "trashed" });
                  setOptionsOpen(false);
                }}
              >
                Move Chapter to Trash
              </button>
            </>
          )}
        </div>
      </Dialog>
      <Dialog
        open={recoveryOpen}
        title="Review recovered writing"
        onClose={() => setRecoveryOpen(false)}
      >
        {recovery && (
          <>
            {recovery.readOnlyReason && (
              <div role="alert">
                <p>{recovery.readOnlyReason}</p>
                <textarea
                  aria-label="Original recovery data"
                  readOnly
                  value={recovery.originalRaw}
                />
              </div>
            )}
            <p>
              Compare this local draft with the saved Chapter before restoring it. Restoring
              replaces only the writing areas shown here; it does not change Entry links.
            </p>
            {recovery.draft.title !== undefined && (
              <p>
                Recovered title: <strong>{recovery.draft.title || "Untitled Chapter"}</strong> ·
                Saved title: {chapterLabel(chapter.snapshot.chapter)}
              </p>
            )}
            {Object.entries(recovery.draft.documents).map(([area, content]) => (
              <section key={area}>
                <h3>{area}</h3>
                <div className="recovery-comparison">
                  <label>
                    Saved writing
                    <textarea
                      readOnly
                      value={
                        chapter.snapshot.documents.find((d) => d.area === area)?.plainText ?? ""
                      }
                    />
                  </label>
                  <label>
                    Recovered writing
                    <textarea readOnly value={plainDocument(content!)} />
                  </label>
                </div>
              </section>
            ))}
            <div className="row">
              <button
                disabled={!canRestore || readOnly}
                onClick={() => {
                  chapter.restoreDraft(recovery.draft);
                  setEditorEpoch((n) => n + 1);
                  setRecovery(null);
                  setRecoveryOpen(false);
                }}
              >
                Use recovered draft
              </button>
              <button onClick={discardRecovery}>Keep saved Chapter</button>
            </div>
            {!canRestore && (
              <p>
                Copy the recovered writing above. An unsupported document cannot be overwritten.
              </p>
            )}
          </>
        )}
      </Dialog>
    </section>
  );
}
