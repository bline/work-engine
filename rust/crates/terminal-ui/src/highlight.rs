//! Color annotations over immutable Markdown source; no rendered-text substitution.
use crate::theme::*;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::{ops::Range, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color as SyntaxColor, StyleModifier, Theme, ThemeItem},
    parsing::SyntaxSet,
};
use tuirealm::ratatui::style::{Color, Modifier, Style};

pub struct Paint {
    pub range: Range<usize>,
    pub style: Style,
}
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}
fn color(c: Color) -> SyntaxColor {
    let Color::Rgb(r, g, b) = c else {
        unreachable!()
    };
    SyntaxColor { r, g, b, a: 255 }
}
fn theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let mut theme = Theme::default();
        theme.settings.foreground = Some(color(INK));
        for (scope, fg) in [
            ("comment", MUTED),
            ("string", ACCENT),
            ("keyword, storage", LAVENDER),
            ("constant", AMBER),
            ("entity.name.function, support.function", BLUE),
            ("entity.name.type, support.type", AMBER),
            ("markup.inserted", ACCENT),
            ("markup.deleted, invalid", ROSE),
        ] {
            theme.scopes.push(ThemeItem {
                scope: scope.parse().expect("static scope selector"),
                style: StyleModifier {
                    foreground: Some(color(fg)),
                    ..Default::default()
                },
            });
        }
        theme
    })
}
pub fn language(
    hint: &str,
    first_line: &str,
) -> Option<&'static syntect::parsing::SyntaxReference> {
    let hint = hint.split_whitespace().next().unwrap_or("");
    let token = match hint {
        "bash" | "sh" | "shell" => "sh",
        "js" => "javascript",
        "py" => "python",
        other => other,
    };
    if token.is_empty() {
        syntaxes().find_syntax_by_first_line(first_line)
    } else {
        syntaxes().find_syntax_by_token(token)
    }
}

pub fn markdown(source: &str) -> Vec<Paint> {
    let mut paints = Vec::new();
    let mut code: Option<(String, Option<HighlightLines<'static>>, bool)> = None;
    for (event, range) in Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .into_offset_iter()
    {
        let style = match &event {
            Event::Start(Tag::Heading { .. }) => {
                Some(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            }
            Event::Start(Tag::Strong) => {
                Some(Style::default().fg(INK).add_modifier(Modifier::BOLD))
            }
            Event::Start(Tag::Emphasis) => {
                Some(Style::default().fg(LAVENDER).add_modifier(Modifier::ITALIC))
            }
            Event::Start(Tag::Strikethrough) => Some(
                Style::default()
                    .fg(MUTED)
                    .add_modifier(Modifier::CROSSED_OUT),
            ),
            Event::Start(Tag::Link { .. }) => {
                Some(Style::default().fg(BLUE).add_modifier(Modifier::UNDERLINED))
            }
            Event::Start(Tag::BlockQuote(_)) => Some(Style::default().fg(QUIET)),
            Event::Code(_) => Some(Style::default().fg(AMBER).bg(CODE_BG)),
            Event::TaskListMarker(_) => Some(Style::default().fg(ACCENT)),
            Event::Start(Tag::CodeBlock(kind)) => {
                let hint = match kind {
                    CodeBlockKind::Fenced(hint) => hint.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((hint, None, false));
                Some(Style::default().fg(MUTED).bg(CODE_BG))
            }
            _ => None,
        };
        if let Some(style) = style {
            paints.push(Paint {
                range: range.clone(),
                style,
            });
        }
        if let Event::Text(_) = event
            && let Some((hint, highlighter, detected)) = &mut code
        {
            // Parse the original source slice, never parser-normalized text.
            let raw = &source[range.clone()];
            if !*detected {
                *detected = true;
                *highlighter = language(hint, raw.lines().next().unwrap_or(""))
                    .map(|s| HighlightLines::new(s, theme()));
            }
            paints.push(Paint {
                range: range.clone(),
                style: Style::default().fg(INK).bg(CODE_BG),
            });
            if let Some(highlighter) = highlighter {
                let mut offset = range.start;
                for line in raw.split_inclusive('\n') {
                    if let Ok(tokens) = highlighter.highlight_line(line, syntaxes()) {
                        for (token, text) in tokens {
                            let c = token.foreground;
                            paints.push(Paint {
                                range: offset..offset + text.len(),
                                style: Style::default().fg(Color::Rgb(c.r, c.g, c.b)),
                            });
                            offset += text.len();
                        }
                    } else {
                        offset += line.len();
                    }
                }
            }
        }
        if matches!(event, Event::End(TagEnd::CodeBlock)) {
            code = None;
        }
    }
    paints
}
