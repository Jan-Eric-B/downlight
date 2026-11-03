# 05: Files in any common encoding

**What to build:** A document reads correctly whatever common encoding its file uses: a byte order mark decides first, then valid UTF-8, then Windows-1252. A file written by Windows PowerShell 5.1 and an older file with umlauts both show the right characters.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [ ] A UTF-8, UTF-16 LE or UTF-16 BE byte order mark decides the encoding, and the mark itself doesn't show.
- [ ] Without a mark, a file that is valid UTF-8 is read as UTF-8.
- [ ] Otherwise the file is read as Windows-1252.
- [ ] A file written with Windows PowerShell 5.1's `Out-File` shows correctly, and a Windows-1252 file with umlauts shows umlauts.
- [ ] Core tests cover each case. There are no other loading rules (no retries, no special cases).
