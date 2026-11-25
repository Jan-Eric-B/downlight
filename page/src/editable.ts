/** The kinds of `input` the reader types text into; the others, such as a radio button, take none. */
const TEXT_INPUT_TYPES = new Set([
  "text",
  "search",
  "url",
  "tel",
  "email",
  "password",
  "number",
]);

/**
 * Whether `target`, where a key was pressed or a copy was raised, is a field the reader types text
 * into. The shortcuts (`commands.ts`) and the clean copy (`clean-copy.ts`) leave such a field to
 * the webview.
 */
export function isEditable(target: EventTarget | null): boolean {
  if (target instanceof HTMLTextAreaElement) return true;
  if (target instanceof HTMLInputElement)
    return TEXT_INPUT_TYPES.has(target.type);
  return target instanceof HTMLElement && target.isContentEditable;
}
