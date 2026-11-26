import { afterEach, beforeEach, expect, test } from "vitest";
import { documentHeadings } from "./headings";
import { afterNextFrame } from "./reading-position";
import {
  closeRulerWindow,
  drawn,
  findMarks,
  marks,
  openRulerWindow,
  readInTurn,
  showInTurn,
  startShowing,
  typeInFind,
} from "./overview-ruler-test-support";
import { layOut, nextShown, saveTheFile } from "./test-support";

// The overview ruler on a large document (overview ruler ticket 05): its list of headings is
// built a slice at a time after first paint, and the strip shows what is listed as it grows; each
// count of Find places its matches from what the render's earlier counts learned of its blocks'
// text. The layout fake makes each line of the file 20 px high, and the scroller 736 px high.

beforeEach(openRulerWindow);

afterEach(closeRulerWindow);

/**
 * A document of more top-level blocks than the list of headings takes in per task: an h2 on each
 * of its 10,000 lines, 200,000 px, which the 736 px strip shows at 736/200,000, so the last one,
 * on line 10,000, is in row 735.
 */
const LONGER_THAN_A_SLICE = Array.from(
  { length: 10_000 },
  (_, line) =>
    `<h2 id="heading-${String(line)}" data-sourcepos="${String(line + 1)}:1-${String(line + 1)}:12">Heading ${String(line)}</h2>`,
).join("\n");

test("a long document's heading marks show as its list grows, and all of them once it is complete", async () => {
  layOut(1);
  showInTurn(LONGER_THAN_A_SLICE);
  await startShowing();

  /** The lowest mark's row each frame, and whether the list was complete then. */
  const frames: { complete: boolean; lowest: number }[] = [];
  for (let frame = 0; frame < 1000; frame++) {
    const { complete } = documentHeadings();
    frames.push({ complete, lowest: marks().at(-1)?.top ?? -1 });
    if (complete) break;
    await afterNextFrame();
  }
  await afterNextFrame();
  await afterNextFrame();

  // Some frame showed the marks of the headings listed so far, before the list was complete.
  expect(
    frames.some(
      ({ complete, lowest }) => !complete && lowest >= 0 && lowest < 735,
    ),
  ).toBe(true);
  expect(marks()).toHaveLength(736);
  expect(marks().at(-1)).toEqual({ kind: "h2", top: 735 });
});

/**
 * A paragraph on lines 1 to 100, 2,000 px, of 400 characters in five text nodes, with "beta" 100
 * characters in and "gamma" `gammaAt` in.
 */
function paragraphWithGammaAt(gammaAt: number): string {
  return [
    '<p data-sourcepos="1:1-100:5">',
    "x".repeat(100),
    "<em>beta</em>",
    "y".repeat(gammaAt - 104),
    "<strong>gamma</strong>",
    "z".repeat(400 - gammaAt - 5),
    "</p>",
  ].join("");
}

test("each count of a render, and a reload's, places its matches by their share of their block's text", async () => {
  layOut(1);
  readInTurn(
    () => Promise.resolve(paragraphWithGammaAt(200)),
    () => Promise.resolve(paragraphWithGammaAt(300)),
  );
  await startShowing();

  // A quarter of the way through the 2,000 px block: 500 px, row 184 of the 736 px strip.
  typeInFind("beta");
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 184 },
    { kind: "current", top: 184 },
  ]);

  // Half way: row 368.
  typeInFind("gamma");
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 368 },
    { kind: "current", top: 368 },
  ]);

  // The reload's paragraph has "gamma" three quarters of the way through: 1,500 px, row 552.
  const shown = nextShown();
  await saveTheFile();
  await shown;
  await drawn();
  expect(findMarks()).toEqual([
    { kind: "match", top: 552 },
    { kind: "current", top: 552 },
  ]);
});
