import assert from "node:assert/strict";
import test from "node:test";

import {
  shouldShowUrlToggle,
  shouldCollapseUrlAfterSelectionChange,
} from "../src/utils/detail-url-collapse.js";

test("shows URL toggle for long URLs", () => {
  assert.equal(shouldShowUrlToggle("https://example.com/" + "a".repeat(140)), true);
});

test("does not show URL toggle for short URLs", () => {
  assert.equal(shouldShowUrlToggle("https://example.com/api"), false);
});

test("collapses URL after selected detail changes", () => {
  assert.equal(
    shouldCollapseUrlAfterSelectionChange({
      visible: true,
      currentId: 2,
      previousId: 1,
    }),
    true,
  );
});
