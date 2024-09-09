use std::time::Duration;

use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::{KeyEvent, MouseEvent};

use crate::log::LogEvent;

pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Log(LogEvent),
    Invalid,
}

pub struct EventHandler {
    tx: flume::Sender<Event>,
    rx: flume::Receiver<Event>,
    //handler: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = flume::unbounded();

        let _tx = tx.clone();
        let _handler = tokio::spawn(async move {
            let tx = _tx;
            let mut reader = crossterm::event::EventStream::new();
            let mut tick = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        tx.send(Event::Tick).unwrap() // NOTE: Arguably I don't really need the
                                                      // ticks
                    },
                    Some(Ok(event)) = crossterm_event => {
                        match event {
                            crossterm::event::Event::Key(key) => {
                                if key.kind == crossterm::event::KeyEventKind::Press {
                                    tx.send(Event::Key(key)).unwrap();
                                }
                            },
                            crossterm::event::Event::Mouse(mouse) => {
                                tx.send(Event::Mouse(mouse)).unwrap();
                            },
                            crossterm::event::Event::Resize(x, y) => {
                                tx.send(Event::Resize(x, y)).unwrap();
                            },
                            crossterm::event::Event::FocusLost => {},
                            crossterm::event::Event::FocusGained => {},
                            crossterm::event::Event::Paste(_) => {},
                        }
                    }
                }
            }
        });

        Self { tx, rx  }
    }

    pub async fn next(&mut self) -> Event {
        self.rx.recv_async().await.unwrap_or(Event::Invalid)
    }

    pub fn tx(&self) -> flume::Sender<Event> {
        self.tx.clone()
    }
}
