use std::f64::consts::PI;

use foxglove::schemas::{CameraCalibration, FrameTransform, RawImage, Timestamp, Vector3, Quaternion};

foxglove::static_typed_channel!(pub(crate) CAMERA, "/sdk-camera", foxglove::schemas::CameraCalibration);
foxglove::static_typed_channel!(pub(crate) IMAGE, "/sdk-image", foxglove::schemas::RawImage);
foxglove::static_typed_channel!(pub(crate) TF, "/sdk-tf", foxglove::schemas::FrameTransform);

const IMAGE_WIDTH: u32 = 1600;
const IMAGE_HEIGHT: u32 = 900;

pub fn log_camera_calibration(frame_id: &str) {
    let timestamp_sec = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let timestamp = match Timestamp::try_from_epoch_secs_f64(timestamp_sec) {
        Ok(timestamp) => timestamp,
        Err(e) => {
            eprintln!("Error converting timestamp: {}", e);
            return;
        }
    };

    CAMERA.log(&CameraCalibration {
        timestamp: Some(timestamp),
        frame_id: frame_id.to_string(),
        width: IMAGE_WIDTH,
        height: IMAGE_HEIGHT,
        distortion_model: "plumb_bob".to_string(),
        d: vec![],
        k: vec![1266.417203046554, 0.0, 816.2670197447984, 0.0, 1266.417203046554, 491.50706579294757, 0.0, 0.0, 1.0],
        r: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        p: vec![1266.417203046554, 0.0, 816.2670197447984, 0.0, 0.0, 1266.417203046554, 491.50706579294757, 0.0, 0.0, 0.0, 1.0, 0.0],
    });
}

pub fn log_frame_transform(parent_frame_id: &str, child_frame_id: &str, translation: Vec<f64>, rotation: Vec<f64>) {
    let timestamp_sec = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let timestamp = match Timestamp::try_from_epoch_secs_f64(timestamp_sec) {
        Ok(timestamp) => timestamp,
        Err(e) => {
            eprintln!("Error converting timestamp: {}", e);
            return;
        }
    };

    TF.log(&FrameTransform {
        timestamp: Some(timestamp),
        parent_frame_id: parent_frame_id.to_string(),
        child_frame_id: child_frame_id.to_string(),
        translation: Some(Vector3 {
            x: translation[0],
            y: translation[1],
            z: translation[2],
        }),
        rotation: Some(Quaternion {
            x: rotation[0],
            y: rotation[1],
            z: rotation[2],
            w: rotation[3],
        }),
    });
}

pub fn calculate_transform(angle: f64, radius: f64) -> (Vec<f64>, Vec<f64>) {
    // Calculate position on circle
    let x = radius * angle.cos();
    let y = radius * angle.sin();
    let z = radius * (angle + PI / 2.0).sin();
    let translation = vec![x, y, z];

    // Calculate rotation to point camera toward origin
    // Direction vector from camera to origin (normalized)
    let dx = -x;
    let dy = -y;
    let dz = -z;
    
    // Normalize the direction vector
    let magnitude = (dx * dx + dy * dy + dz * dz).sqrt();
    if magnitude < 1e-6 {
        // Camera is at the origin, use default orientation
        return (translation, vec![0.0, 0.0, 0.0, 1.0]);
    }
    
    let forward_x = dx / magnitude;
    let forward_y = dy / magnitude;
    let forward_z = dz / magnitude;
    
    // Create rotation from the default forward direction (0,0,1) to our target direction
    // Using the axis-angle method to quaternion

    // Find the axis of rotation using cross product between (0,0,1) and our forward vector
    let axis_x = -forward_y;  // cross product: (0,0,1) × (fx,fy,fz) = (-fy, fx, 0)
    let axis_y = forward_x;
    let axis_z = 0.0;
    
    // Calculate the dot product to find the angle
    let dot = forward_z; // dot product: (0,0,1)·(fx,fy,fz) = fz
    
    // Special case: if vectors are parallel (or anti-parallel)
    if 1.0 - dot.abs() < 1e-6 {
        if dot > 0.0 {
            // Vectors are identical, no rotation needed
            return (translation, vec![0.0, 0.0, 0.0, 1.0]);
        } else {
            // Vectors are opposite, rotate 180° around any perpendicular axis (e.g., x-axis)
            return (translation, vec![1.0, 0.0, 0.0, 0.0]);
        }
    }
    
    // Normalize the axis
    let axis_mag = (axis_x * axis_x + axis_y * axis_y + axis_z * axis_z).sqrt();
    let axis_x = axis_x / axis_mag;
    let axis_y = axis_y / axis_mag;
    let axis_z = axis_z / axis_mag;
    
    // Calculate the angle between the vectors
    let angle = dot.acos();
    
    // Convert to quaternion
    let half_angle = angle / 2.0;
    let sin_half = half_angle.sin();
    let cos_half = half_angle.cos();
    
    let qx = axis_x * sin_half;
    let qy = axis_y * sin_half;
    let qz = axis_z * sin_half;
    let qw = cos_half;
    
    let rotation = vec![qx, qy, qz, qw];
    
    (translation, rotation)
}

pub fn log_raw_image(frame_id: &str) {
    let timestamp_sec = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    let timestamp = match Timestamp::try_from_epoch_secs_f64(timestamp_sec) {
        Ok(timestamp) => timestamp,
        Err(e) => {
            eprintln!("Error converting timestamp: {}", e);
            return;
        }
    };

    let width = 640;
    let height = 480;
    let data = vec![0u8; width * height * 4]; // RGBA format, all zeros = transparent
    
    IMAGE.log(&RawImage {
        timestamp: Some(timestamp),
        frame_id: frame_id.to_string(),
        width: width as u32,
        height: height as u32,
        encoding: "rgba8".to_string(),
        step: (width * 4) as u32,
        data: data.into(),
    });
} 