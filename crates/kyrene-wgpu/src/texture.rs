pub mod texture_format {
    pub use wgpu::TextureFormat;
    pub const VIEW_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const SDR_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
    pub const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
    pub const HDR_CUBE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
    pub const NORMAL_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
}
