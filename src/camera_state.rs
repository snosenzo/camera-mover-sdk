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
    roll: f64, // roll angle in radians
    roll_rate: f64, // roll angular velocity
    max_velocity: f64,
    velocity_step: f64,
    steering_step: f64,
    roll_step: f64,
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
            roll: 0.0, // 0 radians means no roll
            roll_rate: 0.0, // roll angular velocity
            max_velocity: 0.2,
            velocity_step: 0.05,
            steering_step: 0.01,
            roll_step: 0.01,
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
        self.roll_rate = 0.0;
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

    /// Roll counterclockwise (Q key) by the specified factor
    pub fn roll_counterclockwise(&mut self, step_factor: f64) {
        let step = step_factor * self.roll_step;
        self.roll_rate -= step;
        self.roll_rate = self.roll_rate.clamp(-0.3, 0.3);
    }

    /// Roll clockwise (E key) by the specified factor
    pub fn roll_clockwise(&mut self, step_factor: f64) {
        let step = step_factor * self.roll_step;
        self.roll_rate += step;
        self.roll_rate = self.roll_rate.clamp(-0.3, 0.3);
    }

    /// Updates the camera position based on current velocity and direction
    pub fn update(&mut self) {
        self.heading += self.steer;
        self.roll += self.roll_rate;

        // loop heading around 2pi
        if self.heading > 2.0 * PI {
            self.heading -= 2.0 * PI;
        }
        if self.heading < 0.0 {
            self.heading += 2.0 * PI;
        }

        // loop roll around 2pi
        if self.roll > 2.0 * PI {
            self.roll -= 2.0 * PI;
        }
        if self.roll < 0.0 {
            self.roll += 2.0 * PI;
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

        // Apply damping to steering rate and roll rate
        self.steer *= 0.8;
        self.roll_rate *= 0.8;

        // Create quaternion from heading (y-axis rotation) and roll (z-axis rotation)
        // First calculate quaternion components for heading (y-axis rotation)
        let half_heading = self.heading / 2.0;
        let qy_w = half_heading.cos();
        let qy_x = 0.0;
        let qy_y = half_heading.sin();
        let qy_z = 0.0;
        
        // Calculate quaternion components for roll (z-axis rotation)
        let half_roll = self.roll / 2.0;
        let qz_w = half_roll.cos();
        let qz_x = 0.0;
        let qz_y = 0.0;
        let qz_z = half_roll.sin();
        
        // Multiply quaternions to combine rotations (heading * roll)
        // (w1, x1, y1, z1) * (w2, x2, y2, z2)
        let w = qy_w * qz_w - qy_x * qz_x - qy_y * qz_y - qy_z * qz_z;
        let x = qy_w * qz_x + qz_w * qy_x + qy_y * qz_z - qy_z * qz_y;
        let y = qy_w * qz_y + qz_w * qy_y + qy_z * qz_x - qy_x * qz_z;
        let z = qy_w * qz_z + qz_w * qy_z + qy_x * qz_y - qy_y * qz_x;
        
        // Set rotation quaternion [x, y, z, w]
        self.rotation = vec![x, y, z, w];
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

    /// Gets the current roll angle in radians
    pub fn get_roll(&self) -> f64 {
        self.roll
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
