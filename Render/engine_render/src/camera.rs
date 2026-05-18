use crate::{Mat4, MeshBounds, Transform};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Camera {
    Orthographic(OrthographicCamera),
    Perspective(PerspectiveCamera),
    View(ViewCamera),
}

impl Camera {
    pub fn view_projection(self, aspect_ratio: f32) -> Mat4 {
        match self {
            Self::Orthographic(camera) => camera.view_projection(aspect_ratio),
            Self::Perspective(camera) => camera.view_projection(aspect_ratio),
            Self::View(camera) => camera.view_projection(),
        }
    }

    pub fn position(self) -> [f32; 3] {
        match self {
            Self::Orthographic(camera) => camera.position,
            Self::Perspective(camera) => camera.position,
            Self::View(camera) => camera.position,
        }
    }

    pub fn basis(self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        match self {
            Self::Orthographic(camera) => camera.basis(),
            Self::Perspective(camera) => camera.basis(),
            Self::View(camera) => (camera.right, camera.up, camera.forward),
        }
    }

    pub fn skybox_projection(self, surface_aspect_ratio: f32) -> (f32, f32) {
        let surface_aspect_ratio = surface_aspect_ratio.max(0.0001);
        match self {
            Self::Perspective(camera) => (
                (camera.vertical_fov_radians.max(0.0001) * 0.5).tan(),
                surface_aspect_ratio,
            ),
            Self::Orthographic(_) => (0.0, surface_aspect_ratio),
            Self::View(camera) => match camera.projection_kind {
                ViewCameraProjection::Perspective {
                    vertical_fov_radians,
                    aspect_ratio,
                } => (
                    (vertical_fov_radians.max(0.0001) * 0.5).tan(),
                    aspect_ratio.max(0.0001),
                ),
                ViewCameraProjection::Orthographic => (0.0, surface_aspect_ratio),
            },
        }
    }

    pub fn transparent_sort_depth(self, world_position: [f32; 3]) -> f32 {
        match self {
            Self::Orthographic(camera) => camera.view_matrix().transform_point3(world_position)[2],
            Self::Perspective(camera) => -camera.view_matrix().transform_point3(world_position)[2],
            Self::View(camera) => camera.transparent_sort_depth(world_position),
        }
    }

    pub fn contains_bounds(
        self,
        bounds: MeshBounds,
        transform: Transform,
        aspect_ratio: f32,
    ) -> bool {
        self.contains_bounds_matrix(bounds, transform.to_matrix(), aspect_ratio)
    }

    pub fn contains_bounds_matrix(
        self,
        bounds: MeshBounds,
        model_matrix: Mat4,
        aspect_ratio: f32,
    ) -> bool {
        let matrix = self.view_projection(aspect_ratio) * model_matrix;
        let corners = bounds.corners();
        let clip_corners =
            corners.map(|corner| matrix.transform_point4([corner[0], corner[1], corner[2], 1.0]));

        !outside_clip_plane(&clip_corners, |[x, _, _, w]| x < -w)
            && !outside_clip_plane(&clip_corners, |[x, _, _, w]| x > w)
            && !outside_clip_plane(&clip_corners, |[_, y, _, w]| y < -w)
            && !outside_clip_plane(&clip_corners, |[_, y, _, w]| y > w)
            && !outside_clip_plane(&clip_corners, |[_, _, z, _]| z < 0.0)
            && !outside_clip_plane(&clip_corners, |[_, _, z, w]| z > w)
    }
}

fn outside_clip_plane(corners: &[[f32; 4]; 8], outside: impl Fn([f32; 4]) -> bool) -> bool {
    corners.iter().copied().all(outside)
}

impl From<OrthographicCamera> for Camera {
    fn from(camera: OrthographicCamera) -> Self {
        Self::Orthographic(camera)
    }
}

impl From<PerspectiveCamera> for Camera {
    fn from(camera: PerspectiveCamera) -> Self {
        Self::Perspective(camera)
    }
}

impl From<ViewCamera> for Camera {
    fn from(camera: ViewCamera) -> Self {
        Self::View(camera)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::Orthographic(OrthographicCamera::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrthographicCamera {
    pub position: [f32; 3],
    pub rotation_z_radians: f32,
    pub viewport_height: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicCamera {
    pub const fn new_2d(viewport_height: f32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation_z_radians: 0.0,
            viewport_height,
            near: -1.0,
            far: 1.0,
        }
    }

    pub fn view_projection(self, aspect_ratio: f32) -> Mat4 {
        let aspect_ratio = aspect_ratio.max(0.0001);
        let half_height = self.viewport_height.max(0.0001) * 0.5;
        let half_width = half_height * aspect_ratio;
        let projection = Mat4::orthographic(
            -half_width,
            half_width,
            -half_height,
            half_height,
            self.near,
            self.far,
        );

        projection * self.view_matrix()
    }

    fn view_matrix(self) -> Mat4 {
        Mat4::rotation_z(-self.rotation_z_radians)
            * Mat4::translation([-self.position[0], -self.position[1], -self.position[2]])
    }

    pub fn basis(self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let rotation = Mat4::rotation_z(self.rotation_z_radians);
        (
            normalize_or(rotation.transform_vector3([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]),
            normalize_or(rotation.transform_vector3([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]),
            [0.0, 0.0, -1.0],
        )
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self::new_2d(2.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveCamera {
    pub position: [f32; 3],
    pub rotation_radians: [f32; 3],
    pub vertical_fov_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    pub const fn new(vertical_fov_radians: f32, near: f32, far: f32) -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation_radians: [0.0, 0.0, 0.0],
            vertical_fov_radians,
            near,
            far,
        }
    }

    pub fn view_projection(self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.vertical_fov_radians,
            aspect_ratio,
            self.near.max(0.0001),
            self.far.max(self.near + 0.0001),
        ) * self.view_matrix()
    }

    fn view_matrix(self) -> Mat4 {
        Mat4::rotation_x(-self.rotation_radians[0])
            * Mat4::rotation_y(-self.rotation_radians[1])
            * Mat4::rotation_z(-self.rotation_radians[2])
            * Mat4::translation([-self.position[0], -self.position[1], -self.position[2]])
    }

    pub fn basis(self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let rotation = Mat4::rotation_z(self.rotation_radians[2])
            * Mat4::rotation_y(self.rotation_radians[1])
            * Mat4::rotation_x(self.rotation_radians[0]);
        (
            normalize_or(rotation.transform_vector3([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]),
            normalize_or(rotation.transform_vector3([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]),
            normalize_or(
                rotation.transform_vector3([0.0, 0.0, -1.0]),
                [0.0, 0.0, -1.0],
            ),
        )
    }
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self::new(std::f32::consts::FRAC_PI_3, 0.1, 100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewCameraProjection {
    Perspective {
        vertical_fov_radians: f32,
        aspect_ratio: f32,
    },
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewCamera {
    pub position: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    pub projection: Mat4,
    pub projection_kind: ViewCameraProjection,
}

impl ViewCamera {
    pub fn perspective(
        position: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        forward: [f32; 3],
        vertical_fov_radians: f32,
        aspect_ratio: f32,
        near: f32,
        far: Option<f32>,
    ) -> Self {
        let aspect_ratio = aspect_ratio.max(0.0001);
        let near = near.max(0.0001);
        let projection = far.map_or_else(
            || Mat4::perspective_infinite_rh(vertical_fov_radians, aspect_ratio, near),
            |far| Mat4::perspective_rh(vertical_fov_radians, aspect_ratio, near, far),
        );

        Self::new(
            position,
            right,
            up,
            forward,
            projection,
            ViewCameraProjection::Perspective {
                vertical_fov_radians,
                aspect_ratio,
            },
        )
    }

    pub fn orthographic(
        position: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        forward: [f32; 3],
        xmag: f32,
        ymag: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let projection = Mat4::orthographic(
            -xmag.abs().max(0.0001),
            xmag.abs().max(0.0001),
            -ymag.abs().max(0.0001),
            ymag.abs().max(0.0001),
            near,
            far.max(near + 0.0001),
        );

        Self::new(
            position,
            right,
            up,
            forward,
            projection,
            ViewCameraProjection::Orthographic,
        )
    }

    pub fn view_matrix(self) -> Mat4 {
        Mat4::from_cols_array([
            [self.right[0], self.up[0], -self.forward[0], 0.0],
            [self.right[1], self.up[1], -self.forward[1], 0.0],
            [self.right[2], self.up[2], -self.forward[2], 0.0],
            [
                -dot(self.right, self.position),
                -dot(self.up, self.position),
                dot(self.forward, self.position),
                1.0,
            ],
        ])
    }

    pub fn view_projection(self) -> Mat4 {
        self.projection * self.view_matrix()
    }

    pub fn transparent_sort_depth(self, world_position: [f32; 3]) -> f32 {
        dot(
            [
                world_position[0] - self.position[0],
                world_position[1] - self.position[1],
                world_position[2] - self.position[2],
            ],
            self.forward,
        )
    }

    fn new(
        position: [f32; 3],
        right: [f32; 3],
        up: [f32; 3],
        forward: [f32; 3],
        projection: Mat4,
        projection_kind: ViewCameraProjection,
    ) -> Self {
        Self {
            position,
            right: normalize_or(right, [1.0, 0.0, 0.0]),
            up: normalize_or(up, [0.0, 1.0, 0.0]),
            forward: normalize_or(forward, [0.0, 0.0, -1.0]),
            projection,
            projection_kind,
        }
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length_squared = value[0] * value[0] + value[1] * value[1] + value[2] * value[2];
    if length_squared > f32::EPSILON {
        let inverse_length = 1.0 / length_squared.sqrt();
        [
            value[0] * inverse_length,
            value[1] * inverse_length,
            value[2] * inverse_length,
        ]
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perspective_camera_maps_near_and_far_into_wgpu_depth_range() {
        let camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_2, 0.1, 10.0);
        let projection = camera.view_projection(1.0);

        let near = projection.transform_point3([0.0, 0.0, -0.1]);
        let far = projection.transform_point3([0.0, 0.0, -10.0]);

        assert!((near[2] - 0.0).abs() < 0.0001);
        assert!((far[2] - 1.0).abs() < 0.0001);
    }

    #[test]
    fn perspective_transparent_sort_depth_increases_away_from_camera() {
        let mut camera = PerspectiveCamera::default();
        camera.position = [0.0, 0.0, 4.0];
        let camera = Camera::from(camera);

        assert!(
            camera.transparent_sort_depth([0.0, 0.0, -3.0])
                > camera.transparent_sort_depth([0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn camera_position_returns_active_camera_position() {
        let mut orthographic = OrthographicCamera::default();
        orthographic.position = [1.0, 2.0, 3.0];
        let mut perspective = PerspectiveCamera::default();
        perspective.position = [-1.0, -2.0, -3.0];

        assert_eq!(Camera::from(orthographic).position(), [1.0, 2.0, 3.0]);
        assert_eq!(Camera::from(perspective).position(), [-1.0, -2.0, -3.0]);
    }

    #[test]
    fn view_camera_maps_basis_and_depth_from_supplied_orientation() {
        let camera = ViewCamera::perspective(
            [1.0, 2.0, 3.0],
            [0.0, 0.0, -2.0],
            [0.0, 4.0, 0.0],
            [-3.0, 0.0, 0.0],
            std::f32::consts::FRAC_PI_2,
            2.0,
            0.1,
            None,
        );
        let camera = Camera::from(camera);
        let (right, up, forward) = camera.basis();

        assert_eq!(camera.position(), [1.0, 2.0, 3.0]);
        assert_vec3_close(right, [0.0, 0.0, -1.0]);
        assert_vec3_close(up, [0.0, 1.0, 0.0]);
        assert_vec3_close(forward, [-1.0, 0.0, 0.0]);
        assert!((camera.transparent_sort_depth([-4.0, 2.0, 3.0]) - 5.0).abs() < 0.0001);

        let projected = camera
            .view_projection(1.0)
            .transform_point3([0.9, 2.0, 3.0]);
        assert!(projected[2].abs() < 0.0001);
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for index in 0..3 {
            assert!((actual[index] - expected[index]).abs() < 0.0001);
        }
    }
}
