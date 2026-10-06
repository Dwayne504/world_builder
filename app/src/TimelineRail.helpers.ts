import type { FictionalDate, Occurrence } from "./timelineTypes";

export function occurrenceDateKey(date: FictionalDate | null) {
  return date ? `${date.year}/${date.month}/${date.day}` : "undated";
}

/** Keep the calendar's established order; a group never implies a time of day. */
export function groupTimelineOccurrences(occurrences: Occurrence[]) {
  const dated = new Map<string, { key: string; date: FictionalDate; occurrences: Occurrence[] }>();
  const undated: Occurrence[] = [];
  for (const occurrence of occurrences) {
    if (!occurrence.date) {
      undated.push(occurrence);
      continue;
    }
    const key = occurrenceDateKey(occurrence.date);
    const group = dated.get(key);
    if (group) group.occurrences.push(occurrence);
    else dated.set(key, { key, date: occurrence.date, occurrences: [occurrence] });
  }
  return { dated: [...dated.values()], undated };
}

export function timelineProximity(distance: number) {
  const fraction = Math.max(0, Math.min(1, 1 - distance / 180));
  return fraction * fraction * (3 - 2 * fraction);
}
