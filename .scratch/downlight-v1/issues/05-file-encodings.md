# 05: Files in any common encoding

**What to build:** A document reads correctly whatever common encoding its file uses: a byte order mark decides first, then valid UTF-8, then Windows-1252. A file written by Windows PowerShell 5.1 and an older file with umlauts both show the right characters.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [x] A UTF-8, UTF-16 LE or UTF-16 BE byte order mark decides the encoding, and the mark itself doesn't show.
- [x] Without a mark, a file that is valid UTF-8 is read as UTF-8.
- [x] Otherwise the file is read as Windows-1252.
- [x] A file written with Windows PowerShell 5.1's `Out-File` shows correctly, and a Windows-1252 file with umlauts shows umlauts.
- [x] Core tests cover each case. There are no other loading rules (no retries, no special cases).

## Results

**2026-11-09, measured** with `npm run measure` on the release build, on the work machine, which was busy (20 to 50% CPU from other programs) during both runs. Every stage came out slower than in ticket 04, WebView2's navigation included (690 to 965 ms against ticket 04's numbers), so these runs don't compare with ticket 04's table. `readAndRender` was 3.6 ms for 100 KB, 33.6 ms for 1 MB mixed and 30.4 ms for 1 MB prose.

Timed on their own instead (release build, `render` called directly, median of 21, same busy machine):

| 1 MB document | UTF-8 | UTF-16 LE with mark | Windows-1252 |
| --- | ---: | ---: | ---: |
| mixed, render | 35.1 ms | 37.4 ms | 36.7 ms |
| prose, render | 20.2 ms | 27.0 ms | 39.0 ms |

The UTF-8 check itself takes 0.02 ms on 1 MB. A UTF-8 file, the common case, costs what it did before: `String::from_utf8_lossy` checked the same bytes. UTF-16 and Windows-1252 files are decoded into a new string first. The Windows-1252 case is synthetic (every non-ASCII byte replaced by `0xF6`); all cases stay far within the spec's targets.

## Comments

**2026-11-09, implementation.**

- **Decoding** is `core/src/decode.rs`, which `render` calls first, so the app still passes the file's bytes unchanged. `encoding_rs::Encoding::for_bom` finds a UTF-8, UTF-16 LE or UTF-16 BE mark, and the rest is decoded without the mark. Without one, `std::str::from_utf8` decides, and bytes that aren't valid UTF-8 are decoded as Windows-1252, which gives every byte a character. Nothing is retried: bytes that don't fit the mark's encoding show as U+FFFD.
- **Windows PowerShell 5.1:** `Out-File` (5.1.26100) writes UTF-16 LE with a mark and CRLF line endings. The test uses the bytes it wrote for `'# Größe', '', 'Café'`.
- **Dependency:** `encoding_rs` 0.8.44, as the spec names. Its license is `(Apache-2.0 OR MIT) AND BSD-3-Clause`, so `deny.toml` now allows BSD-3-Clause, which is compatible with GPL-3.0. It brings in `simdutf8`, `core_detect` and `multiversion_no_op`.
- **Tests** are render tests in `core/src/render.rs`: one per mark, valid UTF-8 without a mark, Windows-1252 with umlauts, the PowerShell file, and a mark followed by bytes in another encoding.
