import { createContext, useContext } from "react";

export const AppearanceContext = createContext({
  open: () => {},
  reload: async () => {},
});
export function useAppearance() {
  return useContext(AppearanceContext);
}
