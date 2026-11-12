---
title: Code blocks
tags: [highlighting, copy]
---

# Code blocks

Code in the languages a reader meets most, for the page's copy button tests (ticket 07).

```rust
/// Says hello.
fn main() {
    let name = "<world> & co";
    println!("Hello, {name}!");
}
```

```typescript
export function greet(name: string): string {
  return `Hello, ${name}!`;
}
```

```toml
[package]
name = "lamplighter"
version = "0.1.0"
```

```diff
- old line
+ new line
```

A language bat's grammars don't know, and PowerShell, which the pure-Rust regex engine can't compile:

```no-such-language
plain <text> & more
```

```powershell
Get-ChildItem -Path . | Where-Object { $_.Length -gt 0 }
```

No language:

```
	indented with a tab
trailing spaces   
```

    indented code block
