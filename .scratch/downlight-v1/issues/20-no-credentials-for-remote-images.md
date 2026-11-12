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
