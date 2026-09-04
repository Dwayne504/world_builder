import { describe, expect, it } from "vitest";
import { decideClose } from "./closeDecision";

describe("decideClose", () => {
  it("preserves whether the user is closing just the Project or the native window", () => {
    expect(decideClose("saved", "project")).toBe("close-project");
    expect(decideClose("saved", "native-window")).toBe("close-native-window");
    expect(decideClose("dirty", "project")).toBe("confirm-unsaved-project");
    expect(decideClose("dirty", "native-window")).toBe("confirm-unsaved-native-window");
    expect(decideClose("failed", "project")).toBe("confirm-unsaved-project");
    expect(decideClose("failed", "native-window")).toBe("confirm-unsaved-native-window");
  });

  it("never offers a save already in flight as discardable, since it cannot be cancelled", () => {
    // "saving" must not resolve to a confirm/discard decision: a save in
    // flight keeps running regardless of what the user clicks, so treating
    // it as discardable would let a "discarded" draft be persisted anyway.
    expect(decideClose("saving", "project")).toBe("wait-for-save-project");
    expect(decideClose("saving", "native-window")).toBe("wait-for-save-native-window");
  });
});
