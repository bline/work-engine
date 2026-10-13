//! Paste payloads are owned objects. The editing buffer holds one opaque scalar
//! per object, so cursor movement, selection, deletion, and undo stay atomic.
//! Visible labels are only projections: typing a lookalike never expands data.
use ratatui_textarea::{CursorMove, Input, Key, TextArea, WrapMode};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};
use tuirealm::ratatui::{style::Style, text::Line};

struct Atom {
    text: String,
    label: String,
    folded: bool,
}
pub struct Draft {
    editor: TextArea<'static>,
    atoms: HashMap<char, Atom>,
    next: u32,
    paste_number: usize,
    display: TextArea<'static>,
    display_ready: bool,
}
impl Deref for Draft {
    type Target = TextArea<'static>;
    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}
impl DerefMut for Draft {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.display_ready = false;
        &mut self.editor
    }
}
impl Draft {
    pub fn new() -> Self {
        Self {
            editor: TextArea::default(),
            atoms: HashMap::new(),
            next: 0xf0000,
            paste_number: 0,
            display: TextArea::default(),
            display_ready: false,
        }
    }
    pub fn reserved(c: char) -> bool {
        (0xf0000..=0x10ffff).contains(&(c as u32))
    }
    fn allocate(&mut self, text: String, label: String, folded: bool) -> char {
        let c = char::from_u32(self.next).expect("capacity checked before inserting");
        self.next += 1;
        self.atoms.insert(
            c,
            Atom {
                text,
                label,
                folded,
            },
        );
        c
    }
    pub fn insert_text(&mut self, text: &str, fold: bool) -> Result<(), &'static str> {
        let needed = if fold {
            1
        } else {
            text.chars().filter(|&c| Self::reserved(c)).count()
        };
        if self.next as usize + needed > 0x110000 {
            return Err("Draft object capacity reached; paste was not inserted.");
        }
        let encoded = if fold {
            self.paste_number += 1;
            let label = format!(
                "[Paste #{} · {} lines · {} chars]",
                self.paste_number,
                text.split('\n').count(),
                text.chars().count()
            );
            self.allocate(text.into(), label, true).to_string()
        } else {
            text.chars()
                .map(|c| {
                    if Self::reserved(c) {
                        self.allocate(c.to_string(), c.to_string(), false)
                    } else {
                        c
                    }
                })
                .collect()
        };
        self.editor.insert_str(encoded);
        self.display_ready = false;
        Ok(())
    }
    pub fn paste(&mut self, text: &str) -> Result<bool, &'static str> {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let fold = text.split('\n').count() >= 6 || text.chars().count() >= 1000;
        self.insert_text(&text, fold)?;
        Ok(fold)
    }
    pub fn text(&self) -> String {
        self.editor
            .lines()
            .iter()
            .map(|line| {
                line.chars()
                    .map(|c| {
                        self.atoms
                            .get(&c)
                            .map_or_else(|| c.to_string(), |a| a.text.clone())
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn display_lines(&self) -> Vec<String> {
        self.editor
            .lines()
            .iter()
            .map(|line| {
                line.chars()
                    .map(|c| {
                        self.atoms
                            .get(&c)
                            .map_or_else(|| c.to_string(), |a| a.label.clone())
                    })
                    .collect()
            })
            .collect()
    }
    pub fn folded_count(&self) -> usize {
        self.editor
            .lines()
            .iter()
            .flat_map(|l| l.chars())
            .filter(|c| self.atoms.get(c).is_some_and(|a| a.folded))
            .count()
    }
    fn display_position(&self, row: usize, col: usize) -> (u16, u16) {
        let column = self.editor.lines()[row]
            .chars()
            .take(col)
            .map(|c| self.atoms.get(&c).map_or(1, |a| a.label.chars().count()))
            .sum::<usize>();
        (
            row.min(u16::MAX as usize) as u16,
            column.min(u16::MAX as usize) as u16,
        )
    }
    fn editor_position(&self, row: usize, col: usize) -> (u16, u16) {
        let mut offset = 0;
        let mut column = 0;
        for c in self.editor.lines()[row].chars() {
            let width = self.atoms.get(&c).map_or(1, |a| a.label.chars().count());
            if offset + width > col {
                if col - offset > width / 2 {
                    column += 1;
                }
                break;
            }
            offset += width;
            column += 1;
        }
        (
            row.min(u16::MAX as usize) as u16,
            column.min(u16::MAX as usize) as u16,
        )
    }
    pub fn input(&mut self, input: Input) -> bool {
        if let Key::Char(c) = input.key
            && !input.ctrl
            && !input.alt
            && Self::reserved(c)
        {
            return self.insert_text(&c.to_string(), false).is_ok();
        }
        if self.display_ready
            && matches!(input.key, Key::Up | Key::Down | Key::PageUp | Key::PageDown)
        {
            if matches!(input.key, Key::PageUp | Key::PageDown) {
                // These scroll the editor viewport without moving the insertion point.
                return self.display.input(input);
            }
            let shift = input.shift;
            self.display.input(input);
            let pos = self.display.cursor();
            let (r, c) = self.editor_position(pos.0, pos.1);
            if shift {
                if self.editor.selection_range().is_none() {
                    self.editor.start_selection();
                }
            } else {
                self.editor.cancel_selection();
            }
            self.editor.move_cursor(CursorMove::Jump(r, c));
            self.display_ready = false;
            return false;
        }
        self.display_ready = false;
        self.editor.input(input)
    }
    pub fn view(&mut self) -> &mut TextArea<'static> {
        if self.display_ready {
            return &mut self.display;
        }
        self.display = TextArea::new(self.display_lines());
        self.display.set_wrap_mode(WrapMode::WordOrGlyph);
        self.display.set_cursor_line_style(Style::default());
        let cursor = self.editor.cursor();
        if let Some((a, b)) = self.editor.selection_range() {
            let anchor = if (cursor.0, cursor.1) == a { b } else { a };
            let (r, c) = self.display_position(anchor.0, anchor.1);
            self.display.move_cursor(CursorMove::Jump(r, c));
            self.display.start_selection();
        }
        let (r, c) = self.display_position(cursor.0, cursor.1);
        self.display.move_cursor(CursorMove::Jump(r, c));
        self.display_ready = true;
        &mut self.display
    }
    pub fn height(&self, width: u16, terminal_height: u16) -> u16 {
        // Approximate soft-wrap rows; the editor owns exact wrap/viewport geometry.
        let width = width.saturating_sub(2).max(1) as usize;
        let rows = self
            .display_lines()
            .iter()
            .map(|line| {
                Line::raw(line.replace('\t', "    "))
                    .width()
                    .max(1)
                    .div_ceil(width)
            })
            .sum::<usize>();
        let cap = (terminal_height / 3).clamp(5, 12);
        rows.saturating_add(2).clamp(5, cap as usize) as u16
    }
}
