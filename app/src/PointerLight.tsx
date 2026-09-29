import { useEffect } from "react";

/** Decorative only: one animation-frame update, no React renders or hit targets. */
export function PointerLight() {
  useEffect(() => {
    if (!window.matchMedia) return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const contrast = window.matchMedia("(forced-colors: active)");
    const root = document.documentElement;
    let frame = 0;
    let x = 0;
    let y = 0;
    let enabled = false;
    const clear = () => {
      cancelAnimationFrame(frame);
      frame = 0;
      delete root.dataset.pointerLight;
    };
    const mediaChanged = () => {
      enabled = !reduced.matches && !contrast.matches;
      if (!enabled) clear();
    };
    // Actual mouse events remain reliable when a hybrid device reports no hover.
    const move = (event: PointerEvent) => {
      if (!enabled || event.pointerType !== "mouse") {
        clear();
        return;
      }
      x = event.clientX;
      y = event.clientY;
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        root.style.setProperty("--pointer-x", `${x}px`);
        root.style.setProperty("--pointer-y", `${y}px`);
        root.dataset.pointerLight = "on";
      });
    };
    const leave = (event: PointerEvent) => {
      if (!event.relatedTarget) clear();
    };
    const visibility = () => {
      if (document.hidden) clear();
    };
    [reduced, contrast].forEach((media) => media.addEventListener("change", mediaChanged));
    mediaChanged();
    document.addEventListener("pointermove", move, { passive: true });
    document.addEventListener("pointerout", leave, { passive: true });
    document.addEventListener("visibilitychange", visibility);
    window.addEventListener("blur", clear);
    return () => {
      clear();
      root.style.removeProperty("--pointer-x");
      root.style.removeProperty("--pointer-y");
      [reduced, contrast].forEach((media) => media.removeEventListener("change", mediaChanged));
      document.removeEventListener("pointermove", move);
      document.removeEventListener("pointerout", leave);
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("blur", clear);
    };
  }, []);
  return (
    <>
      <div className="ambient-pattern" aria-hidden="true" />
      <div className="pointer-light" aria-hidden="true" />
    </>
  );
}
