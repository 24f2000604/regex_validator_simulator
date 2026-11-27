//! Postfix → NFA (the article's `post2nfa`), using Thompson's construction.
//!
//! Every state is one of: a character test (with one `out` arrow), a `Split`
//! (two unlabeled arrows `out`/`out1`), or `Match`. A partially built NFA is a
//! `Frag`: a start state plus a list of dangling arrows still to be patched.
//!
//! Alongside the machine we compute a 2-D layout for drawing it, built up
//! fragment by fragment the same way the machine is.

use std::collections::HashMap;

use super::parse::Tok;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Char(char),
    Any,
    Split,
    Match,
}

#[derive(Clone, Debug)]
pub struct State {
    pub kind: Kind,
    pub out: Option<usize>,
    pub out1: Option<usize>,
}

impl State {
    pub fn accepts(&self, c: char) -> bool {
        match self.kind {
            Kind::Char(x) => x == c,
            Kind::Any => true,
            Kind::Split | Kind::Match => false,
        }
    }
}

/// One of a state's two arrows: `(state, false)` is `out`, `(state, true)` is `out1`.
pub type Arrow = (usize, bool);

#[derive(Clone, Debug)]
pub struct Nfa {
    pub states: Vec<State>,
    pub start: usize,
    pub layout: Layout,
}

impl Nfa {
    pub fn target(&self, (s, second): Arrow) -> Option<usize> {
        if second { self.states[s].out1 } else { self.states[s].out }
    }

    pub fn arrows(&self) -> impl Iterator<Item = (Arrow, usize)> + '_ {
        (0..self.states.len()).flat_map(move |s| {
            [false, true]
                .into_iter()
                .filter_map(move |b| self.target((s, b)).map(|t| ((s, b), t)))
        })
    }
}

/// Grid layout: each state gets a cell `(col, row)` (fractional rows allowed),
/// and some arrows get a routing level — a row coordinate the arrow should
/// detour through, used for loops and bypasses so they don't cut through nodes.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub pos: HashMap<usize, (f32, f32)>,
    pub route: HashMap<Arrow, f32>,
    pub w: f32,
    pub h: f32,
}

impl Layout {
    fn single(s: usize) -> Self {
        Layout { pos: HashMap::from([(s, (0.0, 0.0))]), route: HashMap::new(), w: 1.0, h: 1.0 }
    }

    fn shift(mut self, dx: f32, dy: f32) -> Self {
        for p in self.pos.values_mut() {
            p.0 += dx;
            p.1 += dy;
        }
        for r in self.route.values_mut() {
            *r += dy;
        }
        self
    }

    fn merge(&mut self, other: Layout) {
        self.pos.extend(other.pos);
        self.route.extend(other.route);
    }

    /// Row (top-left convention) that centres a 1-row node vertically.
    fn mid(&self) -> f32 {
        self.h / 2.0 - 0.5
    }
}

struct Frag {
    start: usize,
    out: Vec<Arrow>,
    layout: Layout,
}

struct Builder {
    states: Vec<State>,
}

impl Builder {
    fn state(&mut self, kind: Kind, out: Option<usize>, out1: Option<usize>) -> usize {
        self.states.push(State { kind, out, out1 });
        self.states.len() - 1
    }

    /// Point every dangling arrow in `list` at state `s`.
    fn patch(&mut self, list: &[Arrow], s: usize) {
        for &(st, second) in list {
            let slot = if second { &mut self.states[st].out1 } else { &mut self.states[st].out };
            *slot = Some(s);
        }
    }
}

/// Extra vertical room (in rows) reserved above/below a loop for its arrows.
const LOOP_PAD: f32 = 0.5;

pub fn post2nfa(post: &[Tok]) -> Option<Nfa> {
    let mut b = Builder { states: Vec::new() };
    let mut stack: Vec<Frag> = Vec::new();

    for &t in post {
        match t {
            Tok::Lit(_) | Tok::Any => {
                let kind = if let Tok::Lit(c) = t { Kind::Char(c) } else { Kind::Any };
                let s = b.state(kind, None, None);
                stack.push(Frag { start: s, out: vec![(s, false)], layout: Layout::single(s) });
            }
            Tok::Concat => {
                let e2 = stack.pop()?;
                let e1 = stack.pop()?;
                b.patch(&e1.out, e2.start);
                let h = e1.layout.h.max(e2.layout.h);
                let (w1, w2, h1, h2) = (e1.layout.w, e2.layout.w, e1.layout.h, e2.layout.h);
                let mut layout = e1.layout.shift(0.0, (h - h1) / 2.0);
                layout.merge(e2.layout.shift(w1, (h - h2) / 2.0));
                layout.w = w1 + w2;
                layout.h = h;
                stack.push(Frag { start: e1.start, out: e2.out, layout });
            }
            Tok::Alt => {
                let e2 = stack.pop()?;
                let e1 = stack.pop()?;
                let s = b.state(Kind::Split, Some(e1.start), Some(e2.start));
                let (h1, h2) = (e1.layout.h, e2.layout.h);
                let w = 1.0 + e1.layout.w.max(e2.layout.w);
                let mut layout = Layout { w, h: h1 + h2, ..Layout::default() };
                layout.pos.insert(s, (0.0, layout.mid()));
                layout.merge(e1.layout.shift(1.0, 0.0));
                layout.merge(e2.layout.shift(1.0, h1));
                let mut out = e1.out;
                out.extend(e2.out);
                stack.push(Frag { start: s, out, layout });
            }
            Tok::Quest => {
                let e = stack.pop()?;
                let s = b.state(Kind::Split, Some(e.start), None);
                let mut layout = Layout { w: e.layout.w + 1.0, h: e.layout.h + LOOP_PAD, ..Layout::default() };
                // Bypass arrow runs above the fragment.
                layout.merge(e.layout.shift(1.0, LOOP_PAD));
                layout.pos.insert(s, (0.0, LOOP_PAD + (layout.h - LOOP_PAD) / 2.0 - 0.5));
                layout.route.insert((s, true), LOOP_PAD * 0.45);
                let mut out = e.out;
                out.push((s, true));
                stack.push(Frag { start: s, out, layout });
            }
            Tok::Star => {
                let e = stack.pop()?;
                let s = b.state(Kind::Split, Some(e.start), None);
                b.patch(&e.out, s);
                let inner_h = e.layout.h;
                // Loop-back arrows from the upper half run above the fragment,
                // the rest below; the bypass runs below, outside the loops.
                let loops: Vec<(Arrow, f32)> = e
                    .out
                    .iter()
                    .map(|&a| {
                        let upper = e.layout.pos[&a.0].1 + 0.5 < inner_h / 2.0 - 0.01;
                        (a, if upper { LOOP_PAD * 0.45 } else { LOOP_PAD + inner_h + LOOP_PAD * 0.4 })
                    })
                    .collect();
                let mut layout = Layout { w: e.layout.w + 1.0, h: inner_h + 2.0 * LOOP_PAD, ..Layout::default() };
                layout.merge(e.layout.shift(1.0, LOOP_PAD));
                layout.pos.insert(s, (0.0, layout.mid()));
                layout.route.extend(loops);
                layout.route.insert((s, true), LOOP_PAD + inner_h + LOOP_PAD * 0.85);
                stack.push(Frag { start: s, out: vec![(s, true)], layout });
            }
            Tok::Plus => {
                let e = stack.pop()?;
                let s = b.state(Kind::Split, Some(e.start), None);
                b.patch(&e.out, s);
                let inner_h = e.layout.h;
                // Symmetric padding keeps the fragment's centre line on its nodes.
                let mut layout = Layout { w: e.layout.w + 1.0, h: inner_h + 2.0 * LOOP_PAD, ..Layout::default() };
                let ew = e.layout.w;
                layout.merge(e.layout.shift(0.0, LOOP_PAD));
                layout.pos.insert(s, (ew, layout.mid()));
                layout.route.insert((s, false), LOOP_PAD + inner_h + LOOP_PAD * 0.55);
                stack.push(Frag { start: e.start, out: vec![(s, true)], layout });
            }
        }
    }

    let e = stack.pop()?;
    if !stack.is_empty() {
        return None;
    }
    let m = b.state(Kind::Match, None, None);
    b.patch(&e.out, m);
    let mut layout = e.layout;
    layout.pos.insert(m, (layout.w, layout.mid()));
    layout.w += 1.0;
    Some(Nfa { states: b.states, start: e.start, layout })
}

#[cfg(test)]
mod tests {
    use super::super::parse::re2post;
    use super::*;

    #[test]
    fn state_counts() {
        // One state per literal, one Split per |*+?, plus Match.
        let nfa = post2nfa(&re2post("a(bb)+a").unwrap()).unwrap();
        assert_eq!(nfa.states.len(), 4 + 1 + 1);
        let nfa = post2nfa(&re2post("(a|b)*c?").unwrap()).unwrap();
        assert_eq!(nfa.states.len(), 3 + 3 + 1);
        assert_eq!(nfa.layout.pos.len(), nfa.states.len());
        assert!(nfa.states.iter().all(|s| s.kind == Kind::Match || s.out.is_some()));
    }
}
