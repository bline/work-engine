export async function evaluate(session, expression, timeout = 20000, { userGesture = false } = {}) {
  const result = await session.call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true, userGesture }, timeout);
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text || "Runtime evaluation failed");
  return result.result?.value;
}
export const documentObservationExpression = `(() => ({url: location.href,title: document.title,readyState: document.readyState,lang: document.documentElement.lang || null,text: document.body?.innerText || ""}))()`;
export function domActionExpression(kind, selector, value) {
  return `(() => { const element=document.querySelector(${JSON.stringify(selector)}); if(!element) return {found:false}; ${kind === "click" ? "element.click();" : `element.value=${JSON.stringify(value)};element.dispatchEvent(new Event("input",{bubbles:true}));element.dispatchEvent(new Event("change",{bubbles:true}));`} return {found:true,tag:element.tagName.toLowerCase()}; })()`;
}
