use crate::logger;
use std::f64::consts::PI;

/// Manages the state of the camera including position, orientation, and physics
#[derive(Clone)]
pub struct CameraState {
    frame_id: String,
    parent_frame_id: String,
    translation: Vec<f64>,
    rotation: Vec<f64>,
    velocity:f64,
    // radians in the XZ plane
    heading: f64,
    steer: f64, // radial velocity
    max_velocity: f64,
    velocity_step: f64,
    steering_step: f64,
}

impl CameraState {
    /// Creates a new CameraState with default position and orientation
    pub fn new(parent_frame_id: &str, frame_id: &str) -> Self {
        Self {
            parent_frame_id: parent_frame_id.to_string(),
            frame_id: frame_id.to_string(),
            translation: vec![0.0, 0.0, 0.0],
            rotation: vec![0.0, 0.0, 0.0, 1.0], // Default quaternion (no rotation)
            velocity: 0.0,
            heading: 0.0, // 0 radians means facing positive Z axis
            steer: 0.0, // radial velocity
            max_velocity: 0.2,
            velocity_step: 0.01,
            steering_step: 0.01,
        }
    }

     /// Increases forward velocity by the specified factor
    pub fn accelerate(&mut self, step_factor: f64) {
        let step = step_factor * self.velocity_step;
        self.velocity = (self.velocity + step).min(self.max_velocity);
    }

    /// Decreases forward velocity by the specified factor
    pub fn decelerate(&mut self, step_factor: f64) {
        let step = step_factor * self.velocity_step;
        self.velocity = (self.velocity - step).max(-self.max_velocity);
    }

    /// Immediately stops all movement
    pub fn stop(&mut self) {
        self.velocity = 0.0;
        self.steer = 0.0;
    }

    /// Steers left (counterclockwise in XZ plane) by the specified factor
    pub fn steer_left(&mut self, step_factor: f64) {
        let step = step_factor * self.steering_step;
        self.steer -= step;
        self.steer = self.steer.clamp(-0.3, 0.3);
    }

    /// Steers right (clockwise in XZ plane) by the specified factor
    pub fn steer_right(&mut self, step_factor: f64) {
        let step = step_factor * self.steering_step;
        self.steer += step;
        self.steer = self.steer.clamp(-0.3, 0.3);
    }

    /// Updates the camera position based on current velocity and direction
    pub fn update(&mut self) {
        self.heading += self.steer;

        // loop heading around 2pi
        if self.heading > 2.0 * PI {
            self.heading -= 2.0 * PI;
        }
        if self.heading < 0.0 {
            self.heading += 2.0 * PI;
        }

        if self.velocity.abs() > 1e-6 {
            // In this coordinate system, Z is forward, X is right, Y is up
            // The direction angle rotates in the XZ plane (horizontal plane)
            let dx = self.velocity * self.heading.sin();
            let dy = 0.0; // Maintain constant height
            let dz = self.velocity * self.heading.cos();

            // Update position
            self.translation[0] += dx;
            self.translation[1] += dy;
            self.translation[2] += dz;

            self.velocity *= 0.8;
        }

        // Calculate rotation to look in direction of travel
        let heading = self.heading;
        
        // Create quaternion for rotation around Y axis (for horizontal turning)
        // When heading is 0, the camera looks in the +Z direction
        let qw = (heading / 2.0).cos();
        let qy = (heading / 2.0).sin();
        
        // Set rotation quaternion [x, y, z, w]
        self.rotation = vec![0.0, qy, 0.0, qw];
    }

    /// Gets the current velocity
    pub fn get_velocity(&self) -> f64 {
        self.velocity
    }

    /// Gets the maximum velocity
    pub fn get_max_velocity(&self) -> f64 {
        self.max_velocity
    }

    /// Gets the current translation vector
    pub fn get_translation(&self) -> &Vec<f64> {
        &self.translation
    }

    /// Logs the current camera state (calibration, image, and transform)
    pub fn log_state(&self) {
        logger::log_camera_calibration(&self.frame_id);
        logger::log_raw_image(&self.frame_id);
        logger::log_frame_transform(
            &self.parent_frame_id,
            &self.frame_id,
            self.translation.clone(),
            self.rotation.clone(),
        );
    }
}
