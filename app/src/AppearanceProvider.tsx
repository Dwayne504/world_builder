import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { getAppearance, setAppearance, type Appearance } from "./api";
import { AppearanceContext, useAppearance } from "./appearanceContext";
import { Dialog } from "./Dialog";

const styles: { id: Appearance; name: string; description: string }[] = [
  {
    id: "storybook",
    name: "Storybook",
    description: "Warm parchment, ink, and a stitched leather cover.",
  },
  {
    id: "starship",
    name: "Starship",
    description: "Quiet charcoal panels, soft cyan light, and brushed metal.",
  },
];

export function AppearanceButton() {
  const { open } = useAppearance();
  return (
    <button className="quiet-button" onClick={open}>
      Appearance
    </button>
  );
}

export function AppearanceProvider({ children }: { children: ReactNode }) {
  const [appearance, updateAppearance] = useState<Appearance>("storybook");
  const [opened, setOpened] = useState(false);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const reload = useCallback(async () => {
    const request = ++generation.current;
    setBusy(true);
    try {
      const next = await getAppearance();
      if (request !== generation.current) return;
      updateAppearance(next);
      setError(null);
    } catch (reason) {
      if (request === generation.current)
        setError(reason instanceof Error ? reason.message : "Appearance could not be loaded.");
    } finally {
      if (request === generation.current) setBusy(false);
    }
  }, []);
  const invalidate = useCallback(() => {
    ++generation.current;
  }, []);
  useEffect(() => {
    void reload();
    return invalidate;
  }, [reload, invalidate]);
  useEffect(() => {
    document.documentElement.dataset.appearance = appearance;
    return () => {
      delete document.documentElement.dataset.appearance;
    };
  }, [appearance]);
  async function choose(next: Appearance) {
    if (busy) return;
    setBusy(true);
    const request = ++generation.current;
    try {
      const saved = await setAppearance(next);
      if (request !== generation.current) return;
      updateAppearance(saved);
      setError(null);
    } catch (reason) {
      if (request === generation.current)
        setError(
          reason instanceof Error
            ? reason.message
            : "Appearance could not be saved. Your previous style is still active.",
        );
    } finally {
      if (request === generation.current) setBusy(false);
    }
  }
  return (
    <AppearanceContext.Provider
      value={{
        open: () => {
          setOpened(true);
          if (!busy) void reload();
        },
        reload,
      }}
    >
      {children}
      <Dialog open={opened} title="Appearance" onClose={() => setOpened(false)}>
        <p>
          Choose the setting for your writing. This applies to the app and stays with you between
          Projects.
        </p>
        {error && (
          <p role="alert" className="error-banner">
            {error}{" "}
            <button disabled={busy} onClick={() => void reload()}>
              Retry loading appearance
            </button>
          </p>
        )}
        <div className="appearance-options" aria-label="App style">
          {styles.map((style) => (
            <button
              key={style.id}
              className={`appearance-card preview-${style.id}`}
              aria-pressed={appearance === style.id}
              disabled={busy || !!error}
              onClick={() => void choose(style.id)}
            >
              <span className="appearance-swatch" aria-hidden="true" />
              <strong>{style.name}</strong>
              <span>{style.description}</span>
              {appearance === style.id && <small>Current style</small>}
            </button>
          ))}
        </div>
        {busy && <p role="status">Updating appearance…</p>}
      </Dialog>
    </AppearanceContext.Provider>
  );
}
