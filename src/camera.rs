use crate::math::{Ray, Vec3};

const PITCH_MIN_DEG: f32 = 5.0;
const PITCH_MAX_DEG: f32 = 85.0;

pub struct Camera {
    pub center: Vec3,
    pub yaw: f32,   // radians, free
    pub pitch: f32, // radians, clamped
    pub dist: f32,
    pub dist_min: f32,
    pub dist_max: f32,
    pub fov_deg: f32,
}

impl Camera {
    pub fn new(center: Vec3, yaw_deg: f32, pitch_deg: f32, dist: f32, dist_min: f32, dist_max: f32, fov_deg: f32) -> Self {
        let mut cam = Camera {
            center,
            yaw: yaw_deg.to_radians(),
            pitch: pitch_deg.to_radians(),
            dist,
            dist_min,
            dist_max,
            fov_deg,
        };
        cam.clamp_state();
        cam
    }

    fn clamp_state(&mut self) {
        self.pitch = self.pitch.clamp(PITCH_MIN_DEG.to_radians(), PITCH_MAX_DEG.to_radians());
        self.dist = self.dist.clamp(self.dist_min, self.dist_max);
    }

    pub fn orbit(&mut self, dyaw_rad: f32, dpitch_rad: f32) {
        self.yaw += dyaw_rad;
        self.pitch += dpitch_rad;
        self.clamp_state();
    }

    pub fn zoom(&mut self, delta: f32) {
        self.dist += delta;
        self.clamp_state();
    }

    pub fn position(&self) -> Vec3 {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        self.center + Vec3::new(self.dist * cp * sy, self.dist * sp, self.dist * cp * cy)
    }

    /// Returns (forward, right, up) orthonormal basis, forward pointing from eye to center.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let eye = self.position();
        let forward = (self.center - eye).normalize();
        let world_up = Vec3::new(0.0, 1.0, 0.0);
        let right = forward.cross(world_up).normalize();
        let up = right.cross(forward).normalize();
        (forward, right, up)
    }

    /// Precomputes everything that's constant for the whole frame (eye,
    /// basis, fov/aspect terms) so the per-pixel path only does the cheap
    /// part. Call once per render, not once per pixel.
    pub fn frame(&self, width: u32, height: u32) -> CameraFrame {
        let eye = self.position();
        let (forward, right, up) = self.basis();
        CameraFrame {
            eye,
            forward,
            right,
            up,
            aspect: width as f32 / height as f32,
            tan_half_fov: (self.fov_deg.to_radians() * 0.5).tan(),
            width: width as f32,
            height: height as f32,
        }
    }

}

/// Per-frame camera state baked out of yaw/pitch/dist/fov so `ray_for_pixel`
/// is pure per-pixel arithmetic (no trig, no allocation).
pub struct CameraFrame {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    aspect: f32,
    tan_half_fov: f32,
    width: f32,
    height: f32,
}

impl CameraFrame {
    #[inline]
    pub fn ray_for_pixel(&self, px: f32, py: f32) -> Ray {
        let ndc_x = (px + 0.5) / self.width;
        let ndc_y = (py + 0.5) / self.height;
        let screen_x = (2.0 * ndc_x - 1.0) * self.aspect * self.tan_half_fov;
        let screen_y = (1.0 - 2.0 * ndc_y) * self.tan_half_fov;

        let dir = (self.forward + self.right * screen_x + self.up * screen_y).normalize();
        Ray::new(self.eye, dir)
    }
}
