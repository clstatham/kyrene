use encase::ShaderType;

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::from_rgb(1.0, 1.0, 1.0);
    pub const RED: Self = Self::from_rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::from_rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::from_rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::from_rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::from_rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::from_rgb(1.0, 0.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_tuple(&self) -> (f32, f32, f32, f32) {
        (self.r, self.g, self.b, self.a)
    }

    pub fn to_u32(&self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        let a = (self.a * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    pub fn from_u32(color: u32) -> Self {
        let r = ((color >> 16) & 0xff) as f32 / 255.0;
        let g = ((color >> 8) & 0xff) as f32 / 255.0;
        let b = (color & 0xff) as f32 / 255.0;
        let a = ((color >> 24) & 0xff) as f32 / 255.0;
        Self::from_rgba(r, g, b, a)
    }

    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        Self::new(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
            a.a + (b.a - a.a) * t,
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from(array: [f32; 4]) -> Self {
        Self::new(array[0], array[1], array[2], array[3])
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from(tuple: (f32, f32, f32, f32)) -> Self {
        Self::new(tuple.0, tuple.1, tuple.2, tuple.3)
    }
}

impl From<u32> for Color {
    fn from(color: u32) -> Self {
        Self::from_u32(color)
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> [f32; 4] {
        color.to_array()
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(color: Color) -> (f32, f32, f32, f32) {
        color.to_tuple()
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> u32 {
        color.to_u32()
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> wgpu::Color {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}

impl From<wgpu::Color> for Color {
    fn from(color: wgpu::Color) -> Color {
        Color::new(
            color.r as f32,
            color.g as f32,
            color.b as f32,
            color.a as f32,
        )
    }
}

impl std::ops::Add<Color> for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Self::Output {
        Color::new(
            self.r + rhs.r,
            self.g + rhs.g,
            self.b + rhs.b,
            self.a + rhs.a,
        )
    }
}

impl std::ops::Sub<Color> for Color {
    type Output = Color;

    fn sub(self, rhs: Color) -> Self::Output {
        Color::new(
            self.r - rhs.r,
            self.g - rhs.g,
            self.b - rhs.b,
            self.a - rhs.a,
        )
    }
}

impl std::ops::Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Color::new(self.r * rhs, self.g * rhs, self.b * rhs, self.a * rhs)
    }
}

impl std::ops::Mul<Color> for f32 {
    type Output = Color;

    fn mul(self, rhs: Color) -> Self::Output {
        Color::new(self * rhs.r, self * rhs.g, self * rhs.b, self * rhs.a)
    }
}

impl std::ops::Mul<Color> for Color {
    type Output = Color;

    fn mul(self, rhs: Color) -> Self::Output {
        Color::new(
            self.r * rhs.r,
            self.g * rhs.g,
            self.b * rhs.b,
            self.a * rhs.a,
        )
    }
}

impl std::ops::Div<f32> for Color {
    type Output = Color;

    fn div(self, rhs: f32) -> Self::Output {
        Color::new(self.r / rhs, self.g / rhs, self.b / rhs, self.a / rhs)
    }
}

impl std::ops::Div<Color> for f32 {
    type Output = Color;

    fn div(self, rhs: Color) -> Self::Output {
        Color::new(self / rhs.r, self / rhs.g, self / rhs.b, self / rhs.a)
    }
}

impl std::ops::Div<Color> for Color {
    type Output = Color;

    fn div(self, rhs: Color) -> Self::Output {
        Color::new(
            self.r / rhs.r,
            self.g / rhs.g,
            self.b / rhs.b,
            self.a / rhs.a,
        )
    }
}
