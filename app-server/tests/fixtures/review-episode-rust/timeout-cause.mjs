export function isChildTimeout(error) {
  return error?.transport?.timedOut === true || error?.kind === "timeout"
    || error?.kind === "ETIMEDOUT" || error?.code === "ETIMEDOUT";
}

export function summarizeChildTimeouts(measured) {
  const names = [...new Set(measured.flatMap((item) => (item.components ?? [])
    .filter((part) => isChildTimeout(part.error)).map((part) => part.name)))];
  return {
    timedOutOperations: measured.filter((item) => isChildTimeout(item.error)
      || item.components?.some((part) => isChildTimeout(part.error))).length,
    timedOutComponents: Object.fromEntries(names.map((name) => [name,
      measured.filter((item) => item.components?.some((part) => part.name === name
        && isChildTimeout(part.error))).length])),
  };
}
