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

// use crate::errors::*;

trait TUIState<B: Backend> {
    fn title(&self) -> &'static str;
    fn render(&mut self, f: &mut Frame<B>, area: Rect);
    fn handle_key_event(&mut self, event: KeyEvent) -> Option<AppAction> {
        None
    }
}

#[derive(Default)]
struct SearchState {
    title: &'static str,
    input: String,
    results: Vec<crate::utils::RomHackDetails>,
    table_state: TableState,
}

impl SearchState {
    fn new(title: &'static str) -> Self {
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

#[derive(Default)]
struct ConfigState {
    title: &'static str,
    input: String,
}

impl ConfigState {
    fn new(title: &'static str) -> Self {
        let mut state = Self::default();
        state.title = title;
        state
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

enum AppAction {
    Exit,
    SwitchState,
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

    app_loop(&mut terminal)?;

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

fn handle_key_event<B: Backend>(app: &mut App<B>, event: KeyEvent) -> Option<AppAction> {
    match event.code {
        KeyCode::Esc => return Some(AppAction::Exit),
        KeyCode::Tab => app.select_next(),
        _ => {}
    }
    None
}

// enum AppState {
//     Config(ConfigState),
//     Search(SearchState),
// }

// impl AppState {
//     fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
//         match self {
//             AppState::Config(ref mut x) => x.render(f),
//             AppState::Search(ref mut x) => x.render(f),
//         }
//     }

//     fn handle_key_event(&mut self, event: KeyEvent) -> Option<AppAction> {
//         match self {
//             AppState::Config(ref mut x) => x.handle_key_event(event),
//             AppState::Search(ref mut x) => x.handle_key_event(event),
//         }
//     }
// }

// fn test() {
//     let mut config_state = ConfigState::default();
//     let mut search_state = SearchState::default();
//     let states: Vec<&dyn TUIState> = vec![&config_state, &search_state];
// }

#[derive(Default)]
struct App<B: Backend> {
    index: usize,
    // section_titles: Vec<&'a str>,
    pages: Vec<Box<dyn TUIState<B>>>,
}

impl<B: Backend> App<B> {
    pub fn navigation(&self) -> Tabs {
        let titles = self
            .pages
            .iter()
            .map(|s| {
                let (first, rest) = s.title().split_at(1);
                Spans::from(vec![
                    Span::styled(first, Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(rest, Style::default()),
                ])
            })
            .collect();
        Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().add_modifier(Modifier::BOLD))
                    .title("Navigation"),
            )
            .select(self.index)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White), // .bg(Color::Black),
            )
    }

    pub fn select_next(&mut self) {
        let length = self.pages.len();
        if (self.index < length - 1) {
            self.index += 1;
        } else if (self.index == length - 1) {
            self.index = 0;
        }
    }

    pub fn render(&mut self, frame: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            // .margin(2)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(frame.size());
        let navigation = self.navigation();
        frame.render_widget(navigation, chunks[0]);
        self.pages[self.index].render(frame, chunks[1]);
    }
}

fn system_layout<B: Backend>(frame: &mut Frame<B>, app: &App<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        // .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.size());
    let navigation = app.navigation();
    frame.render_widget(navigation, chunks[0]);
}

fn app_loop<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut config_state = ConfigState::new("Config");
    let mut search_state = SearchState::new("Search");
    // let mut state = &mut config_state;
    let mut app = App {
        index: 0,
        // section_titles: vec!["Library", "Search", "Config"],
        pages: vec![Box::new(config_state), Box::new(search_state)],
    };
    let rx = init_input_thread(200);
    loop {
        // terminal.draw(|f| state.render(f));
        terminal.draw(|f| app.render(f));

        if let Ok(event) = rx.try_recv() {
            if let Some(action) = handle_key_event(&mut app, event) {
                match action {
                    AppAction::Exit => break,
                    // AppAction::SwitchState => state = &mut search_state,
                    _ => {}
                }
            };
        }
    }
    Ok(())
}
