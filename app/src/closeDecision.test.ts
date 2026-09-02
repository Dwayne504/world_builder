import { describe, expect, it } from "vitest";
import { decideClose } from "./closeDecision";

describe("decideClose", () => {
  it("requires explicit confirmation for every unsaved state", () => {
    expect(decideClose("saved")).toBe("close");
    expect(decideClose("dirty")).toBe("confirm-unsaved");
    expect(decideClose("saving")).toBe("confirm-unsaved");
    expect(decideClose("failed")).toBe("confirm-unsaved");
  });
});
