const FONT_SIZE_KEY = "flowly.fontSize";
export const DEFAULT_FONT_SIZE = 14;
export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 20;

export function normalizeFontSize(value) {
  const size = Number(value);
  if (!Number.isFinite(size)) return DEFAULT_FONT_SIZE;
  return Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(size)));
}

export function applyFontSize(value) {
  const size = normalizeFontSize(value);
  document.documentElement.style.setProperty("--app-font-size", `${size}px`);
  document.documentElement.style.setProperty(
    "--el-font-size-base",
    `${size}px`,
  );
  document.documentElement.style.setProperty(
    "--el-font-size-small",
    `${Math.max(12, size - 1)}px`,
  );
  document.documentElement.style.setProperty(
    "--el-font-size-extra-small",
    `${Math.max(11, size - 2)}px`,
  );
  return size;
}

export function loadFontSize() {
  return normalizeFontSize(localStorage.getItem(FONT_SIZE_KEY));
}

export function saveFontSize(value) {
  const size = applyFontSize(value);
  localStorage.setItem(FONT_SIZE_KEY, String(size));
  return size;
}
