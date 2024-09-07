use std::{
    collections::HashMap,
    fmt::Display,
    fs::OpenOptions,
    io,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use chrono::{DateTime, Local};
use colored::Colorize;
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    widgets::{Block, Paragraph},
    Frame,
};
use tokio::time::sleep;
use tracing::{
    debug,
    field::{Field, Visit},
    info, instrument, trace, Level,
};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    registry::{LookupSpan, SpanRef},
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

struct TuiLayer<T> {
    tx: flume::Sender<T>,
}

impl<T> TuiLayer<T>
where
    T: Send + Sync + 'static,
{
    fn new() -> (Self, Arc<Mutex<Vec<T>>>) {
        let (tx, rx) = flume::unbounded();

        let logs = Arc::new(Mutex::new(vec![]));

        let result = logs.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv_async().await {
                let mut logs = logs.lock().unwrap_or_else(PoisonError::into_inner);
                logs.push(event);
            }
        });

        (Self { tx }, result)
    }
}

#[derive(Debug, Default)]
struct LogSpan {
    //target: Box<str>,
    //name: Box<str>,
    //fields: Fields,
    scope: Arc<str>,
}

#[derive(Debug)]
struct LogEvent {
    level: Level,
    target: Box<str>,
    name: Box<str>,
    fields: Fields,
    timestamp: DateTime<Local>,
    span: Option<LogSpan>,
}

impl LogEvent {
    fn with_span(mut self, span: LogSpan) -> Self {
        self.span = Some(span);

        self
    }
}

#[derive(Debug)]
struct Fields(HashMap<Box<str>, Box<str>>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{:?}", value).into());
    }
}

impl<'a, R> From<SpanRef<'a, R>> for LogSpan
where
    R: LookupSpan<'a>,
{
    fn from(span: SpanRef<'a, R>) -> Self {
        let scope = span
            .scope()
            .map(|span| span.name())
            .collect::<Vec<_>>()
            .join(":")
            .into();
        Self { scope }
    }
}

impl<'a> From<&'a tracing::Event<'a>> for LogEvent {
    fn from(value: &'a tracing::Event<'a>) -> Self {
        let meta = value.metadata();
        let level = meta.level().to_owned();
        let target = meta.target().into();
        let name = meta.name().into();
        let mut fields = Fields(HashMap::new());
        value.record(&mut fields);

        let timestamp = Local::now();

        Self {
            level,
            target,
            name,
            fields,
            timestamp,
            span: None,
        }
    }
}

impl Display for LogEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope = if let Some(span) = &self.span {
            format!("{}:{}", self.target, span.scope)
        } else {
            self.target.to_string()
        };

        let timestamp = self.timestamp.format("[%Y-%m-%d][%H:%M:%S]");

        let _name = self.name.as_ref();

        let level = match self.level {
            Level::INFO => "INFO".green(),
            Level::DEBUG => "DEBUG".white(),
            Level::TRACE => "TRACE".yellow(),
            Level::WARN => "WARN".red(),
            Level::ERROR => "ERROR".red()
        };

        let fields = &self.fields.0;

        write!(f, "{timestamp} {level:5} {scope:30.30} {fields:?}",)
    }
}

impl<S> Layer<S> for TuiLayer<LogEvent>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut event: LogEvent = event.into();
        if let Some(span) = ctx.lookup_current() {
            event = event.with_span(span.into())
        }
        self.tx.send(event).ok();
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> io::Result<()> {
    let logging_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("ez_rke.log")
        .unwrap();

    let (tui_layer, logs) = TuiLayer::new();

    tracing_subscriber::registry()
        .with(fmt::layer().json().with_writer(logging_file))
        .with(tui_layer)
        .with(EnvFilter::from_default_env())
        .init();

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
    Ok(())
}

#[instrument(skip_all)]
fn draw(frame: &mut Frame, logs: Arc<Mutex<Vec<LogEvent>>>) {
    // To view this event, run the example with `RUST_LOG=tracing=debug cargo run --example tracing`
    trace!(frame_count = frame.count());
    let height = frame.area().height;

    let logs = logs.lock().unwrap_or_else(PoisonError::into_inner);
    let logs = logs
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
