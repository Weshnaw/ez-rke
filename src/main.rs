use std::{
    fs::OpenOptions,
    io,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    widgets::{Block, Paragraph},
    Frame,
};
use tokio::time::sleep;
use tracing::{debug, info, instrument, trace};
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

struct TuiLogger {
    tx: flume::Sender<Box<[u8]>>,
}

impl TuiLogger {
    fn new() -> (Self, flume::Receiver<Box<[u8]>>) {
        let (tx, rx) = flume::unbounded();

        (Self { tx }, rx)
    }
}

// I've seen other folks use a sink, is that needed?
impl<'a> io::Write for &'a TuiLogger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .send(buf.into())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TuiLogger {
    type Writer = &'a TuiLogger;

    fn make_writer(&'a self) -> Self::Writer {
        self
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> io::Result<()> {
    let logging_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("ez_rke.log")
        .unwrap();

    let (tui_logger, rx) = TuiLogger::new();

    tracing_subscriber::registry()
        .with(fmt::layer().json().with_writer(logging_file))
        .with(fmt::layer().with_writer(tui_logger))
        .with(EnvFilter::from_default_env())
        .init();

    let logs = init_tui_logs(rx);
    info!("Initialized ez_rke loggers...");

    let mut terminal = ratatui::init();
    terminal.clear()?;

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(1000)).await;
            debug!("Test");
        }
    });

    let mut exit = false;
    while !exit {
        handle_events(&mut exit)?;
        terminal.draw(|frame| draw(frame, logs.clone()))?;
    }
    ratatui::restore();

    Ok(())
}

#[instrument]
fn handle_events(exit: &mut bool) -> io::Result<()> {
    // Render the UI at least once every 100ms
    if event::poll(Duration::from_millis(100))? {
        let event = event::read()?;
        debug!(?event);
        if let Event::Key(key) = event {
            *exit = KeyCode::Char('q') == key.code;
        }
    }
    //events.truncate(10);
    Ok(())
}

#[instrument(skip_all)]
fn init_tui_logs(rx: flume::Receiver<Box<[u8]>>) -> Arc<Mutex<Vec<Box<str>>>> {
    let logs = Arc::new(Mutex::new(vec![]));
    
    let result = logs.clone();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv_async().await {
            let mut logs = logs.lock().unwrap_or_else(PoisonError::into_inner);
            logs.push(std::str::from_utf8(&event).unwrap_or_default().into());
        }
    });

    result
}

#[instrument(skip_all)]
fn draw(frame: &mut Frame, logs: Arc<Mutex<Vec<Box<str>>>>) {
    // To view this event, run the example with `RUST_LOG=tracing=debug cargo run --example tracing`
    trace!(frame_count = frame.count());
    let height = frame.area().height;

    let logs = logs.lock().unwrap_or_else(PoisonError::into_inner);
    let logs = logs
        .iter()
        .rev()
        .take(height.into())
        .map(|s| s.as_ref())
        .rev()
        .collect::<Box<[&str]>>();
    let paragraph = Paragraph::new(logs.join(""))
        .block(Block::bordered().title("Tracing example. Press 'q' to quit."));
    frame.render_widget(paragraph, frame.area());
}
