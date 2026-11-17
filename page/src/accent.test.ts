// The icon draws with the accent scale's own colors, so a shade changed in the scale can't leave
// the icon behind (spec: Accent color, story 22). The page takes the accent only through the roles
// of the spec's use-to-shade table, so swapping the scale recolors every use (ticket 03).
import { expect, test } from "vitest";
import icon from "../../app/icons/icon.svg?raw";
import accentCss from "./accent.css?raw";

/**
 * Every other stylesheet of the page, by file, as text, so a stylesheet added later is checked
 * too.
 */
const pageStylesheets = Object.entries(
  import.meta.glob<string>("./*.css", {
    query: "?raw",
    import: "default",
    eager: true,
  }),
).filter(([file]) => file !== "./accent.css");

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

/**
 * The roles of the spec's use-to-shade table, each an `--accent-<role>` variable: the shade it
 * takes in light and in dark, and the reader's system color it takes in high contrast.
 */
const ROLES: Record<string, { light: string; dark: string; forced: string }> = {
  link: { light: "dark-2", dark: "light-1", forced: "LinkText" },
  selection: { light: "light-2", dark: "dark-2", forced: "Highlight" },
  "find-match": { light: "light-3", dark: "dark-3", forced: "Mark" },
  "find-current": { light: "base", dark: "base", forced: "Highlight" },
  emphasis: { light: "dark-1", dark: "light-1", forced: "Highlight" },
};

const FORCED_COLORS = "(forced-colors: active)";

/** A stylesheet's rules, parsed through happy-dom's CSSOM. */
function rulesOf(css: string): CSSRule[] {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(css);
  return [...sheet.cssRules];
}

/** The value each role has in the style rules among `rules`. */
function rolesIn(rules: CSSRule[]): Record<string, string> {
  const values: Record<string, string> = {};
  for (const rule of rules) {
    if (!(rule instanceof CSSStyleRule)) continue;
    for (const role of Object.keys(ROLES)) {
      const value = rule.style.getPropertyValue(`--accent-${role}`).trim();
      if (value) values[role] = value;
    }
  }
  return values;
}

/** The value each role has in the scale's stylesheet outside any media condition. */
function rolesOutsideMedia(): Record<string, string> {
  return rolesIn(rulesOf(accentCss));
}

/** The value each role has under a media condition of the scale's stylesheet. */
function rolesUnder(condition: string): Record<string, string> {
  return rolesIn(
    rulesOf(accentCss)
      .filter((rule) => rule instanceof CSSMediaRule)
      .filter((rule) => rule.media.mediaText === condition)
      .flatMap((rule) => [...rule.cssRules]),
  );
}

function rolesTaking(
  value: (role: { light: string; dark: string; forced: string }) => string,
): Record<string, string> {
  return Object.fromEntries(
    Object.entries(ROLES).map(([name, role]) => [name, value(role)]),
  );
}

/** A stylesheet without its comments, which may name a variable without using it. */
function withoutComments(css: string): string {
  return css.replace(/\/\*[\s\S]*?\*\//g, "");
}

test("each role takes the spec's shade in light and in dark", () => {
  expect(rolesOutsideMedia()).toEqual(
    rolesTaking(
      (role) =>
        `light-dark(var(--accent-${role.light}), var(--accent-${role.dark}))`,
    ),
  );
});

test("in high contrast each role takes the reader's system color", () => {
  expect(rolesUnder(FORCED_COLORS)).toEqual(rolesTaking((role) => role.forced));
});

test("the page colors with the accent only through the roles, and uses each of them", () => {
  const used = new Set(
    pageStylesheets.flatMap(([, css]) =>
      [...withoutComments(css).matchAll(/var\(--accent-([a-z0-9-]+)/g)].map(
        ([, name]) => name,
      ),
    ),
  );

  expect([...used].sort()).toEqual(Object.keys(ROLES).sort());
});

/**
 * Every use of a role in the page's stylesheets, as `selector / property` (behind its media
 * condition, if it has one), and the role it takes: the spec's use-to-shade table, row by row.
 */
const ROLE_USES: Record<string, string> = {
  ".markdown-body a / color": "link",
  // A notice's folder that opens (window design ticket 11).
  ".folder-link / color": "link",
  ".folder-link:focus-visible / outline": "emphasis",
  "::selection / background-color": "selection",
  "::highlight(find-match) / background-color": "find-match",
  "::highlight(find-current) / background-color": "find-current",
  '.markdown-body input[type="checkbox"] / accent-color': "emphasis",
  // github-markdown-css's own focus rings, on links and checkboxes.
  ".markdown-body / --focus-outlineColor": "emphasis",
  ".copy-code:hover / color": "emphasis",
  "@media (forced-colors: active) .copy-code:hover::before / background-color":
    "emphasis",
  ".copy-code:focus-visible / outline": "emphasis",
  ".empty-window button:focus-visible / outline": "emphasis",
  ".find-bar input:focus-visible / outline": "emphasis",
  // The find bar's arrow and close buttons (v1 ticket 14).
  ".find-button:focus-visible / outline": "emphasis",
  // The title bar's command buttons (window design ticket 05).
  ".command-button:focus-visible / outline": "emphasis",
  // A switched-on button: Full width and Keep on top (window design tickets 06 and 07).
  '.command-button[aria-pressed="true"] / color': "emphasis",
};

/**
 * A rule's declarations as the stylesheet writes them, each property with its value. happy-dom
 * expands a shorthand that holds a var(), such as `outline: 2px solid var(--accent-emphasis)`,
 * into its longhands, each with the shorthand's whole value, so longhands of one shorthand that
 * share one value count once, as that shorthand.
 */
function declarationsOf(style: CSSStyleDeclaration): Map<string, string> {
  /** Each declaration's property, by its shorthand and value. */
  const properties = new Map<string, string>();
  const values = new Map<string, string>();
  for (let index = 0; index < style.length; index++) {
    const property = style.item(index);
    const value = style.getPropertyValue(property);
    const shorthand = property.startsWith("--")
      ? property
      : (property.split("-")[0] ?? property);
    const key = `${shorthand} ${value}`;
    properties.set(key, properties.has(key) ? shorthand : property);
    values.set(key, value);
  }
  return new Map(
    [...properties].map(([key, property]) => [property, values.get(key) ?? ""]),
  );
}

/** Each role the style rules among `rules` take, keyed as in `ROLE_USES`. */
function roleUsesIn(rules: CSSRule[], media = ""): Record<string, string> {
  const uses: Record<string, string> = {};
  for (const rule of rules) {
    if (rule instanceof CSSMediaRule) {
      Object.assign(
        uses,
        roleUsesIn([...rule.cssRules], `@media ${rule.media.mediaText} `),
      );
    }
    if (!(rule instanceof CSSStyleRule)) continue;
    for (const [property, value] of declarationsOf(rule.style)) {
      for (const [, role] of value.matchAll(/var\(--accent-([a-z0-9-]+)\)/g)) {
        if (role) uses[`${media}${rule.selectorText} / ${property}`] = role;
      }
    }
  }
  return uses;
}

/**
 * Each role Find's highlights take outside any media condition, keyed as in `ROLE_USES`, read
 * from the text: happy-dom drops every rule with a `::highlight()` selector, so `roleUsesIn` never
 * sees them.
 */
function highlightRoleUsesIn(css: string): Record<string, string> {
  const uses: Record<string, string> = {};
  const outsideMedia = withoutComments(css).replace(
    /@media[^{]*\{(?:[^{}]*\{[^}]*\})*\s*\}/g,
    "",
  );
  for (const [, selector, body = ""] of outsideMedia.matchAll(
    /(::highlight\([a-z-]+\))\s*\{([^}]*)\}/g,
  )) {
    for (const [, property, role] of body.matchAll(
      /([a-z-]+)\s*:[^;]*var\(--accent-([a-z0-9-]+)\)/g,
    )) {
      if (selector && role) uses[`${selector} / ${property ?? ""}`] = role;
    }
  }
  return uses;
}

test("each use of the accent takes its row's role", () => {
  const uses = Object.assign(
    {},
    ...pageStylesheets.flatMap(([, css]) => [
      roleUsesIn(rulesOf(css)),
      highlightRoleUsesIn(css),
    ]),
  ) as Record<string, string>;

  expect(uses).toEqual(ROLE_USES);
});

test("GitHub's own accent variables stay GitHub's, for its Note alert, and the page uses none of them", () => {
  const stylesheets: [string, string][] = [
    ["./accent.css", accentCss],
    ...pageStylesheets,
  ];
  for (const [file, css] of stylesheets) {
    expect(withoutComments(css), file).not.toMatch(
      /--fgColor-accent\b|--borderColor-accent-emphasis\b/,
    );
  }
});
