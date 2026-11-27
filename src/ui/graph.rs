//! Draws a Thompson NFA: nodes are absolutely positioned divs, arrows are
//! Bézier paths painted on a canvas underneath them.

use std::collections::HashSet;

use gpui::{Hsla, PathBuilder, Pixels, Point, canvas, div, point, prelude::*, px};

use crate::engine::nfa::{Kind, Nfa};

use super::theme;

const COL_W: f32 = 84.0;
const ROW_H: f32 = 62.0;
const MARGIN: f32 = 28.0;
const MARGIN_L: f32 = 56.0;
const R_NODE: f32 = 19.0;
const R_SPLIT: f32 = 12.0;

#[derive(Default)]
pub struct Highlight {
    pub states: HashSet<usize>,
    pub splits: HashSet<usize>,
    pub fired: HashSet<usize>,
}

struct Edge {
    pts: [Point<Pixels>; 4],
    color: Hsla,
    width: f32,
}

fn radius(k: Kind, z: f32) -> f32 {
    z * if k == Kind::Split { R_SPLIT } else { R_NODE }
}

pub fn size(nfa: &Nfa, z: f32) -> (f32, f32) {
    (z * (MARGIN_L + nfa.layout.w * COL_W + MARGIN), z * (nfa.layout.h * ROW_H + 2.0 * MARGIN))
}

fn center(nfa: &Nfa, s: usize, z: f32) -> (f32, f32) {
    let (x, y) = nfa.layout.pos[&s];
    (z * (MARGIN_L + (x + 0.5) * COL_W), z * (MARGIN + (y + 0.5) * ROW_H))
}

fn p(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

fn edges(nfa: &Nfa, hi: &Highlight, z: f32) -> Vec<Edge> {
    let mut out = Vec::new();
    for ((s, second), t) in nfa.arrows() {
        let (sx, sy) = center(nfa, s, z);
        let (tx, ty) = center(nfa, t, z);
        let (rs, rt) = (radius(nfa.states[s].kind, z), radius(nfa.states[t].kind, z));

        let routed = nfa.layout.route.get(&(s, second)).copied().or_else(|| {
            // Any other backwards arrow detours below its endpoints.
            (tx <= sx).then(|| ((sy.max(ty) / z - MARGIN) / ROW_H) + 0.3)
        });

        let pts = match routed {
            Some(level) => {
                let ly = z * (MARGIN + level * ROW_H);
                let dir = if ly < sy { -1.0 } else { 1.0 };
                let p0 = (sx, sy + dir * rs);
                let p3 = (tx, ty + dir * rt);
                // A cubic with both controls at height c peaks at 0.25·ends + 0.75·c.
                let c = (ly - 0.25 * (p0.1 + p3.1) / 2.0) / 0.75;
                let pull = ((tx - sx).abs() * 0.15).min(20.0 * z) * (tx - sx).signum();
                [p(p0.0, p0.1), p(sx + pull, c), p(tx - pull, c), p(p3.0, p3.1)]
            }
            None => {
                let p0 = (sx + rs, sy);
                let p3 = (tx - rt, ty);
                let dx = (p3.0 - p0.0) * 0.5;
                [p(p0.0, p0.1), p(p0.0 + dx, sy), p(p3.0 - dx, ty), p(p3.0, p3.1)]
            }
        };

        let (color, width) = if !second && hi.fired.contains(&s) {
            (theme::accent(), 2.5)
        } else if hi.splits.contains(&s) {
            (theme::accent().opacity(0.45), 2.0)
        } else {
            (theme::muted().opacity(0.5), 1.5)
        };
        out.push(Edge { pts, color, width });
    }

    // Entry arrow into the start state.
    let (sx, sy) = center(nfa, nfa.start, z);
    let r = radius(nfa.states[nfa.start].kind, z);
    let a = p(sx - r - 34.0 * z, sy);
    let b = p(sx - r, sy);
    out.push(Edge { pts: [a, a, b, b], color: theme::muted(), width: 2.0 });
    out
}

fn paint_edge(e: &Edge, origin: Point<Pixels>, window: &mut gpui::Window) {
    let [p0, c1, c2, p3] = e.pts.map(|q| q + origin);
    let mut b = PathBuilder::stroke(px(e.width));
    b.move_to(p0);
    b.cubic_bezier_to(p3, c1, c2);
    if let Ok(path) = b.build() {
        window.paint_path(path, e.color);
    }

    // Arrowhead along the final tangent.
    let from = if c2 != p3 { c2 } else { p0 };
    let (dx, dy) = (f32::from(p3.x - from.x), f32::from(p3.y - from.y));
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / len, dy / len);
    let (l, w) = (9.0, 4.5);
    let base = (f32::from(p3.x) - ux * l, f32::from(p3.y) - uy * l);
    let mut b = PathBuilder::fill();
    b.move_to(p3);
    b.line_to(p(base.0 - uy * w, base.1 + ux * w));
    b.line_to(p(base.0 + uy * w, base.1 - ux * w));
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, e.color);
    }
}

pub fn render(nfa: &Nfa, hi: &Highlight, z: f32) -> impl IntoElement {
    let (w, h) = size(nfa, z);
    let edges = edges(nfa, hi, z);

    let mut root = div().relative().w(px(w)).h(px(h)).flex_none().child(
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                for e in &edges {
                    paint_edge(e, bounds.origin, window);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .w(px(w))
        .h(px(h)),
    );

    for (id, st) in nfa.states.iter().enumerate() {
        let (cx, cy) = center(nfa, id, z);
        let r = radius(st.kind, z);
        let active = hi.states.contains(&id);
        let in_closure = hi.splits.contains(&id);

        let (label, fg) = match st.kind {
            Kind::Char(' ') => ("␣".to_string(), theme::text()),
            Kind::Char(c) => (c.to_string(), theme::text()),
            Kind::Any => ("·".to_string(), theme::text()),
            Kind::Split => ("ε".to_string(), theme::muted()),
            Kind::Match => ("✓".to_string(), theme::muted()),
        };
        let (fill, ring) = match (st.kind, active, in_closure) {
            (Kind::Match, true, _) => (theme::ok().opacity(0.18), theme::ok()),
            (_, true, _) => (theme::accent_dim(), theme::accent()),
            (_, _, true) => (theme::panel(), theme::accent().opacity(0.45)),
            (Kind::Match, ..) => (theme::panel(), theme::muted()),
            _ => (theme::panel(), theme::muted().opacity(0.6)),
        };

        let mut node = div()
            .absolute()
            .left(px(cx - r))
            .top(px(cy - r))
            .size(px(2.0 * r))
            .rounded_full()
            .border_2()
            .border_color(ring)
            .bg(fill)
            .flex()
            .items_center()
            .justify_center()
            .text_color(fg)
            .font_family(theme::MONO)
            .text_size(px(z * if st.kind == Kind::Split { 12.0 } else { 16.0 }))
            .child(label);
        if st.kind == Kind::Match {
            // Double circle for the accepting state.
            node = node.child(
                div()
                    .absolute()
                    .top(px(2.))
                    .left(px(2.))
                    .size(px(2.0 * r - 8.0))
                    .rounded_full()
                    .border_1()
                    .border_color(ring),
            );
        }
        root = root.child(node).child(
            div()
                .absolute()
                .left(px(cx + r * 0.75))
                .top(px(cy + r * 0.45))
                .text_size(px(10. * z.max(0.8)))
                .text_color(theme::muted())
                .child(format!("s{id}")),
        );
    }
    root
}
