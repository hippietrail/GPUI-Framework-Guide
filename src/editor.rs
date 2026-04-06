use std::ops::Range;
use std::time::Instant;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, FontWeight, GlobalElementId, Hsla, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window,
    actions, div, fill, hsla, point, prelude::*, px, relative, size,
};
use numnum_core::lexer::{Lexer, TokenKind};
use numnum_core::types::{CurrencyTable, UnitTable};
use unicode_segmentation::*;

use crate::theme::Theme;

actions!(
    editor,
    [
        Enter,
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
    ]
);

#[derive(Clone)]
struct UndoEntry {
    content: String,
    selected_range: Range<usize>,
}

type OnChangeFn = Box<dyn Fn(&str, &mut Window, &mut App)>;

/// Fixed gutter width for line numbers (in px).
const GUTTER_WIDTH: f32 = 40.0;
/// Padding between gutter and editor text.
const GUTTER_PADDING_RIGHT: f32 = 8.0;

pub struct Editor {
    focus_handle: FocusHandle,
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    is_selecting: bool,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    on_change: Option<OnChangeFn>,
    theme: Theme,
    font_family: SharedString,
    font_size: Pixels,
    unit_table: UnitTable,
    currency_table: CurrencyTable,
    // Per-line diagnostics (error messages shown as inlay below the line)
    pub diagnostics: Vec<Option<String>>,
    // Per-line layout cache (rebuilt each frame)
    line_layouts: Vec<Option<ShapedLine>>,
    last_bounds: Option<Bounds<Pixels>>,
    line_height: Pixels,
    // Cursor blink: timestamp of last cursor movement
    cursor_last_moved: Instant,
}

impl Editor {
    pub fn new(
        cx: &mut Context<Self>,
        theme: Theme,
        font_family: String,
        font_size: f32,
        on_change: Option<OnChangeFn>,
    ) -> Self {
        Editor {
            focus_handle: cx.focus_handle(),
            content: String::new(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            is_selecting: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            on_change,
            theme,
            font_family: SharedString::from(font_family),
            font_size: px(font_size),
            diagnostics: Vec::new(),
            unit_table: UnitTable::new(),
            currency_table: CurrencyTable::new(),
            line_layouts: Vec::new(),
            last_bounds: None,
            line_height: px(26.0),
            cursor_last_moved: Instant::now(),
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let offset = self.cursor_offset();
        let before = &self.content[..offset];
        let line = before.matches('\n').count();
        let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = offset - last_newline;
        (line, col)
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(UndoEntry {
            content: self.content.clone(),
            selected_range: self.selected_range.clone(),
        });
        self.redo_stack.clear();
    }

    fn fire_change(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(cb) = &self.on_change {
            let content = self.content.clone();
            cb(&content, window, cx);
        }
    }

    // --- Line helpers ---

    fn line_count(&self) -> usize {
        self.content.split('\n').count()
    }

    /// Given a byte offset, return (line_index, col_byte_offset)
    fn line_col_for_offset(&self, offset: usize) -> (usize, usize) {
        let before = &self.content[..offset.min(self.content.len())];
        let line = before.matches('\n').count();
        let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        (line, offset - last_newline)
    }

    /// Given (line, col), return byte offset
    fn offset_for_line_col(&self, line: usize, col: usize) -> usize {
        let mut offset = 0;
        for (i, l) in self.content.split('\n').enumerate() {
            if i == line {
                return offset + col.min(l.len());
            }
            offset += l.len() + 1;
        }
        self.content.len()
    }

    /// Byte offset of the start of a given line
    fn line_start_offset(&self, line: usize) -> usize {
        self.offset_for_line_col(line, 0)
    }

    /// Byte offset of the end of a given line (before the newline)
    fn line_end_offset(&self, line: usize) -> usize {
        let lines: Vec<&str> = self.content.split('\n').collect();
        if line >= lines.len() {
            return self.content.len();
        }
        self.line_start_offset(line) + lines[line].len()
    }

    // --- Navigation actions ---

    fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        self.push_undo();
        let range = self.selected_range.clone();
        self.content = format!(
            "{}\n{}",
            &self.content[..range.start],
            &self.content[range.end..]
        );
        let new_pos = range.start + 1;
        self.selected_range = new_pos..new_pos;
        self.marked_range.take();
        self.fire_change(window, cx);
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, cx);
        }
        self.push_undo();
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, cx);
        }
        self.push_undo();
        self.replace_text_in_range(None, "", window, cx);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.line_col_for_offset(self.cursor_offset());
        if line > 0 {
            self.move_to(self.offset_for_line_col(line - 1, col), cx);
        } else {
            self.move_to(0, cx);
        }
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.line_col_for_offset(self.cursor_offset());
        let count = self.line_count();
        if line + 1 < count {
            self.move_to(self.offset_for_line_col(line + 1, col), cx);
        } else {
            self.move_to(self.content.len(), cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _col) = self.line_col_for_offset(self.cursor_offset());
        self.move_to(self.line_start_offset(line), cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let (line, _col) = self.line_col_for_offset(self.cursor_offset());
        self.move_to(self.line_end_offset(line), cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.push_undo();
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.push_undo();
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(UndoEntry {
                content: self.content.clone(),
                selected_range: self.selected_range.clone(),
            });
            self.content = entry.content;
            self.selected_range = entry.selected_range;
            self.fire_change(window, cx);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(UndoEntry {
                content: self.content.clone(),
                selected_range: self.selected_range.clone(),
            });
            self.content = entry.content;
            self.selected_range = entry.selected_range;
            self.fire_change(window, cx);
            cx.notify();
        }
    }

    // --- Mouse handling ---

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        let idx = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(idx, cx);
        } else {
            self.move_to(idx, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let idx = self.index_for_mouse_position(event.position);
            self.select_to(idx, cx);
        }
    }

    // --- Core movement and selection helpers ---

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.cursor_last_moved = Instant::now();
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
        self.cursor_last_moved = Instant::now();
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    // --- Mouse hit-testing for multi-line ---

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };

        if self.content.is_empty() {
            return 0;
        }

        // Determine which line was clicked
        let y_offset = position.y - bounds.top();
        let line_idx = if y_offset < px(0.) {
            0
        } else {
            (y_offset / self.line_height) as usize
        };

        let lines: Vec<&str> = self.content.split('\n').collect();
        let line_idx = line_idx.min(lines.len().saturating_sub(1));

        // Find byte offset within line using shaped line
        let col: usize = if let Some(Some(shaped)) = self.line_layouts.get(line_idx) {
            let x: Pixels = position.x - bounds.left();
            shaped.closest_index_for_x(x)
        } else {
            0
        };

        self.offset_for_line_col(line_idx, col)
    }

    // --- UTF-16 conversion helpers (for IME) ---

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    // --- Syntax highlighting ---

    fn highlight_line(&self, line: &str, style: &TextRun) -> Vec<TextRun> {
        if line.is_empty() {
            return vec![TextRun {
                len: 0,
                ..style.clone()
            }];
        }

        let mut lexer = Lexer::new(line, &self.unit_table, &self.currency_table);
        let tokens = lexer.tokenize();

        let mut runs = Vec::new();
        let mut last_end = 0usize;

        for tok in &tokens {
            let start = tok.span.start;
            let end = tok.span.end;
            if start > last_end {
                // Gap -- whitespace or unknown chars, use default text color
                runs.push(TextRun {
                    len: start - last_end,
                    color: self.theme.text,
                    ..style.clone()
                });
            }
            if end > start {
                let color = self.color_for_token(&tok.kind);
                runs.push(TextRun {
                    len: end - start,
                    color,
                    ..style.clone()
                });
            }
            last_end = end;
        }

        // If no runs were generated, fallback to full-line default
        if runs.is_empty() {
            runs.push(TextRun {
                len: line.len(),
                ..style.clone()
            });
        }

        // Verify total length matches
        let total: usize = runs.iter().map(|r| r.len).sum();
        if total < line.len() {
            runs.push(TextRun {
                len: line.len() - total,
                color: self.theme.text,
                ..style.clone()
            });
        }

        runs
    }

    fn color_for_token(&self, kind: &TokenKind) -> gpui::Hsla {
        match kind {
            TokenKind::Number(_) | TokenKind::NumberRepr(_, _) => self.theme.syn_number,
            TokenKind::Op(_) => self.theme.syn_operator,
            TokenKind::LParen | TokenKind::RParen => self.theme.syn_operator,
            TokenKind::Percent => self.theme.syn_percent,
            TokenKind::Assign | TokenKind::CompoundAssign(_) => self.theme.syn_operator,
            TokenKind::Convert => self.theme.syn_keyword,
            TokenKind::Of | TokenKind::From | TokenKind::On | TokenKind::Off => {
                self.theme.syn_keyword
            }
            TokenKind::AsAPctOf | TokenKind::AsAPctOn | TokenKind::AsAPctOff => {
                self.theme.syn_keyword
            }
            TokenKind::OfWhatIs | TokenKind::OnWhatIs | TokenKind::OffWhatIs => {
                self.theme.syn_keyword
            }
            TokenKind::Func(_) => self.theme.syn_function,
            TokenKind::Unit(_) => self.theme.syn_unit,
            TokenKind::Currency(_) => self.theme.syn_currency,
            TokenKind::CurrencySymbol(_) => self.theme.text, // $ € £ etc blend with numbers
            TokenKind::Scale(_) => self.theme.syn_scale,
            TokenKind::Repr(_) => self.theme.syn_keyword,
            TokenKind::Ident(_) => self.theme.syn_variable,
            TokenKind::Agg(_) => self.theme.syn_function,
            TokenKind::Comment => self.theme.syn_comment,
            TokenKind::Header => self.theme.syn_header,
            TokenKind::Label(_) => self.theme.syn_label,
            TokenKind::Eof => self.theme.text,
        }
    }
}

// --- EntityInputHandler (IME support) ---

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        );
        let new_pos = range.start + new_text.len();
        self.selected_range = new_pos..new_pos;
        self.marked_range.take();
        self.cursor_last_moved = Instant::now();
        self.fire_change(window, cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        );
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| {
                let pos = range.start + new_text.len();
                pos..pos
            });
        self.fire_change(window, cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        // Determine which line this range start falls on
        let (line, col_start) = self.line_col_for_offset(range.start);
        let (_, col_end) = self.line_col_for_offset(range.end);
        let shaped = self.line_layouts.get(line)?.as_ref()?;
        let y = bounds.top() + self.line_height * line as f32;
        Some(Bounds::from_corners(
            point(bounds.left() + shaped.x_for_index(col_start), y),
            point(
                bounds.left() + shaped.x_for_index(col_end),
                y + self.line_height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds.as_ref()?;
        let y_offset = point.y - bounds.top();
        let line_idx = if y_offset < px(0.) {
            0
        } else {
            (y_offset / self.line_height) as usize
        };
        let lines: Vec<&str> = self.content.split('\n').collect();
        let line_idx = line_idx.min(lines.len().saturating_sub(1));
        let shaped = self.line_layouts.get(line_idx)?.as_ref()?;
        let x = point.x - bounds.left();
        let col = shaped.index_for_x(x)?;
        let utf8_offset = self.offset_for_line_col(line_idx, col);
        Some(self.offset_to_utf16(utf8_offset))
    }
}

// --- EditorLineElement: custom Element for each line ---

pub struct EditorLineElement {
    editor: Entity<Editor>,
    line_index: usize,
}

pub struct EditorLinePrepaint {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    cursor_visible: bool,
    is_cursor_line: bool,
    active_line_bg: Hsla,
}

impl IntoElement for EditorLineElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorLineElement {
    type RequestLayoutState = ();
    type PrepaintState = EditorLinePrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let editor = self.editor.read(cx);
        let lh = editor.line_height;
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = lh.into();
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
        // Extract everything we need from the editor, then drop the borrow
        let (line_text_owned, theme_text, selected_range, cursor_off, line_start, marked_range, lh, cursor_last_moved) = {
            let editor = self.editor.read(cx);
            let lines: Vec<&str> = editor.content.split('\n').collect();
            let lt = lines.get(self.line_index).copied().unwrap_or("").to_string();
            let ls = editor.line_start_offset(self.line_index);
            (
                lt,
                editor.theme.text,
                editor.selected_range.clone(),
                editor.cursor_offset(),
                ls,
                editor.marked_range.clone(),
                editor.line_height,
                editor.cursor_last_moved,
            )
        };

        // Cursor blink: visible for 500ms, hidden for 500ms
        let elapsed_ms = cursor_last_moved.elapsed().as_millis();
        let cursor_visible = (elapsed_ms % 1000) < 500;

        // Check if this line is the cursor line (for active line highlight)
        let line_end_for_cursor = line_start + line_text_owned.len();
        let is_cursor_line = cursor_off >= line_start && cursor_off <= line_end_for_cursor;
        let active_line_bg = hsla(0.0, 0.0, 1.0, 0.03);

        let ws = window.text_style();
        let is_header = line_text_owned.starts_with('#');
        let font_size = if is_header {
            ws.font_size.to_pixels(window.rem_size()) * 1.25
        } else {
            ws.font_size.to_pixels(window.rem_size())
        };

        let mut base_font = ws.font();
        if is_header {
            base_font.weight = FontWeight::BOLD;
        }

        let base_run = TextRun {
            len: 0,
            font: base_font,
            color: theme_text,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let display_text: SharedString = if line_text_owned.is_empty() {
            " ".into()
        } else {
            SharedString::from(line_text_owned.clone())
        };

        let runs = if line_text_owned.is_empty() {
            vec![TextRun {
                len: 1,
                ..base_run.clone()
            }]
        } else {
            let editor = self.editor.read(cx);
            let mut runs = editor.highlight_line(&line_text_owned, &base_run);
            let _ = editor;

            // Apply marked_range underline if on this line
            if let Some(marked) = &marked_range {
                let line_end = line_start + line_text_owned.len();
                let m_start = marked.start.max(line_start);
                let m_end = marked.end.min(line_end);
                if m_start < m_end {
                    let local_start = m_start - line_start;
                    let local_end = m_end - line_start;
                    let mut new_runs = Vec::new();
                    let mut pos = 0;
                    for run in &runs {
                        let run_end = pos + run.len;
                        let overlap_start = local_start.max(pos);
                        let overlap_end = local_end.min(run_end);
                        if overlap_start < overlap_end {
                            if pos < overlap_start {
                                new_runs.push(TextRun {
                                    len: overlap_start - pos,
                                    ..run.clone()
                                });
                            }
                            new_runs.push(TextRun {
                                len: overlap_end - overlap_start,
                                underline: Some(UnderlineStyle {
                                    color: Some(run.color),
                                    thickness: px(1.0),
                                    wavy: false,
                                }),
                                ..run.clone()
                            });
                            if overlap_end < run_end {
                                new_runs.push(TextRun {
                                    len: run_end - overlap_end,
                                    ..run.clone()
                                });
                            }
                        } else {
                            new_runs.push(run.clone());
                        }
                        pos = run_end;
                    }
                    runs = new_runs;
                }
            }
            runs
        };

        let shaped = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        // Compute selection and cursor for this line
        let line_end = line_start + line_text_owned.len();
        let cursor_color = {
            let editor = self.editor.read(cx);
            editor.theme.cursor
        };
        let selection_color = {
            let editor = self.editor.read(cx);
            editor.theme.selection
        };

        let (selection, cursor) = if selected_range.is_empty() {
            // Cursor
            let cursor_q = if cursor_off >= line_start && cursor_off <= line_end {
                let local_col = cursor_off - line_start;
                let x = shaped.x_for_index(local_col);
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + x, bounds.top()),
                        size(px(2.), lh),
                    ),
                    cursor_color,
                ))
            } else {
                None
            };
            (None, cursor_q)
        } else {
            // Selection
            let sel_start = selected_range.start.max(line_start);
            let sel_end = selected_range.end.min(line_end + 1);
            if sel_start <= line_end && sel_end > line_start {
                let local_start = sel_start.saturating_sub(line_start);
                let local_end = (sel_end - line_start).min(line_text_owned.len());
                let x_start = shaped.x_for_index(local_start);
                let x_end = if sel_end > line_end {
                    shaped.width() + px(4.)
                } else {
                    shaped.x_for_index(local_end)
                };
                (
                    Some(fill(
                        Bounds::from_corners(
                            point(bounds.left() + x_start, bounds.top()),
                            point(bounds.left() + x_end, bounds.bottom()),
                        ),
                        selection_color,
                    )),
                    None,
                )
            } else {
                (None, None)
            }
        };

        EditorLinePrepaint {
            line: Some(shaped),
            cursor,
            selection,
            cursor_visible,
            is_cursor_line,
            active_line_bg,
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
        // Read out data we need before mutable borrows
        let (focus_handle, editor_bounds, lh, is_focused) = {
            let editor = self.editor.read(cx);
            (
                editor.focus_handle.clone(),
                editor.last_bounds.unwrap_or(bounds),
                editor.line_height,
                editor.focus_handle.is_focused(window),
            )
        };

        // Register input handler on the first line element
        if self.line_index == 0 {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(editor_bounds, self.editor.clone()),
                cx,
            );
        }

        // Active line highlight
        if prepaint.is_cursor_line && is_focused {
            window.paint_quad(fill(bounds, prepaint.active_line_bg));
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            lh,
            TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if is_focused
            && prepaint.cursor_visible
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        // Store the shaped line layout back
        self.editor.update(cx, |editor, _| {
            while editor.line_layouts.len() <= self.line_index {
                editor.line_layouts.push(None);
            }
            editor.line_layouts[self.line_index] = Some(line);
        });
    }
}

// --- Render implementation ---

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let line_count = self.line_count();
        self.line_height = window.line_height();
        self.line_layouts.clear();

        let entity = cx.entity().clone();

        div()
            .flex()
            .flex_col()
            .key_context("Editor")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .size_full()
            .bg(self.theme.editor_background)
            .p(px(12.))
            .font_family(self.font_family.clone())
            .text_size(self.font_size)
            .text_color(self.theme.text)
            .children({
                let gutter_w = px(GUTTER_WIDTH);
                let gutter_pad = px(GUTTER_PADDING_RIGHT);
                let dimmed = self.theme.text_dimmed;
                let error_color = self.theme.error;
                let lh = self.line_height;
                let mut children: Vec<gpui::AnyElement> = Vec::new();
                for i in 0..line_count {
                    // Row: line number gutter + editor line
                    children.push(
                        div()
                            .flex()
                            .flex_row()
                            .w_full()
                            .child(
                                // Line number gutter
                                div()
                                    .w(gutter_w)
                                    .h(lh)
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .pr(gutter_pad)
                                    .text_size(px(12.))
                                    .text_color(dimmed)
                                    .child(format!("{}", i + 1)),
                            )
                            .child(
                                // The editor line itself
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(EditorLineElement {
                                        editor: entity.clone(),
                                        line_index: i,
                                    }),
                            )
                            .into_any_element(),
                    );
                    // Inlay diagnostic below error lines
                    if let Some(Some(diag)) = self.diagnostics.get(i) {
                        children.push(
                            div()
                                .w_full()
                                .pl(gutter_w + gutter_pad)
                                .py(px(2.))
                                .text_size(px(12.))
                                .text_color(error_color)
                                .child(diag.clone())
                                .into_any_element(),
                        );
                    }
                }
                children
            })
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

