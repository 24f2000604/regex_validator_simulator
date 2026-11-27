//! The article's lazily built DFA ("Caching the NFA to build a DFA").
//!
//! Each DFA state stands for one *set* of NFA states (a sorted state list).
//! Transitions are computed on demand by running one NFA `step` and are then
//! cached, so a character seen again from the same DFA state costs one lookup.
//! The cache persists across matches, as in the article.

use std::collections::HashMap;

use super::nfa::Nfa;
use super::sim::Sim;

#[derive(Clone, Debug)]
pub struct DState {
    pub nfa_states: Vec<usize>,
    pub accepting: bool,
    pub next: HashMap<char, usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Dfa {
    pub states: Vec<DState>,
    index: HashMap<Vec<usize>, usize>,
    pub start: Option<usize>,
    pub hits: usize,
    pub misses: usize,
}

#[derive(Clone, Debug)]
pub struct DfaRun {
    /// DFA states visited, one per prefix of the input (len = input + 1, or shorter if dead).
    pub path: Vec<usize>,
    /// Whether the run ended in an accepting state (the UI uses the NFA's verdict; tests check they agree).
    #[allow(dead_code)]
    pub matched: bool,
}

impl Dfa {
    fn dstate(&mut self, nfa: &Nfa, mut l: Vec<usize>) -> usize {
        l.sort_unstable();
        if let Some(&d) = self.index.get(&l) {
            return d;
        }
        let accepting = Sim::new(nfa).is_match(&l);
        self.states.push(DState { nfa_states: l.clone(), accepting, next: HashMap::new() });
        let d = self.states.len() - 1;
        self.index.insert(l, d);
        d
    }

    pub fn run(&mut self, nfa: &Nfa, input: &str) -> DfaRun {
        let mut sim = Sim::new(nfa);
        let mut scratch = Vec::new();
        let start = match self.start {
            Some(s) => s,
            None => {
                let l = sim.start(&mut scratch);
                let s = self.dstate(nfa, l);
                self.start = Some(s);
                s
            }
        };
        let mut d = start;
        let mut path = vec![d];
        for c in input.chars() {
            if let Some(&n) = self.states[d].next.get(&c) {
                self.hits += 1;
                d = n;
            } else {
                self.misses += 1;
                let (mut sp, mut fired) = (Vec::new(), Vec::new());
                let l = sim.step(&self.states[d].nfa_states.clone(), c, &mut sp, &mut fired);
                let n = self.dstate(nfa, l);
                self.states[d].next.insert(c, n);
                d = n;
            }
            path.push(d);
        }
        DfaRun { matched: self.states[d].accepting, path }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{nfa::post2nfa, parse::re2post};
    use super::*;

    #[test]
    fn caches_and_agrees() {
        let nfa = post2nfa(&re2post("(a|b)*abb").unwrap()).unwrap();
        let mut dfa = Dfa::default();
        assert!(dfa.run(&nfa, "babb").matched);
        let misses = dfa.misses;
        assert!(dfa.run(&nfa, "babb").matched);
        assert_eq!(dfa.misses, misses, "second run must be all cache hits");
        assert!(!dfa.run(&nfa, "abab").matched);
        // Textbook minimal DFA for (a|b)*abb has 4 states; the subset DFA here can't exceed 2^n.
        assert!(dfa.states.len() >= 4);
    }
}
