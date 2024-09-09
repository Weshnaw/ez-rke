use std::{collections::HashMap, fmt::Display, fs::OpenOptions, sync::Arc};

use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};
use tracing::{
    field::{Field, Visit},
    info, Level,
};

use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    registry::{LookupSpan, SpanRef},
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

use crate::event::{Event, EventHandler};

struct TuiLayer<T> {
    tx: flume::Sender<T>,
}

impl<T> TuiLayer<T>
where
    T: Send + Sync + 'static,
{
    fn new(tx: flume::Sender<T>) -> Self {
        Self { tx }
    }
}

#[derive(Clone, Debug, Default)]
struct LogSpan {
    //target: Box<str>,
    //name: Box<str>,
    //fields: Fields,
    scope: Arc<str>,
}

#[derive(Clone, Debug)]
pub struct LogEvent {
    level: Level,
    target: Arc<str>,
    name: Arc<str>,
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

#[derive(Clone, Debug)]
struct Fields(HashMap<Arc<str>, Arc<str>>);

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
        let _name = self.name.as_ref();
        let scope = if let Some(span) = &self.span {
            format!("{}:{}", self.target, span.scope)
        } else {
            self.target.to_string()
        };
        let timestamp = self.timestamp.format("[%Y-%m-%d][%H:%M:%S]");
        let level = self.level;
        let fields = &self.fields.0;

        write!(f, "{timestamp} {level:5} {scope:30.30} {fields:?}",)
    }
}

impl<S> Layer<S> for TuiLayer<Event>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut event: LogEvent = event.into();
        if let Some(span) = ctx.lookup_current() {
            event = event.with_span(span.into())
        }
        self.tx.send(Event::Log(event)).ok();
    }
}

impl From<&'_ LogEvent> for ListItem<'_> {
    fn from(event: &'_ LogEvent) -> Self {
        let style = match event.level {
            Level::INFO => Style::default().fg(Color::Green),
            Level::DEBUG => Style::default().fg(Color::Blue),
            Level::TRACE => Style::default().fg(Color::White),
            Level::WARN => Style::default().fg(Color::Yellow),
            Level::ERROR => Style::default().fg(Color::Red),
        };

        let timestamp = event.timestamp.format("[%Y-%m-%d][%H:%M:%S%.6f]");
        let level = event.level;
        let scope = if let Some(span) = &event.span {
            format!("{}:{}", event.target, span.scope)
        } else {
            event.target.to_string()
        };
        let fields = &event.fields.0;

        let content = vec![Line::from(vec![
            Span::raw(timestamp.to_string()),
            Span::styled(format!(" {level:<5} "), style),
            Span::raw(scope),
            Span::raw(format!(" {fields:?}")),
        ])];

        Self::new(content)
    }
}

pub fn init_logger(event_handler: &EventHandler) {
    let logging_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("ez_rke.log")
        .unwrap();

    let tui_layer = TuiLayer::new(event_handler.tx());

    tracing_subscriber::registry()
        .with(fmt::layer().json().with_writer(logging_file))
        .with(tui_layer)
        .with(EnvFilter::from_default_env())
        .init();

    info!("Initialized ez_rke loggers...");
}
