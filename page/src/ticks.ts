/**
 * Ticks (ticket 13): the reader's marks on a document's task list items. They belong to the
 * window and live only in the page, so they are never written to the file and go with the page
 * when its window closes. A tick is kept as where the reader's mark differs from what the file
 * says, so a reload that keeps the ticks still shows what the file now says of every item the
 * reader left alone.
 */

/**
 * The core's task list checkboxes. Sanitizing keeps `input` only where rendering writes it, so
 * every one in a rendered document is a task item's, never the document's own.
 */
const TASK_ITEM_SELECTOR = "input.task-list-item-checkbox";

/** The ticks of a shown document, read from its view. */
export interface Ticks {
  /** How many task items the document had, which decides whether the ticks still fit it. */
  taskItems: number;
  /** The reader's marks, by the task item's place in the document: ticked or unticked. */
  marks: ReadonlyMap<number, boolean>;
}

/** No ticks, as a document shown anew has. */
export const NO_TICKS: Ticks = { taskItems: 0, marks: new Map() };

function taskItemsIn(root: ParentNode): NodeListOf<HTMLInputElement> {
  return root.querySelectorAll<HTMLInputElement>(TASK_ITEM_SELECTOR);
}

/**
 * Makes the task items in a rendered document tickable. The core writes them `disabled`, as
 * GitHub does, which keeps the rendered document and its fixtures GitHub's; ticking is the
 * page's, since the page handles the reader's input.
 */
export function makeTickable(content: ParentNode): void {
  for (const item of taskItemsIn(content)) item.disabled = false;
}

/** The ticks the reader made in the document `view` shows. */
export function readTicks(view: ParentNode): Ticks {
  const items = taskItemsIn(view);
  const marks = new Map<number, boolean>();
  items.forEach((item, index) => {
    // `defaultChecked` is what the file says, from the render's `checked` attribute.
    if (item.checked !== item.defaultChecked) marks.set(index, item.checked);
  });
  return { taskItems: items.length, marks };
}

/**
 * Gives the document `view` shows the reader's `ticks` back, if it has as many task items as they
 * were read from. With another number, an item's place no longer says which item it is, so the
 * ticks are dropped rather than land on the wrong items.
 */
export function restoreTicks(view: ParentNode, ticks: Ticks): void {
  if (ticks.marks.size === 0) return;
  const items = taskItemsIn(view);
  if (items.length !== ticks.taskItems) return;
  for (const [index, ticked] of ticks.marks) {
    const item = items[index];
    if (item) item.checked = ticked;
  }
}
