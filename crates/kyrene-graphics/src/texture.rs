use kyrene_asset::{AssetLoaderPlugin, Load, LoadSource};
use kyrene_core::{
    plugin::Plugin,
    prelude::{tokio, World, WorldHandle},
};
use wgpu::util::DeviceExt;

pub mod texture_format {
    pub use wgpu::TextureFormat;
    pub const VIEW_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const SDR_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
    pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const HDR_CUBE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
    pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
}

pub struct Texture {
    pub image: image::RgbaImage,
}

impl Texture {
    pub fn new(image: image::RgbaImage) -> Self {
        Self { image }
    }

    pub fn from_rgba8(rgba8: &[u8], width: u32, height: u32) -> Self {
        let image = image::RgbaImage::from_raw(width, height, rgba8.to_vec()).unwrap();
        Self { image }
    }

    pub fn to_rgba8(&self) -> Vec<u8> {
        self.image.clone().into_raw()
    }

    pub fn from_rgb8(rgb8: &[u8], width: u32, height: u32) -> Self {
        let mut rgba8 = Vec::new();

        for i in 0..(width * height) as usize {
            rgba8.push(rgb8[i * 3]);
            rgba8.push(rgb8[i * 3 + 1]);
            rgba8.push(rgb8[i * 3 + 2]);
            rgba8.push(255);
        }

        Self::from_rgba8(&rgba8, width, height)
    }

    pub fn to_rgb8(&self) -> Vec<u8> {
        let mut rgb8 = Vec::new();

        for i in 0..self.image.width() * self.image.height() {
            let pixel = self
                .image
                .get_pixel(i % self.image.width(), i / self.image.width());
            rgb8.push(pixel[0]);
            rgb8.push(pixel[1]);
            rgb8.push(pixel[2]);
        }

        rgb8
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn resize(&mut self, width: u32, height: u32, filter: image::imageops::FilterType) {
        self.image = image::imageops::resize(&self.image, width, height, filter);
    }
}

pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    async fn build(self, world: &mut World) {
        world.add_plugin(AssetLoaderPlugin::<TextureLoader>::default());
    }
}

#[derive(Default)]
pub struct TextureLoader;

impl Load for TextureLoader {
    type Asset = Texture;
    type Error = image::ImageError;

    async fn load(
        &self,
        source: LoadSource,
        _world: WorldHandle,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = match source {
            LoadSource::Path(path) => tokio::fs::read(path)
                .await
                .map_err(image::ImageError::IoError)?,
            LoadSource::Bytes(bytes) => bytes,
            LoadSource::Existing(asset) => return Ok(asset.downcast().unwrap()),
        };

        let image = image::load_from_memory(&bytes)?;

        Ok(Texture::new(image.to_rgba8()))
    }
}

#[derive(Clone)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl GpuTexture {
    pub fn new(
        device: &wgpu::Device,
        label: Option<&str>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Texture,
        format: wgpu::TextureFormat,
    ) -> Option<Self> {
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Texture"),
                size: wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &image.to_rgba8(),
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Some(Self { texture, view })
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }
}
