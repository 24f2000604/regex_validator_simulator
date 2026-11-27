# nfagrep: a small grep built on a Thompson NFA

A C grep after Russ Cox's article
["Regular Expression Matching Can Be Simple And Fast"](https://swtch.com/~rsc/regexp/regexp1.html).
It parses the pattern to postfix (`re2post`), builds a Thompson NFA
(`post2nfa`), and runs every state in lock-step (`addstate`/`step`), so
matching takes O(m·n) time with no exponential backtracking.

```sh
make
./nfagrep 'a(bb)+a' file.txt
./nfagrep -in 'colou?r' *.txt
cat log | ./nfagrep -c '^(GET|POST) '
```

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
