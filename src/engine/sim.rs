//! Simulating the NFA (the article's `match`, `step`, `addstate`).
//!
//! The simulation tracks the *set* of states the NFA could be in, advancing
//! all of them in lock-step one character at a time. `addstate` follows
//! unlabeled `Split` arrows eagerly, so lists only hold character-test states
//! and `Match`. Each state is added at most once per step (the `lastlist`
//! trick), which is what bounds the run time to O(m·n).

use super::nfa::{Kind, Nfa};

#[derive(Clone, Debug)]
pub struct Step {
    /// Number of input characters consumed before this snapshot.
    pub pos: usize,
    /// States in the list (character tests and Match).
    pub states: Vec<usize>,
    /// Split states walked through while building the list.
    pub splits: Vec<usize>,
    /// `states` (after the previous step) whose test accepted the character that led here.
    pub fired: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Trace {
    pub input: Vec<char>,
    pub steps: Vec<Step>,
    pub matched: bool,
}

pub struct Sim<'a> {
    nfa: &'a Nfa,
    lastlist: Vec<usize>,
    listid: usize,
}

impl<'a> Sim<'a> {
    pub fn new(nfa: &'a Nfa) -> Self {
        Sim { nfa, lastlist: vec![usize::MAX; nfa.states.len()], listid: 0 }
    }

    fn addstate(&mut self, l: &mut Vec<usize>, splits: &mut Vec<usize>, s: Option<usize>) {
        let Some(s) = s else { return };
        if self.lastlist[s] == self.listid {
            return;
        }
        self.lastlist[s] = self.listid;
        let st = &self.nfa.states[s];
        if st.kind == Kind::Split {
            splits.push(s);
            let (out, out1) = (st.out, st.out1);
            self.addstate(l, splits, out);
            self.addstate(l, splits, out1);
            return;
        }
        l.push(s);
    }

    /// Compute the initial state list (`startlist`).
    pub fn start(&mut self, splits: &mut Vec<usize>) -> Vec<usize> {
        self.listid += 1;
        let mut l = Vec::new();
        self.addstate(&mut l, splits, Some(self.nfa.start));
        l
    }

    /// Advance every state in `clist` past character `c` (`step`).
    pub fn step(&mut self, clist: &[usize], c: char, splits: &mut Vec<usize>, fired: &mut Vec<usize>) -> Vec<usize> {
        self.listid += 1;
        let mut nlist = Vec::new();
        for &s in clist {
            let st = &self.nfa.states[s];
            if st.accepts(c) {
                fired.push(s);
                let out = st.out;
                self.addstate(&mut nlist, splits, out);
            }
        }
        nlist
    }

    pub fn is_match(&self, l: &[usize]) -> bool {
        l.iter().any(|&s| self.nfa.states[s].kind == Kind::Match)
    }
}

/// Run the NFA on `input`, recording every intermediate state list.
pub fn trace(nfa: &Nfa, input: &str) -> Trace {
    let input: Vec<char> = input.chars().collect();
    let mut sim = Sim::new(nfa);
    let mut splits = Vec::new();
    let mut clist = sim.start(&mut splits);
    let mut steps = vec![Step { pos: 0, states: clist.clone(), splits, fired: vec![] }];
    for (i, &c) in input.iter().enumerate() {
        let (mut splits, mut fired) = (Vec::new(), Vec::new());
        clist = sim.step(&clist, c, &mut splits, &mut fired);
        steps.push(Step { pos: i + 1, states: clist.clone(), splits, fired });
    }
    let matched = sim.is_match(&clist);
    Trace { input, steps, matched }
}

/// Plain full-string match without recording a trace.
pub fn is_match(nfa: &Nfa, input: &str) -> bool {
    let mut sim = Sim::new(nfa);
    let mut scratch = Vec::new();
    let mut fired = Vec::new();
    let mut clist = sim.start(&mut scratch);
    for c in input.chars() {
        scratch.clear();
        fired.clear();
        clist = sim.step(&clist, c, &mut scratch, &mut fired);
        if clist.is_empty() {
            return false;
        }
    }
    sim.is_match(&clist)
}

#[cfg(test)]
mod tests {
    use super::super::{nfa::post2nfa, parse::re2post};
    use super::*;

    fn m(re: &str, s: &str) -> bool {
        let nfa = post2nfa(&re2post(re).unwrap()).unwrap();
        let t = trace(&nfa, s);
        assert_eq!(t.matched, is_match(&nfa, s));
        t.matched
    }

    #[test]
    fn article_examples() {
        assert!(m("a(bb)+a", "abba"));
        assert!(m("a(bb)+a", "abbbba"));
        assert!(!m("a(bb)+a", "aa"));
        assert!(!m("a(bb)+a", "abbba"));
    }

    #[test]
    fn operators() {
        assert!(m("(a|b)*c", "c"));
        assert!(m("(a|b)*c", "ababbc"));
        assert!(!m("(a|b)*c", "abd"));
        assert!(m("colou?r", "color") && m("colou?r", "colour"));
        assert!(m("h.llo", "hallo") && !m("h.llo", "hllo"));
        assert!(m("a\\*", "a*") && !m("a\\*", "aa"));
        assert!(m("(a*)*", "") && m("(a*)*", "aaa"));
    }

    #[test]
    fn pathological_is_fast() {
        let n = 100;
        let re = "a?".repeat(n) + &"a".repeat(n);
        assert!(m(&re, &"a".repeat(n)));
    }
}
