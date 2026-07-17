import { describe, expect, it } from "vitest";
import { groupMaterialsByCategory } from "@/lib/materials";

describe("groupMaterialsByCategory", () => {
  it("groups assets without changing their order", () => {
    const assets = [
      { id: "light-1", category: "lighting" },
      { id: "element-1", category: "element" },
      { id: "light-2", category: "lighting" },
    ] as any[];

    expect(groupMaterialsByCategory(assets)).toEqual({
      lighting: [assets[0], assets[2]],
      element: [assets[1]],
    });
  });
});