use encase::ShaderType;

use crate::geom::{Point3, Quat, Transform, Vec3};

#[derive(Debug, Clone, Copy, ShaderType)]
pub struct PerspectiveCamera3d {
    pub position: Point3,
    pub direction: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for PerspectiveCamera3d {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            fov: 60.0,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

impl PerspectiveCamera3d {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view_transform(&self) -> Transform {
        Transform::look_at(self.position, self.position + self.direction, self.up)
    }

    pub fn projection_transform(&self) -> Transform {
        Transform::perspective(self.fov, self.aspect_ratio, self.near, self.far)
    }

    pub fn view_projection_transform(&self) -> Transform {
        self.projection_transform() * self.view_transform()
    }

    pub fn forward(&self) -> Vec3 {
        let mut forward = self.direction;
        forward.normalize();
        forward
    }

    pub fn right(&self) -> Vec3 {
        let mut right = self.forward().cross(self.up);
        right.normalize();
        right
    }

    pub fn up(&self) -> Vec3 {
        let mut up = self.right().cross(self.forward());
        up.normalize();
        up
    }

    pub fn move_forward(&mut self, amount: f32) {
        self.position += self.forward() * amount;
    }

    pub fn move_right(&mut self, amount: f32) {
        self.position += self.right() * amount;
    }

    pub fn move_up(&mut self, amount: f32) {
        self.position += self.up() * amount;
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        let right = self.right();
        let up = self.up();
        let forward = self.forward();

        let yaw = Transform::from_rotation(Quat::from_axis_angle(up, yaw));
        let pitch = Transform::from_rotation(Quat::from_axis_angle(right, pitch));

        self.direction = yaw * pitch * forward;
    }

    pub fn rotate_yaw(&mut self, yaw: f32) {
        let up = self.up();
        let yaw = Transform::from_rotation(Quat::from_axis_angle(up, yaw));
        self.direction = yaw * self.direction;
    }

    pub fn rotate_pitch(&mut self, pitch: f32) {
        let right = self.right();
        let pitch = Transform::from_rotation(Quat::from_axis_angle(right, pitch));
        self.direction = pitch * self.direction;
    }

    pub fn rotate_around(&mut self, axis: Vec3, angle: f32) {
        let rotation = Transform::from_rotation(Quat::from_axis_angle(axis, angle));
        self.direction = rotation * self.direction;
        self.up = rotation * self.up;
    }
}
