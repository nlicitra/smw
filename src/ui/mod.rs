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

mod config;
mod search;

pub enum AppAction {
    Exit,
    SwitchState,
}

pub trait TUIState<B: Backend> {
    fn title(&self) -> &'static str;
    fn render(&mut self, f: &mut Frame<B>, area: Rect);
    fn handle_key_event(&mut self, event: KeyEvent) -> Option<AppAction> {
        None
    }
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

fn app_loop<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut config_state = config::ConfigState::new("Config");
    let mut search_state = search::SearchState::new("Search");
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

fn handle_key_event<B: Backend>(app: &mut App<B>, event: KeyEvent) -> Option<AppAction> {
    match event.code {
        KeyCode::Esc => return Some(AppAction::Exit),
        KeyCode::Tab => app.select_next(),
        _ => {}
    }
    None
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

#[derive(Default)]
struct App<B: Backend> {
    index: usize,
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
