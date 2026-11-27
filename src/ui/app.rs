use std::{collections::HashSet, time::Duration, time::Instant};

use gpui::{
    AnyElement, Context, Entity, Focusable, FontWeight, Hsla, SharedString, Subscription, Task, Window, div,
    prelude::*, px,
};

use crate::engine::{
    self, Compiled,
    backtrack::{backtrack, thompson_steps},
    dfa::{Dfa, DfaRun},
    nfa::Kind,
    parse::{ParseError, postfix_string},
    sim::{self, Trace},
};

use crate::presets::PRESETS;

use super::{
    graph::{self, Highlight},
    text_input::{InputEvent, TextInput},
    theme,
};

const BT_BUDGET: u64 = 400_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Nfa,
    Dfa,
    Bench,
}

#[derive(Clone)]
struct BenchRow {
    n: usize,
    bt_steps: u64,
    bt_done: bool,
    bt_time: Duration,
    th_steps: u64,
    th_time: Duration,
}

pub struct RegexApp {
    pattern: Entity<TextInput>,
    input: Entity<TextInput>,
    compiled: Result<Compiled, ParseError>,
    trace: Option<Trace>,
    step: usize,
    dfa: Dfa,
    dfa_run: Option<DfaRun>,
    tests: Vec<String>,
    tab: Tab,
    playing: Option<Task<()>>,
    bench_n: usize,
    zoom: f32,
    /// Example last loaded, and whether the example picker is open.
    preset: Option<usize>,
    picker_open: bool,
    bench: Vec<BenchRow>,
    bench_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl RegexApp {
    /// `pattern`/`input` override the first example (from the command line).
    pub fn new(pattern: Option<String>, input: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let preset = if pattern.is_some() { None } else { Some(0) };
        let tests: &[&str] = if pattern.is_some() { &[] } else { PRESETS[0].inputs };
        let re = pattern.unwrap_or(PRESETS[0].pattern.to_string());
        let first = input.unwrap_or(tests.first().unwrap_or(&"").to_string());
        let pattern = cx.new(|cx| TextInput::new(&re, "regular expression, e.g. a(bb)+a", cx));
        let input = cx.new(|cx| TextInput::new(&first, "input string (Enter adds it to the test suite)", cx));
        let subs = vec![
            cx.subscribe(&pattern, |this, _, ev: &InputEvent, cx| {
                if let InputEvent::Changed = ev {
                    this.recompile(cx);
                }
            }),
            cx.subscribe(&input, |this, _, ev: &InputEvent, cx| match ev {
                InputEvent::Changed => this.rerun(cx),
                InputEvent::Submit => {
                    let s = this.input.read(cx).text();
                    if !this.tests.contains(&s) {
                        this.tests.push(s);
                    }
                    cx.notify();
                }
            }),
        ];
        pattern.read(cx).focus_handle(cx).focus(window);
        let mut app = RegexApp {
            pattern,
            input,
            compiled: engine::compile(&re),
            trace: None,
            step: 0,
            dfa: Dfa::default(),
            dfa_run: None,
            tests: if tests.is_empty() { vec![first] } else { tests.iter().map(|s| s.to_string()).collect() },
            tab: Tab::Nfa,
            playing: None,
            bench_n: 20,
            zoom: 1.0,
            preset,
            picker_open: false,
            bench: Vec::new(),
            bench_task: None,
            _subs: subs,
        };
        app.recompile(cx);
        app
    }

    fn recompile(&mut self, cx: &mut Context<Self>) {
        self.compiled = engine::compile(&self.pattern.read(cx).text());
        self.dfa = Dfa::default();
        self.rerun(cx);
    }

    fn rerun(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text();
        match &self.compiled {
            Ok(c) => {
                let t = sim::trace(&c.nfa, &text);
                self.step = t.steps.len() - 1;
                self.trace = Some(t);
                self.dfa_run = Some(self.dfa.run(&c.nfa, &text));
            }
            Err(_) => {
                self.trace = None;
                self.dfa_run = None;
                self.step = 0;
            }
        }
        self.playing = None;
        cx.notify();
    }

    fn last_step(&self) -> usize {
        self.trace.as_ref().map_or(0, |t| t.steps.len() - 1)
    }

    fn set_step(&mut self, s: usize, cx: &mut Context<Self>) {
        self.step = s.min(self.last_step());
        cx.notify();
    }

    fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.playing.take().is_some() {
            cx.notify();
            return;
        }
        if self.step >= self.last_step() {
            self.step = 0;
        }
        self.playing = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(700)).await;
                let keep = this
                    .update(cx, |this, cx| {
                        if this.step < this.last_step() {
                            this.step += 1;
                            cx.notify();
                        }
                        let more = this.step < this.last_step();
                        if !more {
                            this.playing = None;
                            cx.notify();
                        }
                        more
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        }));
        cx.notify();
    }

    fn load_preset(&mut self, i: usize, cx: &mut Context<Self>) {
        let p = &PRESETS[i];
        self.preset = Some(i);
        self.picker_open = false;
        self.tests = p.inputs.iter().map(|s| s.to_string()).collect();
        self.pattern.update(cx, |t, cx| t.set_text(p.pattern, cx));
        self.input.update(cx, |t, cx| t.set_text(p.inputs[0], cx));
    }

    fn run_bench(&mut self, cx: &mut Context<Self>) {
        let max_n = self.bench_n;
        self.bench.clear();
        self.bench_task = Some(cx.spawn(async move |this, cx| {
            for n in 1..=max_n {
                let row = cx
                    .background_executor()
                    .spawn(async move {
                        let re = "a?".repeat(n) + &"a".repeat(n);
                        let c = engine::compile(&re).expect("pathological pattern is valid");
                        let input = vec!['a'; n];
                        let t0 = Instant::now();
                        let bt = backtrack(&c.nfa, &input, BT_BUDGET);
                        let bt_time = t0.elapsed();
                        let t0 = Instant::now();
                        let (_, th_steps) = thompson_steps(&c.nfa, &input);
                        let th_time = t0.elapsed();
                        BenchRow { n, bt_steps: bt.steps, bt_done: bt.matched.is_some(), bt_time, th_steps, th_time }
                    })
                    .await;
                let stop = this
                    .update(cx, |this, cx| {
                        let gave_up = !row.bt_done;
                        this.bench.push(row);
                        cx.notify();
                        gave_up
                    })
                    .unwrap_or(true);
                if stop {
                    break;
                }
            }
            this.update(cx, |this, cx| {
                this.bench_task = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }
}

// ───────────────────────────── widgets ─────────────────────────────

fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, active: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(if active { theme::accent() } else { theme::border() })
        .bg(if active { theme::accent_dim() } else { theme::panel() })
        .text_color(theme::text())
        .text_size(px(13.))
        .cursor_pointer()
        .hover(|s| s.border_color(theme::accent()))
        .child(label.into())
}

fn badge(label: impl Into<SharedString>, color: Hsla) -> gpui::Div {
    div().text_color(color).text_size(px(12.)).font_weight(FontWeight::BOLD).child(label.into())
}

fn section(title: &str) -> gpui::Div {
    div()
        .text_size(px(11.))
        .text_color(theme::muted())
        .font_weight(FontWeight::BOLD)
        .child(title.to_uppercase())
}

fn card() -> gpui::Div {
    div().min_h_0().p_3().rounded_lg().border_1().border_color(theme::border()).bg(theme::panel()).flex().flex_col().gap_2()
}

fn show(s: &str) -> String {
    if s.is_empty() { "ε (empty)".into() } else { format!("\"{s}\"") }
}

fn set_str(nfa: &engine::nfa::Nfa, ids: &[usize]) -> String {
    let mut v: Vec<_> = ids.to_vec();
    v.sort_unstable();
    let parts: Vec<String> = v
        .iter()
        .map(|&s| match nfa.states[s].kind {
            Kind::Match => format!("s{s}✓"),
            _ => format!("s{s}"),
        })
        .collect();
    format!("{{{}}}", parts.join(", "))
}

// ───────────────────────────── panels ─────────────────────────────

impl RegexApp {
    /// Pattern and input fields, each with its verdict; an error line only when needed.
    fn inputs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pattern_badge = match &self.compiled {
            Ok(_) => badge("VALID", theme::ok()),
            Err(_) => badge("INVALID", theme::bad()),
        };
        let verdict = match &self.trace {
            Some(t) if t.matched => badge("MATCH", theme::ok()),
            Some(_) => badge("NO MATCH", theme::bad()),
            None => badge("—", theme::muted()),
        };
        let error = self.compiled.as_ref().err().map(|e| {
            let pat = self.pattern.read(cx).text();
            let caret = " ".repeat(e.pos.min(pat.chars().count())) + "^";
            div()
                .pl(px(80.))
                .flex()
                .gap_4()
                .text_size(px(13.))
                .text_color(theme::bad())
                .child(div().font_family(theme::MONO).child(caret))
                .child(e.msg.clone())
        });
        let row = |label: &str, field: AnyElement, badge: gpui::Div| {
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(div().w(px(68.)).text_size(px(13.)).text_color(theme::muted()).child(label.to_string()))
                .child(field)
                .child(div().w(px(84.)).flex().justify_end().child(badge))
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row("Pattern", self.pattern.clone().into_any_element(), pattern_badge))
            .children(error)
            .child(row("Input", self.input.clone().into_any_element(), verdict))
    }

    /// Tabs on the left, example patterns on the right.
    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = |id: &str, label: &str, t: Tab| {
            button(id.to_string(), label.to_string(), self.tab == t).on_click(cx.listener(move |this, _, _, cx| {
                this.tab = t;
                this.picker_open = false;
                cx.notify();
            }))
        };
        let n = PRESETS.len();
        let current = self.pattern.read(cx).text();
        let label = match self.preset {
            Some(i) if PRESETS[i].pattern == current => format!("{} / {n}  {}", i + 1, PRESETS[i].name),
            _ => "Custom pattern".to_string(),
        };
        let cur = self.preset.unwrap_or(0);
        let picker = div()
            .flex()
            .gap_1()
            .child(button("prev-ex", "‹", false).on_click(cx.listener(move |this, _, _, cx| this.load_preset((cur + n - 1) % n, cx))))
            .child(
                button("examples", format!("{label}  ▾"), self.picker_open).min_w(px(260.)).on_click(cx.listener(|this, _, _, cx| {
                    this.picker_open = !this.picker_open;
                    cx.notify();
                })),
            )
            .child(button("next-ex", "›", false).on_click(cx.listener(move |this, _, _, cx| {
                let next = if this.preset.is_some() { (cur + 1) % n } else { 0 };
                this.load_preset(next, cx)
            })));
        div()
            .flex()
            .gap_2()
            .items_center()
            .child(tab("tab-nfa", "NFA", Tab::Nfa))
            .child(tab("tab-dfa", "DFA", Tab::Dfa))
            .child(tab("tab-bench", "Benchmark", Tab::Bench))
            .child(div().flex_1())
            .child(div().text_size(px(12.)).text_color(theme::muted()).child("Examples"))
            .child(picker)
    }

    /// Every example, grouped by category; shown in place of the main panel.
    fn picker(&self, cx: &Context<Self>) -> AnyElement {
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, p) in PRESETS.iter().enumerate() {
            if i == 0 || PRESETS[i - 1].category != p.category {
                rows.push(div().pt_2().pb_1().px_2().child(section(p.category)).into_any_element());
            }
            rows.push(
                div()
                    .id(("example", i))
                    .flex()
                    .gap_3()
                    .px_2()
                    .py(px(3.))
                    .rounded_sm()
                    .cursor_pointer()
                    .text_size(px(13.))
                    .when(self.preset == Some(i), |d| d.bg(theme::accent_dim()))
                    .hover(|d| d.bg(theme::accent_dim()))
                    .child(div().w(px(200.)).flex_none().child(p.name))
                    .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().font_family(theme::MONO).text_color(theme::muted()).child(p.pattern))
                    .on_click(cx.listener(move |this, _, _, cx| this.load_preset(i, cx)))
                    .into_any_element(),
            );
        }
        card()
            .flex_1()
            .child(div().id("picker").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().children(rows))
            .into_any_element()
    }

    fn tape(&self, t: &Trace) -> impl IntoElement {
        let pos = t.steps[self.step].pos;
        let cells = t.input.iter().enumerate().map(|(i, &c)| {
            let consumed = i < pos;
            div()
                .w(px(22.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(if i == pos { theme::accent() } else { gpui::transparent_black() })
                .bg(if consumed { theme::accent_dim() } else { theme::field() })
                .text_color(if consumed { theme::muted() } else { theme::text() })
                .font_family(theme::MONO)
                .child(if c == ' ' { "␣".to_string() } else { c.to_string() })
        });
        div().flex().flex_wrap().gap_1().children(cells)
    }

    fn nfa_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let (Ok(c), Some(t)) = (&self.compiled, &self.trace) else {
            return card().flex_1().child(div().text_color(theme::muted()).child("Fix the pattern to see its NFA.")).into_any_element();
        };
        let step = &t.steps[self.step];
        let hi = Highlight {
            states: step.states.iter().copied().collect(),
            splits: step.splits.iter().copied().collect(),
            fired: if self.step > 0 { step.fired.iter().copied().collect() } else { HashSet::new() },
        };
        let last = self.last_step();
        let playing = self.playing.is_some();

        let icon = |id: &str, label: &str| button(id.to_string(), label.to_string(), false);
        let controls = div()
            .flex()
            .gap_1()
            .items_center()
            .child(icon("first", "⏮").on_click(cx.listener(|this, _, _, cx| this.set_step(0, cx))))
            .child(icon("prev", "◀").on_click(cx.listener(|this, _, _, cx| {
                let s = this.step.saturating_sub(1);
                this.set_step(s, cx)
            })))
            .child(
                button("play", if playing { "⏸" } else { "▶ Play" }, playing)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_play(cx))),
            )
            .child(icon("next", "▶").on_click(cx.listener(|this, _, _, cx| {
                let s = this.step + 1;
                this.set_step(s, cx)
            })))
            .child(icon("last", "⏭").on_click(cx.listener(move |this, _, _, cx| this.set_step(last, cx))));

        let zoom = div()
            .flex()
            .gap_1()
            .items_center()
            .child(icon("zoom-out", "−").on_click(cx.listener(|this, _, _, cx| {
                this.zoom = (this.zoom / 1.2).max(0.4);
                cx.notify();
            })))
            .child(icon("zoom-in", "+").on_click(cx.listener(|this, _, _, cx| {
                this.zoom = (this.zoom * 1.2).min(2.5);
                cx.notify();
            })));

        let has_match = step.states.iter().any(|&s| c.nfa.states[s].kind == Kind::Match);
        let status = div()
            .flex()
            .gap_2()
            .items_center()
            .text_size(px(13.))
            .text_color(theme::muted())
            .child(match self.step {
                0 => "start:".to_string(),
                n => format!("after '{}':", t.input[n - 1]),
            })
            .child(div().font_family(theme::MONO).text_color(theme::text()).child(set_str(&c.nfa, &step.states)))
            .when(step.states.is_empty(), |d| d.child(badge("dead", theme::bad())))
            .when(has_match, |d| d.child(badge("match", theme::ok())))
            .child(div().flex_1())
            .child(format!("postfix {}", postfix_string(&c.postfix)));

        card()
            .flex_1()
            .child(
                div()
                    .id("graph-scroll")
                    .flex_1()
                    .min_h(px(180.))
                    .overflow_scroll()
                    .rounded_md()
                    .bg(theme::field())
                    .child({
                        // Centre the graph when it fits; when it doesn't, grow to its
                        // size so it scrolls from the left edge instead of being clipped.
                        let (gw, gh) = graph::size(&c.nfa, self.zoom);
                        div()
                            .w(px(gw))
                            .h(px(gh))
                            .min_w_full()
                            .min_h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(graph::render(&c.nfa, &hi, self.zoom))
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(controls)
                    .child(div().flex_1().min_w_0().child(self.tape(t)))
                    .child(zoom),
            )
            .child(status)
            .into_any_element()
    }

    fn dfa_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let Ok(c) = &self.compiled else {
            return card().flex_1().child(div().text_color(theme::muted()).child("Fix the pattern first.")).into_any_element();
        };
        let path: HashSet<usize> = self.dfa_run.as_ref().map(|r| r.path.iter().copied().collect()).unwrap_or_default();
        let end = self.dfa_run.as_ref().and_then(|r| r.path.last().copied());

        let rows = self.dfa.states.iter().enumerate().map(|(i, d)| {
            let mut trans: Vec<_> = d.next.iter().collect();
            trans.sort();
            let trans = trans.into_iter().map(|(ch, to)| format!("{ch}→D{to}")).collect::<Vec<_>>().join("   ");
            let on_path = path.contains(&i);
            div()
                .flex()
                .gap_3()
                .px_2()
                .py_1()
                .rounded_sm()
                .font_family(theme::MONO)
                .text_size(px(13.))
                .bg(if Some(i) == end { theme::accent_dim() } else { theme::panel() })
                .child(
                    div()
                        .w(px(52.))
                        .text_color(if d.accepting { theme::ok() } else if on_path { theme::accent() } else { theme::text() })
                        .child(if d.accepting { format!("D{i} ✓") } else { format!("D{i}") }),
                )
                .child(div().w(px(240.)).text_color(theme::muted()).child(set_str(&c.nfa, &d.nfa_states)))
                .child(div().child(trans))
        });

        card()
            .flex_1()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .text_size(px(13.))
                    .text_color(theme::muted())
                    .child(format!("{} states", self.dfa.states.len()))
                    .child(format!("{} cache hits, {} misses", self.dfa.hits, self.dfa.misses))
                    .child(div().flex_1())
                    .child(button("clear-dfa", "Clear cache", false).on_click(cx.listener(|this, _, _, cx| {
                        this.dfa = Dfa::default();
                        this.rerun(cx);
                    }))),
            )
            .child(div().id("dfa-scroll").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap_1().children(rows))
            .into_any_element()
    }

    fn bench_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let max_steps = self.bench.iter().map(|r| r.bt_steps.max(r.th_steps)).max().unwrap_or(1).max(10) as f64;
        let bar = |v: u64, color: Hsla| {
            let frac = ((v.max(1) as f64).log10() / max_steps.log10()).clamp(0.0, 1.0) as f32;
            div().h(px(8.)).w(gpui::relative(frac.max(0.005))).rounded_sm().bg(color)
        };
        let rows = self.bench.iter().map(|r| {
            div()
                .flex()
                .gap_3()
                .items_center()
                .text_size(px(12.))
                .font_family(theme::MONO)
                .child(div().w(px(36.)).text_color(theme::muted()).child(format!("n={}", r.n)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(bar(r.bt_steps, theme::muted()))
                        .child(bar(r.th_steps, theme::accent())),
                )
                .child(
                    div()
                        .w(px(110.))
                        .flex()
                        .flex_col()
                        .items_end()
                        .child(div().text_color(theme::muted()).child(format!("{}{:.1?}", if r.bt_done { "" } else { ">" }, r.bt_time)))
                        .child(div().text_color(theme::accent()).child(format!("{:.1?}", r.th_time))),
                )
        });

        let running = self.bench_task.is_some();
        card()
            .flex_1()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .text_size(px(13.))
                    .child(div().font_family(theme::MONO).child("a?ⁿaⁿ vs aⁿ"))
                    .child(button("n-", "−", false).on_click(cx.listener(|this, _, _, cx| {
                        this.bench_n = this.bench_n.saturating_sub(1).max(1);
                        cx.notify();
                    })))
                    .child(div().w(px(44.)).flex().justify_center().child(format!("n≤{}", self.bench_n)))
                    .child(button("n+", "+", false).on_click(cx.listener(|this, _, _, cx| {
                        this.bench_n = (this.bench_n + 1).min(40);
                        cx.notify();
                    })))
                    .child(button("run-bench", if running { "Running…" } else { "Run" }, running).on_click(cx.listener(
                        |this, _, _, cx| {
                            if this.bench_task.is_none() {
                                this.run_bench(cx)
                            }
                        },
                    )))
                    .child(div().flex_1())
                    .child(div().text_size(px(12.)).text_color(theme::muted()).child("■ backtracking"))
                    .child(div().text_size(px(12.)).text_color(theme::accent()).child("■ Thompson")),
            )
            .child(div().id("bench-scroll").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap_2().children(rows))
            .into_any_element()
    }

    fn test_suite(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.input.read(cx).text();
        let results: Vec<Option<bool>> =
            self.tests.iter().map(|s| self.compiled.as_ref().ok().map(|c| sim::is_match(&c.nfa, s))).collect();
        let passed = results.iter().filter(|r| **r == Some(true)).count();
        let rows = self.tests.iter().zip(&results).enumerate().map(|(i, (s, res))| {
            let (mark, fg) = match res {
                Some(true) => ("✓", theme::ok()),
                Some(false) => ("✗", theme::bad()),
                None => ("?", theme::muted()),
            };
            let s_owned = s.clone();
            div()
                .id(("test", i))
                .flex()
                .gap_2()
                .items_center()
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .bg(if *s == current { theme::accent_dim() } else { gpui::transparent_black() })
                .hover(|d| d.bg(theme::field()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let s = s_owned.clone();
                    this.input.update(cx, |p, cx| p.set_text(&s, cx));
                }))
                .child(div().w(px(14.)).text_color(fg).child(mark))
                .child(div().flex_1().font_family(theme::MONO).text_size(px(13.)).overflow_hidden().child(show(s)))
                .child(
                    div()
                        .id(("del", i))
                        .px_1()
                        .text_color(theme::muted())
                        .hover(|d| d.text_color(theme::bad()))
                        .child("×")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.tests.remove(i);
                            cx.notify();
                        })),
                )
        });
        card()
            .w(px(240.))
            .flex_none()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(section("tests"))
                    .child(div().text_size(px(12.)).text_color(theme::muted()).child(format!("{passed}/{}", self.tests.len()))),
            )
            .child(div().id("tests-scroll").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().children(rows))
            .child(div().text_size(px(11.)).text_color(theme::muted()).child("Enter in Input adds a test"))
    }
}

impl Render for RegexApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let main = match self.tab {
            _ if self.picker_open => self.picker(cx),
            Tab::Nfa => self.nfa_tab(cx),
            Tab::Dfa => self.dfa_tab(cx),
            Tab::Bench => self.bench_tab(cx),
        };
        div()
            .size_full()
            .bg(theme::bg())
            .text_color(theme::text())
            .font_family("DejaVu Sans")
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.inputs(cx))
            .child(self.toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_3()
                    .child(div().flex_1().min_w_0().flex().flex_col().child(main))
                    .child(self.test_suite(cx)),
            )
    }
}
