# Highlight with GitHub's classes and bat's grammars

The core highlights code blocks with syntect, using bat's grammars from two-face and syntect's pure-Rust regex engine (fancy-regex). A small theme built in code maps each TextMate scope to one of GitHub's `pl-*` classes (`pl-k` for keywords, `pl-s` for strings, and so on), and an adapter of Downlight's own plugs it into comrak's highlighter interface. We chose this because github-markdown-css already colors the `pl-*` classes with GitHub's light and dark colors, so the page needs no color CSS of its own, a switch between light and dark recolors without a new render, and each colored run is one flat span. comrak's own syntect adapter writes `<pre class="syntax-highlighting">` in its CSS mode and drops the `data-sourcepos` line number, which the margin and reload need.

## Considered options

- **syntect's built-in grammars.** About 0.6 MB smaller, but no TypeScript, TOML, Dockerfile, Kotlin or Swift.
- **syntect's own CSS classes**, with a light and a dark stylesheet generated from two of its themes. Colors that only resemble GitHub's, and a nested span for every scope.
- **Oniguruma** as the regex engine. Also highlights PowerShell, whose grammar fancy-regex can't compile, but adds a C library to the build.

## Consequences

- `downlight.exe` grew from 5.14 MB to 7.52 MB (ticket 07).
- Each grammar compiles its patterns on first use: TypeScript takes about 310 ms, Rust, Python and Bash 30 to 40 ms. The app renders a window's document while its webview starts, so a cold start still meets its first paint target (760 ms on the 100 KB test document, ticket 07's Results), but a document in a slow grammar still pays that cost on the background thread.
- PowerShell, ARM assembly, Salt and Babel's JavaScript show as plain code; JavaScript itself is highlighted.
- syntect reads its grammar dump with bincode 1, which is unmaintained (RUSTSEC-2025-0141); `deny.toml` ignores the advisory with that reason.
- The scope-to-class table in `core/src/highlight.rs` is Downlight's to tune: a language that looks wrong on screen gets a rule there.
