export const DETAIL_URL_TOGGLE_THRESHOLD = 120;

export function shouldShowUrlToggle(url, threshold = DETAIL_URL_TOGGLE_THRESHOLD) {
  return String(url || "").length > threshold;
}

export function shouldCollapseUrlAfterSelectionChange({ visible, currentId, previousId }) {
  return Boolean(visible && currentId != null && currentId !== previousId);
}
