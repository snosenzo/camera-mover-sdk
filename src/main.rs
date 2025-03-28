use std::{fs::File, io::BufReader, path::PathBuf, sync::{
    atomic::{AtomicBool, Ordering},
    Arc, 
}, time::Duration};

use anyhow::Context;
use clap::Parser;
use controls::Controls;
use foxglove::{websocket::Capability, McapWriter};

mod logger;
mod camera_state;
mod controls;
mod mcap_replay;

use camera_state::CameraState;
use chrono::Local;
use mcap::sans_io::read::LinearReader;
use mcap_replay::{advance_reader, Summary};
use tracing::info;

const FILE_NAME_PREFIX: &str = "quickstart-rust";
#[derive(Debug, Parser)]
struct Cli {
    /// MCAP file to read.
    #[arg(short, long)]
    file: PathBuf,
    /// Whether to loop.
    #[arg(long)]
    r#loop: bool,
    /// Whether to write the file again with the camera state
    #[arg(long)]
    r#write: bool,
}

fn main() {
    let env = env_logger::Env::default().default_filter_or("debug");
    env_logger::init_from_env(env);


    let args = Cli::parse();
    let read_file_name = args
        .file
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();

    let done = Arc::new(AtomicBool::default());
    ctrlc::set_handler({
        let done = done.clone();
        move || {
            done.store(true, Ordering::Relaxed);
        }
    })
    .expect("Failed to set SIGINT handler");

    let server = foxglove::WebSocketServer::new()
        .name(read_file_name)
        .capabilities([Capability::Time])
        .start_blocking()
        .expect("Server failed to start");

    let mcap = if args.r#write {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let write_file_name = format!("{}-{}.mcap", FILE_NAME_PREFIX, timestamp);

        println!("Writing to mcap");
        Some(
            McapWriter::new()
                .create_new_buffered_file(&write_file_name)
                .expect("Failed to start mcap writer")
        )
    } else {
        println!("Not writing to mcap");
        None
    };

    let camera = CameraState::new("base_link", "camera");

    // Non-blocking key check
    let mut camera = camera;
    let mut controls = Controls::new();
    controls.set_done_flag(done.clone());


    info!("Loading mcap summary");
    let summary = Summary::load_from_mcap(&args.file).unwrap();

    info!("Waiting for client");
    std::thread::sleep(Duration::from_secs(1));

    info!("Starting stream");

    while !done.load(Ordering::Relaxed) {
        let mut file_stream = summary.file_stream();
        let mut file = BufReader::new(File::open(&args.file).unwrap());
        let mut reader = LinearReader::new();
        let mut last_camera_update_time = std::time::Instant::now();
        while !done.load(Ordering::Relaxed)
            && advance_reader(&mut reader, &mut file, |rec| {
                file_stream.handle_record(&server, rec);
                Ok(())
            })
            .context("read data").unwrap()
        {
            let time_since_last_camera_update = std::time::Instant::now().duration_since(last_camera_update_time);
            if time_since_last_camera_update > std::time::Duration::from_millis(33) {
                controls.capture_keys(&mut camera);
                controls.debug_print(&camera);
                camera.update();
                camera.log_state();
                last_camera_update_time = std::time::Instant::now();
            }
        }
        if !args.r#loop {
            done.store(true, Ordering::Relaxed);
        } else {
            info!("Looping");
            server.clear_session(None);
        }
      
        // Sleep to maintain a consistent frame rate
        std::thread::sleep(std::time::Duration::from_millis(33));
    }

    server.stop();
    if let Some(mcap) = mcap {
        mcap.close().expect("Failed to close mcap writer");
    }
    controls.close();
}
