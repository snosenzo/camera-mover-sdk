use termion::event::Key;
use camera_state::CameraState;
use std::io::{self, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::input::TermRead;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::camera_state;


pub struct Controls {
    rx: std::sync::mpsc::Receiver<Key>,
    w_pressed: bool,
    a_pressed: bool,
    s_pressed: bool,
    d_pressed: bool,
    q_pressed: bool,
    e_pressed: bool,
    stdout: RawTerminal<Stdout>,
    done: Option<Arc<AtomicBool>>,
}

 impl Controls {
    pub fn new() -> Self {

        // Set up a channel for async keyboard input
        let (tx, rx) = std::sync::mpsc::channel();
        
        let stdin = io::stdin();
        // Start a thread to handle keyboard input
        std::thread::spawn(move || {
            for c in stdin.keys() {
                match c {
                    Ok(key) => tx.send(key).unwrap(),
                    Err(_) => {}
                }
            }
        });
    
        // Set terminal to raw mode 
        let mut stdout = io::stdout().into_raw_mode().unwrap();
        write!(stdout, "{}{}Camera control simulation started!\r\nUse WASD keys to control the camera (one at a time)\r\nPress Q/E for roll control\r\nPress SPACE to stop\r\n",
        termion::clear::All,
        termion::cursor::Goto(1, 1)).unwrap();
        stdout.flush().unwrap();
        Self { 
            w_pressed: false, 
            a_pressed: false, 
            s_pressed: false, 
            d_pressed: false, 
            q_pressed: false,
            e_pressed: false,
            rx, 
            stdout,
            done: None,
        }
    }

    pub fn set_done_flag(&mut self, done: Arc<AtomicBool>) {
        self.done = Some(done);
    }

    pub fn capture_keys(&mut self, camera: &mut CameraState) {

        self.w_pressed = false;
        self.a_pressed = false;
        self.s_pressed = false;
        self.d_pressed = false;
        self.q_pressed = false;
        self.e_pressed = false;
     // Check for keyboard events
        if let Ok(key) = self.rx.try_recv() {
            // Reset all key states first (only one key can be active at a time)
            self.w_pressed = false;
            self.a_pressed = false;
            self.s_pressed = false;
            self.d_pressed = false;
            self.q_pressed = false;
            self.e_pressed = false;
            
            match key {
                Key::Char('w') | Key::Char('W') => self.w_pressed = true,
                Key::Char('a') | Key::Char('A') => self.a_pressed = true, 
                Key::Char('s') | Key::Char('S') => self.s_pressed = true,
                Key::Char('d') | Key::Char('D') => self.d_pressed = true,
                Key::Char('q') | Key::Char('Q') => self.q_pressed = true,
                Key::Char('e') | Key::Char('E') => self.e_pressed = true,
                Key::Char(' ') => {
                    camera.stop();
                },
                Key::Ctrl('c') => {
                    // Set the done flag if available
                    if let Some(done) = &self.done {
                        done.store(true, Ordering::Relaxed);
                    }
                },
                _ => {}
            }
        }
        
        // Forward/backward movement
        if self.w_pressed {
            camera.accelerate(0.5);
        } 
        if self.s_pressed {
            camera.decelerate(0.5);
        }
        
        // Steering
        if self.a_pressed {
            camera.steer_left(0.2);
        }
        if self.d_pressed {
            camera.steer_right(0.2);
        }

        // Roll control
        if self.q_pressed {
            camera.roll_counterclockwise(0.3);
        }
        if self.e_pressed {
            camera.roll_clockwise(0.3);
        }
    }

    pub fn debug_print(&mut self, camera: &CameraState) {
        // Display current position and active controls
        write!(self.stdout, "{}Position: ({:.2}, {:.2}, {:.2})  Velocity: {:.2}  Roll: {:.2}  {}{}{}{}{}{}",
               termion::cursor::Goto(1, 4),
               camera.get_translation()[0],
               camera.get_translation()[1],
               camera.get_translation()[2],
               camera.get_velocity(),
               camera.get_roll(),
               if self.w_pressed { "W " } else { "  " },
               if self.a_pressed { "A " } else { "  " },
               if self.s_pressed { "S " } else { "  " },
               if self.d_pressed { "D " } else { "  " },
               if self.q_pressed { "Q " } else { "  " },
               if self.e_pressed { "E " } else { "  " }).unwrap();
        self.stdout.flush().unwrap();
    }
    pub fn close(&mut self) {
        // Reset terminal
        write!(self.stdout, "{}", termion::cursor::Show).unwrap();
    }
}