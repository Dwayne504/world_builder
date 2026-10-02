# Timeline foundation

Status: approved roadmap; first dated slice. The author approved optional dates,
one configurable fictional calendar, major events as full Entries and lightweight
occurrences for smaller moments. This explicitly opens timeline work beyond the
older Milestone 01 exclusion. Architecture V2's identity, durability, migration and
data-preservation requirements still apply.

## First author workflow

- Timeline is a desktop sidebar destination. Add an unnamed, undated occurrence
  without any calendar or linked records. Edit its title and notes with autosave.
- Configure one Project calendar when dates become useful: name, optional era
  label, 1–60 ordered named months, and fixed lengths of 1–1,000 days. The initial
  form suggests twelve generically named 30-day months; nothing is saved automatically.
- Exact dates use a signed whole year (−1,000,000 through 1,000,000, including year
  zero), month and day. All three coordinates are present, or the item is undated.
  These are fictional coordinates, never operating-system dates or UTC timestamps.
  Calendar months repeat without leap rules. Same-day events share a date heading;
  their stable identity provides a display tie-breaker, not an asserted time of day.
- Dated occurrences appear chronologically; undated material follows. Search
  titles, notes and linked labels; filter by Entry, Chapter and Archive/Trash state.
  Initially show ten occurrences, with a control to reveal more.
- Link any number of existing Entries and Chapters for context. A major event can
  additionally use a full Entry as its event page, explicitly enabling Event
  capability. Its name, Fields and Relationships stay on that canonical Entry.
  An optional occurrence title labels this particular occurrence, not a copy of
  the event Entry's name. Several occurrences can refer to one major event.
- Entry and Chapter pages expose derived timeline backlinks. Project Search
  includes a Timeline group. Written mentions never create structural links.
- Archive, Trash and Restore operate only on the occurrence. Related Entries,
  Chapter order, manuscript text, Story Roles and relationship states are unchanged.

## Safe calendar changes

Before any dates exist, month counts and lengths are editable. Once any occurrence
has a date, including Archive or Trash, those structural settings are fixed.
Calendar, era and month labels can still be renamed. This first slice does not
silently reinterpret stored dates when month lengths change. Future structural
calendar changes need an explicit reviewed conversion workflow. Undating every
occurrence permits changing the structure again.

## Storage and transaction boundaries

Schema 11 adds a `temporal_occurrence` registry kind, separate from Entry and Story
Unit identity, and an `event` Entry capability. Neither is inferred from Category
names. Occurrence lifecycle remains in `record_identity`. Calendar payload schema
version 1 is validated on read and write; corrupt or unsupported versions cannot
be replaced by timeline commands. The initial calendar is absent, not invented.

Occurrence–Entry and occurrence–Chapter links have unique pairs and restrictive
foreign keys. The event-page reference is distinct from contextual links. Commands
validate targets in the current Project, preserve already-linked inactive targets,
and use the existing serialized database worker and expected global revision.
Notes, dates, links and Event opt-in publish in one transaction. Failed saves retain
the UI draft; stale writes are never automatically rebased. Pending writes drain
before navigation and native close. Only acknowledged changes are described as saved.

Registry reconstruction uses the existing transactional migration path with a
validated external recovery snapshot, a final foreign-key check and fail-closed
newer-schema handling. Restore as Copy changes Project identity only and preserves
all internal occurrence/calendar/link identities. Search remains derived and its
cache version advances without editing authored records.

## Acceptance and deferrals

Tests cover optional stubs, date validity and sorting, same-day/undated behavior,
calendar mutation guards, corrupt/newer configuration, atomic link publication,
competing writes, interrupted processes, migration failure/retry/recovery,
backup/Restore as Copy, search and backlink identity, and UI autosave/focus/errors.
All existing frontend and Rust checks remain required. Only synthetic temporary
Projects may be created or modified during automated verification.

Deferred: time of day, partial/approximate dates, ranges and durations, leap rules,
multiple calendars and conversions, multiple independent timelines, recurrence,
calendar structural conversion, computed ages, historical owner/location/state
queries, continuity calculations, automatic relationship transitions, graph views,
storyboard lanes, permanent deletion and manuscript rewriting.
