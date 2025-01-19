use kyrene_core::{
    event::Event,
    handler::{Res, ResMut},
    plugin::Plugin,
    prelude::World,
};

use crate::{color::Color, ActiveCommandEncoder, CurrentFrame, Render};

pub struct ClearColor(pub Color);

impl ClearColor {
    pub fn new(color: Color) -> Self {
        Self(color)
    }

    pub fn color(&self) -> Color {
        self.0
    }
}

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

impl From<Color> for ClearColor {
    fn from(color: Color) -> Self {
        Self::new(color)
    }
}

pub async fn render_clear_color(
    _event: Event<Render>,
    color: Res<ClearColor>,
    current_frame: Res<CurrentFrame>,
    mut encoder: ResMut<ActiveCommandEncoder>,
) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear_color"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: current_frame.color_view().unwrap(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color.0.into()),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
}

pub struct ClearColorPlugin(pub Color);

impl ClearColorPlugin {
    pub fn new(color: Color) -> Self {
        Self(color)
    }
}

impl Default for ClearColorPlugin {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

impl Plugin for ClearColorPlugin {
    async fn build(self, world: &mut World) {
        world.insert_resource(ClearColor::new(self.0)).await;
        world.add_event_handler(render_clear_color);
    }
}
