use std::{
    io,
    time::Duration,
};

use ez_rke::{app::App, event::EventHandler, log::init_logger};



#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> io::Result<()> {

    let event_handler = EventHandler::new(Duration::from_millis(250));
    init_logger(&event_handler);
    
    let app = App::new(event_handler);

    app.run().await
}

