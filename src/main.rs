use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, 
};

use controls::Controls;
use foxglove::McapWriter;

mod logger;
mod camera_state;
mod controls;

use camera_state::CameraState;
use chrono::Local;

const FILE_NAME_PREFIX: &str = "quickstart-rust";

fn main() {
    let env = env_logger::Env::default().default_filter_or("debug");
    env_logger::init_from_env(env);

    let done = Arc::new(AtomicBool::default());
    ctrlc::set_handler({
        let done = done.clone();
        move || {
            done.store(true, Ordering::Relaxed);
        }
    })
    .expect("Failed to set SIGINT handler");

    foxglove::WebSocketServer::new()
        .start_blocking()
        .expect("Server failed to start");

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let file_name = format!("{}-{}.mcap", FILE_NAME_PREFIX, timestamp);

    let mcap = McapWriter::new()
        .create_new_buffered_file(&file_name)
        .expect("Failed to start mcap writer");

    let camera = CameraState::new("base_link", "camera");

    // Non-blocking key check
    let mut camera = camera;
    let mut controls = Controls::new();
    controls.set_done_flag(done.clone());

    while !done.load(Ordering::Relaxed) {
        controls.capture_keys(&mut camera);
        controls.debug_print(&camera);
       // Apply physics update
        camera.update();
        
        // Log camera state
        camera.log_state();
       
        // Sleep to maintain a consistent frame rate
        std::thread::sleep(std::time::Duration::from_millis(33));
    }

    mcap.close().expect("Failed to close mcap writer");
    controls.close();
}
