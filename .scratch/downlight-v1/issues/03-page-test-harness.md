# 03: Page test harness

**What to build:** The page's own TypeScript can be tested with Vitest and happy-dom against HTML that the core really produces. The core's tests render a small set of fixture documents and check them against committed rendered fixtures; the page's tests load the same rendered fixtures, so both sides agree on the HTML's shape.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 02 (Performance spike and stack decision)

**Status:** ready-for-agent

- [ ] Vitest with happy-dom runs the page tests with one command.
- [ ] A core test renders the fixture documents and fails when its output differs from the committed rendered fixtures. There is a documented way to update them.
- [ ] One page test passes against a rendered fixture.
- [ ] The commands for the core tests and the page tests are written down where contributors look.
