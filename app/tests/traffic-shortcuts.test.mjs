import assert from "node:assert/strict";
import test from "node:test";

import { shouldClearTrafficOnKeydown } from "../src/utils/traffic-shortcuts.js";

test("Ctrl+X clears traffic sessions", () => {
  assert.equal(
    shouldClearTrafficOnKeydown({
      ctrlKey: true,
      shiftKey: false,
      altKey: false,
      key: "x",
    }),
    true,
  );
});

test("Ctrl+W no longer clears traffic sessions", () => {
  assert.equal(
    shouldClearTrafficOnKeydown({
      ctrlKey: true,
      shiftKey: false,
      altKey: false,
      key: "w",
    }),
    false,
  );
});

test("modified Ctrl+X does not clear traffic sessions", () => {
  assert.equal(
    shouldClearTrafficOnKeydown({
      ctrlKey: true,
      shiftKey: true,
      altKey: false,
      key: "x",
    }),
    false,
  );
});
