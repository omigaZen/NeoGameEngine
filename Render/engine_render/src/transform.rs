#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    cols: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub const fn from_cols_array(cols: [[f32; 4]; 4]) -> Self {
        Self { cols }
    }

    pub const fn to_cols_array(self) -> [[f32; 4]; 4] {
        self.cols
    }

    pub fn translation(translation: [f32; 3]) -> Self {
        let [x, y, z] = translation;

        Self::from_cols_array([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ])
    }

    pub fn scale(scale: [f32; 3]) -> Self {
        let [x, y, z] = scale;

        Self::from_cols_array([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_quaternion(rotation: [f32; 4]) -> Self {
        let [mut x, mut y, mut z, mut w] = rotation;
        let length = (x * x + y * y + z * z + w * w).sqrt();
        if length <= f32::EPSILON {
            return Self::IDENTITY;
        }
        x /= length;
        y /= length;
        z /= length;
        w /= length;

        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        Self::from_cols_array([
            [1.0 - yy - zz, xy + wz, xz - wy, 0.0],
            [xy - wz, 1.0 - xx - zz, yz + wx, 0.0],
            [xz + wy, yz - wx, 1.0 - xx - yy, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_x(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();

        Self::from_cols_array([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, cos, sin, 0.0],
            [0.0, -sin, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_y(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();

        Self::from_cols_array([
            [cos, 0.0, -sin, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [sin, 0.0, cos, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_z(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();

        Self::from_cols_array([
            [cos, sin, 0.0, 0.0],
            [-sin, cos, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let width = right - left;
        let height = top - bottom;
        let depth = far - near;

        Self::from_cols_array([
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0 / depth, 0.0],
            [
                -(right + left) / width,
                -(top + bottom) / height,
                -near / depth,
                1.0,
            ],
        ])
    }

    pub fn perspective_rh(
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let f = 1.0 / (vertical_fov_radians.max(0.0001) * 0.5).tan();
        let aspect_ratio = aspect_ratio.max(0.0001);
        let depth = near - far;

        Self::from_cols_array([
            [f / aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / depth, -1.0],
            [0.0, 0.0, (near * far) / depth, 0.0],
        ])
    }

    pub fn perspective_infinite_rh(
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near: f32,
    ) -> Self {
        let f = 1.0 / (vertical_fov_radians.max(0.0001) * 0.5).tan();
        let aspect_ratio = aspect_ratio.max(0.0001);
        let near = near.max(0.0001);

        Self::from_cols_array([
            [f / aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, -1.0, -1.0],
            [0.0, 0.0, -near, 0.0],
        ])
    }

    pub fn transform_point3(self, point: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = point;
        let [tx, ty, tz, tw] = self.transform_point4([x, y, z, 1.0]);

        if tw.abs() > f32::EPSILON {
            [tx / tw, ty / tw, tz / tw]
        } else {
            [tx, ty, tz]
        }
    }

    pub fn transform_point4(self, point: [f32; 4]) -> [f32; 4] {
        let [x, y, z, w] = point;

        [
            self.cols[0][0] * x + self.cols[1][0] * y + self.cols[2][0] * z + self.cols[3][0] * w,
            self.cols[0][1] * x + self.cols[1][1] * y + self.cols[2][1] * z + self.cols[3][1] * w,
            self.cols[0][2] * x + self.cols[1][2] * y + self.cols[2][2] * z + self.cols[3][2] * w,
            self.cols[0][3] * x + self.cols[1][3] * y + self.cols[2][3] * z + self.cols[3][3] * w,
        ]
    }

    pub fn transform_vector3(self, vector: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = vector;

        [
            self.cols[0][0] * x + self.cols[1][0] * y + self.cols[2][0] * z,
            self.cols[0][1] * x + self.cols[1][1] * y + self.cols[2][1] * z,
            self.cols[0][2] * x + self.cols[1][2] * y + self.cols[2][2] * z,
        ]
    }

    pub fn normal_matrix(self) -> Self {
        let m00 = self.cols[0][0];
        let m01 = self.cols[1][0];
        let m02 = self.cols[2][0];
        let m10 = self.cols[0][1];
        let m11 = self.cols[1][1];
        let m12 = self.cols[2][1];
        let m20 = self.cols[0][2];
        let m21 = self.cols[1][2];
        let m22 = self.cols[2][2];

        let c00 = m11 * m22 - m12 * m21;
        let c01 = -(m10 * m22 - m12 * m20);
        let c02 = m10 * m21 - m11 * m20;
        let c10 = -(m01 * m22 - m02 * m21);
        let c11 = m00 * m22 - m02 * m20;
        let c12 = -(m00 * m21 - m01 * m20);
        let c20 = m01 * m12 - m02 * m11;
        let c21 = -(m00 * m12 - m02 * m10);
        let c22 = m00 * m11 - m01 * m10;
        let det = m00 * c00 + m01 * c01 + m02 * c02;
        let inv_det = if det.abs() > f32::EPSILON {
            1.0 / det
        } else {
            0.0
        };

        Self::from_cols_array([
            [c00 * inv_det, c10 * inv_det, c20 * inv_det, 0.0],
            [c01 * inv_det, c11 * inv_det, c21 * inv_det, 0.0],
            [c02 * inv_det, c12 * inv_det, c22 * inv_det, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn mul_mat4(self, rhs: Self) -> Self {
        let mut cols = [[0.0; 4]; 4];

        for col in 0..4 {
            for row in 0..4 {
                cols[col][row] = self.cols[0][row] * rhs.cols[col][0]
                    + self.cols[1][row] * rhs.cols[col][1]
                    + self.cols[2][row] * rhs.cols[col][2]
                    + self.cols[3][row] * rhs.cols[col][3];
            }
        }

        Self::from_cols_array(cols)
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.mul_mat4(rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    pub rotation_x_radians: f32,
    pub rotation_y_radians: f32,
    pub rotation_z_radians: f32,
    pub scale: [f32; 3],
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0, 0.0],
        rotation_x_radians: 0.0,
        rotation_y_radians: 0.0,
        rotation_z_radians: 0.0,
        scale: [1.0, 1.0, 1.0],
    };

    pub const fn new(translation: [f32; 3], rotation_z_radians: f32, scale: [f32; 3]) -> Self {
        Self {
            translation,
            rotation_x_radians: 0.0,
            rotation_y_radians: 0.0,
            rotation_z_radians,
            scale,
        }
    }

    pub const fn new_3d(
        translation: [f32; 3],
        rotation_radians: [f32; 3],
        scale: [f32; 3],
    ) -> Self {
        Self {
            translation,
            rotation_x_radians: rotation_radians[0],
            rotation_y_radians: rotation_radians[1],
            rotation_z_radians: rotation_radians[2],
            scale,
        }
    }

    pub fn to_matrix(self) -> Mat4 {
        Mat4::translation(self.translation)
            * Mat4::rotation_z(self.rotation_z_radians)
            * Mat4::rotation_y(self.rotation_y_radians)
            * Mat4::rotation_x(self.rotation_x_radians)
            * Mat4::scale(self.scale)
    }

    pub fn normal_matrix(self) -> Mat4 {
        Mat4::rotation_z(self.rotation_z_radians)
            * Mat4::rotation_y(self.rotation_y_radians)
            * Mat4::rotation_x(self.rotation_x_radians)
            * Mat4::scale([
                inverse_scale(self.scale[0]),
                inverse_scale(self.scale[1]),
                inverse_scale(self.scale[2]),
            ])
    }
}

fn inverse_scale(value: f32) -> f32 {
    if value.abs() > f32::EPSILON {
        1.0 / value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_normal_matrix_uses_inverse_scale_without_translation() {
        let transform = Transform::new_3d([10.0, 20.0, 30.0], [0.0, 0.0, 0.0], [2.0, 4.0, 8.0]);
        let normal = transform.normal_matrix().transform_vector3([0.0, 1.0, 0.0]);

        assert_eq!(normal, [0.0, 0.25, 0.0]);
    }

    #[test]
    fn matrix_normal_matrix_matches_transform_for_trs() {
        let transform = Transform::new_3d([3.0, 4.0, 5.0], [0.25, -0.5, 0.75], [2.0, 4.0, 8.0]);
        let expected = transform.normal_matrix().to_cols_array();
        let actual = transform.to_matrix().normal_matrix().to_cols_array();

        for col in 0..4 {
            for row in 0..4 {
                assert!((actual[col][row] - expected[col][row]).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn quaternion_rotation_matches_z_axis_rotation() {
        let half_angle = std::f32::consts::FRAC_PI_2 * 0.5;
        let rotation = Mat4::rotation_quaternion([0.0, 0.0, half_angle.sin(), half_angle.cos()]);
        let rotated = rotation.transform_vector3([1.0, 0.0, 0.0]);

        assert!(rotated[0].abs() < 0.0001);
        assert!((rotated[1] - 1.0).abs() < 0.0001);
        assert!(rotated[2].abs() < 0.0001);
    }
}
