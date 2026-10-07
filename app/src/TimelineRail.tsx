import { useCallback, useEffect, useId, useRef, type KeyboardEvent } from "react";
import { dateLabel, occurrenceLabel, type Occurrence, type WorldCalendar } from "./timelineTypes";
import { groupTimelineOccurrences, timelineProximity } from "./TimelineRail.helpers";
import "./TimelineRail.css";

export function TimelineRail({
  occurrences,
  calendar,
  selectedId,
  viewKey,
  disabled,
  onSelect,
}: {
  occurrences: Occurrence[];
  calendar: WorldCalendar | null;
  selectedId: string | null;
  viewKey: string;
  disabled: boolean;
  onSelect: (id: string) => void;
}) {
  const { dated, undated } = groupTimelineOccurrences(occurrences);
  const rail = useRef<HTMLDivElement>(null);
  const frame = useRef<number | null>(null);
  const reduceMotion = useRef(false);
  const instructionsId = useId();

  const resetProximity = useCallback(() => {
    if (frame.current !== null) cancelAnimationFrame(frame.current);
    frame.current = null;
    rail.current?.querySelectorAll<HTMLElement>(".timeline-rail-event").forEach((item) => {
      item.style.removeProperty("--timeline-nearness");
      delete item.dataset.near;
    });
  }, []);

  useEffect(() => {
    resetProximity();
    if (rail.current) rail.current.scrollLeft = 0;
  }, [viewKey, resetProximity]);

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const update = () => {
      reduceMotion.current = media?.matches ?? false;
      if (reduceMotion.current) {
        if (frame.current !== null) cancelAnimationFrame(frame.current);
        frame.current = null;
        rail.current?.querySelectorAll<HTMLElement>(".timeline-rail-event").forEach((item) => {
          item.style.removeProperty("--timeline-nearness");
          delete item.dataset.near;
        });
      }
    };
    update();
    media?.addEventListener("change", update);
    return () => {
      media?.removeEventListener("change", update);
      if (frame.current !== null) cancelAnimationFrame(frame.current);
    };
  }, []);

  function focusAdjacent(event: KeyboardEvent<HTMLButtonElement>) {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const buttons = Array.from(
      rail.current?.querySelectorAll<HTMLButtonElement>(".timeline-rail-event:not(:disabled)") ??
        [],
    );
    const index = buttons.indexOf(event.currentTarget);
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? buttons.length - 1
          : Math.max(
              0,
              Math.min(buttons.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)),
            );
    event.preventDefault();
    buttons[next]?.focus({ preventScroll: true });
    buttons[next]?.scrollIntoView?.({ block: "nearest", inline: "nearest", behavior: "instant" });
  }

  return (
    <div className="timeline-visual">
      {dated.length > 0 && (
        <>
          <div className="timeline-rail-caption">
            <p id={instructionsId}>Event spacing, not to scale</p>
            <div className="timeline-rail-controls">
              <span>Scroll to explore · Arrow keys move between moments</span>
              <button
                className="quiet-button"
                aria-label="Scroll to earlier moments"
                onClick={() =>
                  rail.current?.scrollBy({
                    left: -rail.current.clientWidth * 0.75,
                    behavior: reduceMotion.current ? "instant" : "smooth",
                  })
                }
              >
                ←
              </button>
              <button
                className="quiet-button"
                aria-label="Scroll to later moments"
                onClick={() =>
                  rail.current?.scrollBy({
                    left: rail.current.clientWidth * 0.75,
                    behavior: reduceMotion.current ? "instant" : "smooth",
                  })
                }
              >
                →
              </button>
            </div>
          </div>
          <div
            className="timeline-rail-scroll"
            ref={rail}
            role="region"
            aria-label="Chronological timeline"
            aria-describedby={instructionsId}
            tabIndex={0}
            onPointerMove={(event) => {
              if (reduceMotion.current || event.pointerType === "touch") return;
              if (frame.current !== null) cancelAnimationFrame(frame.current);
              const { clientX, clientY } = event;
              frame.current = requestAnimationFrame(() => {
                frame.current = null;
                rail.current
                  ?.querySelectorAll<HTMLElement>(".timeline-rail-event")
                  .forEach((item) => {
                    // Measure the fixed anchor, never the scaled dot, to avoid feedback jitter.
                    const anchor = item
                      .querySelector(".timeline-rail-anchor")
                      ?.getBoundingClientRect();
                    if (!anchor) return;
                    const amount = timelineProximity(
                      Math.hypot(
                        clientX - anchor.left - anchor.width / 2,
                        clientY - anchor.top - anchor.height / 2,
                      ),
                    );
                    item.style.setProperty("--timeline-nearness", amount.toFixed(3));
                    item.dataset.near = amount > 0.25 ? "true" : "false";
                  });
              });
            }}
            onPointerLeave={resetProximity}
            onScroll={resetProximity}
          >
            <ol className="timeline-rail-groups">
              {dated.map((group) => (
                <li
                  key={group.key}
                  className="timeline-rail-group"
                  style={{ width: `${group.occurrences.length * 12.5}rem` }}
                >
                  <div className="timeline-rail-date">
                    <h3>{dateLabel(group.date, calendar)}</h3>
                    <small>
                      {group.occurrences.length > 1
                        ? `${group.occurrences.length} moments · same day`
                        : ""}
                    </small>
                  </div>
                  <ol className="timeline-rail-events">
                    {group.occurrences.map((occurrence) => (
                      <li key={occurrence.id}>
                        <button
                          className="timeline-rail-event"
                          data-navigation-focus={`occurrence-${occurrence.id}`}
                          aria-pressed={selectedId === occurrence.id}
                          aria-label={`${occurrenceLabel(occurrence)} · ${occurrence.eventEntry ? "Event" : "Moment"} · ${dateLabel(occurrence.date, calendar)}`}
                          title={`${occurrenceLabel(occurrence)} — ${dateLabel(occurrence.date, calendar)}`}
                          disabled={disabled}
                          onKeyDown={focusAdjacent}
                          onClick={() => onSelect(occurrence.id)}
                        >
                          <span className="timeline-rail-anchor" aria-hidden="true">
                            <span className="timeline-rail-dot" />
                          </span>
                          <span className="timeline-rail-card" aria-hidden="true">
                            <small>{occurrence.eventEntry ? "Event" : "Moment"}</small>
                            <span className="timeline-rail-title">
                              {occurrenceLabel(occurrence)}
                            </span>
                            <span className="timeline-rail-open">
                              Open moment <span>↗</span>
                            </span>
                          </span>
                        </button>
                      </li>
                    ))}
                  </ol>
                </li>
              ))}
            </ol>
          </div>
        </>
      )}
      {undated.length > 0 && (
        <section className="timeline-undated" aria-label="Undated occurrences">
          <div>
            <h3>Undated</h3>
            <p>Ideas waiting for their place in time.</p>
          </div>
          <ul>
            {undated.map((occurrence) => (
              <li key={occurrence.id}>
                <button
                  className="timeline-undated-event"
                  data-navigation-focus={`occurrence-${occurrence.id}`}
                  aria-pressed={selectedId === occurrence.id}
                  aria-label={`${occurrenceLabel(occurrence)} · ${occurrence.eventEntry ? "Event" : "Moment"} · Undated`}
                  disabled={disabled}
                  onClick={() => onSelect(occurrence.id)}
                >
                  <span>{occurrenceLabel(occurrence)}</span>
                  <small>{occurrence.eventEntry ? "Event" : "Moment"}</small>
                  <span aria-hidden="true">↗</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
