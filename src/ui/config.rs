use crate::ui::{AppAction, TUIState};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState, Tabs,
    },
    Frame, Terminal,
};

use unicode_width::UnicodeWidthStr;
#[derive(Default)]
pub struct ConfigState {
    title: &'static str,
    input: String,
}

impl ConfigState {
    pub fn new(title: &'static str) -> Self {
        Self {
            title,
            ..Self::default()
        }
    }
}

impl<B: Backend> TUIState<B> for ConfigState {
    fn title(&self) -> &'static str {
        self.title
    }
    fn render(&mut self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);

        let spans = Spans::from(vec![
            Span::raw("Press "),
            Span::styled("ENTER", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to execute your search"),
        ]);
        let mut text = Text::from(spans);
        text.patch_style(Style::default().fg(Color::Yellow));
        let instructions = Paragraph::new(text);
        f.render_widget(instructions, chunks[0]);

        let input_text = Text::styled(&self.input, Style::default().fg(Color::White));
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[1]);
        f.set_cursor(
            // Put cursor past the end of the input text
            chunks[1].x + self.input.width() as u16 + 1,
            // Move one line down, from the border to the input line
            chunks[1].y + 1,
        );
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Option<AppAction> {
        match event.code {
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Esc => return Some(AppAction::Exit),
            KeyCode::Tab => return Some(AppAction::SwitchState),
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
        None
    }
}
