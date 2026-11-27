//! A Rust translation of Russ Cox, "Regular Expression Matching Can Be Simple
//! And Fast" (https://swtch.com/~rsc/regexp/regexp1.html).

pub mod backtrack;
pub mod dfa;
pub mod nfa;
pub mod parse;
pub mod sim;

use nfa::Nfa;
use parse::{ParseError, Tok};

/// A compiled pattern: its postfix form and Thompson NFA.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub postfix: Vec<Tok>,
    pub nfa: Nfa,
}

pub fn compile(re: &str) -> Result<Compiled, ParseError> {
    let postfix = parse::re2post(re)?;
    let nfa = nfa::post2nfa(&postfix).ok_or_else(|| ParseError { pos: 0, msg: "malformed postfix".into() })?;
    Ok(Compiled { postfix, nfa })
}
