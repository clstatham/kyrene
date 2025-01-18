use std::ops::Deref;

use encase::{
    internal::{WriteInto, Writer},
    ShaderType,
};
use wgpu::util::DeviceExt;

use crate::{Device, Queue};

pub struct Buffer<T: ShaderType + WriteInto> {
    pub cpu_data: T,
    pub gpu_data: wgpu::Buffer,
}

impl<T: ShaderType + WriteInto> Buffer<T> {
    pub fn new(device: &Device, data: T, usage: wgpu::BufferUsages) -> Self {
        let mut bytes: Vec<u8> = Vec::new();
        data.write_into(&mut Writer::new(&data, &mut bytes, 0).unwrap());
        let gpu_data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &bytes,
            usage,
        });

        Self {
            cpu_data: data,
            gpu_data,
        }
    }

    pub fn cpu_data(&self) -> &T {
        &self.cpu_data
    }

    pub fn gpu_data(&self) -> &wgpu::Buffer {
        &self.gpu_data
    }

    pub fn enqueue_update(&mut self, queue: &Queue, data: T) {
        let mut bytes: Vec<u8> = Vec::new();
        self.cpu_data = data;
        self.cpu_data
            .write_into(&mut Writer::new(&self.cpu_data, &mut bytes, 0).unwrap());
        queue.write_buffer(&self.gpu_data, 0, &bytes);
    }
}

impl<T: ShaderType + WriteInto> AsRef<wgpu::Buffer> for Buffer<T> {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.gpu_data
    }
}

impl<T: ShaderType + WriteInto> Deref for Buffer<T> {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.gpu_data
    }
}
