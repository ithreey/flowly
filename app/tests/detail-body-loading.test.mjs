import assert from "node:assert/strict";
import test from "node:test";

import {
  DETAIL_BODY_AUTO_LOAD_BYTES,
  shouldAutoLoadBody,
} from "../src/utils/detail-body-loading.js";

test("auto-loads captured body at or below the size threshold", () => {
  assert.equal(
    shouldAutoLoadBody({
      available: true,
      size: DETAIL_BODY_AUTO_LOAD_BYTES,
    }),
    true,
  );
});

test("requires manual load for captured body above the size threshold", () => {
  assert.equal(
    shouldAutoLoadBody({
      available: true,
      size: DETAIL_BODY_AUTO_LOAD_BYTES + 1,
    }),
    false,
  );
});

test("does not auto-load when body was not captured", () => {
  assert.equal(
    shouldAutoLoadBody({
      available: false,
      size: 12,
    }),
    false,
  );
});
