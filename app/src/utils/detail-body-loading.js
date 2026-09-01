export const DETAIL_BODY_AUTO_LOAD_BYTES = 64 * 1024;

export function shouldAutoLoadBody({
  available,
  size,
  threshold = DETAIL_BODY_AUTO_LOAD_BYTES,
}) {
  return Boolean(available && Number(size || 0) <= threshold);
}
