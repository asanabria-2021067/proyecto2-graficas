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

    pub fn ray_for_pixel(&self, px: f32, py: f32, width: u32, height: u32) -> Ray {
        let eye = self.position();
        let (forward, right, up) = self.basis();
        let aspect = width as f32 / height as f32;
        let tan_half_fov = (self.fov_deg.to_radians() * 0.5).tan();

        let ndc_x = (px + 0.5) / width as f32;
        let ndc_y = (py + 0.5) / height as f32;
        let screen_x = (2.0 * ndc_x - 1.0) * aspect * tan_half_fov;
        let screen_y = (1.0 - 2.0 * ndc_y) * tan_half_fov;

        let dir = (forward + right * screen_x + up * screen_y).normalize();
        Ray::new(eye, dir)
    }
}
