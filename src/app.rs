use std::{
    io::{self, Stdout}, sync::Arc
};

use crossterm::event::{KeyEvent, KeyModifiers};
use futures::lock::Mutex;
use ratatui::{
    backend::CrosstermBackend, crossterm::event::KeyCode, layout::{Constraint, Layout}, widgets::{Block, List, ListItem, ListState}, Frame, Terminal
};
use tracing::debug;

use crate::{event::EventHandler, log::LogEvent};

pub struct App<T>
where
    T: ratatui::backend::Backend,
{
    running: bool,
    debug: bool,
    terminal: Arc<Mutex<Terminal<T>>>,
    events: EventHandler,
    logs: Vec<LogEvent>,
}

impl App<CrosstermBackend<Stdout>> {
    pub fn new(events: EventHandler) -> Self {
        let terminal = Arc::new(Mutex::new(ratatui::init()));
        let logs = vec![];

        Self {
            running: false,
            debug: false,
            terminal,
            events,
            logs,
        }
    }

    pub async fn run(mut self) -> io::Result<()> {
        self.running = true;

        let terminal = self.terminal.clone();
        let mut terminal = terminal.lock().await;

        terminal.clear()?;
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await;
        }
        
        ratatui::restore();
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let main_area = if self.debug {
            let split = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(frame.area());
            let log_area = split[1];
            let height = log_area.height;
            let len = self.logs.len();       
            let mut state = ListState::default().with_offset(len.saturating_sub(height.into()));
            frame.render_stateful_widget(self.draw_logs(), log_area, &mut state);
            split[0]
        } else {
            frame.area()
        };
        
        let split = Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)]).split(main_area);

        let mut config_state = ListState::default();
        frame.render_stateful_widget(List::new(vec!["Test config"]).block(Block::bordered().title("Configuration")), split[0], &mut config_state);
        
        let mut server_state = ListState::default();
        frame.render_stateful_widget(List::new(vec!["Test server"]).block(Block::bordered().title("Configuration")), split[1], &mut server_state);
    }

    fn draw_logs(&self) -> List<'_> {
        let logs = self.logs
            .iter()
            .map(|s| s.into())
            .collect::<Vec<ListItem>>();
        
        List::new(logs).block(Block::bordered().title("Tracing Logs"))
    }


    async fn handle_events(&mut self) {
        match self.events.next().await {
            crate::event::Event::Tick => {}
            crate::event::Event::Key(key) => self.handle_key_events(key),
            crate::event::Event::Mouse(_) => {}
            crate::event::Event::Resize(_, _) => {}
            crate::event::Event::Log(log) => self.logs.push(log),
            crate::event::Event::Invalid => {}
        }
    }

    pub fn handle_key_events(&mut self, key_event: KeyEvent) {
        debug!(?key_event);
        match key_event.code {
            // Exit application on `ESC` or `q`
            KeyCode::Esc | KeyCode::Char('q') => {
                self.running = false;
            }
            // Exit application on `Ctrl-C`
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if key_event.modifiers == KeyModifiers::CONTROL {
                    self.running = false;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.debug = !self.debug;
            }
            // Other handlers you could add here.
            _ => {}
        }
    }
}
