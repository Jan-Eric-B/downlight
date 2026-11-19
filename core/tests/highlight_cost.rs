//! What highlighting costs with each of bat's grammars (ticket 31): how long the core takes to
//! render a large code block in each language, through its public render interface. It measures
//! rather than checks, so it is ignored; run it in a release build, as the app is:
//!
//! `cargo test --release -p downlight-core --test highlight_cost -- --ignored --nocapture`
//!
//! It prints the slowest grammars on a block of many lines and on a block of one long line, the
//! shape of minified code, each as large as the highlight limit lets a block be, so a grammar the
//! time limit stops shows as plain. Ticket 31 chose the limits from a run with them lifted, on
//! blocks of 256 KB and a line of 32 KB. Nothing here lifts them: for such a run, raise
//! `HIGHLIGHT_LIMIT`'s `block`, `line` and `time` in `core/src/highlight.rs` past the sizes
//! measured, set [`LINES_BLOCK_SIZE`] and [`LONG_LINE_SIZE`] to those sizes, and put all of them
//! back afterwards.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::Nothing;

const KB: usize = 1024;

// The two sizes below must stay just under `HIGHLIGHT_LIMIT`'s `block` and `line` (128 KB and
// 4 KB), and change with them: an integration test can't read the `pub(crate)` constant.

/// The block of many lines that every grammar is measured on: as large as the highlight limit
/// lets a block be, less a line, since [`code_of`] can end a line past the size.
const LINES_BLOCK_SIZE: usize = 127 * KB;

/// The block of one line, as long as the highlight limit lets a line be, less a line's worth.
const LONG_LINE_SIZE: usize = 3800;

/// How many of the slowest grammars each table lists.
const SLOWEST: usize = 15;

/// Code with the things grammars tell apart: comments of several kinds, strings with escapes,
/// numbers, operators, brackets, tags and templates.
fn code_line(n: usize) -> String {
    format!(
        "fn item_{n}(value, other) {{ let text = \"a \\\"string\\\" {n}\"; \
         if (value < {n} && other >= 2.5e3) {{ return [1, 2, {n}]; }} }} \
         // comment <b>{n}</b> # hash ; semi -- dash {{{{ template }}}} 'quoted' $var @attr\n"
    )
}

/// Lines of [`code_line`] up to `size` bytes, joined with `\n`, or with spaces into one line.
fn code_of(size: usize, one_line: bool) -> String {
    let mut code = String::with_capacity(size + 256);
    let mut n = 0;
    while code.len() < size {
        let line = code_line(n);
        if one_line {
            code.push_str(line.trim_end());
            code.push(' ');
        } else {
            code.push_str(&line);
        }
        n += 1;
    }
    if one_line {
        code.push('\n');
    }
    code
}

/// Renders `code` as one block in `token`'s language, and how long that took.
// A helper outside a #[test] fn needs its own allow (clippy.toml).
#[allow(clippy::expect_used)]
fn render_block(token: &str, code: &str) -> (Duration, String) {
    let markdown = format!("```{token}\n{code}```\n");
    let started = Instant::now();
    let html = downlight_core::render(
        markdown.as_bytes(),
        Path::new("cost.md"),
        "http://image.localhost",
        Nothing,
    )
    .expect("the code is text, so it renders");
    (started.elapsed(), html.as_str().to_owned())
}

/// A grammar's cost on one block: its first render in the process, which compiles its patterns,
/// and its second, and whether it highlighted at all.
struct Cost {
    language: String,
    first: Duration,
    second: Duration,
    highlighted: bool,
}

/// The info string token that names each grammar, as the core finds it, for every grammar a
/// token finds.
fn grammar_tokens() -> Vec<(String, String)> {
    let syntaxes = two_face::syntax::extra_newlines();
    syntaxes
        .syntaxes()
        .iter()
        .filter_map(|syntax| {
            let token = syntax
                .file_extensions
                .first()
                .cloned()
                .unwrap_or_else(|| syntax.name.to_lowercase());
            let found = syntaxes.find_syntax_by_token(&token)?;
            (found.name == syntax.name && !token.contains(char::is_whitespace))
                .then(|| (syntax.name.clone(), token))
        })
        .collect()
}

fn costs(grammars: &[(String, String)], code: &str) -> Vec<Cost> {
    let mut costs: Vec<Cost> = grammars
        .iter()
        .map(|(name, token)| {
            let (first, _) = render_block(token, code);
            let (second, html) = render_block(token, code);
            Cost {
                language: format!("{name} (`{token}`)"),
                first,
                second,
                highlighted: html.contains("<span class=\"pl-"),
            }
        })
        .collect();
    costs.sort_by_key(|cost| std::cmp::Reverse(cost.second));
    costs
}

fn print_slowest(title: &str, size: usize, costs: &[Cost]) {
    let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
    let per_mb = |duration: Duration| ms(duration) * (1024.0 * KB as f64) / size as f64;
    println!("\n{title}, {size} bytes:\n");
    println!("| Grammar | First render, ms | Second render, ms | Second, ms per MB |");
    println!("| --- | ---: | ---: | ---: |");
    for cost in costs.iter().take(SLOWEST) {
        println!(
            "| {}{} | {:.0} | {:.0} | {:.0} |",
            cost.language,
            if cost.highlighted { "" } else { ", plain" },
            ms(cost.first),
            ms(cost.second),
            per_mb(cost.second),
        );
    }
    let total: Duration = costs.iter().map(|cost| cost.second).sum();
    println!(
        "\n{} grammars; median second render {:.0} ms, all together {:.0} ms.",
        costs.len(),
        ms(costs[costs.len() / 2].second),
        ms(total),
    );
}

#[test]
#[ignore = "measures highlighting; run it in a release build, as the file's comment says"]
fn slowest_grammars_on_a_large_block() {
    let grammars = grammar_tokens();
    // Plain code first, so the first grammar's numbers don't carry the grammar set's load.
    render_block("", "warm up\n");
    let (plain, _) = render_block("", &code_of(LINES_BLOCK_SIZE, false));
    println!(
        "A {} KB block without a language: {:.0} ms",
        LINES_BLOCK_SIZE / KB,
        plain.as_secs_f64() * 1000.0
    );
    print_slowest(
        "Many lines",
        LINES_BLOCK_SIZE,
        &costs(&grammars, &code_of(LINES_BLOCK_SIZE, false)),
    );
    print_slowest(
        "One line",
        LONG_LINE_SIZE,
        &costs(&grammars, &code_of(LONG_LINE_SIZE, true)),
    );
}
