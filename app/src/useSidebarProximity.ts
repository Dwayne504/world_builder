import { useEffect, type RefObject } from "react";

/** Fixed hit areas and grid tracks; only the painted labels grow. */
export function useSidebarProximity(ref: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const sidebar = ref.current;
    if (!sidebar || !window.matchMedia) return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const contrast = window.matchMedia("(forced-colors: active)");
    let frame = 0;
    let pointerY = 0;
    const clear = () => {
      cancelAnimationFrame(frame);
      frame = 0;
      sidebar.querySelectorAll<HTMLElement>(".sidebar-destination").forEach((row) => {
        row.style.removeProperty("--sidebar-nearness");
      });
    };
    const move = (event: PointerEvent) => {
      if (event.pointerType !== "mouse" || reduced.matches || contrast.matches) {
        clear();
        return;
      }
      pointerY = event.clientY;
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        sidebar.querySelectorAll<HTMLButtonElement>(".sidebar-destination").forEach((row) => {
          const bounds = row.getBoundingClientRect();
          const distance = Math.abs(pointerY - (bounds.top + bounds.height / 2));
          const nearness = row.disabled ? 0 : Math.max(0, 1 - distance / 76);
          row.style.setProperty("--sidebar-nearness", nearness.toFixed(3));
        });
      });
    };
    sidebar.addEventListener("pointermove", move, { passive: true });
    sidebar.addEventListener("pointerleave", clear);
    sidebar.addEventListener("scroll", clear, { passive: true });
    window.addEventListener("blur", clear);
    reduced.addEventListener("change", clear);
    contrast.addEventListener("change", clear);
    return () => {
      clear();
      sidebar.removeEventListener("pointermove", move);
      sidebar.removeEventListener("pointerleave", clear);
      sidebar.removeEventListener("scroll", clear);
      window.removeEventListener("blur", clear);
      reduced.removeEventListener("change", clear);
      contrast.removeEventListener("change", clear);
    };
  }, [ref]);
}
