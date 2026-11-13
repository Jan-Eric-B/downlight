# 20: No credentials sent for remote images

**What to build:** A remote image can't make Windows send the reader's credentials either. Ticket 06 removes images on network shares, but a remote `http(s)` image on an intranet host (`http://fileserver/logo.png`) still loads, and WebView2, like Chromium, may answer that host's NTLM or Negotiate challenge with the reader's Windows sign-in without asking. User story 28 promises that opening a document can't make Windows send credentials to a server.

Spec: [Downlight v1](../spec.md)

**Blocked by:** 06 (Raw HTML and images, safely)

**Status:** ready-for-agent

- [ ] Find out whether WebView2 answers an NTLM or Negotiate challenge for an image request without asking, and for which hosts (intranet zone, dotless names, any host). Record the answer and its source in this ticket.
- [ ] If it does, the webview answers no such challenge for any request a document causes, for example through Chromium's `--auth-server-allowlist` browser argument on the WebView2 environment. The image doesn't load, and the page shows its alt text.
- [ ] The change fits ticket 19's handling of browser arguments: a second launch whose `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` differs from the running process's still shows the document, or shows the message box as ticket 19 does.
- [ ] Remote images on ordinary internet hosts still load.
- [ ] Checked by hand on the release build against a server that asks for NTLM (for example a local IIS site with Windows authentication): no credentials are sent.

## Comments

**2026-11-13, from the architecture review.** Two more ways a document can make Windows send the reader's credentials, for the same promise (story 28):

- **Links to shares.** One click on a link to `\host\share\x.md`, or a folder or file there, connects to that host: the link's lookup (`path_kind` in `app/src/links.rs`) and Explorer both do. The spec chose this ("paths on network shares are treated like local paths, since they only connect when clicked"), and the hover bubble shows the target. But a downloaded README can name any host, and one click sends the reader's NTLM hash. Decide here: keep it, ask before following a link to a share, or follow only shares the document itself lives on.
- **Symlinked local images.** `is_local` (`core/src/image.rs`) checks the path's prefix, and `std::fs::read` (`app/src/image.rs`) then follows reparse points. A `logo.png` that is a symlink to `\host\share\x.png` (from a git clone with `core.symlinks`, or a tar extracted in Developer Mode) makes Windows connect. Check that no component of the path is a link to a non-local target without opening the target, since `canonicalize` would open it and so connect, then test it with a symlink in a temporary folder.
