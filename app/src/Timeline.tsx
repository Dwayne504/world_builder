import { useEffect, useState } from "react";
import { listEntries, readStory, readTimeline } from "./api";
import { Dialog } from "./Dialog";
import { TimelineRail } from "./TimelineRail";
import { occurrenceDateKey } from "./TimelineRail.helpers";
import type { ChapterController } from "./storyTypes";
import { useTimeline } from "./useTimeline";
import { useDesktopCommands } from "./desktopMenuContext";
import {
  dateError,
  dateLabel,
  occurrenceLabel,
  type TimelineLink,
  type TimelineView,
  type WorldCalendar,
  type TimelineSnapshot,
} from "./timelineTypes";

function LinkPages({
  page,
  total,
  label,
  onPage,
}: {
  page: number;
  total: number;
  label: string;
  onPage: (page: number) => void;
}) {
  if (total <= 10) return null;
  return (
    <nav className="timeline-link-pages" aria-label={`${label} pages`}>
      <small>
        {page * 10 + 1}–{Math.min(page * 10 + 10, total)} of {total}
      </small>
      <button
        className="quiet-button"
        aria-label={`Previous ${label}`}
        disabled={page === 0}
        onClick={() => onPage(page - 1)}
      >
        Previous
      </button>
      <button
        className="quiet-button"
        aria-label={`Next ${label}`}
        disabled={(page + 1) * 10 >= total}
        onClick={() => onPage(page + 1)}
      >
        Next
      </button>
    </nav>
  );
}

function LinkPicker({
  title,
  items,
  selected,
  onChange,
  single = false,
}: {
  title: string;
  items: TimelineLink[];
  selected: string[];
  onChange: (ids: string[]) => void;
  single?: boolean;
}) {
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const selectedIds = new Set(selected);
  const search = query.trim().toLocaleLowerCase();
  const selectedItems = items.filter(
    (e) => selectedIds.has(e.id) && e.label.toLocaleLowerCase().includes(search),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(selectedItems.length / 10) - 1));
  const matches = items
    .filter(
      (e) =>
        e.workspaceState === "active" &&
        !selectedIds.has(e.id) &&
        e.label.toLocaleLowerCase().includes(search),
    )
    .slice(0, 10);
  return (
    <details className="timeline-picker">
      <summary>
        {title} <small>{selected.length || "None"}</small>
      </summary>
      <label>
        Find {title.toLowerCase()}
        <input
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setPage(0);
          }}
          type="search"
        />
      </label>
      {!!selected.length && (
        <p className="timeline-picker-heading">
          Linked {title.toLowerCase()} · {selectedItems.length}
        </p>
      )}
      <ul
        key={`${currentPage}:${search}`}
        className="timeline-link-list"
        aria-label={`Selected ${title}`}
      >
        {selectedItems.slice(currentPage * 10, currentPage * 10 + 10).map((item) => (
          <li key={item.id}>
            <span className="timeline-picker-label" title={item.label}>
              {item.label}
              {item.workspaceState !== "active" ? ` · ${item.workspaceState}` : ""}
            </span>
            <button
              className="quiet-button"
              onClick={() => onChange(selected.filter((id) => id !== item.id))}
              aria-label={`Unlink ${item.label}`}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
      {!!selected.length && !selectedItems.length && (
        <p className="muted">No linked records match this search.</p>
      )}
      <LinkPages
        page={currentPage}
        total={selectedItems.length}
        label={`selected ${title}`}
        onPage={setPage}
      />
      <p className="timeline-picker-heading">Available {title.toLowerCase()}</p>
      <ul className="timeline-link-list" aria-label={`Available ${title}`}>
        {matches.map((item) => (
          <li key={item.id}>
            <span className="timeline-picker-label" title={item.label}>
              {item.label}
            </span>
            <button
              className="quiet-button"
              onClick={() => {
                onChange(single ? [item.id] : [...selected, item.id]);
                setQuery("");
                setPage(0);
              }}
              aria-label={`Link ${item.label}`}
            >
              {single ? "Use as event page" : "Link"}
            </button>
          </li>
        ))}
      </ul>
      {!matches.length && <p className="muted">No matching unlinked records.</p>}
      <small className="muted">Up to 10 matches. Type to narrow the list.</small>
    </details>
  );
}

function LinkedNavigation({
  items,
  onOpen,
}: {
  items: (TimelineLink & { kind: "Entry" | "Chapter" })[];
  onOpen: (id: string, kind: "Entry" | "Chapter") => void;
}) {
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const matches = items.filter((item) =>
    item.label.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const currentPage = Math.min(page, Math.max(0, Math.ceil(matches.length / 10) - 1));
  if (!items.length) return null;
  return (
    <details className="timeline-picker timeline-linked-pages">
      <summary>
        Open linked pages <small>{items.length}</small>
      </summary>
      {items.length > 10 && (
        <label>
          Find a linked page
          <input
            type="search"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(0);
            }}
          />
        </label>
      )}
      <ul key={`${currentPage}:${query}`} className="timeline-link-list" aria-label="Linked pages">
        {matches.slice(currentPage * 10, currentPage * 10 + 10).map((item) => (
          <li key={`${item.kind}:${item.id}`}>
            <button
              className="quiet-button timeline-picker-label"
              title={item.label}
              onClick={() => onOpen(item.id, item.kind)}
            >
              {item.label}
            </button>
            <small className="muted">{item.kind}</small>
          </li>
        ))}
      </ul>
      {!matches.length && <p className="muted">No linked pages match this search.</p>}
      <LinkPages page={currentPage} total={matches.length} label="linked pages" onPage={setPage} />
    </details>
  );
}
function CalendarForm({
  value,
  onChange,
  locked,
  onSave,
}: {
  value: WorldCalendar;
  onChange: (value: WorldCalendar) => void;
  locked: boolean;
  onSave: () => void;
}) {
  return (
    <div className="timeline-calendar-form">
      <p>
        One calendar for this world. Years include zero and may be negative. Dates are optional;
        Chapter reading order stays separate.
      </p>
      <div className="timeline-date">
        <label>
          Calendar name
          <input
            value={value.name}
            maxLength={200}
            onChange={(e) => onChange({ ...value, name: e.target.value })}
          />
        </label>
        <label>
          Era label (optional)
          <input
            value={value.eraLabel}
            maxLength={80}
            placeholder="After landing"
            onChange={(e) => onChange({ ...value, eraLabel: e.target.value })}
          />
        </label>
      </div>
      <p className="muted">
        The era label is shown after the year, for example “120 After Landing”. It does not change
        dates or calculations.
      </p>
      <p className="muted">
        Fixed month lengths, repeated every year.{" "}
        {locked
          ? "Existing dates keep month counts and lengths fixed. You can rename the labels."
          : "Choose month names and day counts before dating your occurrences."}
      </p>
      <ol className="timeline-months">
        {value.months.map((month, i) => (
          <li key={i}>
            <label>
              <span className="sr-only">Month {i + 1} name</span>
              <input
                value={month.name}
                maxLength={200}
                onChange={(e) =>
                  onChange({
                    ...value,
                    months: value.months.map((m, n) =>
                      n === i ? { ...m, name: e.target.value } : m,
                    ),
                  })
                }
              />
            </label>
            <label>
              Days
              <input
                type="number"
                min={1}
                max={1000}
                disabled={locked}
                value={month.days || ""}
                onChange={(e) =>
                  onChange({
                    ...value,
                    months: value.months.map((m, n) =>
                      n === i ? { ...m, days: Number(e.target.value) } : m,
                    ),
                  })
                }
              />
            </label>
            <button
              className="quiet-button"
              disabled={locked || value.months.length === 1}
              onClick={() => onChange({ ...value, months: value.months.filter((_, n) => n !== i) })}
              aria-label={`Remove month ${i + 1}`}
            >
              Remove
            </button>
          </li>
        ))}
      </ol>
      <div className="row">
        <button
          disabled={locked || value.months.length >= 60}
          onClick={() =>
            onChange({
              ...value,
              months: [...value.months, { name: `Month ${value.months.length + 1}`, days: 30 }],
            })
          }
        >
          Add month
        </button>
        <button onClick={onSave}>Save calendar</button>
      </div>
    </div>
  );
}
const newCalendar = (): WorldCalendar => ({
  name: "World calendar",
  eraLabel: "",
  months: Array.from({ length: 12 }, (_, i) => ({ name: `Month ${i + 1}`, days: 30 })),
});

export function Timeline({
  projectId,
  view,
  onViewChange,
  onController,
  onRevision,
  onEntry,
  onChapter,
  locked,
}: {
  projectId: string;
  view: TimelineView;
  onViewChange: (view: TimelineView) => void;
  onController: (controller: ChapterController) => void;
  onRevision: (revision: number) => void;
  onEntry: (id: string) => void;
  onChapter: (id: string) => void;
  locked: boolean;
}) {
  const timeline = useTimeline(projectId, onRevision);
  const { snapshot, draft, state, error, flush } = timeline;
  const [entries, setEntries] = useState<TimelineLink[]>([]);
  const [chapters, setChapters] = useState<TimelineLink[]>([]);
  const [catalogError, setCatalogError] = useState(false);
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [calendarValue, setCalendarValue] = useState<WorldCalendar>(newCalendar);
  const [entryFilter, setEntryFilter] = useState("");
  const [chapterFilter, setChapterFilter] = useState("");
  const [presentation, setPresentation] = useState<"rail" | "list">("rail");
  const [page, setPage] = useState({ filter: "", index: 0 });
  useEffect(() => {
    let current = true;
    void Promise.all([listEntries(projectId), readStory(projectId)])
      .then(([es, cs]) => {
        if (!current) return;
        setEntries(
          es.map((e) => ({
            id: e.id,
            label: e.authoredName ?? "[Unnamed Entry]",
            workspaceState: "active",
          })),
        );
        setChapters(
          cs.chapters.map((c) => ({
            id: c.id,
            label: c.title.trim() || "[Untitled Chapter]",
            workspaceState: c.workspaceState,
          })),
        );
      })
      .catch(() => {
        if (current) setCatalogError(true);
      });
    return () => {
      current = false;
    };
  }, [projectId]);
  useEffect(() => {
    onController({
      state,
      submit: flush,
      canSubmit: !dateError(draft?.date ?? null, snapshot?.calendar ?? null),
      autoFlush: true,
    });
  }, [state, draft, snapshot?.calendar, flush, onController]);
  const chosen = snapshot?.occurrences.find((o) => o.id === view.occurrenceId);
  // A navigation visit loads the latest committed occurrence; saves keep its inputs mounted.
  useEffect(() => {
    if (view.occurrenceId && !draft) timeline.edit(view.occurrenceId);
  }, [view.occurrenceId, snapshot, draft, timeline]);
  const busy = locked || state === "saving";
  const problem = dateError(draft?.date ?? null, snapshot?.calendar ?? null);
  async function closeEditor() {
    const result = await flush();
    if (result.kind === "committed" || result.kind === "no-op") {
      timeline.edit(null);
      onViewChange({ ...view, occurrenceId: null });
    }
  }
  async function openOccurrence(id: string) {
    const result = await flush();
    if (result.kind === "committed" || result.kind === "no-op") {
      timeline.edit(id);
      onViewChange({ ...view, occurrenceId: id });
    }
  }
  async function create() {
    const before = new Set(snapshot?.occurrences.map((o) => o.id));
    const result = await timeline.run({ kind: "create" });
    if (result.kind === "committed") {
      // The acknowledged create is durable before selecting its identity.
      const item = timeline.getSnapshot()?.occurrences.find((o) => !before.has(o.id));
      if (item) onViewChange({ ...view, occurrenceId: item.id });
    }
  }
  const mergeLinks = (items: TimelineLink[], held: TimelineLink[]) => [
    ...new Map([...items, ...held].map((e) => [e.id, e])).values(),
  ];
  const allEntries = mergeLinks(
    entries,
    snapshot?.occurrences.flatMap((o) => [...o.entries, ...(o.eventEntry ? [o.eventEntry] : [])]) ??
      [],
  );
  const allChapters = mergeLinks(chapters, snapshot?.occurrences.flatMap((o) => o.chapters) ?? []);
  const matches =
    snapshot?.occurrences.filter(
      (o) =>
        o.workspaceState === view.state &&
        (!view.entryId ||
          o.eventEntry?.id === view.entryId ||
          o.entries.some((e) => e.id === view.entryId)) &&
        (!view.chapterId || o.chapters.some((c) => c.id === view.chapterId)) &&
        [
          occurrenceLabel(o),
          o.notes,
          ...o.entries.map((e) => e.label),
          ...o.chapters.map((c) => c.label),
          o.eventEntry?.label ?? "",
        ]
          .join(" ")
          .toLocaleLowerCase()
          .includes(view.query.toLocaleLowerCase()),
    ) ?? [];
  const filterKey = JSON.stringify([
    projectId,
    view.query,
    view.state,
    view.entryId,
    view.chapterId,
  ]);
  useEffect(() => {
    setPage({ filter: filterKey, index: 0 });
  }, [filterKey]);
  const pageIndex =
    page.filter === filterKey
      ? Math.min(page.index, Math.max(0, Math.ceil(matches.length / 50) - 1))
      : 0;
  const pageStart = pageIndex * 50;
  const visibleLimit = Math.min(50, Math.max(10, view.limit));
  const visible = matches.slice(pageStart, pageStart + visibleLimit);
  const filtered = !!(view.query || view.entryId || view.chapterId);
  function clearFilters() {
    onViewChange({ ...view, entryId: "", chapterId: "", query: "", limit: 10 });
  }
  function openCalendar() {
    setCalendarValue(snapshot?.calendar ?? newCalendar());
    setCalendarOpen(true);
  }
  useDesktopCommands(
    "timeline",
    {
      edit: [
        {
          id: "timeline",
          label: "Timeline",
          children: [
            {
              id: "occurrence-add",
              label: "Add occurrence",
              disabled: !snapshot || busy || !!draft,
              action: () => void create(),
            },
            {
              id: "calendar",
              label: snapshot?.calendar ? "Calendar settings…" : "Set up calendar…",
              disabled: !snapshot || busy || !!draft,
              action: openCalendar,
            },
          ],
        },
      ],
    },
    20,
  );
  return (
    <section className="panel timeline" aria-label="Timeline">
      <div className="section-heading">
        <div>
          <p className="eyebrow">FICTIONAL CHRONOLOGY</p>
          <h2>Timeline</h2>
        </div>
        <div className="row">
          <button
            className="quiet-button"
            disabled={!snapshot || busy || !!draft}
            onClick={openCalendar}
          >
            {snapshot?.calendar ? "Calendar settings" : "Set up calendar"}
          </button>
          <button disabled={!snapshot || busy || !!draft} onClick={() => void create()}>
            Add occurrence
          </button>
        </div>
      </div>
      <p className="muted">
        Small moments or major events, in your world’s time. Undated ideas are welcome.
      </p>
      {!snapshot && !error && <p role="status">Loading timeline…</p>}
      {error && !draft && !calendarOpen && (
        <div role="alert">
          <p>{error}</p>
          <button onClick={() => void timeline.reload()}>Reload timeline</button>
        </div>
      )}
      <div className="timeline-workspace-toolbar">
        <label>
          Search timeline
          <input
            type="search"
            placeholder="Find a moment, person, place…"
            value={view.query}
            onChange={(e) => onViewChange({ ...view, query: e.target.value, limit: 10 })}
          />
        </label>
        <label>
          View
          <select
            value={view.state}
            onChange={(e) =>
              onViewChange({ ...view, state: e.target.value as TimelineView["state"], limit: 10 })
            }
          >
            <option value="active">Timeline</option>
            <option value="archived">Archive</option>
            <option value="trashed">Trash</option>
          </select>
        </label>
        <div className="timeline-view-switch" role="group" aria-label="Timeline presentation">
          <button aria-pressed={presentation === "rail"} onClick={() => setPresentation("rail")}>
            Timeline
          </button>
          <button aria-pressed={presentation === "list"} onClick={() => setPresentation("list")}>
            List
          </button>
        </div>
      </div>
      <details className="timeline-filters-more">
        <summary>
          Filter linked Entries or Chapters{view.entryId || view.chapterId ? " · filtered" : ""}
        </summary>
        <div className="timeline-filters">
          <label>
            Find an Entry
            <input
              type="search"
              value={entryFilter}
              onChange={(e) => setEntryFilter(e.target.value)}
            />
            <select
              aria-label="Linked Entry filter"
              value={view.entryId}
              onChange={(e) => onViewChange({ ...view, entryId: e.target.value, limit: 10 })}
            >
              <option value="">All Entries</option>
              {allEntries
                .filter(
                  (e) =>
                    e.id === view.entryId ||
                    e.label.toLocaleLowerCase().includes(entryFilter.toLocaleLowerCase()),
                )
                .sort((a, b) => Number(b.id === view.entryId) - Number(a.id === view.entryId))
                .slice(0, 10)
                .map((e) => (
                  <option key={e.id} value={e.id}>
                    {e.label}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Find a Chapter
            <input
              type="search"
              value={chapterFilter}
              onChange={(e) => setChapterFilter(e.target.value)}
            />
            <select
              aria-label="Linked Chapter filter"
              value={view.chapterId}
              onChange={(e) => onViewChange({ ...view, chapterId: e.target.value, limit: 10 })}
            >
              <option value="">All Chapters</option>
              {allChapters
                .filter(
                  (c) =>
                    c.id === view.chapterId ||
                    c.label.toLocaleLowerCase().includes(chapterFilter.toLocaleLowerCase()),
                )
                .sort((a, b) => Number(b.id === view.chapterId) - Number(a.id === view.chapterId))
                .slice(0, 10)
                .map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.label}
                  </option>
                ))}
            </select>
          </label>
        </div>
        <small className="muted">Up to 10 matches per picker. Search to find others.</small>
      </details>
      <div className="timeline-results-heading">
        <p className="muted" role="status">
          {matches.length} occurrences
          {snapshot?.calendar ? ` · ${snapshot.calendar.name}` : " · No calendar set up"}
        </p>
        {filtered && (
          <button className="quiet-button" onClick={clearFilters}>
            Clear filters
          </button>
        )}
      </div>
      {presentation === "rail" ? (
        <TimelineRail
          occurrences={visible}
          calendar={snapshot?.calendar ?? null}
          selectedId={view.occurrenceId}
          viewKey={`${filterKey}:${pageIndex}`}
          disabled={busy}
          onSelect={(id) => void openOccurrence(id)}
        />
      ) : (
        <ol className="timeline-list" aria-label="Timeline occurrences">
          {visible.map((o, i) => (
            <li key={o.id}>
              {(i === 0 ||
                occurrenceDateKey(visible[i - 1].date) !== occurrenceDateKey(o.date)) && (
                <h3 className="timeline-date-heading">
                  {dateLabel(o.date, snapshot?.calendar ?? null)}
                </h3>
              )}
              <button
                className="timeline-occurrence quiet-button"
                data-navigation-focus={`occurrence-${o.id}`}
                aria-pressed={view.occurrenceId === o.id}
                onClick={() => void openOccurrence(o.id)}
                disabled={busy}
              >
                <span>{occurrenceLabel(o)}</span>
                <small className="muted">
                  {[
                    o.eventEntry ? "Event" : "Moment",
                    ...o.entries.map((e) => e.label),
                    ...o.chapters.map((c) => c.label),
                  ].join(" · ")}
                </small>
                <span aria-hidden="true">›</span>
              </button>
            </li>
          ))}
        </ol>
      )}
      {snapshot && !matches.length && (
        <div className="timeline-empty">
          <h3>
            {filtered
              ? "No moments match these filters"
              : view.state === "active"
                ? "Every world begins with a moment"
                : view.state === "archived"
                  ? "Your archive is clear"
                  : "No occurrences in Trash"}
          </h3>
          <p>
            {filtered
              ? "Try another name, search the notes, or clear your filters to see the whole timeline."
              : view.state === "active"
                ? "Add your first occurrence above. A title, a date, and links can all come later."
                : "Occurrences you move here keep their dates, notes, and links."}
          </p>
        </div>
      )}
      {matches.length > 0 && (
        <div className="timeline-pagination">
          <p>
            Showing {pageStart + 1}–{pageStart + visible.length} of {matches.length}
          </p>
          <div className="row">
            {pageIndex > 0 && (
              <button onClick={() => setPage({ filter: filterKey, index: pageIndex - 1 })}>
                Previous 50
              </button>
            )}
            {pageStart + visible.length < matches.length && visibleLimit < 50 && (
              <button onClick={() => onViewChange({ ...view, limit: visibleLimit + 10 })}>
                Show 10 more
              </button>
            )}
            {visibleLimit === 50 && pageStart + 50 < matches.length && (
              <button onClick={() => setPage({ filter: filterKey, index: pageIndex + 1 })}>
                Next 50
              </button>
            )}
          </div>
        </div>
      )}
      <Dialog
        open={!!chosen && !!draft}
        title={chosen ? occurrenceLabel(chosen) : "Occurrence"}
        onClose={() => void closeEditor()}
        className="timeline-dialog"
      >
        {chosen && draft && (
          <>
            <fieldset
              className="plain-fieldset"
              disabled={locked || chosen.workspaceState !== "active"}
            >
              <section className="timeline-editor-section" aria-label="The moment">
                <h3>The moment</h3>
                <label>
                  Occurrence title (optional)
                  <input
                    value={draft.title}
                    maxLength={1000}
                    placeholder={chosen.eventEntry?.label ?? "A moment in your world"}
                    onChange={(e) => timeline.change({ ...draft, title: e.target.value })}
                  />
                </label>
                <label>
                  Notes
                  <textarea
                    rows={5}
                    value={draft.notes}
                    placeholder="What happened, and why does it matter?"
                    onChange={(e) => timeline.change({ ...draft, notes: e.target.value })}
                  />
                </label>
              </section>
              <section className="timeline-editor-section" aria-label="When it happened">
                <h3>When it happened</h3>
                <p className="muted">
                  Dates are optional. Moments on the same day share a date, with no time of day
                  implied.
                </p>
                <div className="row">
                  <label className="timeline-check">
                    <input
                      type="checkbox"
                      disabled={!snapshot?.calendar}
                      checked={!!draft.date}
                      onChange={(e) =>
                        timeline.change({
                          ...draft,
                          date: e.target.checked ? { year: 1, month: 1, day: 1 } : null,
                        })
                      }
                    />
                    Give this occurrence a date
                  </label>
                  {!snapshot?.calendar && (
                    <small className="muted">Set up your calendar from the Timeline first.</small>
                  )}
                </div>
                {draft.date && snapshot?.calendar && (
                  <div className="timeline-date">
                    <label>
                      Year
                      <input
                        type="number"
                        min={-1000000}
                        max={1000000}
                        value={Number.isFinite(draft.date.year) ? draft.date.year : ""}
                        onChange={(e) =>
                          timeline.change({
                            ...draft,
                            date: {
                              ...draft.date!,
                              year: e.target.value === "" ? NaN : Number(e.target.value),
                            },
                          })
                        }
                      />
                    </label>
                    <label>
                      Month
                      <select
                        value={draft.date.month}
                        onChange={(e) =>
                          timeline.change({
                            ...draft,
                            date: { ...draft.date!, month: Number(e.target.value) },
                          })
                        }
                      >
                        {snapshot.calendar.months.map((m, i) => (
                          <option key={i} value={i + 1}>
                            {m.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      Day
                      <input
                        type="number"
                        min={1}
                        max={snapshot.calendar.months[draft.date.month - 1]?.days}
                        value={draft.date.day || ""}
                        onChange={(e) =>
                          timeline.change({
                            ...draft,
                            date: { ...draft.date!, day: Number(e.target.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                )}
                {problem && <p role="alert">{problem}</p>}
              </section>
              <section className="timeline-editor-section" aria-label="Connections">
                <h3>Connections</h3>
                <p className="muted">Connect the people, places, and Chapters involved.</p>
                {catalogError ? (
                  <p role="alert">
                    Entry and Chapter choices could not be loaded. Reopen the Timeline to retry.
                  </p>
                ) : (
                  <>
                    <LinkPicker
                      title="Entries"
                      items={allEntries}
                      selected={draft.entryIds}
                      onChange={(ids) => timeline.change({ ...draft, entryIds: ids })}
                    />
                    <LinkPicker
                      title="Chapters"
                      items={allChapters}
                      selected={draft.chapterIds}
                      onChange={(ids) => timeline.change({ ...draft, chapterIds: ids })}
                    />
                    <p className="muted">
                      Choose the Entry describing the event itself, such as “The Coronation”, to
                      give this occurrence a full page with Fields and Relationships. Link people
                      and places under Entries above. Choosing an event page marks it as an Event;
                      it does not copy its data or change its Category.
                    </p>
                    <LinkPicker
                      title="Event page"
                      single
                      items={allEntries}
                      selected={draft.eventEntryId ? [draft.eventEntryId] : []}
                      onChange={(ids) =>
                        timeline.change({ ...draft, eventEntryId: ids[0] ?? null })
                      }
                    />
                  </>
                )}
              </section>
            </fieldset>
            {error && (
              <div role="alert">
                <p>{error}</p>
                <button disabled={busy || !!problem} onClick={() => void flush()}>
                  Retry save
                </button>
                <button
                  disabled={busy}
                  onClick={() => {
                    void timeline.discard();
                    onViewChange({ ...view, occurrenceId: null });
                  }}
                >
                  Discard edits and reload
                </button>
              </div>
            )}
            <LinkedNavigation
              key={chosen.id}
              items={[
                ...mergeLinks(chosen.entries, chosen.eventEntry ? [chosen.eventEntry] : []).map(
                  (item) => ({ ...item, kind: "Entry" as const }),
                ),
                ...chosen.chapters.map((item) => ({ ...item, kind: "Chapter" as const })),
              ]}
              onOpen={async (id, kind) => {
                const result = await flush();
                if (result.kind === "no-op" || result.kind === "committed") {
                  if (kind === "Entry") onEntry(id);
                  else onChapter(id);
                }
              }}
            />
            <details>
              <summary>Archive &amp; Trash</summary>
              <p className="muted">
                Keeps this occurrence, its date, notes and links. Linked Entries and Chapters stay
                unchanged.
              </p>
              <div className="row">
                {(chosen.workspaceState === "active"
                  ? (["archived", "trashed"] as const)
                  : (["active"] as const)
                ).map((next) => (
                  <button
                    disabled={busy}
                    key={next}
                    onClick={async () => {
                      const result = await flush();
                      if (result.kind !== "no-op" && result.kind !== "committed") return;
                      const changed = await timeline.run({
                        kind: "set_state",
                        id: chosen.id,
                        state: next,
                      });
                      if (changed.kind === "committed") {
                        timeline.edit(null);
                        onViewChange({ ...view, occurrenceId: null });
                      }
                    }}
                  >
                    {next === "active"
                      ? "Restore occurrence"
                      : next === "archived"
                        ? "Archive occurrence"
                        : "Move occurrence to Trash"}
                  </button>
                ))}
              </div>
            </details>
            <small role="status" className="muted">
              {state === "saved"
                ? "Saved"
                : state === "saving"
                  ? "Saving…"
                  : state === "failed"
                    ? "Not saved"
                    : "Unsaved changes"}
            </small>
          </>
        )}
      </Dialog>
      <Dialog
        open={calendarOpen}
        title="World calendar"
        onClose={async () => {
          const result = await flush();
          if (result.kind === "no-op" || result.kind === "committed") setCalendarOpen(false);
        }}
        className="timeline-dialog"
      >
        <fieldset className="plain-fieldset" disabled={busy}>
          <CalendarForm
            value={calendarValue}
            onChange={(next) => {
              setCalendarValue(next);
              timeline.changeCalendar(next);
            }}
            locked={snapshot?.occurrences.some((o) => !!o.date) ?? false}
            onSave={async () => {
              timeline.changeCalendar(calendarValue);
              const result = await flush();
              if (result.kind === "committed") setCalendarOpen(false);
            }}
          />
        </fieldset>
        {calendarOpen && error && <p role="alert">{error}</p>}
        <button
          className="quiet-button"
          disabled={busy}
          onClick={() => {
            void timeline.discard();
            setCalendarOpen(false);
          }}
        >
          Cancel calendar edits
        </button>
      </Dialog>
    </section>
  );
}

export function TimelineUsage({
  projectId,
  entryId,
  chapterId,
  onOpen,
}: {
  projectId: string;
  entryId?: string;
  chapterId?: string;
  onOpen: (id: string) => void;
}) {
  const [snapshot, setSnapshot] = useState<TimelineSnapshot | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    let current = true;
    void readTimeline(projectId)
      .then((s) => {
        if (current) setSnapshot(s);
      })
      .catch(() => {
        if (current) setError(true);
      });
    return () => {
      current = false;
    };
  }, [projectId, entryId, chapterId]);
  const items =
    snapshot?.occurrences.filter(
      (o) =>
        (entryId && (o.eventEntry?.id === entryId || o.entries.some((e) => e.id === entryId))) ||
        (chapterId && o.chapters.some((c) => c.id === chapterId)),
    ) ?? [];
  if (!items.length && !error) return null;
  return (
    <section className="story-usage" aria-label="Timeline usage">
      <h3>On the timeline</h3>
      {error && <p role="alert">Timeline links could not be loaded.</p>}
      <ul className="timeline-link-list">
        {items.map((o) => (
          <li key={o.id}>
            <button className="quiet-button" onClick={() => onOpen(o.id)}>
              {occurrenceLabel(o)}
            </button>
            <small className="muted">
              {dateLabel(o.date, snapshot?.calendar ?? null)}
              {o.workspaceState !== "active" ? ` · ${o.workspaceState}` : ""}
            </small>
          </li>
        ))}
      </ul>
    </section>
  );
}
