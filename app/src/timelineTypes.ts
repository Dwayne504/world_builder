import type { WorkspaceState } from "./storyTypes";

export interface WorldCalendar {
  name: string;
  eraLabel: string;
  months: { name: string; days: number }[];
}
export interface FictionalDate {
  year: number;
  month: number;
  day: number;
}
export interface TimelineLink {
  id: string;
  label: string;
  workspaceState: string;
}
export interface Occurrence {
  id: string;
  title: string;
  notes: string;
  date: FictionalDate | null;
  eventEntry: TimelineLink | null;
  entries: TimelineLink[];
  chapters: TimelineLink[];
  workspaceState: WorkspaceState;
}
export interface OccurrenceDraft {
  title: string;
  notes: string;
  date: FictionalDate | null;
  eventEntryId: string | null;
  entryIds: string[];
  chapterIds: string[];
}
export interface TimelineSnapshot {
  globalRevision: number;
  calendar: WorldCalendar | null;
  occurrences: Occurrence[];
}
export type TimelineCommand =
  | { kind: "create" }
  | { kind: "configure_calendar"; calendar: WorldCalendar }
  | { kind: "save"; id: string; draft: OccurrenceDraft }
  | { kind: "set_state"; id: string; state: WorkspaceState };
export interface TimelineView {
  occurrenceId: string | null;
  query: string;
  entryId: string;
  chapterId: string;
  state: WorkspaceState;
  limit: number;
}
export const initialTimelineView: TimelineView = {
  occurrenceId: null,
  query: "",
  entryId: "",
  chapterId: "",
  state: "active",
  limit: 10,
};
export const occurrenceLabel = (o: Occurrence) =>
  o.title.trim() || o.eventEntry?.label || "[Untitled occurrence]";
export function occurrenceDraft(o: Occurrence): OccurrenceDraft {
  return {
    title: o.title,
    notes: o.notes,
    date: o.date,
    eventEntryId: o.eventEntry?.id ?? null,
    entryIds: o.entries.map((e) => e.id),
    chapterIds: o.chapters.map((c) => c.id),
  };
}
export function dateLabel(date: FictionalDate | null, calendar: WorldCalendar | null) {
  return date && calendar
    ? `${date.day} ${calendar.months[date.month - 1]?.name ?? "?"}, ${date.year}${calendar.eraLabel ? ` ${calendar.eraLabel}` : ""}`
    : "Undated";
}
export function dateError(
  date: FictionalDate | null,
  calendar: WorldCalendar | null,
): string | null {
  if (!date) return null;
  if (!calendar) return "Set up a calendar before adding dates.";
  if (!Number.isInteger(date.year) || Math.abs(date.year) > 1_000_000)
    return "Enter a whole year from −1,000,000 to 1,000,000.";
  const month = calendar.months[date.month - 1];
  if (!Number.isInteger(date.month) || !month) return "Choose a month.";
  if (!Number.isInteger(date.day) || date.day < 1 || date.day > month.days)
    return `${month.name} has days 1–${month.days}.`;
  return null;
}
