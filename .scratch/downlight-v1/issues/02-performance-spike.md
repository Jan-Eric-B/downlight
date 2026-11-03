# 02: Performance spike and stack decision

**What to build:** Measure the walking skeleton against the spec's performance targets and decide whether Tauri, comrak and the system webview hold. The app gets a timing switch, off by default, that reports from inside the app the time from the process start to the page's first frame after the document is inserted (**first paint**), and every page task longer than 100 ms. This ticket is the gate: no feature work starts until it passes.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 01 (Walking skeleton)

**Status:** ready-for-agent

- [ ] A timing switch, off by default, reports first paint and page tasks over 100 ms. Later tickets reuse it.
- [ ] A generator in the repo writes the test documents: 100 KB and 1 MB of Markdown with a table and a code block about every 30 lines, and 1 MB of prose only.
- [ ] Measured on the work machine with Mica on, as the median of 5 runs, with the results written under a "Results" heading in this ticket:
  - [ ] cold start to first paint for the 100 KB document (target: under 1.0 s);
  - [ ] first paint of a second window in the same process (target: under 300 ms). Until ticket 08 exists, the skeleton may open two files given in one launch;
  - [ ] the 1 MB documents: the longest page task (target: the window never stops answering for more than 100 ms) and the time until fully shown (target: under 2 s);
  - [ ] memory across all processes, recorded with no ceiling.
- [ ] Mica checked and the findings recorded: it shows in the native title bar; with "Transparency effects" off the background is solid; in high contrast it is solid and readable; whether the window flashes white while it opens.
- [ ] If cold first paint is at most 1.5 s: ADR 0002 is written, saying why rendering happens in Rust with comrak and not with a JavaScript renderer in the page.
- [ ] If cold first paint is over 1.5 s: work stops here, the results are reported to the user, and the stack is reopened before any other ticket starts.
