# Regex validator & simulator (Thompson NFA, GPUI)

A Rust translation of Russ Cox's article
[“Regular Expression Matching Can Be Simple And Fast”](https://swtch.com/~rsc/regexp/regexp1.html),
with a [GPUI](https://www.gpui.rs) front end that shows each stage at work.

```sh
cargo run --release                      # starts with the article's a(bb)+a example
cargo run --release -- '(a|b)*abb' aabb  # or give a pattern and input
cargo test                               # engine tests
```

## The article, piece by piece

| Article            | Here                         | What it does |
|--------------------|------------------------------|--------------|
| `re2post`          | `src/engine/parse.rs`        | Infix → postfix with explicit concatenation (`a(bb)+a` → `abb·+·a·`); reports syntax errors with their position |
| `post2nfa`, `Frag`, `patch` | `src/engine/nfa.rs` | Thompson's construction: one state per literal, a `Split` per `\|*+?`, plus `Match`. Also lays out the graph for drawing |
| `match`, `step`, `addstate`, `lastlist` | `src/engine/sim.rs` | Lock-step simulation over a *set* of states: O(m·n) |
| “Caching the NFA to build a DFA” | `src/engine/dfa.rs` | DFA states are NFA state lists, with transitions built on demand and cached |
| Backtracking (Perl, PCRE, …) | `src/engine/backtrack.rs` | Recursive matcher over the same NFA, for comparison |

Syntax: literals, `|`, `*`, `+`, `?`, `( )` as in the article, plus `.` (any character) and `\x` (escape). A match must cover the whole input, as in the article's `match`.

## The window

- **Examples**: 130 built-in patterns in 9 groups (the article, formal languages, binary, backtracking traps, words, numbers & dates, code formats, DNA, games), each with sample inputs that match and don't. ‹ › steps through them; the middle button opens the full list. They live in `src/presets.rs`.
- **Pattern / Input**: VALID / INVALID (with a caret under the error) and MATCH / NO MATCH, updated as you type.
- **NFA**: the Thompson NFA graph. Step or play through the input: blue states are in `clist`, purple Splits were walked by `addstate`. The postfix form is shown bottom-right; − / + zoom.
- **DFA**: the lazily built DFA states (NFA state sets and cached transitions), cache hits and misses, and the current input's path highlighted.
- **Benchmark**: `a?ⁿaⁿ` against `aⁿ`, backtracking vs Thompson, for n = 1…N (log-scale bars).
- **Tests** (right): press Enter in Input to add a string. Every string is re-checked when the pattern changes; click one to load it.

Text field keys: arrows, Home/End, Backspace/Delete, Ctrl+Backspace (delete word), Ctrl+U (delete to start), Ctrl+C / Ctrl+V.

## Build note

gpui 0.2.2 pulls in `xattr 0.2.3`, which uses `libc::ENOATTR`, and current `libc` releases no longer provide it on Linux. `vendor/xattr` is a copy with that one constant mapped to `ENODATA` (the same errno on Linux), wired in through `[patch.crates-io]` in `Cargo.toml`.
