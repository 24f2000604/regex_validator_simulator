# nfagrep: a small grep built on a Thompson NFA

A C grep after Russ Cox's article
["Regular Expression Matching Can Be Simple And Fast"](https://swtch.com/~rsc/regexp/regexp1.html).
It parses the pattern to postfix (`re2post`), builds a Thompson NFA
(`post2nfa`), and runs every state in lock-step (`addstate`/`step`), so
matching takes O(m·n) time with no exponential backtracking.

```sh
meson setup build
meson compile -C build
meson install -C build        # optional, installs to /usr/local/bin by default

build/nfagrep 'a(bb)+a' file.txt
build/nfagrep -in 'colou?r' *.txt
cat log | build/nfagrep -c '^(GET|POST) '
```

## Layout

| File | What it does |
|------|--------------|
| `src/parse.c` | `re2post`: infix to postfix with explicit concatenation (`a(bb)+a` → `abb·+·a·`), with error positions |
| `src/nfa.c` | `post2nfa`: Thompson's construction, one state per literal, a Split per `\|*+?`, plus Match |
| `src/sim.c` | `addstate` / `step`: lock-step search over the state set, restarted at every position for unanchored matching |
| `src/utf8.c` | UTF-8 decoding |
| `src/util.c` | allocation helpers |
| `src/main.c` | the command line: options, reading files, printing |
| `docs/` | the Milestone II reports (PDF) and slides (PPTX) |

## Syntax

Literals, `|`, `*`, `+`, `?`, `( )`, `.` (any character) and `\x` (escape).
A line matches if the pattern matches any part of it. A leading `^` or a trailing `$`
anchors the match to the start or end of the line. Input is read as UTF-8, so `.` matches a whole character.

## Options

| Flag | Meaning |
|------|---------|
| `-i` | ignore case (ASCII letters only) |
| `-v` | print lines that don't match |
| `-c` | print a count of matching lines instead |
| `-n` | put the line number before each line |
| `-q` | print nothing; only set the exit status |
| `-H` / `-h` | always / never put the file name before each line (the default is to show it when there is more than one file) |

With no files, or with the file name `-`, it reads standard input. It exits with 0 if any line matched, 1 if none did, and 2 on an error.
