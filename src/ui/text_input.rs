//! A small single-line text field. GPUI ships no input widget, so this handles
//! key events directly: printable characters, editing keys and paste.

use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, SharedString, Window, div,
    prelude::*, px,
};

use super::theme;

pub enum InputEvent {
    Changed,
    Submit,
}

pub struct TextInput {
    chars: Vec<char>,
    cursor: usize,
    placeholder: SharedString,
    focus: FocusHandle,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    pub fn new(text: &str, placeholder: &str, cx: &mut Context<Self>) -> Self {
        let chars: Vec<char> = text.chars().collect();
        TextInput { cursor: chars.len(), chars, placeholder: placeholder.to_string().into(), focus: cx.focus_handle() }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn insert(&mut self, s: &str) {
        for c in s.chars().filter(|c| !c.is_control()) {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        let ctrl = m.control || m.platform;
        let before = (self.chars.clone(), self.cursor);
        match ks.key.as_str() {
            "enter" => {
                cx.emit(InputEvent::Submit);
                cx.stop_propagation();
                return;
            }
            "backspace" if ctrl => {
                // Delete back to the previous word boundary.
                while self.cursor > 0 && self.chars[self.cursor - 1].is_whitespace() {
                    self.cursor -= 1;
                    self.chars.remove(self.cursor);
                }
                while self.cursor > 0 && !self.chars[self.cursor - 1].is_whitespace() {
                    self.cursor -= 1;
                    self.chars.remove(self.cursor);
                }
            }
            "backspace" => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.chars.remove(self.cursor);
                }
            }
            "delete" => {
                if self.cursor < self.chars.len() {
                    self.chars.remove(self.cursor);
                }
            }
            "left" => self.cursor = self.cursor.saturating_sub(1),
            "right" => self.cursor = (self.cursor + 1).min(self.chars.len()),
            "home" => self.cursor = 0,
            "end" => self.cursor = self.chars.len(),
            "a" if ctrl => self.cursor = 0,
            "e" if ctrl => self.cursor = self.chars.len(),
            "u" if ctrl => {
                self.chars.drain(..self.cursor);
                self.cursor = 0;
            }
            "v" if ctrl => {
                if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    self.insert(&t);
                }
            }
            "c" if ctrl => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.text()));
            }
            _ => {
                if ctrl || m.alt {
                    return;
                }
                match &ks.key_char {
                    Some(s) => self.insert(s),
                    None => return,
                }
            }
        }
        cx.stop_propagation();
        if (self.chars.clone(), self.cursor) != before {
            if self.chars != before.0 {
                cx.emit(InputEvent::Changed);
            }
            cx.notify();
        }
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let caret = div().w(px(2.)).h(px(20.)).bg(if focused { theme::accent() } else { gpui::transparent_black() });
        let left: String = self.chars[..self.cursor].iter().collect();
        let right: String = self.chars[self.cursor..].iter().collect();

        let content = if self.chars.is_empty() {
            div()
                .flex()
                .items_center()
                .child(caret)
                .child(div().text_color(theme::muted()).child(self.placeholder.clone()))
        } else {
            div()
                .flex()
                .items_center()
                .child(div().whitespace_nowrap().child(left))
                .child(caret)
                .child(div().whitespace_nowrap().child(right))
        };

        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.focus.focus(window);
                    cx.notify();
                }),
            )
            .flex_1()
            .h(px(38.))
            .px_3()
            .flex()
            .items_center()
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(if focused { theme::accent() } else { theme::border() })
            .bg(theme::field())
            .font_family(theme::MONO)
            .text_size(px(16.))
            .cursor_text()
            .child(content)
    }
}
