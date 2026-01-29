import { main } from "./index.js";

test("main returns hello", () => {
  expect(main()).toBe("hello");
});
