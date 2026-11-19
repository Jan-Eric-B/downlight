// Assembles the third-party notices that the installer puts beside downlight.exe (ticket 36):
// the Rust crates' licenses from cargo-about, the page's packages' from Vite's build.license, and
// the syntax grammars' from two-face. `npm run check:notices` fails when the committed file no
// longer matches what they give for the current lockfiles; `npm run fixtures` writes it again
// (`--write`).

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { build } from "vite";

const ROOT = join(import.meta.dirname, "..");
const NOTICES_FILE = join(ROOT, "app", "notices", "THIRD-PARTY-NOTICES.txt");
const ABOUT_CONFIG = join(ROOT, "app", "notices", "about.toml");
const FLUENT_ICONS = "@fluentui/svg-icons";
const FLUENT_ICONS_LICENSE = join(ROOT, "page", "fluent-icons-license.txt");
const PAGE_LICENSES = "licenses.json";
const RULE = "-".repeat(80);

/** Runs a cargo subcommand in the repo and gives its standard output. */
function cargo(args, hint) {
  try {
    return execFileSync("cargo", args, {
      cwd: ROOT,
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
      stdio: ["ignore", "pipe", "inherit"],
    });
  } catch (error) {
    throw new Error(`\`cargo ${args.join(" ")}\` failed. ${hint}`, {
      cause: error,
    });
  }
}

/** The license texts of the crates in the app's release build, each with the crates under it.
 * The target, the accepted licenses and the left-out crates are app/notices/about.toml's. */
function rustCrateLicenses() {
  const about = JSON.parse(
    cargo(
      [
        "about",
        "generate",
        "--locked",
        "--fail",
        "--manifest-path",
        join("app", "Cargo.toml"),
        "--config",
        relative(ROOT, ABOUT_CONFIG),
        "--format",
        "json",
      ],
      "The check command needs cargo-about: `cargo install --locked cargo-about --features cli`.",
    ),
  );
  return about.licenses.map((license) => ({
    name: license.name,
    id: license.id,
    text: license.text,
    crates: license.used_by
      .map(({ crate }) => `${crate.name} ${crate.version}`)
      .sort(),
  }));
}

/** The licenses of the packages Vite bundles into the page, from a build that writes nothing. */
async function pagePackageLicenses() {
  const result = await build({
    configFile: join(ROOT, "vite.config.ts"),
    root: join(ROOT, "page"),
    logLevel: "warn",
    build: { write: false, license: { fileName: PAGE_LICENSES } },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap(
    (output) => output.output,
  );
  const asset = outputs.find((file) => file.fileName === PAGE_LICENSES);
  if (asset?.type !== "asset") {
    throw new Error(`Vite's build wrote no ${PAGE_LICENSES}.`);
  }
  const source =
    typeof asset.source === "string"
      ? asset.source
      : Buffer.from(asset.source).toString("utf8");
  return JSON.parse(source).map((dependency) => {
    // Fluent UI System Icons ships no license file, so Vite finds no text for it.
    const text =
      dependency.text ??
      (dependency.name === FLUENT_ICONS
        ? readFileSync(FLUENT_ICONS_LICENSE, "utf8")
        : null);
    if (!text) {
      throw new Error(
        `Vite found no license text for ${dependency.name}; its notice is needed before it ships.`,
      );
    }
    return { ...dependency, text };
  });
}

/** The licenses of the syntax grammars the core embeds, from two-face. */
function grammarLicenses() {
  return JSON.parse(
    cargo(
      [
        "run",
        "--quiet",
        "--locked",
        "--package",
        "downlight",
        "--example",
        "grammar_licenses",
      ],
      "app/notices/grammar_licenses.rs prints them.",
    ),
  );
}

/** A license text as the notices show it: Unix line ends, no blank lines around it. */
function cleanText(text) {
  return text
    .replaceAll("\r\n", "\n")
    .replace(/^\s*\n/, "")
    .trimEnd();
}

function heading(text, underline) {
  return `${text}\n${underline.repeat(text.length)}`;
}

function noticesText(crateLicenses, pagePackages, grammars) {
  const crateCount = new Set(crateLicenses.flatMap((license) => license.crates))
    .size;
  const parts = [
    heading("Third-party notices for Downlight", "="),
    "Downlight is free software under the GNU General Public License, version 3, in the file\n" +
      "LICENSE.txt beside this one. It is built with the software below, each part under its\n" +
      "own license. These are the notices those licenses ask to go with every copy.\n\n" +
      `1. Rust crates, which downlight.exe is built from (${String(crateCount)} crates)\n` +
      `2. The page's packages, bundled into downlight.exe (${String(pagePackages.length)} packages)\n` +
      `3. Syntax grammars, which highlight code blocks (${String(grammars.length)} licenses)`,
    heading("1. Rust crates", "="),
    ...crateLicenses.map((license) =>
      [
        RULE,
        `${license.name} (${license.id}), used by:\n${license.crates.map((name) => `  ${name}`).join("\n")}`,
        cleanText(license.text),
      ].join("\n\n"),
    ),
    heading("2. The page's packages", "="),
    ...pagePackages.map((dependency) =>
      [
        RULE,
        `${dependency.name} ${dependency.version} (${dependency.identifier})`,
        cleanText(dependency.text),
      ].join("\n\n"),
    ),
    heading("3. Syntax grammars", "="),
    "Downlight highlights code with the syntax grammars collected by the bat project\n" +
      "(https://github.com/sharkdp/bat), packaged by two-face\n" +
      "(https://codeberg.org/CosmicHarper/two-face). These are the licenses of the grammars that\n" +
      "ask to be acknowledged.",
    ...grammars.map((grammar) =>
      [RULE, grammar.path, cleanText(grammar.text)].join("\n\n"),
    ),
  ];
  return `${parts.join("\n\n")}\n`;
}

const notices = noticesText(
  rustCrateLicenses(),
  await pagePackageLicenses(),
  grammarLicenses(),
);
const shown = "app/notices/THIRD-PARTY-NOTICES.txt";

if (process.argv.includes("--write")) {
  writeFileSync(NOTICES_FILE, notices);
  console.log(`Wrote ${shown}; review its diff.`);
} else {
  let committed = null;
  try {
    committed = readFileSync(NOTICES_FILE, "utf8");
  } catch {
    // A missing file is a mismatch too.
  }
  if (committed !== notices) {
    console.error(
      `${shown} doesn't hold the notices that cargo-about, Vite and two-face give for the ` +
        "current lockfiles. Run `npm run fixtures` to write it again, and review the diff.",
    );
    process.exitCode = 1;
  } else {
    console.log(`${shown} matches the current dependencies.`);
  }
}
