// The icon draws with the accent scale's own colors, so a shade changed in the scale can't leave
// the icon behind (spec: Accent color, story 22).
import { expect, test } from "vitest";
import icon from "../../app/icons/icon.svg?raw";
import accentCss from "./accent.css?raw";

/** The scale's shades, as the spec names them. */
const SHADES = [
  "light-3",
  "light-2",
  "light-1",
  "base",
  "dark-1",
  "dark-2",
  "dark-3",
];

/** The SVG attributes that take a color. */
const PAINT_ATTRIBUTES = [
  "fill",
  "stroke",
  "stop-color",
  "flood-color",
  "lighting-color",
  "color",
];

/**
 * The elements whose fill paints. A `line` is left out: it has no inside, so its fill never shows.
 */
const FILLED_SHAPES = new Set([
  "path",
  "rect",
  "circle",
  "ellipse",
  "polyline",
  "polygon",
  "text",
]);

/** SVG's initial `fill` and `stop-color`. */
const DEFAULT_PAINT = "#000000";

/** Each `--accent-<shade>` the stylesheet defines, with its color in lowercase. */
function accentScale(): Map<string, string> {
  const scale = new Map<string, string>();
  for (const [, shade, color] of accentCss.matchAll(
    /--accent-([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\s*;/g,
  )) {
    if (shade && color) scale.set(shade, color.toLowerCase());
  }
  return scale;
}

function iconElements(): Element[] {
  const svg = new DOMParser().parseFromString(icon, "image/svg+xml");
  return [...svg.querySelectorAll("*")];
}

/**
 * The black an element paints with when no attribute names its color: a shape with no `fill` of
 * its own or from an ancestor, and a stop with no `stop-color` of its own, since stop-color isn't
 * inherited. A missing `stroke` paints nothing, so it needs no such default.
 */
function defaultColors(element: Element): string[] {
  const unfilledShape =
    FILLED_SHAPES.has(element.localName) && element.closest("[fill]") === null;
  const uncoloredStop =
    element.localName === "stop" && !element.hasAttribute("stop-color");
  return unfilledShape || uncoloredStop ? [DEFAULT_PAINT] : [];
}

/** Every color the icon paints with, in lowercase; a gradient's own colors are its stops'. */
function iconColors(): string[] {
  return iconElements().flatMap((element) => [
    ...PAINT_ATTRIBUTES.map((name) => element.getAttribute(name))
      .filter((value) => value !== null)
      .filter((value) => value !== "none" && !value.startsWith("url(#"))
      .map((value) => value.toLowerCase()),
    ...defaultColors(element),
  ]);
}

test("the accent scale defines exactly the seven shades", () => {
  expect([...accentScale().keys()].sort()).toEqual([...SHADES].sort());
});

test("the icon paints only with the accent scale's shades", () => {
  const shades = new Set(accentScale().values());
  const colors = iconColors();

  expect(colors).not.toEqual([]);
  expect(colors.filter((color) => !shades.has(color))).toEqual([]);
});

test("the icon keeps every color in a paint attribute, where the scale's test sees it", () => {
  const styled = iconElements().filter(
    (element) => element.tagName === "style" || element.hasAttribute("style"),
  );

  expect(styled).toEqual([]);
});
