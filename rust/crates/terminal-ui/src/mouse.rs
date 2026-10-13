//! Source offsets, never screen scraping: wrapping and chrome cannot enter a copy.
use crate::model::{Pane, View};
use std::ops::Range;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct MouseLayout {
    pub commands: Rect,
    pub panes: Vec<(Rect, Pane)>,
    pub tabs: Vec<(Rect, View)>,
    pub items: Vec<(Rect, usize)>,
}

#[derive(Clone)]
pub struct Glyph {
    pub style: Style,
    pub text: String,
    pub width: u16,
    pub source: Range<usize>,
}
pub struct Row {
    pub glyphs: Vec<Glyph>,
    pub metadata: Option<String>,
    pub boundary: usize,
}
#[derive(Default)]
pub struct Transcript {
    cached_entries: Vec<crate::presentation::Entry>,
    cached_width: u16,
    expanded: bool,
    paints: Vec<Vec<crate::highlight::Paint>>,
    pub layout_builds: usize,
    pub highlight_builds: usize,
    pub source: String,
    pub rows: Vec<Row>,
    pub entry_rows: Vec<usize>,
    pub area: Rect,
    pub scroll: usize,
    pub selection: Option<Range<usize>>,
    anchor: Option<Range<usize>>,
    origin: (u16, u16),
    pointer: Option<(u16, u16)>,
}
impl Transcript {
    #[cfg(test)]
    pub fn prepare(&mut self, entries: &[&crate::presentation::Entry], width: u16) {
        self.prepare_detail(entries, width, false);
    }
    pub fn prepare_detail(
        &mut self,
        entries: &[&crate::presentation::Entry],
        width: u16,
        expanded: bool,
    ) {
        let changed =
            self.expanded != expanded || !self.cached_entries.iter().eq(entries.iter().copied());
        self.expanded = expanded;
        if !changed && self.cached_width == width && !self.rows.is_empty() {
            return;
        }
        if changed {
            self.highlight_builds += 1;
            self.paints = entries
                .iter()
                .map(|entry| crate::highlight::markdown(&entry.display_text(expanded)))
                .collect();
            self.cached_entries = entries.iter().map(|entry| (*entry).clone()).collect();
            self.clear();
        }
        self.cached_width = width;
        self.layout_builds += 1;
        self.rows.clear();
        self.entry_rows.clear();
        self.source.clear();
        self.metadata(String::new());
        self.metadata("Drag text to select · Ctrl-C copy Markdown".into());
        self.metadata(String::new());
        for (index, entry) in entries.iter().enumerate() {
            self.entry_rows.push(self.rows.len());
            if index > 0 {
                self.source.push_str("\n\n");
            }
            self.metadata(
                if let crate::presentation::EntryKind::System { state, context, .. } = &entry.kind {
                    format!("SYSTEM · {} · context {}", state.label(), context)
                } else {
                    match entry.recipient {
                        Some(role) => format!(
                            "{} · SIMULATED to {} · {}",
                            entry.speaker, role, entry.subject
                        ),
                        None => format!("{} · {}", entry.speaker, entry.subject),
                    }
                },
            );
            let offset = self.source.len();
            let first_row = self.rows.len();
            self.append(&entry.display_text(expanded), width);
            let mut glyphs: Vec<_> = self.rows[first_row..]
                .iter_mut()
                .flat_map(|r| r.glyphs.iter_mut())
                .collect();
            if let crate::presentation::EntryKind::System { state, .. } = entry.kind {
                let color = match state {
                    crate::presentation::SystemState::Progress
                    | crate::presentation::SystemState::Recorded => crate::theme::LAVENDER,
                    crate::presentation::SystemState::Ready
                    | crate::presentation::SystemState::Accepted => crate::theme::ACCENT,
                    crate::presentation::SystemState::Uncertain
                    | crate::presentation::SystemState::Rejected => crate::theme::AMBER,
                };
                for glyph in &mut glyphs {
                    glyph.style = Style::default().fg(color).bg(crate::theme::CODE_BG);
                }
            }
            for paint in &self.paints[index] {
                let start = glyphs.partition_point(|g| g.source.end <= paint.range.start + offset);
                let end = glyphs.partition_point(|g| g.source.start < paint.range.end + offset);
                for glyph in &mut glyphs[start..end] {
                    glyph.style = glyph.style.patch(paint.style);
                }
            }
            self.metadata(String::new());
        }
        self.metadata(String::new());
    }
    pub fn clear(&mut self) {
        self.selection = None;
        self.stop_drag();
    }
    pub fn stop_drag(&mut self) {
        self.anchor = None;
        self.pointer = None;
    }
    /// Called by the UI timer even when a stationary pointer emits no events.
    pub fn auto_scroll(&mut self) -> bool {
        let Some((x, y)) = self.pointer else {
            return false;
        };
        if self.anchor.is_none()
            || self.area.is_empty()
            || x < self.area.x
            || x >= self.area.right()
        {
            return false;
        }
        let max = self
            .rows
            .len()
            .saturating_sub(self.area.height as usize)
            .min(u16::MAX as usize);
        let next = if y <= self.area.y {
            self.scroll
                .saturating_sub(1 + usize::from(self.area.y - y).min(8) / 2)
        } else if y >= self.area.bottom() - 1 {
            self.scroll
                .saturating_add(1 + usize::from(y - (self.area.bottom() - 1)).min(8) / 2)
                .min(max)
        } else {
            return false;
        };
        if next == self.scroll {
            return false;
        }
        self.scroll = next;
        self.drag(x, y);
        true
    }
    pub fn metadata(&mut self, text: String) {
        self.rows.push(Row {
            glyphs: vec![],
            metadata: Some(text),
            boundary: self.source.len(),
        });
    }
    pub fn append(&mut self, text: &str, width: u16) {
        let offset = self.source.len();
        self.source.push_str(text);
        let mut row = Row {
            glyphs: vec![],
            metadata: None,
            boundary: offset,
        };
        let mut used = 0;
        let word_widths: std::collections::HashMap<_, _> = text
            .split_word_bound_indices()
            .filter(|(_, word)| !word.chars().any(char::is_whitespace))
            .map(|(index, word)| (index, UnicodeWidthStr::width(word)))
            .collect();
        for (index, g) in text.grapheme_indices(true) {
            let start = offset + index;
            if g == "\n" || g == "\r\n" {
                self.rows.push(row);
                row = Row {
                    glyphs: vec![],
                    metadata: None,
                    boundary: start + g.len(),
                };
                used = 0;
                continue;
            }
            // Preserve source controls but display inert glyphs; tabs have visible width.
            let display = if g == "\t" {
                "    ".into()
            } else if g.chars().any(char::is_control) || UnicodeWidthStr::width(g) == 0 {
                "�".into()
            } else {
                g.to_owned()
            };
            let cells = UnicodeWidthStr::width(display.as_str())
                .max(1)
                .min(width.max(1) as usize) as u16;
            let wrap_word = word_widths
                .get(&index)
                .is_some_and(|w| *w <= width as usize && used as usize + w > width as usize);
            if used + cells > width.max(1) || wrap_word {
                self.rows.push(row);
                row = Row {
                    glyphs: vec![],
                    metadata: None,
                    boundary: start,
                };
                used = 0;
            }
            row.glyphs.push(Glyph {
                style: Style::default(),
                text: display,
                width: cells,
                source: start..start + g.len(),
            });
            used += cells;
        }
        self.rows.push(row);
    }
    pub fn hit(&self, x: u16, y: u16) -> Option<Range<usize>> {
        if self.area.is_empty() {
            return None;
        }
        let y = y.clamp(self.area.y, self.area.bottom() - 1);
        let row = self
            .rows
            .get(self.scroll + usize::from(y - self.area.y))
            .or_else(|| self.rows.last())?;
        let mut column = self.area.x;
        for glyph in &row.glyphs {
            if x < column.saturating_add(glyph.width) {
                return Some(glyph.source.clone());
            }
            column = column.saturating_add(glyph.width);
        }
        let end = row.glyphs.last().map_or(row.boundary, |g| g.source.end);
        Some(end..end)
    }
    pub fn down(&mut self, x: u16, y: u16) {
        self.clear();
        self.origin = (x, y);
        self.anchor = self.hit(x, y);
    }
    pub fn drag(&mut self, x: u16, y: u16) {
        if self.anchor.is_some() {
            self.pointer = Some((x, y));
        }
        if (x, y) == self.origin && self.selection.is_none() {
            return;
        }
        if let (Some(anchor), Some(end)) = (&self.anchor, self.hit(x, y)) {
            self.selection = Some(anchor.start.min(end.start)..anchor.end.max(end.end));
        }
    }
    pub fn up(&mut self, x: u16, y: u16) {
        self.drag(x, y);
        self.stop_drag();
    }
    pub fn selected(&self) -> Option<String> {
        self.selection
            .as_ref()
            .filter(|r| !r.is_empty())
            .and_then(|r| self.source.get(r.clone()))
            .map(str::to_owned)
    }
}
