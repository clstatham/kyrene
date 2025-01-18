use encase::ShaderType;

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Vec3 {
    value: glam::Vec3,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            value: glam::Vec3::new(x, y, z),
        }
    }

    pub fn x(&self) -> f32 {
        self.value.x
    }

    pub fn y(&self) -> f32 {
        self.value.y
    }

    pub fn z(&self) -> f32 {
        self.value.z
    }

    pub fn set_x(&mut self, x: f32) {
        self.value.x = x;
    }

    pub fn set_y(&mut self, y: f32) {
        self.value.y = y;
    }

    pub fn set_z(&mut self, z: f32) {
        self.value.z = z;
    }

    pub fn length(&self) -> f32 {
        self.value.length()
    }

    pub fn normalize(&mut self) {
        self.value = self.value.normalize();
    }

    pub fn dot(&self, other: Self) -> f32 {
        self.value.dot(other.value)
    }

    pub fn cross(&self, other: Self) -> Self {
        Self {
            value: self.value.cross(other.value),
        }
    }

    pub fn lerp(&self, other: Self, t: f32) -> Self {
        Self {
            value: self.value.lerp(other.value, t),
        }
    }

    pub fn transform(&self, transform: Transform) -> Self {
        transform.transform_vector(*self)
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
        }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            value: self.value * rhs,
        }
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self {
        Self {
            value: self.value / rhs,
        }
    }
}

impl std::ops::Add<Point3> for Vec3 {
    type Output = Point3;

    fn add(self, rhs: Point3) -> Point3 {
        Point3 {
            value: self.value + rhs.value,
        }
    }
}

impl std::ops::Sub<Point3> for Vec3 {
    type Output = Point3;

    fn sub(self, rhs: Point3) -> Point3 {
        Point3 {
            value: self.value - rhs.value,
        }
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self { value: -self.value }
    }
}

impl std::ops::Mul for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
        }
    }
}

impl std::ops::Div for Vec3 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            value: self.value / rhs.value,
        }
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value;
    }
}

impl std::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.value -= rhs.value;
    }
}

impl std::ops::MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        self.value *= rhs;
    }
}

impl std::ops::DivAssign<f32> for Vec3 {
    fn div_assign(&mut self, rhs: f32) {
        self.value /= rhs;
    }
}

impl std::ops::Mul<Transform> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Transform) -> Self {
        rhs.transform_vector(self)
    }
}

impl From<Vec3> for glam::Vec3 {
    fn from(val: Vec3) -> Self {
        val.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Point3 {
    value: glam::Vec3,
}

impl Point3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            value: glam::Vec3::new(x, y, z),
        }
    }

    pub fn x(&self) -> f32 {
        self.value.x
    }

    pub fn y(&self) -> f32 {
        self.value.y
    }

    pub fn z(&self) -> f32 {
        self.value.z
    }

    pub fn set_x(&mut self, x: f32) {
        self.value.x = x;
    }

    pub fn set_y(&mut self, y: f32) {
        self.value.y = y;
    }

    pub fn set_z(&mut self, z: f32) {
        self.value.z = z;
    }

    pub fn distance(&self, other: Self) -> f32 {
        self.value.distance(other.value)
    }

    pub fn transform(&self, transform: Transform) -> Self {
        transform.transform_point(*self)
    }
}

impl Default for Point3 {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

impl std::ops::Add<Vec3> for Point3 {
    type Output = Self;

    fn add(self, rhs: Vec3) -> Self {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl std::ops::Sub<Vec3> for Point3 {
    type Output = Self;

    fn sub(self, rhs: Vec3) -> Self {
        Self {
            value: self.value - rhs.value,
        }
    }
}

impl std::ops::Sub for Point3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
        }
    }
}

impl std::ops::Mul<f32> for Point3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            value: self.value * rhs,
        }
    }
}

impl std::ops::Div<f32> for Point3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self {
        Self {
            value: self.value / rhs,
        }
    }
}

impl std::ops::AddAssign<Vec3> for Point3 {
    fn add_assign(&mut self, rhs: Vec3) {
        self.value += rhs.value;
    }
}

impl std::ops::SubAssign<Vec3> for Point3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        self.value -= rhs.value;
    }
}

impl std::ops::MulAssign<f32> for Point3 {
    fn mul_assign(&mut self, rhs: f32) {
        self.value *= rhs;
    }
}

impl std::ops::DivAssign<f32> for Point3 {
    fn div_assign(&mut self, rhs: f32) {
        self.value /= rhs;
    }
}

impl std::ops::Neg for Point3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self { value: -self.value }
    }
}

impl std::ops::Mul for Point3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
        }
    }
}

impl std::ops::Div for Point3 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            value: self.value / rhs.value,
        }
    }
}

impl std::ops::Mul<Transform> for Point3 {
    type Output = Self;

    fn mul(self, rhs: Transform) -> Self {
        rhs.transform_point(self)
    }
}

impl From<Point3> for glam::Vec3 {
    fn from(val: Point3) -> Self {
        val.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    value: glam::Quat,
}

impl Quat {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            value: glam::Quat::from_xyzw(x, y, z, w),
        }
    }

    pub fn identity() -> Self {
        Self {
            value: glam::Quat::IDENTITY,
        }
    }

    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        Self {
            value: glam::Quat::from_axis_angle(axis.value, angle),
        }
    }

    pub fn from_euler(euler: Vec3) -> Self {
        Self {
            value: glam::Quat::from_euler(
                glam::EulerRot::YXZ,
                euler.value.x,
                euler.value.y,
                euler.value.z,
            ),
        }
    }

    pub fn from_rotation_x(angle: f32) -> Self {
        Self {
            value: glam::Quat::from_rotation_x(angle),
        }
    }

    pub fn from_rotation_y(angle: f32) -> Self {
        Self {
            value: glam::Quat::from_rotation_y(angle),
        }
    }

    pub fn from_rotation_z(angle: f32) -> Self {
        Self {
            value: glam::Quat::from_rotation_z(angle),
        }
    }

    pub fn angle(&self) -> f32 {
        self.value.to_axis_angle().1
    }

    pub fn axis(&self) -> Vec3 {
        Vec3 {
            value: self.value.to_axis_angle().0,
        }
    }

    pub fn euler(&self) -> Vec3 {
        let (x, y, z) = self.value.to_euler(glam::EulerRot::YXZ);
        Vec3::new(x, y, z)
    }

    pub fn normalize(&mut self) {
        self.value = self.value.normalize();
    }

    pub fn conjugate(&self) -> Self {
        Self {
            value: self.value.conjugate(),
        }
    }

    pub fn inverse(&self) -> Self {
        Self {
            value: self.value.inverse(),
        }
    }

    pub fn slerp(&self, other: Self, t: f32) -> Self {
        Self {
            value: self.value.slerp(other.value, t),
        }
    }

    pub fn transform(&self, transform: Transform) -> Self {
        transform.rotation() * *self
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::ops::Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
        }
    }
}

impl std::ops::MulAssign for Quat {
    fn mul_assign(&mut self, rhs: Self) {
        self.value *= rhs.value;
    }
}

impl std::ops::Mul<Vec3> for Quat {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            value: self.value * rhs.value,
        }
    }
}

impl From<Quat> for glam::Quat {
    fn from(val: Quat) -> Self {
        val.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Transform {
    value: glam::Mat4,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            value: glam::Mat4::IDENTITY,
        }
    }

    pub fn from_mat4(mat: glam::Mat4) -> Self {
        Self { value: mat }
    }

    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            value: glam::Mat4::from_translation(translation.value),
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            value: glam::Mat4::from_quat(rotation.value),
        }
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            value: glam::Mat4::from_scale(scale.value),
        }
    }

    pub fn from_euler(euler: Vec3) -> Self {
        Self {
            value: glam::Mat4::from_euler(
                glam::EulerRot::YXZ,
                euler.value.x,
                euler.value.y,
                euler.value.z,
            ),
        }
    }

    pub fn translation(&self) -> Vec3 {
        let (t, _, _) = self.value.to_scale_rotation_translation();
        Vec3 { value: t }
    }

    pub fn rotation(&self) -> Quat {
        let (_, r, _) = self.value.to_scale_rotation_translation();
        Quat { value: r }
    }

    pub fn scale(&self) -> Vec3 {
        let (_, _, s) = self.value.to_scale_rotation_translation();
        Vec3 { value: s }
    }

    pub fn transform_vector(&self, vec: Vec3) -> Vec3 {
        Vec3 {
            value: self.value.transform_vector3(vec.value),
        }
    }

    pub fn transform_point(&self, point: Point3) -> Point3 {
        Point3 {
            value: self.value.transform_point3(point.value),
        }
    }

    pub fn look_at(eye: Point3, target: Point3, up: Vec3) -> Self {
        Self {
            value: glam::Mat4::look_at_rh(eye.value, target.value, up.value),
        }
    }

    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            value: glam::Mat4::perspective_rh(fov, aspect, near, far),
        }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self {
            value: glam::Mat4::orthographic_rh(left, right, bottom, top, near, far),
        }
    }

    pub fn inverse(&self) -> Self {
        Self {
            value: self.value.inverse(),
        }
    }

    pub fn transpose(&self) -> Self {
        Self {
            value: self.value.transpose(),
        }
    }

    pub fn inverse_transpose(&self) -> Self {
        Self {
            value: self.value.inverse().transpose(),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Mul for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
        }
    }
}

impl std::ops::MulAssign for Transform {
    fn mul_assign(&mut self, rhs: Self) {
        self.value *= rhs.value;
    }
}

impl std::ops::Mul<Vec3> for Transform {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            value: self.value.transform_vector3(rhs.value),
        }
    }
}

impl std::ops::Mul<Point3> for Transform {
    type Output = Point3;

    fn mul(self, rhs: Point3) -> Point3 {
        Point3 {
            value: self.value.transform_point3(rhs.value),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<glam::Mat4> for Transform {
    fn into(self) -> glam::Mat4 {
        self.value
    }
}

impl From<glam::Mat4> for Transform {
    fn from(mat: glam::Mat4) -> Self {
        Self { value: mat }
    }
}
