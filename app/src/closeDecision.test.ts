import { describe, expect, it } from "vitest";
import { decideClose } from "./closeDecision";

describe("decideClose", () => {
  it("preserves whether the user is closing just the Project or the native window", () => {
    expect(decideClose("saved", "project")).toBe("close-project");
    expect(decideClose("saved", "native-window")).toBe("close-native-window");
    expect(decideClose("dirty", "project")).toBe("confirm-unsaved-project");
    expect(decideClose("dirty", "native-window")).toBe("confirm-unsaved-native-window");
    expect(decideClose("saving", "project")).toBe("confirm-unsaved-project");
    expect(decideClose("failed", "native-window")).toBe("confirm-unsaved-native-window");
  });
});
