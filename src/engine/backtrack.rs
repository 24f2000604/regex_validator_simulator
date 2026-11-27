//! A recursive backtracking matcher over the same NFA — the approach the
//! article contrasts with Thompson's. It explores one path at a time and, on
//! failure, backs up and tries the other arrow of the most recent Split. For
//! `a?ⁿaⁿ` against `aⁿ` that is 2ⁿ paths.

use super::nfa::{Kind, Nfa};

#[derive(Clone, Copy, Debug)]
pub struct BtResult {
    /// `None` if the step budget ran out before an answer was found.
    pub matched: Option<bool>,
    pub steps: u64,
}

struct Bt<'a> {
    nfa: &'a Nfa,
    input: &'a [char],
    steps: u64,
    budget: u64,
    /// Position+1 at which each Split is currently on the recursion stack, to
    /// stop empty loops like `(a*)*` from recursing forever.
    active: Vec<usize>,
}

impl Bt<'_> {
    fn go(&mut self, s: usize, i: usize) -> Option<bool> {
        self.steps += 1;
        if self.steps > self.budget {
            return None;
        }
        let st = &self.nfa.states[s];
        match st.kind {
            Kind::Match => Some(i == self.input.len()),
            Kind::Char(_) | Kind::Any => {
                if i < self.input.len() && st.accepts(self.input[i]) {
                    self.go(st.out.unwrap(), i + 1)
                } else {
                    Some(false)
                }
            }
            Kind::Split => {
                if self.active[s] == i + 1 {
                    return Some(false);
                }
                let saved = self.active[s];
                self.active[s] = i + 1;
                let (out, out1) = (st.out.unwrap(), st.out1.unwrap());
                let r = match self.go(out, i) {
                    Some(false) => self.go(out1, i),
                    r => r,
                };
                self.active[s] = saved;
                r
            }
        }
    }
}

pub fn backtrack(nfa: &Nfa, input: &[char], budget: u64) -> BtResult {
    let mut bt = Bt { nfa, input, steps: 0, budget, active: vec![0; nfa.states.len()] };
    let matched = bt.go(nfa.start, 0);
    BtResult { matched, steps: bt.steps }
}

/// Count the work Thompson's simulation does on the same input: one unit per
/// state visited by `addstate` or tested by `step`.
pub fn thompson_steps(nfa: &Nfa, input: &[char]) -> (bool, u64) {
    use super::sim::Sim;
    let mut sim = Sim::new(nfa);
    let (mut sp, mut fired) = (Vec::new(), Vec::new());
    let mut clist = sim.start(&mut sp);
    let mut steps = (clist.len() + sp.len()) as u64;
    for &c in input {
        sp.clear();
        fired.clear();
        let n = sim.step(&clist, c, &mut sp, &mut fired);
        steps += (clist.len() + n.len() + sp.len()) as u64;
        clist = n;
    }
    (sim.is_match(&clist), steps)
}

#[cfg(test)]
mod tests {
    use super::super::{nfa::post2nfa, parse::re2post};
    use super::*;

    fn nfa(re: &str) -> Nfa {
        post2nfa(&re2post(re).unwrap()).unwrap()
    }

    #[test]
    fn agrees_with_thompson() {
        for (re, s, want) in [
            ("a(bb)+a", "abba", true),
            ("a(bb)+a", "abbba", false),
            ("(a|b)*abb", "aababb", true),
            ("(a*)*b", "aaab", true),
            ("(a*)*b", "aaa", false),
        ] {
            let input: Vec<char> = s.chars().collect();
            let n = nfa(re);
            assert_eq!(backtrack(&n, &input, u64::MAX).matched, Some(want), "{re} {s}");
            assert_eq!(thompson_steps(&n, &input).0, want);
        }
    }

    #[test]
    fn pathological_blows_up() {
        let k = 16;
        let n = nfa(&("a?".repeat(k) + &"a".repeat(k)));
        let input = vec!['a'; k];
        let bt = backtrack(&n, &input, u64::MAX);
        let (_, th) = thompson_steps(&n, &input);
        assert_eq!(bt.matched, Some(true));
        assert!(bt.steps > 100 * th, "backtracking {} vs thompson {}", bt.steps, th);
    }
}
