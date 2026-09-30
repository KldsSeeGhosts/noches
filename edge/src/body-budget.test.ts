import { describe, expect, it } from "vitest";
import { readBody } from "./body-budget";

const request = (chunks: number[], declared?: string) => {
  let cancelled = false;
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      for (const size of chunks) controller.enqueue(new Uint8Array(size));
      controller.close();
    },
    cancel() { cancelled = true; }
  });
  return { request: new Request("https://example.invalid", {
    method: "POST", body, duplex: "half", headers: declared ? { "content-length": declared } : {}
  } as RequestInit), cancelled: () => cancelled };
};
describe("actual HTTP body budgets", () => {
  it("accepts exactly the limit without a declared length", async () => {
    expect((await readBody(request([4, 6]).request, 10))?.length).toBe(10);
  });
  it("rejects oversize chunked and falsely declared bodies", async () => {
    for (const declared of [undefined, "1"]) {
      const value = request([6, 6, 6], declared);
      expect(await readBody(value.request, 10)).toBeUndefined();
    }
  });
  it("rejects excessive declared length before reading", async () => {
    const value = request([1], "100");
    expect(await readBody(value.request, 10)).toBeUndefined();
    expect(value.cancelled()).toBe(true);
  });
});
