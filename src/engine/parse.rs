//! Infix → postfix conversion (the article's `re2post`).
//!
//! Supported syntax, as in the article: literals, `|`, `*`, `+`, `?` and
//! parentheses. Two small extensions: `.` matches any character and `\x`
//! escapes a metacharacter. Concatenation, implicit in the infix form, becomes
//! the explicit `Concat` operator in postfix.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Lit(char),
    Any,
    Concat,
    Alt,
    Star,
    Plus,
    Quest,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Lit(c) if "|*+?().\\".contains(*c) => write!(f, "\\{c}"),
            Tok::Lit(c) => write!(f, "{c}"),
            Tok::Any => write!(f, "."),
            Tok::Concat => write!(f, "·"),
            Tok::Alt => write!(f, "|"),
            Tok::Star => write!(f, "*"),
            Tok::Plus => write!(f, "+"),
            Tok::Quest => write!(f, "?"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Character index in the pattern where the problem was detected.
    pub pos: usize,
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at position {}: {}", self.pos, self.msg)
    }
}

fn err<T>(pos: usize, msg: &str) -> Result<T, ParseError> {
    Err(ParseError { pos, msg: msg.into() })
}

/// Per-parenthesis-level counters, exactly as in the article: `nalt` is the
/// number of `|` seen at this level, `natom` the number of atoms pending
/// concatenation since the last `|`.
#[derive(Clone, Copy, Default)]
struct Level {
    nalt: usize,
    natom: usize,
    open_pos: usize,
}

pub fn re2post(re: &str) -> Result<Vec<Tok>, ParseError> {
    let mut dst = Vec::new();
    let mut stack: Vec<Level> = Vec::new();
    let mut cur = Level::default();
    let mut chars = re.chars().enumerate().peekable();

    fn flush_concat(dst: &mut Vec<Tok>, natom: &mut usize) {
        while *natom > 1 {
            dst.push(Tok::Concat);
            *natom -= 1;
        }
    }

    // Emit the atom we are about to see, concatenating it with the previous
    // one first if needed.
    fn atom(dst: &mut Vec<Tok>, cur: &mut Level, t: Tok) {
        if cur.natom > 1 {
            cur.natom -= 1;
            dst.push(Tok::Concat);
        }
        dst.push(t);
        cur.natom += 1;
    }

    while let Some((i, c)) = chars.next() {
        match c {
            '(' => {
                if cur.natom > 1 {
                    cur.natom -= 1;
                    dst.push(Tok::Concat);
                }
                stack.push(cur);
                cur = Level { open_pos: i, ..Level::default() };
            }
            '|' => {
                if cur.natom == 0 {
                    return err(i, "empty alternative before '|'");
                }
                flush_concat(&mut dst, &mut cur.natom);
                cur.natom = 0;
                cur.nalt += 1;
            }
            ')' => {
                let Some(outer) = stack.pop() else {
                    return err(i, "unmatched ')'");
                };
                if cur.natom == 0 {
                    return err(
                        i,
                        if cur.nalt > 0 { "empty alternative before ')'" } else { "empty group '()'" },
                    );
                }
                flush_concat(&mut dst, &mut cur.natom);
                dst.extend(std::iter::repeat_n(Tok::Alt, cur.nalt));
                cur = outer;
                cur.natom += 1;
            }
            '*' | '+' | '?' => {
                if cur.natom == 0 {
                    return err(i, &format!("'{c}' has nothing to repeat"));
                }
                dst.push(match c {
                    '*' => Tok::Star,
                    '+' => Tok::Plus,
                    _ => Tok::Quest,
                });
            }
            '.' => atom(&mut dst, &mut cur, Tok::Any),
            '\\' => match chars.next() {
                Some((_, e)) => atom(&mut dst, &mut cur, Tok::Lit(e)),
                None => return err(i, "trailing '\\' escapes nothing"),
            },
            _ => atom(&mut dst, &mut cur, Tok::Lit(c)),
        }
    }

    if !stack.is_empty() {
        return err(cur.open_pos, "unclosed '('");
    }
    if cur.natom == 0 {
        let len = re.chars().count();
        return err(len, if cur.nalt > 0 { "empty alternative at end" } else { "pattern is empty" });
    }
    flush_concat(&mut dst, &mut cur.natom);
    dst.extend(std::iter::repeat_n(Tok::Alt, cur.nalt));
    Ok(dst)
}

pub fn postfix_string(post: &[Tok]) -> String {
    post.iter().map(|t| t.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(re: &str) -> String {
        postfix_string(&re2post(re).unwrap())
    }

    #[test]
    fn article_example() {
        // The article: a(bb)+a  →  abb.+.a.
        assert_eq!(p("a(bb)+a"), "abb·+·a·");
    }

    #[test]
    fn alternation_and_ops() {
        assert_eq!(p("a|b"), "ab|");
        assert_eq!(p("ab|cd"), "ab·cd·|");
        assert_eq!(p("a*b?c+"), "a*b?·c+·");
        assert_eq!(p("(a|b)*c"), "ab|*c·");
        assert_eq!(p("a.\\."), "a.·\\.·");
    }

    #[test]
    fn errors() {
        for bad in ["", "*", "a|", "|a", "()", "(a", "a)", "(a|)", "a\\"] {
            assert!(re2post(bad).is_err(), "{bad:?} should fail");
        }
    }
}
