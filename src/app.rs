use std::{
    io::{self, Stdout},
    sync::Arc,
};

use crossterm::event::{KeyEvent, KeyModifiers};
use futures::lock::Mutex;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::KeyCode,
    layout::{Constraint, Layout},
    symbols,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tracing::debug;

use crate::{config::Config, event::EventHandler, log::LogEvent};

pub struct App<T>
where
    T: ratatui::backend::Backend,
{
    running: bool,
    debug: bool,
    terminal: Arc<Mutex<Terminal<T>>>,
    events: EventHandler,
    logs: Vec<LogEvent>,
    #[allow(dead_code)]
    config: Config,
}

impl App<CrosstermBackend<Stdout>> {
    pub fn new(events: EventHandler, config: Config) -> Self {
        let terminal = Arc::new(Mutex::new(ratatui::init()));
        let logs = vec![];

        Self {
            running: false,
            debug: false,
            terminal,
            events,
            logs,
            config,
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
        let (main_area, borders) = if self.debug {
            let split = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(frame.area());
            let log_area = split[1];
            let height = log_area.height;
            let len = self.logs.len();
            let mut state = ListState::default().with_offset(len.saturating_sub(height.into()));
            frame.render_stateful_widget(self.draw_logs(), log_area, &mut state);
            (split[0], (Borders::TOP | Borders::LEFT | Borders::RIGHT))
        } else {
            (frame.area(), Borders::ALL)
        };

        let split =
            Layout::horizontal([Constraint::Min(20), Constraint::Percentage(100)]).split(main_area);

        let mut config_state = ListState::default();

        let left_block = Block::new()
            .borders(borders ^ Borders::RIGHT)
            .title("Configuration");
        frame.render_stateful_widget(
            List::new(vec!["Test config"]).block(left_block),
            split[0],
            &mut config_state,
        );

        let (control_server_area, control_border_set) = if let Some(vip) = &self.config.servers.vip
        {
            let split =
                Layout::vertical([Constraint::Min(2), Constraint::Percentage(100)]).split(split[1]);

            let border_set = symbols::border::Set {
                top_left: symbols::line::NORMAL.horizontal_down,
                ..symbols::border::PLAIN
            };

            let block = Block::new()
                .title("VIP")
                .borders(Borders::ALL ^ Borders::BOTTOM)
                .border_set(border_set);
            frame.render_widget(Paragraph::new(format!("{vip}")).block(block), split[0]);

            (
                split[1],
                symbols::border::Set {
                    top_left: symbols::line::NORMAL.vertical_right,
                    bottom_left: symbols::line::NORMAL.horizontal_up,
                    top_right: symbols::line::NORMAL.vertical_left,
                    ..symbols::border::PLAIN
                },
            )
        } else {
            (
                split[1],
                symbols::border::Set {
                    top_left: symbols::line::NORMAL.horizontal_down,
                    bottom_left: symbols::line::NORMAL.horizontal_up,
                    ..symbols::border::PLAIN
                },
            )
        };

        let (control_server_area, borders) = if self.config.servers.worker.is_empty() {
            (control_server_area, borders)
        } else {
            let split = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(control_server_area);

            let worker: Vec<&str> = self
                .config
                .servers
                .worker
                .iter()
                .map(|s| s.as_ref())
                .collect();

            let border_set = symbols::border::Set {
                top_left: symbols::line::NORMAL.vertical_right,
                bottom_left: symbols::line::NORMAL.horizontal_up,
                top_right: symbols::line::NORMAL.vertical_left,
                ..symbols::border::PLAIN
            };

            let block = Block::new()
                .title("Worker Nodes")
                .borders(borders)
                .border_set(border_set);

            let mut worker_state = ListState::default();
            frame.render_stateful_widget(
                List::new(worker).block(block),
                split[1],
                &mut worker_state,
            );
            (split[0], (Borders::ALL ^ Borders::BOTTOM))
        };

        let control: Vec<&str> = if self.config.servers.control.is_empty() {
            vec!["No control plane nodes configured"]
        } else {
            self.config
                .servers
                .control
                .iter()
                .map(|s| s.as_ref())
                .collect()
        };

        let block = Block::new()
            .title("Control Nodes")
            .borders(borders)
            .border_set(control_border_set);

        let mut control_state = ListState::default();
        frame.render_stateful_widget(
            List::new(control).block(block),
            control_server_area,
            &mut control_state,
        );
    }

    fn draw_logs(&self) -> List<'_> {
        let logs = self
            .logs
            .iter()
            .map(|s| s.into())
            .collect::<Vec<ListItem>>();

        // Logs will be in the bottom half of the window
        let collapsed_top_set = symbols::border::Set {
            top_left: symbols::line::NORMAL.vertical_right,
            top_right: symbols::line::NORMAL.vertical_left,
            ..symbols::border::PLAIN
        };

        List::new(logs).block(
            Block::new()
                .border_set(collapsed_top_set)
                .borders(Borders::ALL)
                .title("Tracing Logs"),
        )
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
