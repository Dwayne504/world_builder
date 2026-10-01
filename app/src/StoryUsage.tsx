import { useEffect, useState } from "react";
import { storyUsage } from "./api";
import { chapterLabel, type StoryUsage as Usage } from "./storyTypes";
export function StoryUsage({
  projectId,
  entryId,
  onOpen,
}: {
  projectId: string;
  entryId: string;
  onOpen: (id: string) => void;
}) {
  const [usage, setUsage] = useState<Usage[]>([]);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void storyUsage(projectId, entryId)
      .then((value) => {
        if (current) setUsage(value);
      })
      .catch(() => {
        if (current) setError("Story usage could not be loaded.");
      });
    return () => {
      current = false;
    };
  }, [projectId, entryId]);
  if (!usage.length && !error) return null;
  return (
    <section className="story-usage" aria-label="Story usage">
      <h3>Story usage</h3>
      {error && <p role="alert">{error}</p>}
      {usage.map((item) => (
        <p key={item.chapter.id}>
          <button className="quiet-button" onClick={() => onOpen(item.chapter.id)}>
            {chapterLabel(item.chapter)}
          </button>{" "}
          <span className="muted">
            {item.roles.map((r) => r.name).join(" · ")}
            {item.chapter.workspaceState !== "active" ? ` · ${item.chapter.workspaceState}` : ""}
          </span>
        </p>
      ))}
    </section>
  );
}
