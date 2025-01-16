use std::{fmt::Debug, sync::Arc};

use encase::ShaderType;
use kyrene_core::{entity::Entity, event::Event, handler::Res, prelude::WorldHandle};

use crate::CurrentFrame;

#[derive(Clone)]
pub struct ViewTarget {
    pub color_target: Arc<wgpu::TextureView>,
    pub depth_target: Arc<wgpu::TextureView>,
}

#[derive(Debug, Clone, Copy, ShaderType)]
#[repr(C)]
pub struct CameraUniform {
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
    pub inv_view: glam::Mat4,
    pub inv_proj: glam::Mat4,
    pub camera_position: glam::Vec3,
    pub _padding: u32,
}

impl From<&Camera> for CameraUniform {
    fn from(camera: &Camera) -> Self {
        let view = camera.view_matrix;
        let proj = camera.projection_matrix;
        let inv_view = view.inverse();
        let inv_proj = proj.inverse();
        let camera_position = inv_view.col(3).truncate();

        Self {
            view,
            proj,
            inv_view,
            inv_proj,
            camera_position,
            _padding: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub active: bool,
    view_matrix: glam::Mat4,
    projection_matrix: glam::Mat4,
}

impl Debug for Camera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Camera")
            .field("view_matrix", &self.view_matrix)
            .field("projection_matrix", &self.projection_matrix)
            .finish()
    }
}

impl Camera {
    pub fn perspective_lookat(
        eye: glam::Vec3,
        center: glam::Vec3,
        up: glam::Vec3,
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let view = glam::Mat4::look_at_rh(eye, center, up);
        let proj = glam::Mat4::perspective_rh_gl(fov, aspect, near, far);
        Self {
            active: true,
            view_matrix: view,
            projection_matrix: proj,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn activate(&mut self) {
        self.set_active(true);
    }

    pub fn deactivate(&mut self) {
        self.set_active(false);
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        self.view_matrix
    }

    pub fn set_view_matrix(&mut self, view_matrix: glam::Mat4) {
        self.view_matrix = view_matrix;
    }

    pub fn projection_matrix(&self) -> glam::Mat4 {
        self.projection_matrix
    }

    pub fn set_projection_matrix(&mut self, projection_matrix: glam::Mat4) {
        self.projection_matrix = projection_matrix;
    }

    pub fn view_projection_matrix(&self) -> glam::Mat4 {
        self.projection_matrix * self.view_matrix
    }

    pub fn set_view_projection_matrix(
        &mut self,
        view_matrix: glam::Mat4,
        projection_matrix: glam::Mat4,
    ) {
        self.view_matrix = view_matrix;
        self.projection_matrix = projection_matrix;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            active: true,
            view_matrix: glam::Mat4::IDENTITY,
            projection_matrix: glam::Mat4::IDENTITY,
        }
    }
}

pub struct GpuCamera {
    pub camera: Camera,
}

pub struct InsertViewTarget {
    pub camera: Entity,
}

pub async fn insert_view_target(
    event: Event<InsertViewTarget>,
    world: WorldHandle,
    current_frame: Res<CurrentFrame>,
) {
    tracing::trace!("insert_view_target");
    let inner = current_frame.inner.as_ref().unwrap();
    let view_target = ViewTarget {
        color_target: inner.color_view.clone(),
        depth_target: inner.depth_view.clone(),
    };
    world.insert(event.camera, view_target).await;
}
