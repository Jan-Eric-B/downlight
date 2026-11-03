// Writes the test documents that performance is measured on (ticket 02) to target/documents/:
// 100 KB and 1 MB of Markdown with a table and a code block about every 30 lines, a second
// 100 KB document of that kind for a further window, and 1 MB of prose only. The text is
// pseudo-random from a fixed seed, so every run writes the same files.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const KB = 1024;
const MB = 1024 * KB;

const DOCUMENTS_FOLDER = join(import.meta.dirname, "..", "target", "documents");

export const DOCUMENTS = {
  mixed100kb: join(DOCUMENTS_FOLDER, "mixed-100kb.md"),
  mixed100kbOther: join(DOCUMENTS_FOLDER, "mixed-100kb-other.md"),
  mixed1mb: join(DOCUMENTS_FOLDER, "mixed-1mb.md"),
  prose1mb: join(DOCUMENTS_FOLDER, "prose-1mb.md"),
};

const WORDS = (
  "the a reader window document file viewer render page line block heading table code list " +
  "quick light small fast open close show reload find tick editor folder link image text " +
  "markdown rust webview process theme dark mica scroll anchor position change save watch " +
  "every when while after before because which other second first last large short long"
).split(" ");

const LANGUAGES = ["rust", "typescript", "python", "json"];

/** mulberry32: a small seeded generator, so the documents are the same on every run. */
function randomFrom(seed) {
  let state = seed;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function writer(seed) {
  const random = randomFrom(seed);
  const pick = (items) => items[Math.floor(random() * items.length)];
  const word = () => {
    const w = pick(WORDS);
    const style = random();
    if (style < 0.03) return `*${w}*`;
    if (style < 0.05) return `**${w}**`;
    if (style < 0.07) return `\`${w}\``;
    return w;
  };
  const sentence = () => {
    const words = Array.from({ length: 6 + Math.floor(random() * 10) }, word);
    const text = words.join(" ");
    return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
  };
  // Lines of at least 70 characters, so the text wraps like hand-written prose.
  const paragraph = (lines) => {
    const out = [];
    let line = "";
    while (out.length < lines) {
      line = line ? `${line} ${sentence()}` : sentence();
      if (line.length >= 70) {
        out.push(line);
        line = "";
      }
    }
    return out;
  };
  return { random, pick, word, sentence, paragraph };
}

// One section is about 30 lines, so a table and a code block come about every 30 lines.
function mixedSection(w, n) {
  const language = LANGUAGES[n % LANGUAGES.length];
  const rows = Array.from(
    { length: 4 },
    (_, i) =>
      `| ${w.word()} ${i} | ${w.pick(WORDS)} | ${Math.floor(w.random() * 10000)} | ${w.sentence()} |`,
  );
  const code = {
    rust: [
      `fn ${w.pick(WORDS)}_${n}(value: u32) -> u32 {`,
      "    let doubled = value * 2;",
      `    // ${w.sentence()}`,
      "    doubled + 1",
      "}",
    ],
    typescript: [
      `export function ${w.pick(WORDS)}${n}(value: number): number {`,
      "  const doubled = value * 2;",
      `  // ${w.sentence()}`,
      "  return doubled + 1;",
      "}",
    ],
    python: [
      `def ${w.pick(WORDS)}_${n}(value: int) -> int:`,
      "    doubled = value * 2",
      `    # ${w.sentence()}`,
      "    return doubled + 1",
      "",
    ],
    json: [
      "{",
      `  "name": "${w.pick(WORDS)}-${n}",`,
      `  "size": ${Math.floor(w.random() * 1000)},`,
      `  "note": "${w.pick(WORDS)} ${w.pick(WORDS)}"`,
      "}",
    ],
  }[language];
  return [
    `## ${n}. ${w.sentence().slice(0, -1)}`,
    "",
    ...w.paragraph(4),
    "",
    ...w.paragraph(3),
    "",
    ...Array.from({ length: 3 }, () => `- ${w.sentence()}`),
    "",
    "| Name | Kind | Size | Notes |",
    "| --- | --- | ---: | --- |",
    ...rows,
    "",
    `\`\`\`${language}`,
    ...code,
    "```",
    "",
    ...w.paragraph(2),
    "",
  ];
}

function proseParagraph(w) {
  return [...w.paragraph(3 + Math.floor(w.random() * 4)), ""];
}

function generate(size, seed, block) {
  const w = writer(seed);
  const lines = ["# Downlight test document", ""];
  let bytes = 0;
  for (let n = 1; bytes < size; n++) {
    const added = block(w, n);
    lines.push(...added);
    bytes += Buffer.byteLength(added.join("\n") + "\n");
  }
  return lines.join("\n");
}

export function writeDocuments() {
  mkdirSync(DOCUMENTS_FOLDER, { recursive: true });
  writeFileSync(DOCUMENTS.mixed100kb, generate(100 * KB, 1, mixedSection));
  writeFileSync(DOCUMENTS.mixed1mb, generate(MB, 2, mixedSection));
  writeFileSync(DOCUMENTS.prose1mb, generate(MB, 3, proseParagraph));
  writeFileSync(DOCUMENTS.mixed100kbOther, generate(100 * KB, 4, mixedSection));
}

if (import.meta.main) {
  writeDocuments();
  console.log(`Wrote the test documents to ${DOCUMENTS_FOLDER}`);
}
