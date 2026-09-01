export function shouldClearTrafficOnKeydown(event) {
  return (
    event.ctrlKey &&
    !event.shiftKey &&
    !event.altKey &&
    event.key.toLowerCase() === "x"
  );
}
