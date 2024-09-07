use std::{
    io::{self, Stdout},
    sync::Arc,
};

use crossterm::event::{KeyEvent, KeyModifiers};
use futures::lock::Mutex;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::KeyCode,
    widgets::{Block, Paragraph},
    Frame, Terminal,
};
use tracing::debug;

use crate::{event::EventHandler, log::LogEvent};

pub struct App<T>
where
    T: ratatui::backend::Backend,
{
    running: bool,
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
        let height = frame.area().height;

        let logs = self
            .logs
            .iter()
            .rev()
            .take(height.into())
            .map(|s| format!("{s}"))
            .rev()
            .collect::<Box<[String]>>();
        let paragraph = Paragraph::new(logs.join("\n"))
            .block(Block::bordered().title("Tracing example. Press 'q' to quit."));
        frame.render_widget(paragraph, frame.area());
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
            // Other handlers you could add here.
            _ => {}
        }
    }
}
