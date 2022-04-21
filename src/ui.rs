use crate::utils::search_smwcentral;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState, Tabs,
    },
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

// use crate::errors::*;

trait TUIState {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>);
    fn handle_key_event(&mut self, event: KeyEvent) -> Option<AppAction> {
        None
    }
}

#[derive(Default)]
struct SearchState {
    input: String,
    results: Vec<crate::utils::RomHackDetails>,
    table_state: TableState,
}

impl SearchState {
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

impl TUIState for SearchState {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
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
            .split(f.size());

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
        if self.results.len() == 0 {
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

enum AppAction {
    Exit,
}

fn init_input_thread(tick_rate: u64) -> Receiver<KeyEvent> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(key).expect("can send events");
                }
            }
        }
    });
    rx
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = SearchState::default();
    let rx = init_input_thread(200);

    app_loop(&mut terminal, state, &rx)?;

    // loop is over, restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor();

    Ok(())
}

fn app_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut state: impl TUIState,
    rx: &Receiver<KeyEvent>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| state.render(f));

        if let Ok(event) = rx.try_recv() {
            if let Some(action) = state.handle_key_event(event) {
                match action {
                    AppAction::Exit => break,
                    _ => {}
                }
            };
        }
    }
    Ok(())
}
