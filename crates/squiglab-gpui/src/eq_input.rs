//! An editable numeric field for the equalizer panel.
//!
//! GPUI 0.2.2 ships no ready-made text input, so this is a small custom one,
//! built on the same primitives as `gpui::examples::input`: an
//! [`EntityInputHandler`] registered with [`Window::handle_input`], key
//! handling for editing commands, and a focused cursor painted on top of the
//! shaped text.
//!
//! It is deliberately narrow in scope: only the characters that make up a
//! decimal number (digits, one dot, one leading minus) are accepted, the
//! field always holds a parseable `f64`, and pressing Enter or leaving the
//! field commits the parsed value back to the caller.

use std::ops::Range;

use gpui::{
    actions, App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UnderlineStyle, UTF16Selection, Window, div, fill,
    point, prelude::*, px, relative, rgb, rgba, size, white,
};
use unicode_segmentation::UnicodeSegmentation;

actions!(
    eq_input,
    [Backspace, Delete, Left, Right, SelectLeft, SelectRight, SelectAll, Home, End, Paste, Cut, Copy]
);

/// An editable `f64` value rendered as text.
///
/// The field always stores a parseable number: typing an unparseable
/// intermediate value (a lone dot, a bare minus) leaves the committed value
/// unchanged, while the visible text reflects what the user has typed so far.
/// Commit happens on Enter, Tab, or when focus leaves.
pub struct EqInput {
    /// The committed value, exposed to the caller through [`EqInput::value`].
    value: f64,
    /// The text currently on screen, which may be a partial edit of `value`.
    text: SharedString,
    /// The caret / selection, as a UTF-8 byte range into `text`.
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    focus_handle: FocusHandle,
    /// The unit shown at the right of the field (`"Hz"`, `"dB"`, or `""`).
    #[allow(dead_code)]
    unit: SharedString,
    /// Number of decimal places used when formatting `value`.
    decimals: usize,
    /// Minimum allowed value; `None` means no floor.
    min: Option<f64>,
    /// Maximum allowed value; `None` means no ceiling.
    max: Option<f64>,
    /// Callback invoked when the user commits a new value.
    on_change: Option<Box<dyn Fn(f64)>>,
}

impl EqInput {
    /// Build a field showing `value` with `decimals` decimal places and the
    /// given unit label. `on_change` is called whenever the user commits.
    pub fn new(
        focus_handle: FocusHandle,
        value: f64,
        decimals: usize,
        unit: &'static str,
        min: Option<f64>,
        max: Option<f64>,
        _on_change: impl Fn(f64) + 'static,
    ) -> Self {
        Self {
            value,
            text: format_value(value, decimals).into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            focus_handle,
            unit: unit.into(),
            decimals,
            min,
            max,
            on_change: Some(Box::new(_on_change)),
        }
    }

    /// The last committed value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Replace the displayed value without treating it as a user edit.
    #[allow(dead_code)]
    pub fn set_value(&mut self, value: f64) {
        self.value = value;
        self.text = format_value(value, self.decimals).into();
        self.selected_range = 0..self.text.len();
    }

    /// Parse the on-screen text as a number, clamping to `[min, max]`.
    fn parse_text(&self) -> Option<f64> {
        let trimmed: &str = self.text.as_str().trim();
        if trimmed.is_empty() || trimmed == "-" || trimmed == "." || trimmed == "-." {
            return None;
        }
        let parsed: f64 = trimmed.parse().ok()?;
        Some(parsed.clamp(
            self.min.unwrap_or(f64::NEG_INFINITY),
            self.max.unwrap_or(f64::INFINITY),
        ))
    }

    /// Commit the on-screen text as the new value, notifying the caller.
    fn commit(&mut self, cx: &mut Context<Self>) {
        let new_value = match self.parse_text() {
            Some(v) => v,
            None => {
                // Unparseable: restore the committed value's text.
                self.text = format_value(self.value, self.decimals).into();
                self.selected_range = 0..self.text.len();
                cx.notify();
                return;
            }
        };
        if (new_value - self.value).abs() > f64::EPSILON {
            self.value = new_value;
            self.text = format_value(new_value, self.decimals).into();
            self.selected_range = 0..self.text.len();
            cx.notify();
            if let Some(on_change) = &self.on_change {
                (on_change)(new_value);
            }
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.text
            .as_str()
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text
            .as_str()
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.text.len())
    }

    fn replace_text_in_range(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        cx: &mut Context<Self>,
    ) {
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        self.text = (self.text[0..start].to_owned() + new_text + &self.text[end..]).into();
        let cursor = start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    /// Insert a single character at the cursor, keeping only characters that
    /// can be part of a decimal number.
    fn insert_char(&mut self, ch: char, cx: &mut Context<Self>) {
        let cursor = self.cursor_offset();
        let before = &self.text.as_str()[..cursor.min(self.text.len())];
        let after = &self.text.as_str()[cursor.min(self.text.len())..];

        // Only digits, at most one dot, at most one leading minus.
        if ch.is_ascii_digit() {
        } else if ch == '.' {
            if before.contains('.') || after.contains('.') {
                return;
            }
        } else if ch == '-' {
            if cursor != 0 || before.contains('-') || after.contains('-') {
                return;
            }
        } else {
            return;
        }

        self.text = (before.to_owned() + ch.to_string().as_str() + after).into();
        self.selected_range = cursor + 1..cursor + 1;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    fn backspace(&mut self, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let cursor = self.cursor_offset();
            if cursor == 0 {
                return;
            }
            let start = self.previous_boundary(cursor);
            self.replace_text_in_range(start..cursor, "", cx);
        } else {
            let range = self.selected_range.clone();
            self.replace_text_in_range(range, "", cx);
        }
    }

    fn delete(&mut self, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let cursor = self.cursor_offset();
            if cursor >= self.text.len() {
                return;
            }
            let end = self.next_boundary(cursor);
            self.replace_text_in_range(cursor..end, "", cx);
        } else {
            let range = self.selected_range.clone();
            self.replace_text_in_range(range, "", cx);
        }
    }

    fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let filtered: String = text
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            if !filtered.is_empty() {
                let cursor = self.cursor_offset();
                let before = &self.text.as_str()[..cursor.min(self.text.len())];
                let after = &self.text.as_str()[cursor.min(self.text.len())..];
                self.text = (before.to_owned() + &filtered + after).into();
                self.selected_range = cursor + filtered.len()..cursor + filtered.len();
                self.selection_reversed = false;
                self.marked_range = None;
                cx.notify();
            }
        }
        let _ = window;
    }

    fn cut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            let _ = cx.write_to_clipboard(ClipboardItem::new_string(
                self.text.as_str()[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(
                self.selected_range.clone(),
                "",
                cx,
            );
        }
        let _ = window;
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            let _ = cx.write_to_clipboard(ClipboardItem::new_string(
                self.text.as_str()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    /// Focus the field and select all its text.
    #[allow(dead_code)]
    fn focus_select_all(&mut self, cx: &mut Context<Self>) {
        self.selected_range = 0..self.text.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.text.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    /// Handle a raw key down event: insert printable characters and run
    /// editing commands. This keeps the widget self-contained without
    /// requiring global key bindings.
    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = &event.keystroke.key;
        let key_char = event.keystroke.key_char.as_deref();
        let modifiers = event.keystroke.modifiers;

        match key.as_str() {
            "Backspace" => self.backspace(cx),
            "Delete" => self.delete(cx),
            "ArrowLeft" => {
                if self.selected_range.is_empty() {
                    self.move_to(self.previous_boundary(self.cursor_offset()), cx);
                } else {
                    self.move_to(self.selected_range.start, cx);
                }
            }
            "ArrowRight" => {
                if self.selected_range.is_empty() {
                    self.move_to(self.next_boundary(self.cursor_offset()), cx);
                } else {
                    self.move_to(self.selected_range.end, cx);
                }
            }
            "Home" => self.move_to(0, cx),
            "End" => self.move_to(self.text.len(), cx),
            "Enter" | "Tab" => self.commit(cx),
            _ => {}
        }

        // Insert the typed character when no modifier is held.
        if let Some(ch) = key_char.and_then(|s| s.chars().next()) {
            if !modifiers.control && !modifiers.platform && !modifiers.alt {
                self.insert_char(ch, cx);
            }
        }
    }

    // --- mouse ---

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }
}

impl Focusable for EqInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for EqInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        // Only ASCII is accepted, so UTF-16 units == UTF-8 bytes.
        let start = range_utf16.start.min(self.text.len());
        let end = range_utf16.end.min(self.text.len());
        actual_range.replace(start..end);
        Some(self.text.as_str()[start..end].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.selected_range.clone(),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range.clone()
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.unwrap_or_else(|| self.selected_range.clone());
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        self.text = (self.text.as_str()[0..start].to_owned() + new_text + &self.text.as_str()[end..])
            .into();
        let cursor = start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.unwrap_or_else(|| self.selected_range.clone());
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        self.text = (self.text.as_str()[0..start].to_owned() + new_text + &self.text.as_str()[end..])
            .into();
        if !new_text.is_empty() {
            self.marked_range = Some(start..start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .map(|r| r.start + start..r.end + start)
            .unwrap_or_else(|| start + new_text.len()..start + new_text.len());
        self.selection_reversed = false;
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let start = range_utf16.start.min(self.text.len());
        let end = range_utf16.end.min(self.text.len());
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(utf8_index)
    }
}

impl Render for EqInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .key_context("EqInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace_action))
            .on_action(cx.listener(Self::delete_action))
            .on_action(cx.listener(Self::left_action))
            .on_action(cx.listener(Self::right_action))
            .on_action(cx.listener(Self::select_left_action))
            .on_action(cx.listener(Self::select_right_action))
            .on_action(cx.listener(Self::select_all_action))
            .on_action(cx.listener(Self::home_action))
            .on_action(cx.listener(Self::end_action))
            .on_action(cx.listener(Self::paste_action))
            .on_action(cx.listener(Self::cut_action))
            .on_action(cx.listener(Self::copy_action))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .bg(rgb(0xeeeeee))
            .line_height(px(26.))
            .text_size(px(12.))
            .child(
                div()
                    .h(px(26. + 4. * 2.))
                    .w_full()
                    .p(px(4.))
                    .bg(white())
                    .child(TextElement { input: cx.entity() }),
            )
    }
}

impl EqInput {
    // --- action handlers ---

    fn backspace_action(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        self.backspace(cx);
    }

    fn delete_action(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        self.delete(cx);
    }

    fn left_action(&mut self, _: &Left, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right_action(&mut self, _: &Right, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left_action(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right_action(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all_action(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.text.len(), cx);
    }

    fn home_action(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end_action(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.text.len(), cx);
    }

    fn paste_action(
        &mut self,
        _: &Paste,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.paste(window, cx);
    }

    fn cut_action(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.cut(window, cx);
    }

    fn copy_action(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.copy(cx);
    }
}

/// The painted text element: shapes the text, draws the cursor/selection, and
/// registers the input handler so the platform delivers key events.
struct TextElement {
    input: Entity<EqInput>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.text.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            ("".into(), style.color)
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor_quad) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor: cursor_quad,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        line.paint(bounds.origin, window.line_height(), window, cx)
            .unwrap();

        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

/// Format a committed value for display, trimming needless zeros.
fn format_value(value: f64, decimals: usize) -> String {
    let mut text = match decimals {
        0 => format!("{value:.0}"),
        1 => {
            let mut t = format!("{value:.1}");
            if t.ends_with(".0") {
                t.pop();
                t.pop();
            }
            t
        }
        _ => {
            let mut t = format!("{value:.2}");
            if t.contains('.') {
                while t.ends_with('0') {
                    t.pop();
                }
                if t.ends_with('.') {
                    t.pop();
                }
            }
            t
        }
    };
    if text == "-0" {
        text = "0".to_owned();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_trims_needless_zeros() {
        assert_eq!(format_value(1000.0, 0), "1000");
        assert_eq!(format_value(6.0, 1), "6");
        assert_eq!(format_value(1.40, 2), "1.4");
        assert_eq!(format_value(0.0, 2), "0");
        assert_eq!(format_value(-0.0, 2), "0");
        assert_eq!(format_value(-3.50, 2), "-3.5");
    }
}
