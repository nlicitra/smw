use crate::ui::{AppAction, TUIState};
use crate::utils::search_smwcentral;
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
pub struct SearchState {
    title: &'static str,
    input: String,
    results: Vec<crate::utils::RomHackDetails>,
    table_state: TableState,
}

impl SearchState {
    pub fn new(title: &'static str) -> Self {
        println!("neat");
        Self {
            title,
            ..Self::default()
        }
    }

    fn search(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.results.clear();
        self.results = search_smwcentral(&self.input)?.unwrap();
        Ok(())
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

impl<B: Backend> TUIState<B> for SearchState {
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
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
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

        let results_block = Block::default()
            .style(Style::default().fg(Color::White))
            .borders(Borders::ALL)
            .title("Search Results");
        if self.results.is_empty() {
            let body = Paragraph::new(Text::raw("No results"))
                .alignment(Alignment::Center)
                .block(results_block);
            f.render_widget(body, chunks[2]);
            return;
        }

        let normal_style = Style::default();
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);

        let header_cells = [
            "Name",
            "Date",
            "Demo",
            "Featured",
            "Length",
            "Type",
            "Authors",
            "Rating",
            "Size",
            "Downloads",
        ]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = self.results.iter().map(|result| {
            let ordered_list = result.ordered_fields();
            let cells = ordered_list.iter().map(|c| Cell::from(c.clone()));
            Row::new(cells).bottom_margin(1)
        });
        let table = Table::new(rows)
            .header(header)
            .block(results_block)
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(5),
                Constraint::Percentage(5),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(5),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
            ]);

        f.render_stateful_widget(table, chunks[2], &mut self.table_state);
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
            KeyCode::Enter => {
                self.search();
                if !self.results.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            KeyCode::Down => {
                self.next();
            }
            KeyCode::Up => {
                self.previous();
            }
            _ => {}
        }
        None
    }
}
