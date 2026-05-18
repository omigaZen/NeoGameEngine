use std::fmt;

use crate::{
    BlendMode, Camera, ColoredVertex, DirectionalLight, Mat4, Material, Mesh, PointLight,
    RenderLighting, SpotLight, TextureAddressMode, TextureFilterMode, TextureSampler,
    TextureTransform, ViewCamera,
};

const GLTF_UNBOUNDED_LIGHT_RANGE: f32 = 1000.0;
const SUPPORTED_GLTF_ASSET_VERSION: (usize, usize) = (2, 0);
const SUPPORTED_GLTF_EXTENSIONS: &[&str] = &[
    "EXT_mesh_gpu_instancing",
    "EXT_texture_webp",
    "KHR_lights_punctual",
    "KHR_materials_anisotropy",
    "KHR_materials_clearcoat",
    "KHR_materials_dispersion",
    "KHR_materials_emissive_strength",
    "KHR_materials_ior",
    "KHR_materials_iridescence",
    "KHR_materials_pbrSpecularGlossiness",
    "KHR_materials_sheen",
    "KHR_materials_specular",
    "KHR_materials_transmission",
    "KHR_materials_unlit",
    "KHR_materials_volume",
    "KHR_mesh_quantization",
    "KHR_texture_basisu",
    "KHR_texture_transform",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GltfLoadError {
    Json(String),
    InvalidGlb(&'static str),
    MissingField(&'static str),
    InvalidField(&'static str),
    Unsupported(&'static str),
    UnsupportedRequiredExtension(String),
    BufferNotFound(String),
    BufferOutOfBounds,
    TooManyVertices,
}

impl fmt::Display for GltfLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(reason) => write!(f, "invalid glTF JSON: {reason}"),
            Self::InvalidGlb(reason) => write!(f, "invalid GLB data: {reason}"),
            Self::MissingField(field) => write!(f, "glTF is missing required field '{field}'"),
            Self::InvalidField(field) => write!(f, "glTF field '{field}' is invalid"),
            Self::Unsupported(reason) => write!(f, "unsupported glTF feature: {reason}"),
            Self::UnsupportedRequiredExtension(extension) => {
                write!(f, "required glTF extension '{extension}' is not supported")
            }
            Self::BufferNotFound(uri) => write!(f, "glTF buffer '{uri}' was not resolved"),
            Self::BufferOutOfBounds => write!(f, "glTF buffer view extends past buffer data"),
            Self::TooManyVertices => write!(f, "glTF mesh has more than u32::MAX vertices"),
        }
    }
}

impl std::error::Error for GltfLoadError {}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfImageData {
    pub label: String,
    pub mime_type: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfMaterial {
    pub material: Material,
    pub base_color_texture_path: Option<String>,
    pub base_color_texture_data: Option<GltfImageData>,
    pub metallic_roughness_texture_path: Option<String>,
    pub metallic_roughness_texture_data: Option<GltfImageData>,
    pub normal_texture_path: Option<String>,
    pub normal_texture_data: Option<GltfImageData>,
    pub emissive_texture_path: Option<String>,
    pub emissive_texture_data: Option<GltfImageData>,
    pub occlusion_texture_path: Option<String>,
    pub occlusion_texture_data: Option<GltfImageData>,
    pub clearcoat_texture_path: Option<String>,
    pub clearcoat_texture_data: Option<GltfImageData>,
    pub clearcoat_roughness_texture_path: Option<String>,
    pub clearcoat_roughness_texture_data: Option<GltfImageData>,
    pub clearcoat_normal_texture_path: Option<String>,
    pub clearcoat_normal_texture_data: Option<GltfImageData>,
    pub sheen_color_texture_path: Option<String>,
    pub sheen_color_texture_data: Option<GltfImageData>,
    pub sheen_roughness_texture_path: Option<String>,
    pub sheen_roughness_texture_data: Option<GltfImageData>,
    pub transmission_texture_path: Option<String>,
    pub transmission_texture_data: Option<GltfImageData>,
    pub specular_texture_path: Option<String>,
    pub specular_texture_data: Option<GltfImageData>,
    pub specular_color_texture_path: Option<String>,
    pub specular_color_texture_data: Option<GltfImageData>,
    pub anisotropy_texture_path: Option<String>,
    pub anisotropy_texture_data: Option<GltfImageData>,
    pub iridescence_texture_path: Option<String>,
    pub iridescence_texture_data: Option<GltfImageData>,
    pub iridescence_thickness_texture_path: Option<String>,
    pub iridescence_thickness_texture_data: Option<GltfImageData>,
    pub thickness_texture_path: Option<String>,
    pub thickness_texture_data: Option<GltfImageData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfPunctualLightKind {
    Directional,
    Point,
    Spot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GltfPunctualLight {
    pub node_index: usize,
    pub kind: GltfPunctualLightKind,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: Option<f32>,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub transform: Mat4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GltfCameraProjection {
    Perspective {
        aspect_ratio: Option<f32>,
        vertical_fov_radians: f32,
        near: f32,
        far: Option<f32>,
    },
    Orthographic {
        xmag: f32,
        ymag: f32,
        near: f32,
        far: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GltfCamera {
    pub node_index: usize,
    pub camera_index: usize,
    pub projection: GltfCameraProjection,
    pub position: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    pub transform: Mat4,
}

impl GltfCamera {
    pub fn to_camera(self, fallback_aspect_ratio: f32) -> Camera {
        match self.projection {
            GltfCameraProjection::Perspective {
                aspect_ratio,
                vertical_fov_radians,
                near,
                far,
            } => Camera::from(ViewCamera::perspective(
                self.position,
                self.right,
                self.up,
                self.forward,
                vertical_fov_radians,
                aspect_ratio.unwrap_or(fallback_aspect_ratio),
                near,
                far,
            )),
            GltfCameraProjection::Orthographic {
                xmag,
                ymag,
                near,
                far,
            } => Camera::from(ViewCamera::orthographic(
                self.position,
                self.right,
                self.up,
                self.forward,
                xmag,
                ymag,
                near,
                far,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfPrimitive {
    pub node_index: Option<usize>,
    pub mesh_index: usize,
    pub primitive_index: usize,
    pub material_index: Option<usize>,
    pub model_matrix: Mat4,
    pub morph_target_count: usize,
    pub skin_joint_count: usize,
    pub mesh: Mesh,
    morph_base_vertices: Vec<ColoredVertex>,
    morph_indices: Vec<u32>,
    morph_targets: Vec<GltfMorphTarget>,
    morph_instance_weights: Vec<f32>,
    morph_joints: Option<Vec<[usize; 4]>>,
    morph_weights: Option<Vec<[f32; 4]>>,
    morph_skin_joint_matrices: Option<Vec<Mat4>>,
    skin_joint_node_indices: Vec<usize>,
    skin_inverse_bind_matrices: Vec<Mat4>,
    skin_node_local_matrices: Vec<Mat4>,
    skin_node_parent_indices: Vec<Option<usize>>,
    morph_has_explicit_normals: bool,
    morph_has_explicit_tangents: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfAnimationInterpolation {
    Step,
    Linear,
    CubicSpline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfAnimationPath {
    Translation,
    Rotation,
    Scale,
    Weights,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GltfAnimationOutput {
    Translations(Vec<[f32; 3]>),
    Rotations(Vec<[f32; 4]>),
    Scales(Vec<[f32; 3]>),
    Weights(Vec<Vec<f32>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GltfAnimationValue {
    Translation([f32; 3]),
    Rotation([f32; 4]),
    Scale([f32; 3]),
    Weights(Vec<f32>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfAnimationSampler {
    pub input: Vec<f32>,
    pub output: GltfAnimationOutput,
    pub in_tangents: Option<GltfAnimationOutput>,
    pub out_tangents: Option<GltfAnimationOutput>,
    pub interpolation: GltfAnimationInterpolation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfAnimationChannel {
    pub target_node: usize,
    pub path: GltfAnimationPath,
    pub sampler: GltfAnimationSampler,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfAnimationSample {
    pub target_node: usize,
    pub path: GltfAnimationPath,
    pub value: GltfAnimationValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfAnimation {
    pub name: Option<String>,
    pub duration: f32,
    pub channels: Vec<GltfAnimationChannel>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GltfAnimationLayer {
    pub animation_index: usize,
    pub time: f32,
    pub weight: f32,
    pub speed: f32,
    pub looping: bool,
    pub playing: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GltfAnimationMixer {
    layers: Vec<GltfAnimationLayer>,
}

#[derive(Debug, Clone, PartialEq)]
struct GltfAnimationBlendSlot {
    target_node: usize,
    path: GltfAnimationPath,
    value: GltfAnimationValue,
    weight: f32,
}

impl GltfAnimation {
    pub fn sample(&self, time: f32) -> Vec<GltfAnimationSample> {
        self.channels
            .iter()
            .map(|channel| GltfAnimationSample {
                target_node: channel.target_node,
                path: channel.path,
                value: channel.sampler.sample(time),
            })
            .collect()
    }
}

impl GltfAnimationLayer {
    pub const fn new(animation_index: usize) -> Self {
        Self {
            animation_index,
            time: 0.0,
            weight: 1.0,
            speed: 1.0,
            looping: true,
            playing: true,
        }
    }

    pub const fn with_time(mut self, time: f32) -> Self {
        self.time = time;
        self
    }

    pub const fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub const fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub const fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    pub const fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    pub fn advance(&mut self, animations: &[GltfAnimation], delta_seconds: f32) {
        if !self.playing {
            return;
        }

        self.time += delta_seconds * self.speed;
        let Some(animation) = animations.get(self.animation_index) else {
            return;
        };
        let duration = animation.duration.max(0.0);
        if duration <= f32::EPSILON {
            self.time = 0.0;
            self.playing = false;
            return;
        }

        if self.looping {
            self.time = self.time.rem_euclid(duration);
        } else if self.time >= duration {
            self.time = duration;
            self.playing = false;
        } else if self.time <= 0.0 {
            self.time = 0.0;
            self.playing = false;
        }
    }
}

impl GltfAnimationMixer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_layer(mut self, layer: GltfAnimationLayer) -> Self {
        self.add_layer(layer);
        self
    }

    pub fn add_layer(&mut self, layer: GltfAnimationLayer) {
        self.layers.push(layer);
    }

    pub fn clear_layers(&mut self) {
        self.layers.clear();
    }

    pub fn layers(&self) -> &[GltfAnimationLayer] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut [GltfAnimationLayer] {
        &mut self.layers
    }

    pub fn advance(
        &mut self,
        animations: &[GltfAnimation],
        delta_seconds: f32,
    ) -> Vec<GltfAnimationSample> {
        for layer in &mut self.layers {
            layer.advance(animations, delta_seconds);
        }
        self.sample(animations)
    }

    pub fn sample(&self, animations: &[GltfAnimation]) -> Vec<GltfAnimationSample> {
        blend_animation_layers(animations, &self.layers)
    }
}

fn blend_animation_layers(
    animations: &[GltfAnimation],
    layers: &[GltfAnimationLayer],
) -> Vec<GltfAnimationSample> {
    let mut slots = Vec::<GltfAnimationBlendSlot>::new();

    for layer in layers {
        if !layer.playing {
            continue;
        }
        let weight = layer.weight.clamp(0.0, 1.0);
        if weight <= f32::EPSILON {
            continue;
        }
        let Some(animation) = animations.get(layer.animation_index) else {
            continue;
        };

        for sample in animation.sample(layer.time) {
            push_blended_sample(&mut slots, sample, weight);
        }
    }

    slots
        .into_iter()
        .map(|slot| GltfAnimationSample {
            target_node: slot.target_node,
            path: slot.path,
            value: slot.value,
        })
        .collect()
}

fn push_blended_sample(
    slots: &mut Vec<GltfAnimationBlendSlot>,
    sample: GltfAnimationSample,
    weight: f32,
) {
    if let Some(slot) = slots
        .iter_mut()
        .find(|slot| slot.target_node == sample.target_node && slot.path == sample.path)
    {
        let total_weight = slot.weight + weight;
        if total_weight <= f32::EPSILON {
            return;
        }
        let t = weight / total_weight;
        if let Some(value) = blend_animation_values(&slot.value, &sample.value, t) {
            slot.value = value;
            slot.weight = total_weight;
        }
    } else {
        slots.push(GltfAnimationBlendSlot {
            target_node: sample.target_node,
            path: sample.path,
            value: sample.value,
            weight,
        });
    }
}

fn blend_animation_values(
    a: &GltfAnimationValue,
    b: &GltfAnimationValue,
    t: f32,
) -> Option<GltfAnimationValue> {
    match (a, b) {
        (GltfAnimationValue::Translation(a), GltfAnimationValue::Translation(b)) => {
            Some(GltfAnimationValue::Translation(lerp_vec3(*a, *b, t)))
        }
        (GltfAnimationValue::Rotation(a), GltfAnimationValue::Rotation(b)) => {
            Some(GltfAnimationValue::Rotation(slerp_quaternion(*a, *b, t)))
        }
        (GltfAnimationValue::Scale(a), GltfAnimationValue::Scale(b)) => {
            Some(GltfAnimationValue::Scale(lerp_vec3(*a, *b, t)))
        }
        (GltfAnimationValue::Weights(a), GltfAnimationValue::Weights(b)) => {
            Some(GltfAnimationValue::Weights(lerp_animation_weights(a, b, t)))
        }
        _ => None,
    }
}

impl GltfAnimationSampler {
    pub fn sample(&self, time: f32) -> GltfAnimationValue {
        let (index, next_index, t) = animation_sample_indices(&self.input, time);
        if matches!(self.interpolation, GltfAnimationInterpolation::CubicSpline) {
            return self.sample_cubic(index, next_index, t);
        }

        match &self.output {
            GltfAnimationOutput::Translations(values) => {
                GltfAnimationValue::Translation(lerp_vec3(
                    values[index],
                    values[next_index],
                    linear_sample_t(self.interpolation, t),
                ))
            }
            GltfAnimationOutput::Rotations(values) => {
                GltfAnimationValue::Rotation(slerp_quaternion(
                    values[index],
                    values[next_index],
                    linear_sample_t(self.interpolation, t),
                ))
            }
            GltfAnimationOutput::Scales(values) => GltfAnimationValue::Scale(lerp_vec3(
                values[index],
                values[next_index],
                linear_sample_t(self.interpolation, t),
            )),
            GltfAnimationOutput::Weights(values) => GltfAnimationValue::Weights(lerp_weights(
                &values[index],
                &values[next_index],
                linear_sample_t(self.interpolation, t),
            )),
        }
    }

    fn sample_cubic(&self, index: usize, next_index: usize, t: f32) -> GltfAnimationValue {
        let Some(in_tangents) = &self.in_tangents else {
            return self.sample_linear_fallback(index, next_index, t);
        };
        let Some(out_tangents) = &self.out_tangents else {
            return self.sample_linear_fallback(index, next_index, t);
        };
        let delta_time = self.input[next_index] - self.input[index];

        match (&self.output, in_tangents, out_tangents) {
            (
                GltfAnimationOutput::Translations(values),
                GltfAnimationOutput::Translations(in_tangents),
                GltfAnimationOutput::Translations(out_tangents),
            ) => GltfAnimationValue::Translation(cubic_vec3(
                values[index],
                out_tangents[index],
                in_tangents[next_index],
                values[next_index],
                t,
                delta_time,
            )),
            (
                GltfAnimationOutput::Rotations(values),
                GltfAnimationOutput::Rotations(in_tangents),
                GltfAnimationOutput::Rotations(out_tangents),
            ) => GltfAnimationValue::Rotation(normalize_quaternion(cubic_vec4(
                values[index],
                out_tangents[index],
                in_tangents[next_index],
                values[next_index],
                t,
                delta_time,
            ))),
            (
                GltfAnimationOutput::Scales(values),
                GltfAnimationOutput::Scales(in_tangents),
                GltfAnimationOutput::Scales(out_tangents),
            ) => GltfAnimationValue::Scale(cubic_vec3(
                values[index],
                out_tangents[index],
                in_tangents[next_index],
                values[next_index],
                t,
                delta_time,
            )),
            (
                GltfAnimationOutput::Weights(values),
                GltfAnimationOutput::Weights(in_tangents),
                GltfAnimationOutput::Weights(out_tangents),
            ) => GltfAnimationValue::Weights(cubic_weights(
                &values[index],
                &out_tangents[index],
                &in_tangents[next_index],
                &values[next_index],
                t,
                delta_time,
            )),
            _ => self.sample_linear_fallback(index, next_index, t),
        }
    }

    fn sample_linear_fallback(
        &self,
        index: usize,
        next_index: usize,
        t: f32,
    ) -> GltfAnimationValue {
        match &self.output {
            GltfAnimationOutput::Translations(values) => {
                GltfAnimationValue::Translation(lerp_vec3(values[index], values[next_index], t))
            }
            GltfAnimationOutput::Rotations(values) => {
                GltfAnimationValue::Rotation(slerp_quaternion(values[index], values[next_index], t))
            }
            GltfAnimationOutput::Scales(values) => {
                GltfAnimationValue::Scale(lerp_vec3(values[index], values[next_index], t))
            }
            GltfAnimationOutput::Weights(values) => {
                GltfAnimationValue::Weights(lerp_weights(&values[index], &values[next_index], t))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GltfAsset {
    pub materials: Vec<GltfMaterial>,
    pub primitives: Vec<GltfPrimitive>,
    pub animations: Vec<GltfAnimation>,
    pub lights: Vec<GltfPunctualLight>,
    pub cameras: Vec<GltfCamera>,
}

impl GltfAsset {
    pub fn punctual_lighting(&self) -> Option<RenderLighting> {
        self.punctual_lighting_with_transform(Mat4::IDENTITY)
    }

    pub fn punctual_lighting_with_transform(&self, transform: Mat4) -> Option<RenderLighting> {
        render_lighting_from_gltf_lights(&self.lights, transform)
    }

    pub fn default_camera(&self, fallback_aspect_ratio: f32) -> Option<Camera> {
        self.cameras
            .first()
            .copied()
            .map(|camera| camera.to_camera(fallback_aspect_ratio))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct LoadedGltfPrimitive {
    mesh_index: usize,
    primitive_index: usize,
    material_index: Option<usize>,
    vertices: Vec<ColoredVertex>,
    indices: Vec<u32>,
    joints: Option<Vec<[usize; 4]>>,
    weights: Option<Vec<[f32; 4]>>,
    morph_targets: Vec<GltfMorphTarget>,
    default_morph_weights: Vec<f32>,
    has_explicit_normals: bool,
    has_explicit_tangents: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct GltfMorphTarget {
    positions: Option<Vec<[f32; 3]>>,
    normals: Option<Vec<[f32; 3]>>,
    tangents: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Clone, PartialEq)]
struct GltfSkin {
    joints: Vec<usize>,
    inverse_bind_matrices: Vec<Mat4>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GltfPunctualLightDefinition {
    kind: GltfPunctualLightKind,
    color: [f32; 3],
    intensity: f32,
    range: Option<f32>,
    inner_cone_angle: f32,
    outer_cone_angle: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GltfCameraDefinition {
    projection: GltfCameraProjection,
}

fn render_lighting_from_gltf_lights(
    lights: &[GltfPunctualLight],
    transform: Mat4,
) -> Option<RenderLighting> {
    if lights.is_empty() {
        return None;
    }

    let mut directional = DirectionalLight::new([0.0, -1.0, 0.0], [1.0, 1.0, 1.0], 0.0);
    let mut has_directional = false;
    let mut point_lights = Vec::new();
    let mut spot_lights = Vec::new();

    for light in lights {
        let position = transform.transform_point3(light.position);
        let direction = normalize_or(
            transform.transform_vector3(light.direction),
            light.direction,
        );
        match light.kind {
            GltfPunctualLightKind::Directional if !has_directional => {
                directional = DirectionalLight::new(direction, light.color, light.intensity);
                has_directional = true;
            }
            GltfPunctualLightKind::Directional => {}
            GltfPunctualLightKind::Point => point_lights.push(PointLight::new(
                position,
                light.color,
                light.intensity,
                light.range.unwrap_or(GLTF_UNBOUNDED_LIGHT_RANGE),
            )),
            GltfPunctualLightKind::Spot => spot_lights.push(SpotLight::new(
                position,
                direction,
                light.color,
                light.intensity,
                light.range.unwrap_or(GLTF_UNBOUNDED_LIGHT_RANGE),
                light.inner_cone_angle,
                light.outer_cone_angle,
            )),
        }
    }

    Some(
        RenderLighting::new([1.0, 1.0, 1.0], 0.0, directional)
            .with_point_lights(&point_lights)
            .with_spot_lights(&spot_lights),
    )
}

impl LoadedGltfPrimitive {
    fn validate_skin_attributes(&self, skin: &GltfSkin) -> Result<(), GltfLoadError> {
        let joints = self
            .joints
            .as_deref()
            .ok_or(GltfLoadError::MissingField("attributes.JOINTS_0"))?;
        self.weights
            .as_deref()
            .ok_or(GltfLoadError::MissingField("attributes.WEIGHTS_0"))?;

        if joints
            .iter()
            .flatten()
            .any(|&joint_index| joint_index >= skin.joints.len())
        {
            return Err(GltfLoadError::InvalidField("attributes.JOINTS_0"));
        }

        Ok(())
    }

    fn instantiate(
        &self,
        node_index: Option<usize>,
        model_matrix: Mat4,
        morph_weights: Option<&[f32]>,
        skin_joint_matrices: Option<&[Mat4]>,
        skin: Option<&GltfSkin>,
        node_local_matrices: &[Mat4],
        node_parent_indices: &[Option<usize>],
    ) -> GltfPrimitive {
        let weights = morph_weights.unwrap_or(&self.default_morph_weights);
        let morphed_vertices = apply_morph_targets(&self.vertices, &self.morph_targets, weights);
        let vertices = skin_joint_matrices.map_or(morphed_vertices.clone(), |joint_matrices| {
            apply_skinning(
                &morphed_vertices,
                self.joints.as_deref(),
                self.weights.as_deref(),
                joint_matrices,
            )
        });

        let mut mesh = Mesh::with_indices(vertices, self.indices.clone());
        if !self.has_explicit_normals {
            mesh.generate_normals();
        }
        if !self.has_explicit_tangents {
            mesh.generate_tangents();
        }

        GltfPrimitive {
            node_index,
            mesh_index: self.mesh_index,
            primitive_index: self.primitive_index,
            material_index: self.material_index,
            model_matrix,
            morph_target_count: self.morph_targets.len(),
            skin_joint_count: skin_joint_matrices.map_or(0, <[Mat4]>::len),
            mesh,
            morph_base_vertices: self.vertices.clone(),
            morph_indices: self.indices.clone(),
            morph_targets: self.morph_targets.clone(),
            morph_instance_weights: weights.to_vec(),
            morph_joints: self.joints.clone(),
            morph_weights: self.weights.clone(),
            morph_skin_joint_matrices: skin_joint_matrices.map(<[Mat4]>::to_vec),
            skin_joint_node_indices: skin.map_or_else(Vec::new, |skin| skin.joints.clone()),
            skin_inverse_bind_matrices: skin
                .map_or_else(Vec::new, |skin| skin.inverse_bind_matrices.clone()),
            skin_node_local_matrices: skin.map_or_else(Vec::new, |_| node_local_matrices.to_vec()),
            skin_node_parent_indices: skin.map_or_else(Vec::new, |_| node_parent_indices.to_vec()),
            morph_has_explicit_normals: self.has_explicit_normals,
            morph_has_explicit_tangents: self.has_explicit_tangents,
        }
    }
}

impl GltfPrimitive {
    pub fn morphed_mesh(&self, weights: &[f32]) -> Option<Mesh> {
        if self.morph_targets.is_empty() {
            return None;
        }

        Some(self.animated_mesh_with_joint_world_matrices(Some(weights), &[]))
    }

    pub fn animated_mesh_with_joint_world_matrices(
        &self,
        morph_weights: Option<&[f32]>,
        sampled_node_local_matrices: &[(usize, Mat4)],
    ) -> Mesh {
        let weights = morph_weights.unwrap_or(&self.morph_instance_weights);
        let morphed_vertices =
            apply_morph_targets(&self.morph_base_vertices, &self.morph_targets, weights);
        let joint_matrices = self.live_skin_joint_matrices(sampled_node_local_matrices);
        let vertices = if let Some(joint_matrices) = joint_matrices.as_deref() {
            apply_skinning(
                &morphed_vertices,
                self.morph_joints.as_deref(),
                self.morph_weights.as_deref(),
                joint_matrices,
            )
        } else {
            morphed_vertices
        };
        let mut mesh = Mesh::with_indices(vertices, self.morph_indices.clone());
        if !self.morph_has_explicit_normals {
            mesh.generate_normals();
        }
        if !self.morph_has_explicit_tangents {
            mesh.generate_tangents();
        }

        mesh
    }

    pub fn has_live_skinning_source(&self) -> bool {
        !self.skin_joint_node_indices.is_empty()
            && self.morph_joints.is_some()
            && self.morph_weights.is_some()
    }

    pub fn is_skin_affected_by_node(&self, node_index: usize) -> bool {
        for joint_node in self.skin_joint_node_indices.iter().copied() {
            let mut current = Some(joint_node);
            while let Some(current_node) = current {
                if current_node == node_index {
                    return true;
                }
                current = self
                    .skin_node_parent_indices
                    .get(current_node)
                    .copied()
                    .flatten();
            }
        }

        false
    }

    fn live_skin_joint_matrices(
        &self,
        sampled_node_local_matrices: &[(usize, Mat4)],
    ) -> Option<Vec<Mat4>> {
        let base_joint_matrices = self.morph_skin_joint_matrices.as_ref()?;
        if sampled_node_local_matrices.is_empty() {
            return Some(base_joint_matrices.clone());
        }

        let mut joint_matrices = base_joint_matrices.clone();
        let mut node_world_cache = vec![None; self.skin_node_local_matrices.len()];
        for (joint_index, joint_node) in self.skin_joint_node_indices.iter().copied().enumerate() {
            if let (Some(joint_world_matrix), Some(inverse_bind_matrix)) = (
                self.skin_node_world_matrix(
                    joint_node,
                    sampled_node_local_matrices,
                    &mut node_world_cache,
                ),
                self.skin_inverse_bind_matrices.get(joint_index).copied(),
            ) {
                joint_matrices[joint_index] = joint_world_matrix * inverse_bind_matrix;
            }
        }

        Some(joint_matrices)
    }

    fn skin_node_world_matrix(
        &self,
        node_index: usize,
        sampled_node_local_matrices: &[(usize, Mat4)],
        node_world_cache: &mut [Option<Mat4>],
    ) -> Option<Mat4> {
        if let Some(matrix) = node_world_cache.get(node_index).copied().flatten() {
            return Some(matrix);
        }

        let local_matrix = sampled_node_local_matrices
            .iter()
            .find_map(|(sampled_node, matrix)| (*sampled_node == node_index).then_some(*matrix))
            .or_else(|| self.skin_node_local_matrices.get(node_index).copied())?;
        let world_matrix = if let Some(parent_index) = self
            .skin_node_parent_indices
            .get(node_index)
            .copied()
            .flatten()
        {
            self.skin_node_world_matrix(
                parent_index,
                sampled_node_local_matrices,
                node_world_cache,
            )? * local_matrix
        } else {
            local_matrix
        };

        if let Some(slot) = node_world_cache.get_mut(node_index) {
            *slot = Some(world_matrix);
        }
        Some(world_matrix)
    }
}

impl GltfAsset {
    pub fn from_gltf_str_with_buffers(
        source: &str,
        mut buffer_resolver: impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, GltfLoadError> {
        let root = JsonParser::new(source).parse()?;
        Self::from_root(root, None, &mut buffer_resolver)
    }

    pub fn from_glb_bytes(bytes: &[u8]) -> Result<Self, GltfLoadError> {
        Self::from_glb_bytes_with_buffers(bytes, |_| None)
    }

    pub fn from_glb_bytes_with_buffers(
        bytes: &[u8],
        mut buffer_resolver: impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, GltfLoadError> {
        let (json_source, binary_chunk) = parse_glb(bytes)?;
        let root = JsonParser::new(json_source).parse()?;
        Self::from_root(root, binary_chunk, &mut buffer_resolver)
    }

    fn from_root(
        root: JsonValue,
        glb_buffer: Option<&[u8]>,
        buffer_resolver: &mut impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, GltfLoadError> {
        validate_asset_header(&root)?;
        validate_required_extensions(&root)?;
        let buffers = load_buffers(&root, glb_buffer, buffer_resolver)?;
        let materials = load_materials(&root, &buffers)?;
        let animations = load_animations(&root, &buffers, node_count(&root))?;
        let mesh_primitives = load_primitives(&root, &buffers, materials.len())?;
        let skins = load_skins(&root, &buffers)?;
        let lights = instantiate_scene_lights(&root)?;
        let cameras = instantiate_scene_cameras(&root)?;
        let primitives = instantiate_scene_primitives(&root, &buffers, mesh_primitives, &skins)?;

        Ok(Self {
            materials,
            primitives,
            animations,
            lights,
            cameras,
        })
    }
}

fn validate_asset_header(root: &JsonValue) -> Result<(), GltfLoadError> {
    let asset = root
        .get("asset")
        .ok_or(GltfLoadError::MissingField("asset"))?;
    let version = asset
        .get("version")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("asset.version"))?;

    if parse_gltf_version(version, "asset.version")? != SUPPORTED_GLTF_ASSET_VERSION {
        return Err(GltfLoadError::Unsupported(
            "only glTF asset version 2.0 is supported",
        ));
    }

    if let Some(min_version) = asset.get("minVersion") {
        let min_version = min_version
            .as_str()
            .ok_or(GltfLoadError::InvalidField("asset.minVersion"))?;
        if parse_gltf_version(min_version, "asset.minVersion")? > SUPPORTED_GLTF_ASSET_VERSION {
            return Err(GltfLoadError::Unsupported(
                "glTF asset minVersion is newer than 2.0",
            ));
        }
    }

    Ok(())
}

fn parse_gltf_version(version: &str, field: &'static str) -> Result<(usize, usize), GltfLoadError> {
    let mut parts = version.split('.');
    let major = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .ok_or(GltfLoadError::InvalidField(field))?;
    let minor = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .ok_or(GltfLoadError::InvalidField(field))?;
    if parts.next().is_some() {
        return Err(GltfLoadError::InvalidField(field));
    }

    Ok((major, minor))
}

fn validate_required_extensions(root: &JsonValue) -> Result<(), GltfLoadError> {
    let Some(required) = root.get("extensionsRequired") else {
        return Ok(());
    };
    let required = required
        .as_array()
        .ok_or(GltfLoadError::InvalidField("extensionsRequired"))?;

    for extension in required {
        let extension = extension
            .as_str()
            .ok_or(GltfLoadError::InvalidField("extensionsRequired"))?;
        if !SUPPORTED_GLTF_EXTENSIONS.contains(&extension) {
            return Err(GltfLoadError::UnsupportedRequiredExtension(
                extension.to_owned(),
            ));
        }
    }

    Ok(())
}

fn node_count(root: &JsonValue) -> usize {
    root.get("nodes")
        .and_then(JsonValue::as_array)
        .map_or(0, <[JsonValue]>::len)
}

fn load_buffers(
    root: &JsonValue,
    glb_buffer: Option<&[u8]>,
    buffer_resolver: &mut impl FnMut(&str) -> Option<Vec<u8>>,
) -> Result<Vec<Vec<u8>>, GltfLoadError> {
    let Some(buffers) = root.get("buffers").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut loaded = Vec::with_capacity(buffers.len());

    for (index, buffer) in buffers.iter().enumerate() {
        let bytes = if let Some(uri) = buffer.get("uri").and_then(JsonValue::as_str) {
            if let Some(data) = decode_data_uri_base64(uri)? {
                data
            } else {
                buffer_resolver(uri).ok_or_else(|| GltfLoadError::BufferNotFound(uri.to_owned()))?
            }
        } else if index == 0 {
            glb_buffer
                .ok_or(GltfLoadError::MissingField("buffers.uri"))?
                .to_vec()
        } else {
            return Err(GltfLoadError::MissingField("buffers.uri"));
        };
        let expected_len = optional_usize(buffer, "byteLength")?;
        if let Some(expected_len) = expected_len {
            if bytes.len() < expected_len {
                return Err(GltfLoadError::BufferOutOfBounds);
            }
        }
        loaded.push(bytes);
    }

    Ok(loaded)
}

fn parse_glb(bytes: &[u8]) -> Result<(&str, Option<&[u8]>), GltfLoadError> {
    const GLB_MAGIC: u32 = 0x4654_6c67;
    const GLB_VERSION: u32 = 2;
    const CHUNK_JSON: u32 = 0x4e4f_534a;
    const CHUNK_BIN: u32 = 0x004e_4942;

    if bytes.len() < 20 {
        return Err(GltfLoadError::InvalidGlb("file is shorter than GLB header"));
    }
    if read_u32_le(bytes, 0)? != GLB_MAGIC {
        return Err(GltfLoadError::InvalidGlb("magic must be glTF"));
    }
    if read_u32_le(bytes, 4)? != GLB_VERSION {
        return Err(GltfLoadError::Unsupported(
            "only GLB version 2 is supported",
        ));
    }
    let declared_len = read_u32_le(bytes, 8)? as usize;
    if declared_len != bytes.len() {
        return Err(GltfLoadError::InvalidGlb(
            "declared length must match input length",
        ));
    }

    let mut offset = 12;
    let mut json_chunk = None;
    let mut binary_chunk = None;

    while offset < declared_len {
        let header_end = offset
            .checked_add(8)
            .ok_or(GltfLoadError::InvalidGlb("chunk header overflows"))?;
        if header_end > declared_len {
            return Err(GltfLoadError::InvalidGlb("chunk header is truncated"));
        }
        let chunk_len = read_u32_le(bytes, offset)? as usize;
        let chunk_type = read_u32_le(bytes, offset + 4)?;
        let chunk_start = header_end;
        let chunk_end = chunk_start
            .checked_add(chunk_len)
            .ok_or(GltfLoadError::InvalidGlb("chunk length overflows"))?;
        if chunk_end > declared_len {
            return Err(GltfLoadError::InvalidGlb("chunk extends past GLB length"));
        }

        match chunk_type {
            CHUNK_JSON => {
                if json_chunk.is_some() {
                    return Err(GltfLoadError::InvalidGlb("multiple JSON chunks"));
                }
                json_chunk = Some(&bytes[chunk_start..chunk_end]);
            }
            CHUNK_BIN => {
                if binary_chunk.is_some() {
                    return Err(GltfLoadError::InvalidGlb("multiple BIN chunks"));
                }
                binary_chunk = Some(&bytes[chunk_start..chunk_end]);
            }
            _ => {}
        }

        offset = chunk_end;
    }

    let json_bytes = json_chunk.ok_or(GltfLoadError::InvalidGlb("missing JSON chunk"))?;
    let json_source = std::str::from_utf8(json_bytes)
        .map_err(|_| GltfLoadError::InvalidGlb("JSON chunk must be UTF-8"))?;

    Ok((json_source, binary_chunk))
}

fn load_materials(
    root: &JsonValue,
    buffers: &[Vec<u8>],
) -> Result<Vec<GltfMaterial>, GltfLoadError> {
    let Some(materials) = root.get("materials").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let textures = root.get("textures").and_then(JsonValue::as_array);
    let images = root.get("images").and_then(JsonValue::as_array);
    let samplers = root.get("samplers").and_then(JsonValue::as_array);
    let mut loaded = Vec::with_capacity(materials.len());

    for material_value in materials {
        let empty_pbr = JsonValue::Object(Vec::new());
        let pbr = material_value
            .get("pbrMetallicRoughness")
            .unwrap_or(&empty_pbr);
        let specular_glossiness =
            gltf_extension(material_value, "KHR_materials_pbrSpecularGlossiness");
        let base_color = if let Some(specular_glossiness) = specular_glossiness {
            optional_vec4(specular_glossiness, "diffuseFactor")?.unwrap_or([1.0, 1.0, 1.0, 1.0])
        } else {
            optional_vec4(pbr, "baseColorFactor")?.unwrap_or([1.0, 1.0, 1.0, 1.0])
        };
        let metallic = if specular_glossiness.is_some() {
            0.0
        } else {
            optional_f32(pbr, "metallicFactor")?.unwrap_or(1.0)
        };
        let roughness = if let Some(specular_glossiness) = specular_glossiness {
            (1.0 - optional_f32(specular_glossiness, "glossinessFactor")?.unwrap_or(1.0))
                .clamp(0.0, 1.0)
        } else {
            optional_f32(pbr, "roughnessFactor")?.unwrap_or(1.0)
        };
        let emissive = optional_vec3(material_value, "emissiveFactor")?.unwrap_or([0.0, 0.0, 0.0]);
        let alpha_mode = material_value
            .get("alphaMode")
            .and_then(JsonValue::as_str)
            .unwrap_or("OPAQUE");
        let mut material = Material::new(base_color).with_surface(roughness, metallic);
        material.double_sided = material_value
            .get("doubleSided")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        material.emissive = emissive;
        material.normal_scale = material_value
            .get("normalTexture")
            .map(|texture_info| optional_f32(texture_info, "scale"))
            .transpose()?
            .flatten()
            .unwrap_or(1.0);
        material.occlusion_strength = material_value
            .get("occlusionTexture")
            .map(|texture_info| optional_f32(texture_info, "strength"))
            .transpose()?
            .flatten()
            .unwrap_or(1.0);
        if let Some(clearcoat) = gltf_extension(material_value, "KHR_materials_clearcoat") {
            material.clearcoat = optional_f32(clearcoat, "clearcoatFactor")?.unwrap_or(0.0);
            material.clearcoat_roughness =
                optional_f32(clearcoat, "clearcoatRoughnessFactor")?.unwrap_or(0.0);
        }
        if let Some(sheen) = gltf_extension(material_value, "KHR_materials_sheen") {
            material.sheen_color =
                optional_vec3(sheen, "sheenColorFactor")?.unwrap_or([0.0, 0.0, 0.0]);
            material.sheen_roughness = optional_f32(sheen, "sheenRoughnessFactor")?.unwrap_or(0.0);
        }
        if let Some(transmission) = gltf_extension(material_value, "KHR_materials_transmission") {
            material = material.with_transmission(
                optional_f32(transmission, "transmissionFactor")?.unwrap_or(0.0),
            );
        }
        if let Some(ior) = gltf_extension(material_value, "KHR_materials_ior") {
            material.ior = optional_f32(ior, "ior")?.unwrap_or(1.5);
        }
        if let Some(emissive_strength) =
            gltf_extension(material_value, "KHR_materials_emissive_strength")
        {
            material.emissive_strength =
                optional_f32(emissive_strength, "emissiveStrength")?.unwrap_or(1.0);
        }
        if let Some(specular) = gltf_extension(material_value, "KHR_materials_specular") {
            material.specular_factor = optional_f32(specular, "specularFactor")?.unwrap_or(1.0);
            material.specular_color =
                optional_vec3(specular, "specularColorFactor")?.unwrap_or([1.0, 1.0, 1.0]);
        }
        if let Some(specular_glossiness) = specular_glossiness {
            material = material.with_specular_glossiness_workflow(true);
            material.specular_factor = 1.0;
            material.specular_color =
                optional_vec3(specular_glossiness, "specularFactor")?.unwrap_or([1.0, 1.0, 1.0]);
        }
        if let Some(anisotropy) = gltf_extension(material_value, "KHR_materials_anisotropy") {
            material.anisotropy_strength =
                optional_f32(anisotropy, "anisotropyStrength")?.unwrap_or(0.0);
            material.anisotropy_rotation =
                optional_f32(anisotropy, "anisotropyRotation")?.unwrap_or(0.0);
        }
        if let Some(iridescence) = gltf_extension(material_value, "KHR_materials_iridescence") {
            material.iridescence_factor =
                optional_f32(iridescence, "iridescenceFactor")?.unwrap_or(0.0);
            material.iridescence_ior = optional_f32(iridescence, "iridescenceIor")?.unwrap_or(1.3);
            material.iridescence_thickness_min =
                optional_f32(iridescence, "iridescenceThicknessMinimum")?.unwrap_or(100.0);
            material.iridescence_thickness_max =
                optional_f32(iridescence, "iridescenceThicknessMaximum")?.unwrap_or(400.0);
        }
        if let Some(volume) = gltf_extension(material_value, "KHR_materials_volume") {
            material = material.with_volume(
                optional_f32(volume, "thicknessFactor")?.unwrap_or(0.0),
                optional_vec3(volume, "attenuationColor")?.unwrap_or([1.0, 1.0, 1.0]),
                optional_f32(volume, "attenuationDistance")?.unwrap_or(0.0),
            );
        }
        if let Some(dispersion) = gltf_extension(material_value, "KHR_materials_dispersion") {
            material.dispersion = optional_f32(dispersion, "dispersion")?.unwrap_or(0.0);
        }
        if gltf_extension(material_value, "KHR_materials_unlit").is_some() {
            material.unlit = true;
        }

        match alpha_mode {
            "BLEND" => {
                material.blend_mode = BlendMode::AlphaBlend;
                material.depth_write = false;
            }
            "MASK" => {
                material.alpha_cutoff = optional_f32(material_value, "alphaCutoff")?.unwrap_or(0.5);
            }
            "OPAQUE" if base_color[3] < 1.0 => {
                material.blend_mode = BlendMode::AlphaBlend;
                material.depth_write = false;
            }
            "OPAQUE" => {}
            _ => return Err(GltfLoadError::InvalidField("materials.alphaMode")),
        }

        let base_color_texture_info = specular_glossiness
            .and_then(|extension| extension.get("diffuseTexture"))
            .or_else(|| pbr.get("baseColorTexture"));
        let (base_color_texture_path, base_color_texture_data) =
            load_texture_info(base_color_texture_info, textures, images, root, buffers)?;
        material.base_color_texture_transform = load_texture_transform(base_color_texture_info)?;
        material.texture_samplers.base_color =
            load_texture_sampler(base_color_texture_info, textures, samplers)?;
        let metallic_roughness_texture_info = if let Some(specular_glossiness) = specular_glossiness
        {
            specular_glossiness.get("specularGlossinessTexture")
        } else {
            pbr.get("metallicRoughnessTexture")
        };
        let (metallic_roughness_texture_path, metallic_roughness_texture_data) = load_texture_info(
            metallic_roughness_texture_info,
            textures,
            images,
            root,
            buffers,
        )?;
        material.metallic_roughness_texture_transform =
            load_texture_transform(metallic_roughness_texture_info)?;
        material.texture_samplers.metallic_roughness =
            load_texture_sampler(metallic_roughness_texture_info, textures, samplers)?;
        let (normal_texture_path, normal_texture_data) = load_texture_info(
            material_value.get("normalTexture"),
            textures,
            images,
            root,
            buffers,
        )?;
        material.normal_texture_transform =
            load_texture_transform(material_value.get("normalTexture"))?;
        material.texture_samplers.normal =
            load_texture_sampler(material_value.get("normalTexture"), textures, samplers)?;
        let (emissive_texture_path, emissive_texture_data) = load_texture_info(
            material_value.get("emissiveTexture"),
            textures,
            images,
            root,
            buffers,
        )?;
        material.emissive_texture_transform =
            load_texture_transform(material_value.get("emissiveTexture"))?;
        material.texture_samplers.emissive =
            load_texture_sampler(material_value.get("emissiveTexture"), textures, samplers)?;
        let (occlusion_texture_path, occlusion_texture_data) = load_texture_info(
            material_value.get("occlusionTexture"),
            textures,
            images,
            root,
            buffers,
        )?;
        material.occlusion_texture_transform =
            load_texture_transform(material_value.get("occlusionTexture"))?;
        material.texture_samplers.occlusion =
            load_texture_sampler(material_value.get("occlusionTexture"), textures, samplers)?;
        let (clearcoat_texture_path, clearcoat_texture_data) = load_texture_info_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatTexture",
            textures,
            images,
            root,
            buffers,
        )?;
        material.clearcoat_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatTexture",
        )?;
        material.texture_samplers.clearcoat = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatTexture",
            textures,
            samplers,
        )?;
        let (clearcoat_roughness_texture_path, clearcoat_roughness_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_clearcoat",
                "clearcoatRoughnessTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.clearcoat_roughness_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatRoughnessTexture",
        )?;
        material.texture_samplers.clearcoat_roughness = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatRoughnessTexture",
            textures,
            samplers,
        )?;
        let (clearcoat_normal_texture_path, clearcoat_normal_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_clearcoat",
                "clearcoatNormalTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.clearcoat_normal_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatNormalTexture",
        )?;
        material.texture_samplers.clearcoat_normal = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_clearcoat",
            "clearcoatNormalTexture",
            textures,
            samplers,
        )?;
        material.clearcoat_normal_scale = gltf_extension(material_value, "KHR_materials_clearcoat")
            .and_then(|extension| extension.get("clearcoatNormalTexture"))
            .map(|texture_info| optional_f32(texture_info, "scale"))
            .transpose()?
            .flatten()
            .unwrap_or(1.0);
        let (sheen_color_texture_path, sheen_color_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_sheen",
                "sheenColorTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.sheen_color_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_sheen",
            "sheenColorTexture",
        )?;
        material.texture_samplers.sheen_color = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_sheen",
            "sheenColorTexture",
            textures,
            samplers,
        )?;
        let (sheen_roughness_texture_path, sheen_roughness_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_sheen",
                "sheenRoughnessTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.sheen_roughness_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_sheen",
            "sheenRoughnessTexture",
        )?;
        material.texture_samplers.sheen_roughness = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_sheen",
            "sheenRoughnessTexture",
            textures,
            samplers,
        )?;
        let (transmission_texture_path, transmission_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_transmission",
                "transmissionTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.transmission_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_transmission",
            "transmissionTexture",
        )?;
        material.texture_samplers.transmission = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_transmission",
            "transmissionTexture",
            textures,
            samplers,
        )?;
        let (mut specular_texture_path, mut specular_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_specular",
                "specularTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.specular_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_specular",
            "specularTexture",
        )?;
        material.texture_samplers.specular = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_specular",
            "specularTexture",
            textures,
            samplers,
        )?;
        let (mut specular_color_texture_path, mut specular_color_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_specular",
                "specularColorTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.specular_color_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_specular",
            "specularColorTexture",
        )?;
        material.texture_samplers.specular_color = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_specular",
            "specularColorTexture",
            textures,
            samplers,
        )?;
        if let Some(specular_glossiness) = specular_glossiness {
            let specular_glossiness_texture = specular_glossiness.get("specularGlossinessTexture");
            if specular_glossiness_texture.is_some() {
                let (texture_path, texture_data) = load_texture_info(
                    specular_glossiness_texture,
                    textures,
                    images,
                    root,
                    buffers,
                )?;
                let texture_transform = load_texture_transform(specular_glossiness_texture)?;
                let texture_sampler =
                    load_texture_sampler(specular_glossiness_texture, textures, samplers)?;
                specular_texture_path = texture_path.clone();
                specular_texture_data = texture_data.clone();
                specular_color_texture_path = texture_path;
                specular_color_texture_data = texture_data;
                material.specular_texture_transform = texture_transform;
                material.specular_color_texture_transform = texture_transform;
                material.texture_samplers.specular = texture_sampler;
                material.texture_samplers.specular_color = texture_sampler;
            }
        }
        let (anisotropy_texture_path, anisotropy_texture_data) = load_texture_info_from_extension(
            material_value,
            "KHR_materials_anisotropy",
            "anisotropyTexture",
            textures,
            images,
            root,
            buffers,
        )?;
        material.anisotropy_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_anisotropy",
            "anisotropyTexture",
        )?;
        material.texture_samplers.anisotropy = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_anisotropy",
            "anisotropyTexture",
            textures,
            samplers,
        )?;
        let (iridescence_texture_path, iridescence_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_iridescence",
                "iridescenceTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.iridescence_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_iridescence",
            "iridescenceTexture",
        )?;
        material.texture_samplers.iridescence = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_iridescence",
            "iridescenceTexture",
            textures,
            samplers,
        )?;
        let (iridescence_thickness_texture_path, iridescence_thickness_texture_data) =
            load_texture_info_from_extension(
                material_value,
                "KHR_materials_iridescence",
                "iridescenceThicknessTexture",
                textures,
                images,
                root,
                buffers,
            )?;
        material.iridescence_thickness_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_iridescence",
            "iridescenceThicknessTexture",
        )?;
        material.texture_samplers.iridescence_thickness = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_iridescence",
            "iridescenceThicknessTexture",
            textures,
            samplers,
        )?;
        let (thickness_texture_path, thickness_texture_data) = load_texture_info_from_extension(
            material_value,
            "KHR_materials_volume",
            "thicknessTexture",
            textures,
            images,
            root,
            buffers,
        )?;
        material.thickness_texture_transform = load_texture_transform_from_extension(
            material_value,
            "KHR_materials_volume",
            "thicknessTexture",
        )?;
        material.texture_samplers.thickness = load_texture_sampler_from_extension(
            material_value,
            "KHR_materials_volume",
            "thicknessTexture",
            textures,
            samplers,
        )?;

        loaded.push(GltfMaterial {
            material,
            base_color_texture_path,
            base_color_texture_data,
            metallic_roughness_texture_path,
            metallic_roughness_texture_data,
            normal_texture_path,
            normal_texture_data,
            emissive_texture_path,
            emissive_texture_data,
            occlusion_texture_path,
            occlusion_texture_data,
            clearcoat_texture_path,
            clearcoat_texture_data,
            clearcoat_roughness_texture_path,
            clearcoat_roughness_texture_data,
            clearcoat_normal_texture_path,
            clearcoat_normal_texture_data,
            sheen_color_texture_path,
            sheen_color_texture_data,
            sheen_roughness_texture_path,
            sheen_roughness_texture_data,
            transmission_texture_path,
            transmission_texture_data,
            specular_texture_path,
            specular_texture_data,
            specular_color_texture_path,
            specular_color_texture_data,
            anisotropy_texture_path,
            anisotropy_texture_data,
            iridescence_texture_path,
            iridescence_texture_data,
            iridescence_thickness_texture_path,
            iridescence_thickness_texture_data,
            thickness_texture_path,
            thickness_texture_data,
        });
    }

    Ok(loaded)
}

fn load_texture_info(
    texture_info: Option<&JsonValue>,
    textures: Option<&[JsonValue]>,
    images: Option<&[JsonValue]>,
    root: &JsonValue,
    buffers: &[Vec<u8>],
) -> Result<(Option<String>, Option<GltfImageData>), GltfLoadError> {
    let Some(texture_index) = texture_info
        .and_then(|texture_info| texture_info.get("index"))
        .map(number_to_usize)
        .transpose()?
    else {
        return Ok((None, None));
    };
    let texture = textures
        .and_then(|textures| textures.get(texture_index))
        .ok_or(GltfLoadError::InvalidField("textures"))?;
    let image_index =
        texture_source_index(texture)?.ok_or(GltfLoadError::MissingField("textures.source"))?;
    let image = images
        .and_then(|images| images.get(image_index))
        .ok_or(GltfLoadError::InvalidField("images"))?;

    if let Some(uri) = image.get("uri").and_then(JsonValue::as_str) {
        if let Some((mime_type, bytes)) = decode_data_uri_base64_with_mime(uri)? {
            let label = image_label(
                mime_type.as_deref(),
                image.get("name").and_then(JsonValue::as_str),
            );
            return Ok((
                None,
                Some(GltfImageData {
                    label,
                    mime_type,
                    bytes,
                }),
            ));
        }

        return Ok((Some(uri.to_owned()), None));
    }

    if let Some(buffer_view_index) = image.get("bufferView").map(number_to_usize).transpose()? {
        let mime_type = image
            .get("mimeType")
            .and_then(JsonValue::as_str)
            .map(str::to_owned);
        let label = image_label(
            mime_type.as_deref(),
            image.get("name").and_then(JsonValue::as_str),
        );
        return Ok((
            None,
            Some(GltfImageData {
                label,
                mime_type,
                bytes: read_buffer_view_bytes(root, buffers, buffer_view_index)?,
            }),
        ));
    }

    Err(GltfLoadError::MissingField("images.uri"))
}

fn texture_source_index(texture: &JsonValue) -> Result<Option<usize>, GltfLoadError> {
    for extension_name in ["KHR_texture_basisu", "EXT_texture_webp"] {
        if let Some(extension) = gltf_extension(texture, extension_name) {
            if let Some(source) = extension.get("source").map(number_to_usize).transpose()? {
                return Ok(Some(source));
            }
        }
    }

    texture.get("source").map(number_to_usize).transpose()
}

fn load_texture_transform(
    texture_info: Option<&JsonValue>,
) -> Result<TextureTransform, GltfLoadError> {
    let Some(texture_info) = texture_info else {
        return Ok(TextureTransform::IDENTITY);
    };
    let mut tex_coord = optional_usize(texture_info, "texCoord")?.unwrap_or(0);
    let Some(transform) = gltf_extension(texture_info, "KHR_texture_transform") else {
        return texture_transform_with_tex_coord([0.0, 0.0], 0.0, [1.0, 1.0], tex_coord);
    };

    let offset = optional_vec2(transform, "offset")?.unwrap_or([0.0, 0.0]);
    let rotation = optional_f32(transform, "rotation")?.unwrap_or(0.0);
    let scale = optional_vec2(transform, "scale")?.unwrap_or([1.0, 1.0]);
    if let Some(override_tex_coord) = optional_usize(transform, "texCoord")? {
        tex_coord = override_tex_coord;
    }
    texture_transform_with_tex_coord(offset, rotation, scale, tex_coord)
}

fn texture_transform_with_tex_coord(
    offset: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
    tex_coord: usize,
) -> Result<TextureTransform, GltfLoadError> {
    let tex_coord = u32::try_from(tex_coord)
        .map_err(|_| GltfLoadError::InvalidField("KHR_texture_transform.texCoord"))?;
    if tex_coord > 1 {
        return Err(GltfLoadError::Unsupported(
            "only TEXCOORD_0 and TEXCOORD_1 texture transforms are supported",
        ));
    }

    Ok(TextureTransform::new(offset, rotation, scale, tex_coord))
}

fn load_texture_sampler(
    texture_info: Option<&JsonValue>,
    textures: Option<&[JsonValue]>,
    samplers: Option<&[JsonValue]>,
) -> Result<TextureSampler, GltfLoadError> {
    let Some(texture_index) = texture_info
        .and_then(|texture_info| texture_info.get("index"))
        .map(number_to_usize)
        .transpose()?
    else {
        return Ok(TextureSampler::DEFAULT);
    };
    let Some(sampler_index) = textures
        .and_then(|textures| textures.get(texture_index))
        .and_then(|texture| texture.get("sampler"))
        .map(number_to_usize)
        .transpose()?
    else {
        return Ok(TextureSampler::DEFAULT);
    };
    let sampler = samplers
        .and_then(|samplers| samplers.get(sampler_index))
        .ok_or(GltfLoadError::InvalidField("textures.sampler"))?;
    let (min_filter, mipmap_filter) = sampler
        .get("minFilter")
        .map(number_to_usize)
        .transpose()?
        .map(gltf_min_filter)
        .transpose()?
        .unwrap_or((TextureFilterMode::Linear, TextureFilterMode::Linear));

    Ok(TextureSampler {
        address_mode_u: load_texture_address_mode(sampler, "wrapS")?,
        address_mode_v: load_texture_address_mode(sampler, "wrapT")?,
        address_mode_w: TextureAddressMode::Repeat,
        mag_filter: sampler
            .get("magFilter")
            .map(number_to_usize)
            .transpose()?
            .map(gltf_mag_filter)
            .transpose()?
            .unwrap_or(TextureFilterMode::Linear),
        min_filter,
        mipmap_filter,
    })
}

fn load_texture_address_mode(
    sampler: &JsonValue,
    field: &'static str,
) -> Result<TextureAddressMode, GltfLoadError> {
    sampler
        .get(field)
        .map(number_to_usize)
        .transpose()?
        .map(gltf_address_mode)
        .transpose()
        .map(|mode| mode.unwrap_or(TextureAddressMode::Repeat))
}

fn gltf_address_mode(value: usize) -> Result<TextureAddressMode, GltfLoadError> {
    match value {
        33071 => Ok(TextureAddressMode::ClampToEdge),
        33648 => Ok(TextureAddressMode::MirrorRepeat),
        10497 => Ok(TextureAddressMode::Repeat),
        _ => Err(GltfLoadError::InvalidField("samplers.wrapS/wrapT")),
    }
}

fn gltf_mag_filter(value: usize) -> Result<TextureFilterMode, GltfLoadError> {
    match value {
        9728 => Ok(TextureFilterMode::Nearest),
        9729 => Ok(TextureFilterMode::Linear),
        _ => Err(GltfLoadError::InvalidField("samplers.magFilter")),
    }
}

fn gltf_min_filter(value: usize) -> Result<(TextureFilterMode, TextureFilterMode), GltfLoadError> {
    match value {
        9728 => Ok((TextureFilterMode::Nearest, TextureFilterMode::Nearest)),
        9729 => Ok((TextureFilterMode::Linear, TextureFilterMode::Nearest)),
        9984 => Ok((TextureFilterMode::Nearest, TextureFilterMode::Nearest)),
        9985 => Ok((TextureFilterMode::Linear, TextureFilterMode::Nearest)),
        9986 => Ok((TextureFilterMode::Nearest, TextureFilterMode::Linear)),
        9987 => Ok((TextureFilterMode::Linear, TextureFilterMode::Linear)),
        _ => Err(GltfLoadError::InvalidField("samplers.minFilter")),
    }
}

fn load_texture_info_from_extension(
    material: &JsonValue,
    extension_name: &str,
    texture_field: &str,
    textures: Option<&[JsonValue]>,
    images: Option<&[JsonValue]>,
    root: &JsonValue,
    buffers: &[Vec<u8>],
) -> Result<(Option<String>, Option<GltfImageData>), GltfLoadError> {
    let texture_info =
        gltf_extension(material, extension_name).and_then(|extension| extension.get(texture_field));
    load_texture_info(texture_info, textures, images, root, buffers)
}

fn load_texture_transform_from_extension(
    material: &JsonValue,
    extension_name: &str,
    texture_field: &str,
) -> Result<TextureTransform, GltfLoadError> {
    let texture_info =
        gltf_extension(material, extension_name).and_then(|extension| extension.get(texture_field));
    load_texture_transform(texture_info)
}

fn load_texture_sampler_from_extension(
    material: &JsonValue,
    extension_name: &str,
    texture_field: &str,
    textures: Option<&[JsonValue]>,
    samplers: Option<&[JsonValue]>,
) -> Result<TextureSampler, GltfLoadError> {
    let texture_info =
        gltf_extension(material, extension_name).and_then(|extension| extension.get(texture_field));
    load_texture_sampler(texture_info, textures, samplers)
}

fn gltf_extension<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.get("extensions")?.get(name)
}

fn load_punctual_light_definitions(
    root: &JsonValue,
) -> Result<Vec<GltfPunctualLightDefinition>, GltfLoadError> {
    let Some(lights) = gltf_extension(root, "KHR_lights_punctual")
        .and_then(|extension| extension.get("lights"))
        .and_then(JsonValue::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut definitions = Vec::with_capacity(lights.len());

    for light in lights {
        let kind = match light.get("type").and_then(JsonValue::as_str).ok_or(
            GltfLoadError::MissingField("extensions.KHR_lights_punctual.lights.type"),
        )? {
            "directional" => GltfPunctualLightKind::Directional,
            "point" => GltfPunctualLightKind::Point,
            "spot" => GltfPunctualLightKind::Spot,
            _ => {
                return Err(GltfLoadError::InvalidField(
                    "extensions.KHR_lights_punctual.lights.type",
                ))
            }
        };
        let color = optional_vec3(light, "color")?.unwrap_or([1.0, 1.0, 1.0]);
        let intensity = optional_f32(light, "intensity")?.unwrap_or(1.0);
        let range = optional_f32(light, "range")?;
        if range.is_some_and(|range| range <= 0.0) {
            return Err(GltfLoadError::InvalidField(
                "extensions.KHR_lights_punctual.lights.range",
            ));
        }
        if matches!(kind, GltfPunctualLightKind::Directional) && range.is_some() {
            return Err(GltfLoadError::InvalidField(
                "extensions.KHR_lights_punctual.lights.range",
            ));
        }

        let spot = light.get("spot");
        if !matches!(kind, GltfPunctualLightKind::Spot) && spot.is_some() {
            return Err(GltfLoadError::InvalidField(
                "extensions.KHR_lights_punctual.lights.spot",
            ));
        }
        let inner_cone_angle = spot
            .map(|spot| optional_f32(spot, "innerConeAngle"))
            .transpose()?
            .flatten()
            .unwrap_or(0.0);
        let outer_cone_angle = spot
            .map(|spot| optional_f32(spot, "outerConeAngle"))
            .transpose()?
            .flatten()
            .unwrap_or(std::f32::consts::FRAC_PI_4);
        if matches!(kind, GltfPunctualLightKind::Spot)
            && (inner_cone_angle < 0.0
                || outer_cone_angle <= inner_cone_angle
                || outer_cone_angle > std::f32::consts::FRAC_PI_2)
        {
            return Err(GltfLoadError::InvalidField(
                "extensions.KHR_lights_punctual.lights.spot",
            ));
        }

        definitions.push(GltfPunctualLightDefinition {
            kind,
            color,
            intensity,
            range,
            inner_cone_angle,
            outer_cone_angle,
        });
    }

    Ok(definitions)
}

fn load_camera_definitions(root: &JsonValue) -> Result<Vec<GltfCameraDefinition>, GltfLoadError> {
    let Some(cameras) = root.get("cameras").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut definitions = Vec::with_capacity(cameras.len());

    for camera in cameras {
        let projection = match camera
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or(GltfLoadError::MissingField("cameras.type"))?
        {
            "perspective" => {
                let perspective = camera
                    .get("perspective")
                    .ok_or(GltfLoadError::MissingField("cameras.perspective"))?;
                let aspect_ratio = optional_f32(perspective, "aspectRatio")?;
                if aspect_ratio.is_some_and(|aspect_ratio| aspect_ratio <= 0.0) {
                    return Err(GltfLoadError::InvalidField(
                        "cameras.perspective.aspectRatio",
                    ));
                }
                let vertical_fov_radians = required_f32(perspective, "yfov")?;
                let near = required_f32(perspective, "znear")?;
                let far = optional_f32(perspective, "zfar")?;
                if vertical_fov_radians <= 0.0
                    || vertical_fov_radians >= std::f32::consts::PI
                    || near <= 0.0
                    || far.is_some_and(|far| far <= near)
                {
                    return Err(GltfLoadError::InvalidField("cameras.perspective"));
                }

                GltfCameraProjection::Perspective {
                    aspect_ratio,
                    vertical_fov_radians,
                    near,
                    far,
                }
            }
            "orthographic" => {
                let orthographic = camera
                    .get("orthographic")
                    .ok_or(GltfLoadError::MissingField("cameras.orthographic"))?;
                let xmag = required_f32(orthographic, "xmag")?;
                let ymag = required_f32(orthographic, "ymag")?;
                let near = required_f32(orthographic, "znear")?;
                let far = required_f32(orthographic, "zfar")?;
                if xmag <= 0.0 || ymag <= 0.0 || near < 0.0 || far <= near {
                    return Err(GltfLoadError::InvalidField("cameras.orthographic"));
                }

                GltfCameraProjection::Orthographic {
                    xmag,
                    ymag,
                    near,
                    far,
                }
            }
            _ => return Err(GltfLoadError::InvalidField("cameras.type")),
        };

        definitions.push(GltfCameraDefinition { projection });
    }

    Ok(definitions)
}

fn load_animations(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    node_count: usize,
) -> Result<Vec<GltfAnimation>, GltfLoadError> {
    let Some(animations) = root.get("animations").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut loaded = Vec::with_capacity(animations.len());

    for animation in animations {
        let samplers = animation
            .get("samplers")
            .and_then(JsonValue::as_array)
            .ok_or(GltfLoadError::MissingField("animations.samplers"))?;
        let channels = animation
            .get("channels")
            .and_then(JsonValue::as_array)
            .ok_or(GltfLoadError::MissingField("animations.channels"))?;
        let mut loaded_channels = Vec::with_capacity(channels.len());

        for channel in channels {
            let sampler_index = required_usize(channel, "sampler")?;
            let sampler = samplers
                .get(sampler_index)
                .ok_or(GltfLoadError::InvalidField("animations.channels.sampler"))?;
            let target = channel
                .get("target")
                .ok_or(GltfLoadError::MissingField("animations.channels.target"))?;
            let target_node = required_usize(target, "node")?;
            if target_node >= node_count {
                return Err(GltfLoadError::InvalidField(
                    "animations.channels.target.node",
                ));
            }
            let path =
                parse_animation_path(target.get("path").and_then(JsonValue::as_str).ok_or(
                    GltfLoadError::MissingField("animations.channels.target.path"),
                )?)?;
            let sampler = load_animation_sampler(root, buffers, sampler, path, target_node)?;

            loaded_channels.push(GltfAnimationChannel {
                target_node,
                path,
                sampler,
            });
        }

        let duration = loaded_channels
            .iter()
            .filter_map(|channel| channel.sampler.input.last().copied())
            .fold(0.0, f32::max);
        loaded.push(GltfAnimation {
            name: animation
                .get("name")
                .and_then(JsonValue::as_str)
                .map(str::to_owned),
            duration,
            channels: loaded_channels,
        });
    }

    Ok(loaded)
}

fn load_animation_sampler(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    sampler: &JsonValue,
    path: GltfAnimationPath,
    target_node: usize,
) -> Result<GltfAnimationSampler, GltfLoadError> {
    let input_accessor = required_usize(sampler, "input")?;
    let output_accessor = required_usize(sampler, "output")?;
    let input = read_scalar_f32_accessor(root, buffers, input_accessor)?;
    if input.is_empty() {
        return Err(GltfLoadError::InvalidField("animations.samplers.input"));
    }
    if input.windows(2).any(|window| window[0] > window[1]) {
        return Err(GltfLoadError::InvalidField("animations.samplers.input"));
    }

    let interpolation = match sampler
        .get("interpolation")
        .and_then(JsonValue::as_str)
        .unwrap_or("LINEAR")
    {
        "STEP" => GltfAnimationInterpolation::Step,
        "LINEAR" => GltfAnimationInterpolation::Linear,
        "CUBICSPLINE" => GltfAnimationInterpolation::CubicSpline,
        _ => {
            return Err(GltfLoadError::InvalidField(
                "animations.samplers.interpolation",
            ))
        }
    };
    let (output, in_tangents, out_tangents) = load_animation_output(
        root,
        buffers,
        output_accessor,
        path,
        target_node,
        &input,
        interpolation,
    )?;

    if animation_output_len(&output) != input.len() {
        return Err(GltfLoadError::InvalidField("animations.samplers.output"));
    }

    Ok(GltfAnimationSampler {
        input,
        output,
        in_tangents,
        out_tangents,
        interpolation,
    })
}

fn load_animation_output(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    output_accessor: usize,
    path: GltfAnimationPath,
    target_node: usize,
    input: &[f32],
    interpolation: GltfAnimationInterpolation,
) -> Result<
    (
        GltfAnimationOutput,
        Option<GltfAnimationOutput>,
        Option<GltfAnimationOutput>,
    ),
    GltfLoadError,
> {
    let cubic = matches!(interpolation, GltfAnimationInterpolation::CubicSpline);
    match path {
        GltfAnimationPath::Translation => {
            let values = read_vec3_accessor(root, buffers, output_accessor)?;
            if cubic {
                let (in_tangents, values, out_tangents) = split_cubic_vec3(values, input.len())?;
                return Ok((
                    GltfAnimationOutput::Translations(values),
                    Some(GltfAnimationOutput::Translations(in_tangents)),
                    Some(GltfAnimationOutput::Translations(out_tangents)),
                ));
            }
            Ok((GltfAnimationOutput::Translations(values), None, None))
        }
        GltfAnimationPath::Rotation => {
            let values = read_rotation_accessor(root, buffers, output_accessor)?;
            if cubic {
                let (in_tangents, values, out_tangents) = split_cubic_vec4(values, input.len())?;
                return Ok((
                    GltfAnimationOutput::Rotations(
                        values.into_iter().map(normalize_quaternion).collect(),
                    ),
                    Some(GltfAnimationOutput::Rotations(in_tangents)),
                    Some(GltfAnimationOutput::Rotations(out_tangents)),
                ));
            }
            Ok((
                GltfAnimationOutput::Rotations(
                    values.into_iter().map(normalize_quaternion).collect(),
                ),
                None,
                None,
            ))
        }
        GltfAnimationPath::Scale => {
            let values = read_vec3_accessor(root, buffers, output_accessor)?;
            if cubic {
                let (in_tangents, values, out_tangents) = split_cubic_vec3(values, input.len())?;
                return Ok((
                    GltfAnimationOutput::Scales(values),
                    Some(GltfAnimationOutput::Scales(in_tangents)),
                    Some(GltfAnimationOutput::Scales(out_tangents)),
                ));
            }
            Ok((GltfAnimationOutput::Scales(values), None, None))
        }
        GltfAnimationPath::Weights => {
            let weights_per_keyframe = node_morph_target_count(root, target_node)?;
            let weights = read_scalar_f32_accessor(root, buffers, output_accessor)?;
            if weights_per_keyframe == 0 {
                return Err(GltfLoadError::InvalidField("animations.samplers.output"));
            }
            if cubic {
                let expected_len = input.len() * weights_per_keyframe * 3;
                if weights.len() != expected_len {
                    return Err(GltfLoadError::InvalidField("animations.samplers.output"));
                }
                let mut in_tangents = Vec::with_capacity(input.len());
                let mut values = Vec::with_capacity(input.len());
                let mut out_tangents = Vec::with_capacity(input.len());
                for frame in weights.chunks(weights_per_keyframe * 3) {
                    in_tangents.push(frame[0..weights_per_keyframe].to_vec());
                    values.push(frame[weights_per_keyframe..weights_per_keyframe * 2].to_vec());
                    out_tangents.push(frame[weights_per_keyframe * 2..].to_vec());
                }
                return Ok((
                    GltfAnimationOutput::Weights(values),
                    Some(GltfAnimationOutput::Weights(in_tangents)),
                    Some(GltfAnimationOutput::Weights(out_tangents)),
                ));
            }
            if weights.len() != input.len() * weights_per_keyframe {
                return Err(GltfLoadError::InvalidField("animations.samplers.output"));
            }
            Ok((
                GltfAnimationOutput::Weights(
                    weights
                        .chunks(weights_per_keyframe)
                        .map(<[f32]>::to_vec)
                        .collect(),
                ),
                None,
                None,
            ))
        }
    }
}

fn parse_animation_path(path: &str) -> Result<GltfAnimationPath, GltfLoadError> {
    match path {
        "translation" => Ok(GltfAnimationPath::Translation),
        "rotation" => Ok(GltfAnimationPath::Rotation),
        "scale" => Ok(GltfAnimationPath::Scale),
        "weights" => Ok(GltfAnimationPath::Weights),
        _ => Err(GltfLoadError::Unsupported("unsupported animation path")),
    }
}

fn animation_output_len(output: &GltfAnimationOutput) -> usize {
    match output {
        GltfAnimationOutput::Translations(values) => values.len(),
        GltfAnimationOutput::Rotations(values) => values.len(),
        GltfAnimationOutput::Scales(values) => values.len(),
        GltfAnimationOutput::Weights(values) => values.len(),
    }
}

fn split_cubic_vec3(
    values: Vec<[f32; 3]>,
    keyframe_count: usize,
) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 3]>), GltfLoadError> {
    if values.len() != keyframe_count * 3 {
        return Err(GltfLoadError::InvalidField("animations.samplers.output"));
    }

    let mut in_tangents = Vec::with_capacity(keyframe_count);
    let mut key_values = Vec::with_capacity(keyframe_count);
    let mut out_tangents = Vec::with_capacity(keyframe_count);
    for triple in values.chunks_exact(3) {
        in_tangents.push(triple[0]);
        key_values.push(triple[1]);
        out_tangents.push(triple[2]);
    }

    Ok((in_tangents, key_values, out_tangents))
}

fn split_cubic_vec4(
    values: Vec<[f32; 4]>,
    keyframe_count: usize,
) -> Result<(Vec<[f32; 4]>, Vec<[f32; 4]>, Vec<[f32; 4]>), GltfLoadError> {
    if values.len() != keyframe_count * 3 {
        return Err(GltfLoadError::InvalidField("animations.samplers.output"));
    }

    let mut in_tangents = Vec::with_capacity(keyframe_count);
    let mut key_values = Vec::with_capacity(keyframe_count);
    let mut out_tangents = Vec::with_capacity(keyframe_count);
    for triple in values.chunks_exact(3) {
        in_tangents.push(triple[0]);
        key_values.push(triple[1]);
        out_tangents.push(triple[2]);
    }

    Ok((in_tangents, key_values, out_tangents))
}

fn node_morph_target_count(root: &JsonValue, node_index: usize) -> Result<usize, GltfLoadError> {
    let node = root
        .get("nodes")
        .and_then(JsonValue::as_array)
        .and_then(|nodes| nodes.get(node_index))
        .ok_or(GltfLoadError::InvalidField(
            "animations.channels.target.node",
        ))?;
    let mesh_index = required_usize(node, "mesh")?;
    let mesh = root
        .get("meshes")
        .and_then(JsonValue::as_array)
        .and_then(|meshes| meshes.get(mesh_index))
        .ok_or(GltfLoadError::InvalidField("nodes.mesh"))?;
    if let Some(weights) = mesh.get("weights").and_then(JsonValue::as_array) {
        return Ok(weights.len());
    }
    mesh.get("primitives")
        .and_then(JsonValue::as_array)
        .and_then(|primitives| primitives.first())
        .and_then(|primitive| primitive.get("targets"))
        .and_then(JsonValue::as_array)
        .map(<[JsonValue]>::len)
        .ok_or(GltfLoadError::InvalidField(
            "animations.channels.target.path",
        ))
}

fn load_primitives(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    material_count: usize,
) -> Result<Vec<LoadedGltfPrimitive>, GltfLoadError> {
    let Some(meshes) = root.get("meshes").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut primitives = Vec::new();

    for (mesh_index, mesh_value) in meshes.iter().enumerate() {
        let default_morph_weights = optional_f32_array(mesh_value, "weights")?.unwrap_or_default();
        let mesh_primitives = mesh_value
            .get("primitives")
            .and_then(JsonValue::as_array)
            .ok_or(GltfLoadError::MissingField("meshes.primitives"))?;

        for (primitive_index, primitive_value) in mesh_primitives.iter().enumerate() {
            let mode = optional_usize(primitive_value, "mode")?.unwrap_or(4);
            if !matches!(mode, 4 | 5 | 6) {
                return Err(GltfLoadError::Unsupported(
                    "only triangle list, strip, or fan primitives are supported",
                ));
            }
            let attributes = primitive_value
                .get("attributes")
                .ok_or(GltfLoadError::MissingField("primitives.attributes"))?;
            let position_accessor = attributes
                .get("POSITION")
                .map(number_to_usize)
                .transpose()?
                .ok_or(GltfLoadError::MissingField("attributes.POSITION"))?;
            let normal_accessor = attributes.get("NORMAL").map(number_to_usize).transpose()?;
            let uv_accessor = attributes
                .get("TEXCOORD_0")
                .map(number_to_usize)
                .transpose()?;
            let uv1_accessor = attributes
                .get("TEXCOORD_1")
                .map(number_to_usize)
                .transpose()?;
            let tangent_accessor = attributes.get("TANGENT").map(number_to_usize).transpose()?;
            let color_accessor = attributes.get("COLOR_0").map(number_to_usize).transpose()?;
            let joints_accessor = attributes
                .get("JOINTS_0")
                .map(number_to_usize)
                .transpose()?;
            let weights_accessor = attributes
                .get("WEIGHTS_0")
                .map(number_to_usize)
                .transpose()?;
            let positions = read_vec3_attribute_accessor(root, buffers, position_accessor)?;
            let normals = normal_accessor
                .map(|index| read_vec3_attribute_accessor(root, buffers, index))
                .transpose()?;
            let uvs = uv_accessor
                .map(|index| read_texcoord_accessor(root, buffers, index))
                .transpose()?;
            let uv1s = uv1_accessor
                .map(|index| read_texcoord_accessor(root, buffers, index))
                .transpose()?;
            let tangents = tangent_accessor
                .map(|index| read_vec4_attribute_accessor(root, buffers, index))
                .transpose()?;
            let colors = color_accessor
                .map(|index| read_color_accessor(root, buffers, index))
                .transpose()?;
            let joints = joints_accessor
                .map(|index| read_joints_accessor(root, buffers, index))
                .transpose()?;
            let weights = weights_accessor
                .map(|index| read_weights_accessor(root, buffers, index))
                .transpose()?;
            match (joints.is_some(), weights.is_some()) {
                (true, false) => return Err(GltfLoadError::MissingField("attributes.WEIGHTS_0")),
                (false, true) => return Err(GltfLoadError::MissingField("attributes.JOINTS_0")),
                _ => {}
            }
            let mut vertices = Vec::with_capacity(positions.len());

            for (index, position) in positions.iter().copied().enumerate() {
                let uv = uvs
                    .as_ref()
                    .and_then(|uvs| uvs.get(index).copied())
                    .unwrap_or([0.0, 0.0]);
                let uv1 = uv1s
                    .as_ref()
                    .and_then(|uvs| uvs.get(index).copied())
                    .unwrap_or(uv);
                let color = colors
                    .as_ref()
                    .and_then(|colors| colors.get(index).copied())
                    .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                vertices.push(ColoredVertex::with_normal_uvs_tangent(
                    position,
                    [color[0], color[1], color[2]],
                    normals
                        .as_ref()
                        .and_then(|normals| normals.get(index).copied())
                        .map(|normal| normalize_or(normal, [0.0, 0.0, 1.0]))
                        .unwrap_or([0.0, 0.0, 1.0]),
                    uv,
                    uv1,
                    [1.0, 0.0, 0.0, 1.0],
                ));
                vertices[index].alpha = color[3];
                if let Some(tangent) = tangents.as_ref().and_then(|tangents| tangents.get(index)) {
                    vertices[index].tangent = normalize_tangent(*tangent);
                }
            }

            let source_indices = if let Some(index_accessor) = primitive_value
                .get("indices")
                .map(number_to_usize)
                .transpose()?
            {
                read_index_accessor(root, buffers, index_accessor)?
            } else {
                (0..vertices.len())
                    .map(u32::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| GltfLoadError::TooManyVertices)?
            };
            let indices = triangulate_primitive_indices(mode, source_indices);
            if indices
                .iter()
                .any(|&index| usize::try_from(index).map_or(true, |index| index >= vertices.len()))
            {
                return Err(GltfLoadError::InvalidField("primitives.indices"));
            }
            let material_index = primitive_value
                .get("material")
                .map(number_to_usize)
                .transpose()?;
            if material_index.is_some_and(|index| index >= material_count) {
                return Err(GltfLoadError::InvalidField("primitives.material"));
            }
            let morph_targets = load_morph_targets(root, buffers, primitive_value, vertices.len())?;
            if normals
                .as_ref()
                .is_some_and(|normals| normals.len() != vertices.len())
            {
                return Err(GltfLoadError::InvalidField("attributes.NORMAL"));
            }
            if colors
                .as_ref()
                .is_some_and(|colors| colors.len() != vertices.len())
            {
                return Err(GltfLoadError::InvalidField("attributes.COLOR_0"));
            }
            if joints
                .as_ref()
                .is_some_and(|joints| joints.len() != vertices.len())
                || weights
                    .as_ref()
                    .is_some_and(|weights| weights.len() != vertices.len())
            {
                return Err(GltfLoadError::InvalidField("attributes.JOINTS_0"));
            }
            if tangents
                .as_ref()
                .is_some_and(|tangents| tangents.len() != vertices.len())
            {
                return Err(GltfLoadError::InvalidField("attributes.TANGENT"));
            }
            if uvs.as_ref().is_some_and(|uvs| uvs.len() != vertices.len()) {
                return Err(GltfLoadError::InvalidField("attributes.TEXCOORD_0"));
            }
            if uv1s
                .as_ref()
                .is_some_and(|uv1s| uv1s.len() != vertices.len())
            {
                return Err(GltfLoadError::InvalidField("attributes.TEXCOORD_1"));
            }

            primitives.push(LoadedGltfPrimitive {
                mesh_index,
                primitive_index,
                material_index,
                vertices,
                indices,
                joints,
                weights,
                morph_targets,
                default_morph_weights: default_morph_weights.clone(),
                has_explicit_normals: normal_accessor.is_some(),
                has_explicit_tangents: tangent_accessor.is_some(),
            });
        }
    }

    Ok(primitives)
}

fn triangulate_primitive_indices(mode: usize, indices: Vec<u32>) -> Vec<u32> {
    match mode {
        4 => indices,
        5 => {
            let mut triangles = Vec::with_capacity(indices.len().saturating_sub(2) * 3);
            for triangle_index in 0..indices.len().saturating_sub(2) {
                let a = indices[triangle_index];
                let b = indices[triangle_index + 1];
                let c = indices[triangle_index + 2];
                if triangle_index % 2 == 0 {
                    triangles.extend_from_slice(&[a, b, c]);
                } else {
                    triangles.extend_from_slice(&[b, a, c]);
                }
            }
            triangles
        }
        6 => {
            let mut triangles = Vec::with_capacity(indices.len().saturating_sub(2) * 3);
            let Some(first) = indices.first().copied() else {
                return triangles;
            };
            for triangle_index in 1..indices.len().saturating_sub(1) {
                triangles.extend_from_slice(&[
                    first,
                    indices[triangle_index],
                    indices[triangle_index + 1],
                ]);
            }
            triangles
        }
        _ => indices,
    }
}

fn instantiate_scene_primitives(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    mesh_primitives: Vec<LoadedGltfPrimitive>,
    skins: &[GltfSkin],
) -> Result<Vec<GltfPrimitive>, GltfLoadError> {
    let Some(nodes) = root.get("nodes").and_then(JsonValue::as_array) else {
        return Ok(mesh_primitives
            .iter()
            .map(|primitive| {
                primitive.instantiate(None, Mat4::IDENTITY, None, None, None, &[], &[])
            })
            .collect());
    };
    let root_nodes = scene_root_nodes(root, nodes)?;
    let node_local_matrices = node_local_matrices(nodes)?;
    let node_parent_indices = node_parent_indices(nodes)?;
    let node_world_matrices = node_world_matrices(nodes, &root_nodes)?;
    let mut primitives = Vec::new();
    let mut stack = Vec::new();

    for node_index in root_nodes {
        append_node_primitives(
            node_index,
            Mat4::IDENTITY,
            root,
            buffers,
            nodes,
            &mesh_primitives,
            skins,
            &node_world_matrices,
            &node_local_matrices,
            &node_parent_indices,
            &mut primitives,
            &mut stack,
        )?;
    }

    Ok(primitives)
}

fn instantiate_scene_lights(root: &JsonValue) -> Result<Vec<GltfPunctualLight>, GltfLoadError> {
    let definitions = load_punctual_light_definitions(root)?;
    if definitions.is_empty() {
        return Ok(Vec::new());
    }
    let Some(nodes) = root.get("nodes").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let root_nodes = scene_root_nodes(root, nodes)?;
    let node_world_matrices = node_world_matrices(nodes, &root_nodes)?;
    let mut lights = Vec::new();

    for (node_index, node) in nodes.iter().enumerate() {
        let Some(extension) = gltf_extension(node, "KHR_lights_punctual") else {
            continue;
        };
        let light_index = extension
            .get("light")
            .map(number_to_usize)
            .transpose()?
            .ok_or(GltfLoadError::MissingField(
                "nodes.extensions.KHR_lights_punctual.light",
            ))?;
        let definition =
            definitions
                .get(light_index)
                .copied()
                .ok_or(GltfLoadError::InvalidField(
                    "nodes.extensions.KHR_lights_punctual.light",
                ))?;
        let Some(transform) = node_world_matrices.get(node_index).copied().flatten() else {
            continue;
        };

        lights.push(GltfPunctualLight {
            node_index,
            kind: definition.kind,
            color: definition.color,
            intensity: definition.intensity,
            range: definition.range,
            inner_cone_angle: definition.inner_cone_angle,
            outer_cone_angle: definition.outer_cone_angle,
            position: transform.transform_point3([0.0, 0.0, 0.0]),
            direction: normalize_or(
                transform.transform_vector3([0.0, 0.0, -1.0]),
                [0.0, 0.0, -1.0],
            ),
            transform,
        });
    }

    Ok(lights)
}

fn instantiate_scene_cameras(root: &JsonValue) -> Result<Vec<GltfCamera>, GltfLoadError> {
    let definitions = load_camera_definitions(root)?;
    if definitions.is_empty() {
        return Ok(Vec::new());
    }
    let Some(nodes) = root.get("nodes").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let root_nodes = scene_root_nodes(root, nodes)?;
    let node_world_matrices = node_world_matrices(nodes, &root_nodes)?;
    let mut cameras = Vec::new();

    for (node_index, node) in nodes.iter().enumerate() {
        let Some(camera_index) = node.get("camera").map(number_to_usize).transpose()? else {
            continue;
        };
        let definition = definitions
            .get(camera_index)
            .copied()
            .ok_or(GltfLoadError::InvalidField("nodes.camera"))?;
        let Some(transform) = node_world_matrices.get(node_index).copied().flatten() else {
            continue;
        };

        cameras.push(GltfCamera {
            node_index,
            camera_index,
            projection: definition.projection,
            position: transform.transform_point3([0.0, 0.0, 0.0]),
            right: normalize_or(
                transform.transform_vector3([1.0, 0.0, 0.0]),
                [1.0, 0.0, 0.0],
            ),
            up: normalize_or(
                transform.transform_vector3([0.0, 1.0, 0.0]),
                [0.0, 1.0, 0.0],
            ),
            forward: normalize_or(
                transform.transform_vector3([0.0, 0.0, -1.0]),
                [0.0, 0.0, -1.0],
            ),
            transform,
        });
    }

    Ok(cameras)
}

fn scene_root_nodes(root: &JsonValue, nodes: &[JsonValue]) -> Result<Vec<usize>, GltfLoadError> {
    if let Some(scenes) = root.get("scenes").and_then(JsonValue::as_array) {
        let scene_index = optional_usize(root, "scene")?.unwrap_or(0);
        let scene = scenes
            .get(scene_index)
            .ok_or(GltfLoadError::InvalidField("scene"))?;
        let Some(scene_nodes) = scene.get("nodes").and_then(JsonValue::as_array) else {
            return Ok(Vec::new());
        };
        return scene_nodes.iter().map(number_to_usize).collect();
    }

    let mut is_child = vec![false; nodes.len()];
    for node in nodes {
        if let Some(children) = node.get("children").and_then(JsonValue::as_array) {
            for child in children {
                let child_index = number_to_usize(child)?;
                if let Some(slot) = is_child.get_mut(child_index) {
                    *slot = true;
                } else {
                    return Err(GltfLoadError::InvalidField("nodes.children"));
                }
            }
        }
    }

    Ok(is_child
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_child)| (!is_child).then_some(index))
        .collect())
}

fn node_world_matrices(
    nodes: &[JsonValue],
    root_nodes: &[usize],
) -> Result<Vec<Option<Mat4>>, GltfLoadError> {
    let mut matrices = vec![None; nodes.len()];
    let mut stack = Vec::new();

    for root_node in root_nodes {
        write_node_world_matrices(*root_node, Mat4::IDENTITY, nodes, &mut matrices, &mut stack)?;
    }

    Ok(matrices)
}

fn node_local_matrices(nodes: &[JsonValue]) -> Result<Vec<Mat4>, GltfLoadError> {
    nodes.iter().map(node_transform).collect()
}

fn node_parent_indices(nodes: &[JsonValue]) -> Result<Vec<Option<usize>>, GltfLoadError> {
    let mut parents = vec![None; nodes.len()];
    for (parent_index, node) in nodes.iter().enumerate() {
        if let Some(children) = node.get("children").and_then(JsonValue::as_array) {
            for child in children {
                let child_index = number_to_usize(child)?;
                let slot = parents
                    .get_mut(child_index)
                    .ok_or(GltfLoadError::InvalidField("nodes.children"))?;
                if slot.is_some() {
                    return Err(GltfLoadError::InvalidField("nodes.children"));
                }
                *slot = Some(parent_index);
            }
        }
    }

    Ok(parents)
}

fn write_node_world_matrices(
    node_index: usize,
    parent_matrix: Mat4,
    nodes: &[JsonValue],
    matrices: &mut [Option<Mat4>],
    stack: &mut Vec<usize>,
) -> Result<(), GltfLoadError> {
    if stack.contains(&node_index) {
        return Err(GltfLoadError::InvalidField("nodes.children"));
    }
    let node = nodes
        .get(node_index)
        .ok_or(GltfLoadError::InvalidField("nodes"))?;
    stack.push(node_index);

    let matrix = parent_matrix * node_transform(node)?;
    let slot = matrices
        .get_mut(node_index)
        .ok_or(GltfLoadError::InvalidField("nodes"))?;
    *slot = Some(matrix);

    if let Some(children) = node.get("children").and_then(JsonValue::as_array) {
        for child in children {
            write_node_world_matrices(number_to_usize(child)?, matrix, nodes, matrices, stack)?;
        }
    }

    stack.pop();
    Ok(())
}

fn append_node_primitives(
    node_index: usize,
    parent_matrix: Mat4,
    root: &JsonValue,
    buffers: &[Vec<u8>],
    nodes: &[JsonValue],
    mesh_primitives: &[LoadedGltfPrimitive],
    skins: &[GltfSkin],
    node_world_matrices: &[Option<Mat4>],
    node_local_matrices: &[Mat4],
    node_parent_indices: &[Option<usize>],
    primitives: &mut Vec<GltfPrimitive>,
    stack: &mut Vec<usize>,
) -> Result<(), GltfLoadError> {
    if stack.contains(&node_index) {
        return Err(GltfLoadError::InvalidField("nodes.children"));
    }
    let node = nodes
        .get(node_index)
        .ok_or(GltfLoadError::InvalidField("nodes"))?;
    stack.push(node_index);

    let model_matrix = parent_matrix * node_transform(node)?;
    let node_weights = optional_f32_array(node, "weights")?;
    let skin_index = node.get("skin").map(number_to_usize).transpose()?;
    let skin = skin_index.and_then(|index| skins.get(index));
    let skin_joint_matrices = skin_index
        .map(|skin_index| skin_joint_matrices(skins, skin_index, node_world_matrices))
        .transpose()?;
    if let Some(mesh_index) = node.get("mesh").map(number_to_usize).transpose()? {
        let matching_primitives = mesh_primitives
            .iter()
            .filter(|primitive| primitive.mesh_index == mesh_index)
            .collect::<Vec<_>>();
        if matching_primitives.is_empty() {
            return Err(GltfLoadError::InvalidField("nodes.mesh"));
        }
        if let Some(skin) = skin {
            for primitive in &matching_primitives {
                primitive.validate_skin_attributes(skin)?;
            }
        }

        let instance_transforms = node_instance_transforms(root, buffers, node)?;
        if let Some(instance_transforms) = instance_transforms {
            for instance_transform in instance_transforms {
                for primitive in &matching_primitives {
                    primitives.push(primitive.instantiate(
                        Some(node_index),
                        model_matrix * instance_transform,
                        node_weights.as_deref(),
                        skin_joint_matrices.as_deref(),
                        skin,
                        node_local_matrices,
                        node_parent_indices,
                    ));
                }
            }
        } else {
            for primitive in matching_primitives {
                primitives.push(primitive.instantiate(
                    Some(node_index),
                    model_matrix,
                    node_weights.as_deref(),
                    skin_joint_matrices.as_deref(),
                    skin,
                    node_local_matrices,
                    node_parent_indices,
                ));
            }
        }
    }

    if let Some(children) = node.get("children").and_then(JsonValue::as_array) {
        for child in children {
            append_node_primitives(
                number_to_usize(child)?,
                model_matrix,
                root,
                buffers,
                nodes,
                mesh_primitives,
                skins,
                node_world_matrices,
                node_local_matrices,
                node_parent_indices,
                primitives,
                stack,
            )?;
        }
    }

    stack.pop();
    Ok(())
}

fn node_instance_transforms(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    node: &JsonValue,
) -> Result<Option<Vec<Mat4>>, GltfLoadError> {
    let Some(extension) = gltf_extension(node, "EXT_mesh_gpu_instancing") else {
        return Ok(None);
    };
    let attributes = extension
        .get("attributes")
        .ok_or(GltfLoadError::MissingField(
            "nodes.extensions.EXT_mesh_gpu_instancing.attributes",
        ))?;
    let translation_accessor = attributes
        .get("TRANSLATION")
        .map(number_to_usize)
        .transpose()?;
    let rotation_accessor = attributes
        .get("ROTATION")
        .map(number_to_usize)
        .transpose()?;
    let scale_accessor = attributes.get("SCALE").map(number_to_usize).transpose()?;

    if translation_accessor.is_none() && rotation_accessor.is_none() && scale_accessor.is_none() {
        return Err(GltfLoadError::MissingField(
            "nodes.extensions.EXT_mesh_gpu_instancing.attributes",
        ));
    }

    let translations = translation_accessor
        .map(|index| read_vec3_accessor(root, buffers, index))
        .transpose()?;
    let rotations = rotation_accessor
        .map(|index| read_rotation_accessor(root, buffers, index))
        .transpose()?;
    let scales = scale_accessor
        .map(|index| read_vec3_accessor(root, buffers, index))
        .transpose()?;
    let instance_count = translations
        .as_ref()
        .map(Vec::len)
        .or_else(|| rotations.as_ref().map(Vec::len))
        .or_else(|| scales.as_ref().map(Vec::len))
        .unwrap_or(0);

    if translations
        .as_ref()
        .is_some_and(|values| values.len() != instance_count)
        || rotations
            .as_ref()
            .is_some_and(|values| values.len() != instance_count)
        || scales
            .as_ref()
            .is_some_and(|values| values.len() != instance_count)
    {
        return Err(GltfLoadError::InvalidField(
            "nodes.extensions.EXT_mesh_gpu_instancing.attributes",
        ));
    }

    let mut transforms = Vec::with_capacity(instance_count);
    for index in 0..instance_count {
        let translation = translations
            .as_ref()
            .and_then(|values| values.get(index).copied())
            .unwrap_or([0.0, 0.0, 0.0]);
        let rotation = rotations
            .as_ref()
            .and_then(|values| values.get(index).copied())
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let scale = scales
            .as_ref()
            .and_then(|values| values.get(index).copied())
            .unwrap_or([1.0, 1.0, 1.0]);

        transforms.push(
            Mat4::translation(translation)
                * Mat4::rotation_quaternion(rotation)
                * Mat4::scale(scale),
        );
    }

    Ok(Some(transforms))
}

fn node_transform(node: &JsonValue) -> Result<Mat4, GltfLoadError> {
    if let Some(matrix) = optional_mat4(node, "matrix")? {
        return Ok(matrix);
    }

    let translation = optional_vec3(node, "translation")?.unwrap_or([0.0, 0.0, 0.0]);
    let rotation = optional_vec4(node, "rotation")?.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let scale = optional_vec3(node, "scale")?.unwrap_or([1.0, 1.0, 1.0]);

    Ok(Mat4::translation(translation) * Mat4::rotation_quaternion(rotation) * Mat4::scale(scale))
}

fn load_skins(root: &JsonValue, buffers: &[Vec<u8>]) -> Result<Vec<GltfSkin>, GltfLoadError> {
    let Some(skins) = root.get("skins").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut loaded = Vec::with_capacity(skins.len());

    for skin in skins {
        let joints = skin
            .get("joints")
            .and_then(JsonValue::as_array)
            .ok_or(GltfLoadError::MissingField("skins.joints"))?
            .iter()
            .map(number_to_usize)
            .collect::<Result<Vec<_>, _>>()?;
        let inverse_bind_matrices = skin
            .get("inverseBindMatrices")
            .map(number_to_usize)
            .transpose()?
            .map(|index| read_mat4_accessor(root, buffers, index))
            .transpose()?
            .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);

        if inverse_bind_matrices.len() != joints.len() {
            return Err(GltfLoadError::InvalidField("skins.inverseBindMatrices"));
        }

        loaded.push(GltfSkin {
            joints,
            inverse_bind_matrices,
        });
    }

    Ok(loaded)
}

fn skin_joint_matrices(
    skins: &[GltfSkin],
    skin_index: usize,
    node_world_matrices: &[Option<Mat4>],
) -> Result<Vec<Mat4>, GltfLoadError> {
    let skin = skins
        .get(skin_index)
        .ok_or(GltfLoadError::InvalidField("nodes.skin"))?;
    let mut joint_matrices = Vec::with_capacity(skin.joints.len());

    for (joint_index, joint_node) in skin.joints.iter().copied().enumerate() {
        let joint_matrix = node_world_matrices
            .get(joint_node)
            .copied()
            .flatten()
            .ok_or(GltfLoadError::InvalidField("skins.joints"))?;
        joint_matrices.push(joint_matrix * skin.inverse_bind_matrices[joint_index]);
    }

    Ok(joint_matrices)
}

fn load_morph_targets(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    primitive_value: &JsonValue,
    vertex_count: usize,
) -> Result<Vec<GltfMorphTarget>, GltfLoadError> {
    let Some(targets) = primitive_value.get("targets").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    let mut morph_targets = Vec::with_capacity(targets.len());

    for target in targets {
        let positions = target
            .get("POSITION")
            .map(number_to_usize)
            .transpose()?
            .map(|index| read_vec3_attribute_accessor(root, buffers, index))
            .transpose()?;
        let normals = target
            .get("NORMAL")
            .map(number_to_usize)
            .transpose()?
            .map(|index| read_vec3_attribute_accessor(root, buffers, index))
            .transpose()?;
        let tangents = target
            .get("TANGENT")
            .map(number_to_usize)
            .transpose()?
            .map(|index| read_vec3_attribute_accessor(root, buffers, index))
            .transpose()?;

        if positions
            .as_ref()
            .is_some_and(|positions| positions.len() != vertex_count)
            || normals
                .as_ref()
                .is_some_and(|normals| normals.len() != vertex_count)
            || tangents
                .as_ref()
                .is_some_and(|tangents| tangents.len() != vertex_count)
        {
            return Err(GltfLoadError::InvalidField("primitives.targets"));
        }

        morph_targets.push(GltfMorphTarget {
            positions,
            normals,
            tangents,
        });
    }

    Ok(morph_targets)
}

fn apply_morph_targets(
    base_vertices: &[ColoredVertex],
    morph_targets: &[GltfMorphTarget],
    weights: &[f32],
) -> Vec<ColoredVertex> {
    if morph_targets.is_empty() {
        return base_vertices.to_vec();
    }

    let mut vertices = base_vertices.to_vec();
    for (target_index, target) in morph_targets.iter().enumerate() {
        let weight = weights.get(target_index).copied().unwrap_or(0.0);
        if weight == 0.0 {
            continue;
        }

        if let Some(position_deltas) = &target.positions {
            for (vertex, delta) in vertices.iter_mut().zip(position_deltas) {
                for axis in 0..3 {
                    vertex.position[axis] += delta[axis] * weight;
                }
            }
        }

        if let Some(normal_deltas) = &target.normals {
            for (vertex, delta) in vertices.iter_mut().zip(normal_deltas) {
                for axis in 0..3 {
                    vertex.normal[axis] += delta[axis] * weight;
                }
                vertex.normal = normalize_or(vertex.normal, [0.0, 0.0, 1.0]);
            }
        }

        if let Some(tangent_deltas) = &target.tangents {
            for (vertex, delta) in vertices.iter_mut().zip(tangent_deltas) {
                let tangent = [
                    vertex.tangent[0] + delta[0] * weight,
                    vertex.tangent[1] + delta[1] * weight,
                    vertex.tangent[2] + delta[2] * weight,
                ];
                let tangent = normalize_or(tangent, [1.0, 0.0, 0.0]);
                vertex.tangent = [
                    tangent[0],
                    tangent[1],
                    tangent[2],
                    if vertex.tangent[3] < 0.0 { -1.0 } else { 1.0 },
                ];
            }
        }
    }

    vertices
}

fn apply_skinning(
    base_vertices: &[ColoredVertex],
    joints: Option<&[[usize; 4]]>,
    weights: Option<&[[f32; 4]]>,
    joint_matrices: &[Mat4],
) -> Vec<ColoredVertex> {
    let (Some(joints), Some(weights)) = (joints, weights) else {
        return base_vertices.to_vec();
    };

    let mut vertices = base_vertices.to_vec();
    for ((vertex, joint_indices), joint_weights) in vertices.iter_mut().zip(joints).zip(weights) {
        let base_position = vertex.position;
        let base_normal = vertex.normal;
        let base_tangent = [vertex.tangent[0], vertex.tangent[1], vertex.tangent[2]];
        let mut skinned_position = [0.0; 3];
        let mut skinned_normal = [0.0; 3];
        let mut skinned_tangent = [0.0; 3];
        let mut total_weight = 0.0;

        for influence in 0..4 {
            let weight = joint_weights[influence];
            if weight == 0.0 {
                continue;
            }
            let Some(joint_matrix) = joint_matrices.get(joint_indices[influence]).copied() else {
                continue;
            };
            let transformed_position = joint_matrix.transform_point3(base_position);
            let normal_matrix = joint_matrix.normal_matrix();
            let transformed_normal = normal_matrix.transform_vector3(base_normal);
            let transformed_tangent = normal_matrix.transform_vector3(base_tangent);

            for axis in 0..3 {
                skinned_position[axis] += transformed_position[axis] * weight;
                skinned_normal[axis] += transformed_normal[axis] * weight;
                skinned_tangent[axis] += transformed_tangent[axis] * weight;
            }
            total_weight += weight;
        }

        if total_weight > f32::EPSILON {
            vertex.position = [
                skinned_position[0] / total_weight,
                skinned_position[1] / total_weight,
                skinned_position[2] / total_weight,
            ];
            vertex.normal = normalize_or(skinned_normal, base_normal);
            let tangent = normalize_or(skinned_tangent, base_tangent);
            vertex.tangent = [tangent[0], tangent[1], tangent[2], vertex.tangent[3]];
        }
    }

    vertices
}

fn read_vec3_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 3]>, GltfLoadError> {
    let view = AccessorView::new(root, buffers, accessor_index, "VEC3", 5126)?;
    let mut values = Vec::with_capacity(view.count);
    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_f32_le(view.data.as_slice(), offset)?,
            read_f32_le(view.data.as_slice(), offset + 4)?,
            read_f32_le(view.data.as_slice(), offset + 8)?,
        ]);
    }
    Ok(values)
}

fn read_vec3_attribute_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 3]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC3" {
        return Err(GltfLoadError::Unsupported(
            "mesh VEC3 attributes must use VEC3 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5120 | 5121 | 5122 | 5123 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "mesh VEC3 attributes support BYTE, UNSIGNED_BYTE, SHORT, UNSIGNED_SHORT, or FLOAT accessors",
        ));
    }
    let normalized = accessor
        .get("normalized")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 3)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_attribute_component(view.data.as_slice(), offset, component_type, normalized)?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
                normalized,
            )?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
                normalized,
            )?,
        ]);
    }

    Ok(values)
}

fn read_scalar_f32_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<f32>, GltfLoadError> {
    let view = AccessorView::new(root, buffers, accessor_index, "SCALAR", 5126)?;
    let mut values = Vec::with_capacity(view.count);
    for index in 0..view.count {
        values.push(read_f32_le(view.data.as_slice(), view.offset_for(index))?);
    }
    Ok(values)
}

fn read_vec4_attribute_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 4]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC4" {
        return Err(GltfLoadError::Unsupported(
            "mesh VEC4 attributes must use VEC4 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5120 | 5121 | 5122 | 5123 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "mesh VEC4 attributes support BYTE, UNSIGNED_BYTE, SHORT, UNSIGNED_SHORT, or FLOAT accessors",
        ));
    }
    let normalized = accessor
        .get("normalized")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 4)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_attribute_component(view.data.as_slice(), offset, component_type, normalized)?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
                normalized,
            )?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
                normalized,
            )?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size * 3,
                component_type,
                normalized,
            )?,
        ]);
    }

    Ok(values)
}

fn read_rotation_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 4]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC4" {
        return Err(GltfLoadError::Unsupported(
            "rotation animation outputs must use VEC4 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5120 | 5122 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "rotation animation outputs support FLOAT, normalized BYTE, or normalized SHORT accessors",
        ));
    }
    let normalized = accessor
        .get("normalized")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    if matches!(component_type, 5120 | 5122) && !normalized {
        return Err(GltfLoadError::InvalidField("animations.samplers.output"));
    }

    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 4)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_attribute_component(view.data.as_slice(), offset, component_type, normalized)?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
                normalized,
            )?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
                normalized,
            )?,
            read_attribute_component(
                view.data.as_slice(),
                offset + component_size * 3,
                component_type,
                normalized,
            )?,
        ]);
    }

    Ok(values)
}

fn read_texcoord_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 2]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC2" {
        return Err(GltfLoadError::Unsupported(
            "TEXCOORD_n must use VEC2 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5121 | 5123 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "TEXCOORD_n supports FLOAT, UNSIGNED_BYTE, or UNSIGNED_SHORT accessors",
        ));
    }

    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 2)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_texcoord_component(view.data.as_slice(), offset, component_type)?,
            read_texcoord_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
            )?,
        ]);
    }

    Ok(values)
}

fn read_color_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 4]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if !matches!(accessor_type, "VEC3" | "VEC4") {
        return Err(GltfLoadError::Unsupported(
            "COLOR_0 must use VEC3 or VEC4 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5121 | 5123 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "COLOR_0 supports FLOAT, UNSIGNED_BYTE, or UNSIGNED_SHORT accessors",
        ));
    }
    let components = components_for_type(accessor_type)?;
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, components)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_color_component(view.data.as_slice(), offset, component_type)?,
            read_color_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
            )?,
            read_color_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
            )?,
            if components == 4 {
                read_color_component(
                    view.data.as_slice(),
                    offset + component_size * 3,
                    component_type,
                )?
            } else {
                1.0
            },
        ]);
    }

    Ok(values)
}

fn read_joints_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[usize; 4]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC4" {
        return Err(GltfLoadError::Unsupported(
            "JOINTS_0 must use VEC4 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5121 | 5123) {
        return Err(GltfLoadError::Unsupported(
            "JOINTS_0 supports UNSIGNED_BYTE or UNSIGNED_SHORT accessors",
        ));
    }
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 4)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_joint_component(view.data.as_slice(), offset, component_type)?,
            read_joint_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
            )?,
            read_joint_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
            )?,
            read_joint_component(
                view.data.as_slice(),
                offset + component_size * 3,
                component_type,
            )?,
        ]);
    }

    Ok(values)
}

fn read_weights_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<[f32; 4]>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let accessor_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if accessor_type != "VEC4" {
        return Err(GltfLoadError::Unsupported(
            "WEIGHTS_0 must use VEC4 accessors",
        ));
    }
    let component_type = required_usize(accessor, "componentType")?;
    if !matches!(component_type, 5121 | 5123 | 5126) {
        return Err(GltfLoadError::Unsupported(
            "WEIGHTS_0 supports FLOAT, UNSIGNED_BYTE, or UNSIGNED_SHORT accessors",
        ));
    }
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 4)?;
    let component_size = component_size(component_type)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        values.push([
            read_color_component(view.data.as_slice(), offset, component_type)?,
            read_color_component(
                view.data.as_slice(),
                offset + component_size,
                component_type,
            )?,
            read_color_component(
                view.data.as_slice(),
                offset + component_size * 2,
                component_type,
            )?,
            read_color_component(
                view.data.as_slice(),
                offset + component_size * 3,
                component_type,
            )?,
        ]);
    }

    Ok(values)
}

fn read_mat4_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<Mat4>, GltfLoadError> {
    let view = AccessorView::new(root, buffers, accessor_index, "MAT4", 5126)?;
    let mut values = Vec::with_capacity(view.count);

    for index in 0..view.count {
        let offset = view.offset_for(index);
        let mut cols = [[0.0; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                cols[col][row] = read_f32_le(view.data.as_slice(), offset + (col * 4 + row) * 4)?;
            }
        }
        values.push(Mat4::from_cols_array(cols));
    }

    Ok(values)
}

fn read_index_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    accessor_index: usize,
) -> Result<Vec<u32>, GltfLoadError> {
    let accessor = accessor(root, accessor_index)?;
    let component_type = required_usize(accessor, "componentType")?;
    let scalar_type = accessor
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or(GltfLoadError::MissingField("accessors.type"))?;
    if scalar_type != "SCALAR" {
        return Err(GltfLoadError::Unsupported(
            "indices must use SCALAR accessors",
        ));
    }
    let view = AccessorView::new_any(root, buffers, accessor_index, component_type, 1)?;
    let mut indices = Vec::with_capacity(view.count);
    for index in 0..view.count {
        let offset = view.offset_for(index);
        indices.push(match component_type {
            5121 => view
                .data
                .get(offset)
                .copied()
                .map(u32::from)
                .ok_or(GltfLoadError::BufferOutOfBounds)?,
            5123 => u32::from(read_u16_le(view.data.as_slice(), offset)?),
            5125 => read_u32_le(view.data.as_slice(), offset)?,
            _ => {
                return Err(GltfLoadError::Unsupported(
                    "unsupported index component type",
                ))
            }
        });
    }
    Ok(indices)
}

struct AccessorView<'a> {
    data: Vec<u8>,
    stride: usize,
    count: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> AccessorView<'a> {
    fn new(
        root: &'a JsonValue,
        buffers: &'a [Vec<u8>],
        accessor_index: usize,
        expected_type: &'static str,
        expected_component_type: usize,
    ) -> Result<Self, GltfLoadError> {
        let accessor = accessor(root, accessor_index)?;
        let accessor_type = accessor
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or(GltfLoadError::MissingField("accessors.type"))?;
        if accessor_type != expected_type {
            return Err(GltfLoadError::Unsupported(
                "accessor type does not match attribute",
            ));
        }
        let component_type = required_usize(accessor, "componentType")?;
        if component_type != expected_component_type {
            return Err(GltfLoadError::Unsupported(
                "only FLOAT vertex attributes are supported",
            ));
        }
        Self::new_any(
            root,
            buffers,
            accessor_index,
            component_type,
            components_for_type(expected_type)?,
        )
    }

    fn new_any(
        root: &'a JsonValue,
        buffers: &'a [Vec<u8>],
        accessor_index: usize,
        component_type: usize,
        components: usize,
    ) -> Result<Self, GltfLoadError> {
        let accessor = accessor(root, accessor_index)?;
        let count = required_usize(accessor, "count")?;
        let component_size = component_size(component_type)?;
        let element_size = component_size * components;
        let data_len = count
            .checked_mul(element_size)
            .ok_or(GltfLoadError::BufferOutOfBounds)?;
        let mut data = vec![0; data_len];

        if let Some(buffer_view_index) = accessor
            .get("bufferView")
            .map(number_to_usize)
            .transpose()?
        {
            let buffer_view = buffer_view(root, buffer_view_index)?;
            let source = buffer_view_data(root, buffers, buffer_view_index)?;
            let accessor_offset = optional_usize(accessor, "byteOffset")?.unwrap_or(0);
            let source_stride = optional_usize(buffer_view, "byteStride")?.unwrap_or(element_size);
            if source_stride < element_size {
                return Err(GltfLoadError::InvalidField("bufferViews.byteStride"));
            }

            for index in 0..count {
                let source_offset = accessor_offset
                    .checked_add(
                        index
                            .checked_mul(source_stride)
                            .ok_or(GltfLoadError::BufferOutOfBounds)?,
                    )
                    .ok_or(GltfLoadError::BufferOutOfBounds)?;
                let destination_offset = index * element_size;
                data[destination_offset..destination_offset + element_size].copy_from_slice(
                    source
                        .get(source_offset..source_offset + element_size)
                        .ok_or(GltfLoadError::BufferOutOfBounds)?,
                );
            }
        }

        if let Some(sparse) = accessor.get("sparse") {
            apply_sparse_accessor(
                root,
                buffers,
                sparse,
                count,
                component_type,
                element_size,
                &mut data,
            )?;
        }

        Ok(Self {
            data,
            stride: element_size,
            count,
            _marker: std::marker::PhantomData,
        })
    }

    fn offset_for(&self, index: usize) -> usize {
        index * self.stride
    }
}

fn apply_sparse_accessor(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    sparse: &JsonValue,
    accessor_count: usize,
    component_type: usize,
    element_size: usize,
    data: &mut [u8],
) -> Result<(), GltfLoadError> {
    let sparse_count = required_usize(sparse, "count")?;
    let indices = sparse
        .get("indices")
        .ok_or(GltfLoadError::MissingField("accessors.sparse.indices"))?;
    let values = sparse
        .get("values")
        .ok_or(GltfLoadError::MissingField("accessors.sparse.values"))?;
    let index_component_type = required_usize(indices, "componentType")?;
    if !matches!(index_component_type, 5121 | 5123 | 5125) {
        return Err(GltfLoadError::Unsupported(
            "sparse accessor indices support UNSIGNED_BYTE, UNSIGNED_SHORT, or UNSIGNED_INT",
        ));
    }
    let index_component_size = component_size(index_component_type)?;
    let index_data = buffer_view_data(root, buffers, required_usize(indices, "bufferView")?)?;
    let index_offset = optional_usize(indices, "byteOffset")?.unwrap_or(0);
    let value_data = buffer_view_data(root, buffers, required_usize(values, "bufferView")?)?;
    let value_offset = optional_usize(values, "byteOffset")?.unwrap_or(0);
    let _ = component_size(component_type)?;

    for sparse_index in 0..sparse_count {
        let index_offset = index_offset
            .checked_add(
                sparse_index
                    .checked_mul(index_component_size)
                    .ok_or(GltfLoadError::BufferOutOfBounds)?,
            )
            .ok_or(GltfLoadError::BufferOutOfBounds)?;
        let destination_index = read_sparse_index(&index_data, index_offset, index_component_type)?;
        if destination_index >= accessor_count {
            return Err(GltfLoadError::InvalidField("accessors.sparse.indices"));
        }

        let sparse_value_offset = value_offset
            .checked_add(
                sparse_index
                    .checked_mul(element_size)
                    .ok_or(GltfLoadError::BufferOutOfBounds)?,
            )
            .ok_or(GltfLoadError::BufferOutOfBounds)?;
        let destination_offset = destination_index
            .checked_mul(element_size)
            .ok_or(GltfLoadError::BufferOutOfBounds)?;
        data[destination_offset..destination_offset + element_size].copy_from_slice(
            value_data
                .get(sparse_value_offset..sparse_value_offset + element_size)
                .ok_or(GltfLoadError::BufferOutOfBounds)?,
        );
    }

    Ok(())
}

fn read_sparse_index(
    bytes: &[u8],
    offset: usize,
    component_type: usize,
) -> Result<usize, GltfLoadError> {
    match component_type {
        5121 => bytes
            .get(offset)
            .copied()
            .map(usize::from)
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5123 => read_u16_le(bytes, offset).map(usize::from),
        5125 => usize::try_from(read_u32_le(bytes, offset)?)
            .map_err(|_| GltfLoadError::InvalidField("accessors.sparse.indices")),
        _ => Err(GltfLoadError::Unsupported(
            "unsupported sparse accessor index component type",
        )),
    }
}

fn accessor(root: &JsonValue, index: usize) -> Result<&JsonValue, GltfLoadError> {
    root.get("accessors")
        .and_then(JsonValue::as_array)
        .and_then(|accessors| accessors.get(index))
        .ok_or(GltfLoadError::MissingField("accessors"))
}

fn buffer_view(root: &JsonValue, index: usize) -> Result<&JsonValue, GltfLoadError> {
    root.get("bufferViews")
        .and_then(JsonValue::as_array)
        .and_then(|views| views.get(index))
        .ok_or(GltfLoadError::MissingField("bufferViews"))
}

fn buffer_view_data(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    index: usize,
) -> Result<Vec<u8>, GltfLoadError> {
    let buffer_view = buffer_view(root, index)?;
    let buffer_index = required_usize(buffer_view, "buffer")?;
    let buffer = buffers
        .get(buffer_index)
        .ok_or(GltfLoadError::MissingField("buffers"))?;
    let view_offset = optional_usize(buffer_view, "byteOffset")?.unwrap_or(0);
    let view_length = required_usize(buffer_view, "byteLength")?;
    let view_end = view_offset
        .checked_add(view_length)
        .ok_or(GltfLoadError::BufferOutOfBounds)?;

    Ok(buffer
        .get(view_offset..view_end)
        .ok_or(GltfLoadError::BufferOutOfBounds)
        .map(<[u8]>::to_vec)?)
}

fn read_buffer_view_bytes(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    index: usize,
) -> Result<Vec<u8>, GltfLoadError> {
    buffer_view_data(root, buffers, index)
}

fn components_for_type(value: &str) -> Result<usize, GltfLoadError> {
    match value {
        "SCALAR" => Ok(1),
        "VEC2" => Ok(2),
        "VEC3" => Ok(3),
        "VEC4" => Ok(4),
        "MAT4" => Ok(16),
        _ => Err(GltfLoadError::Unsupported("unsupported accessor type")),
    }
}

fn component_size(component_type: usize) -> Result<usize, GltfLoadError> {
    match component_type {
        5120 | 5121 => Ok(1),
        5122 | 5123 => Ok(2),
        5125 | 5126 => Ok(4),
        _ => Err(GltfLoadError::Unsupported("unsupported component type")),
    }
}

fn required_usize(value: &JsonValue, field: &'static str) -> Result<usize, GltfLoadError> {
    value
        .get(field)
        .map(number_to_usize)
        .transpose()?
        .ok_or(GltfLoadError::MissingField(field))
}

fn optional_usize(value: &JsonValue, field: &'static str) -> Result<Option<usize>, GltfLoadError> {
    value.get(field).map(number_to_usize).transpose()
}

fn required_f32(value: &JsonValue, field: &'static str) -> Result<f32, GltfLoadError> {
    value
        .get(field)
        .map(number_to_f32)
        .transpose()?
        .ok_or(GltfLoadError::MissingField(field))
}

fn optional_f32(value: &JsonValue, field: &'static str) -> Result<Option<f32>, GltfLoadError> {
    value.get(field).map(number_to_f32).transpose()
}

fn optional_f32_array(
    value: &JsonValue,
    field: &'static str,
) -> Result<Option<Vec<f32>>, GltfLoadError> {
    let Some(array) = value.get(field).and_then(JsonValue::as_array) else {
        return Ok(None);
    };

    array
        .iter()
        .map(number_to_f32)
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_vec2(
    value: &JsonValue,
    field: &'static str,
) -> Result<Option<[f32; 2]>, GltfLoadError> {
    let Some(array) = value.get(field).and_then(JsonValue::as_array) else {
        return Ok(None);
    };
    if array.len() != 2 {
        return Err(GltfLoadError::InvalidField(field));
    }
    Ok(Some([number_to_f32(&array[0])?, number_to_f32(&array[1])?]))
}

fn optional_vec3(
    value: &JsonValue,
    field: &'static str,
) -> Result<Option<[f32; 3]>, GltfLoadError> {
    let Some(array) = value.get(field).and_then(JsonValue::as_array) else {
        return Ok(None);
    };
    if array.len() != 3 {
        return Err(GltfLoadError::InvalidField(field));
    }
    Ok(Some([
        number_to_f32(&array[0])?,
        number_to_f32(&array[1])?,
        number_to_f32(&array[2])?,
    ]))
}

fn optional_vec4(
    value: &JsonValue,
    field: &'static str,
) -> Result<Option<[f32; 4]>, GltfLoadError> {
    let Some(array) = value.get(field).and_then(JsonValue::as_array) else {
        return Ok(None);
    };
    if array.len() != 4 {
        return Err(GltfLoadError::InvalidField(field));
    }
    Ok(Some([
        number_to_f32(&array[0])?,
        number_to_f32(&array[1])?,
        number_to_f32(&array[2])?,
        number_to_f32(&array[3])?,
    ]))
}

fn optional_mat4(value: &JsonValue, field: &'static str) -> Result<Option<Mat4>, GltfLoadError> {
    let Some(array) = value.get(field).and_then(JsonValue::as_array) else {
        return Ok(None);
    };
    if array.len() != 16 {
        return Err(GltfLoadError::InvalidField(field));
    }

    let mut cols = [[0.0; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            cols[col][row] = number_to_f32(&array[col * 4 + row])?;
        }
    }
    Ok(Some(Mat4::from_cols_array(cols)))
}

fn number_to_usize(value: &JsonValue) -> Result<usize, GltfLoadError> {
    let number = value
        .as_number()
        .ok_or(GltfLoadError::InvalidField("number"))?;
    if number < 0.0 || number.fract() != 0.0 {
        return Err(GltfLoadError::InvalidField("number"));
    }
    Ok(number as usize)
}

fn number_to_f32(value: &JsonValue) -> Result<f32, GltfLoadError> {
    value
        .as_number()
        .map(|value| value as f32)
        .ok_or(GltfLoadError::InvalidField("number"))
}

fn animation_sample_indices(input: &[f32], time: f32) -> (usize, usize, f32) {
    if input.len() == 1 || time <= input[0] {
        return (0, 0, 0.0);
    }
    let last = input.len() - 1;
    if time >= input[last] {
        return (last, last, 0.0);
    }

    for index in 0..last {
        let start = input[index];
        let end = input[index + 1];
        if time >= start && time <= end {
            let duration = end - start;
            let t = if duration.abs() > f32::EPSILON {
                (time - start) / duration
            } else {
                0.0
            };
            return (index, index + 1, t.clamp(0.0, 1.0));
        }
    }

    (last, last, 0.0)
}

fn linear_sample_t(interpolation: GltfAnimationInterpolation, t: f32) -> f32 {
    match interpolation {
        GltfAnimationInterpolation::Step => 0.0,
        GltfAnimationInterpolation::Linear | GltfAnimationInterpolation::CubicSpline => t,
    }
}

fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn lerp_weights(a: &[f32], b: &[f32], t: f32) -> Vec<f32> {
    a.iter().zip(b).map(|(a, b)| a + (b - a) * t).collect()
}

fn lerp_animation_weights(a: &[f32], b: &[f32], t: f32) -> Vec<f32> {
    let len = a.len().max(b.len());
    (0..len)
        .map(|index| {
            let a = a.get(index).copied().unwrap_or(0.0);
            let b = b.get(index).copied().unwrap_or(0.0);
            a + (b - a) * t
        })
        .collect()
}

fn cubic_scalar(
    value0: f32,
    out_tangent0: f32,
    in_tangent1: f32,
    value1: f32,
    t: f32,
    delta_time: f32,
) -> f32 {
    if delta_time.abs() <= f32::EPSILON {
        return value0;
    }

    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    h00 * value0 + h10 * delta_time * out_tangent0 + h01 * value1 + h11 * delta_time * in_tangent1
}

fn cubic_vec3(
    value0: [f32; 3],
    out_tangent0: [f32; 3],
    in_tangent1: [f32; 3],
    value1: [f32; 3],
    t: f32,
    delta_time: f32,
) -> [f32; 3] {
    [
        cubic_scalar(
            value0[0],
            out_tangent0[0],
            in_tangent1[0],
            value1[0],
            t,
            delta_time,
        ),
        cubic_scalar(
            value0[1],
            out_tangent0[1],
            in_tangent1[1],
            value1[1],
            t,
            delta_time,
        ),
        cubic_scalar(
            value0[2],
            out_tangent0[2],
            in_tangent1[2],
            value1[2],
            t,
            delta_time,
        ),
    ]
}

fn cubic_vec4(
    value0: [f32; 4],
    out_tangent0: [f32; 4],
    in_tangent1: [f32; 4],
    value1: [f32; 4],
    t: f32,
    delta_time: f32,
) -> [f32; 4] {
    [
        cubic_scalar(
            value0[0],
            out_tangent0[0],
            in_tangent1[0],
            value1[0],
            t,
            delta_time,
        ),
        cubic_scalar(
            value0[1],
            out_tangent0[1],
            in_tangent1[1],
            value1[1],
            t,
            delta_time,
        ),
        cubic_scalar(
            value0[2],
            out_tangent0[2],
            in_tangent1[2],
            value1[2],
            t,
            delta_time,
        ),
        cubic_scalar(
            value0[3],
            out_tangent0[3],
            in_tangent1[3],
            value1[3],
            t,
            delta_time,
        ),
    ]
}

fn cubic_weights(
    value0: &[f32],
    out_tangent0: &[f32],
    in_tangent1: &[f32],
    value1: &[f32],
    t: f32,
    delta_time: f32,
) -> Vec<f32> {
    value0
        .iter()
        .zip(out_tangent0)
        .zip(in_tangent1)
        .zip(value1)
        .map(|(((value0, out_tangent0), in_tangent1), value1)| {
            cubic_scalar(*value0, *out_tangent0, *in_tangent1, *value1, t, delta_time)
        })
        .collect()
}

fn slerp_quaternion(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut cos_theta = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if cos_theta < 0.0 {
        cos_theta = -cos_theta;
        b = [-b[0], -b[1], -b[2], -b[3]];
    }

    if cos_theta > 0.9995 {
        return normalize_quaternion([
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ]);
    }

    let theta = cos_theta.acos();
    let sin_theta = theta.sin();
    if sin_theta.abs() <= f32::EPSILON {
        return a;
    }

    let weight_a = ((1.0 - t) * theta).sin() / sin_theta;
    let weight_b = (t * theta).sin() / sin_theta;
    normalize_quaternion([
        a[0] * weight_a + b[0] * weight_b,
        a[1] * weight_a + b[1] * weight_b,
        a[2] * weight_a + b[2] * weight_b,
        a[3] * weight_a + b[3] * weight_b,
    ])
}

fn normalize_quaternion(value: [f32; 4]) -> [f32; 4] {
    let length_squared =
        value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3];
    if length_squared > f32::EPSILON {
        let inverse_length = 1.0 / length_squared.sqrt();
        [
            value[0] * inverse_length,
            value[1] * inverse_length,
            value[2] * inverse_length,
            value[3] * inverse_length,
        ]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
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

fn normalize_tangent(tangent: [f32; 4]) -> [f32; 4] {
    let normalized = normalize_or([tangent[0], tangent[1], tangent[2]], [1.0, 0.0, 0.0]);
    [
        normalized[0],
        normalized[1],
        normalized[2],
        if tangent[3] < 0.0 { -1.0 } else { 1.0 },
    ]
}

fn read_color_component(
    bytes: &[u8],
    offset: usize,
    component_type: usize,
) -> Result<f32, GltfLoadError> {
    match component_type {
        5121 => bytes
            .get(offset)
            .copied()
            .map(|value| f32::from(value) / 255.0)
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5123 => read_u16_le(bytes, offset).map(|value| f32::from(value) / 65535.0),
        5126 => read_f32_le(bytes, offset),
        _ => Err(GltfLoadError::Unsupported(
            "unsupported COLOR_0 component type",
        )),
    }
}

fn read_texcoord_component(
    bytes: &[u8],
    offset: usize,
    component_type: usize,
) -> Result<f32, GltfLoadError> {
    match component_type {
        5121 => bytes
            .get(offset)
            .copied()
            .map(|value| f32::from(value) / 255.0)
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5123 => read_u16_le(bytes, offset).map(|value| f32::from(value) / 65535.0),
        5126 => read_f32_le(bytes, offset),
        _ => Err(GltfLoadError::Unsupported(
            "unsupported TEXCOORD_n component type",
        )),
    }
}

fn read_attribute_component(
    bytes: &[u8],
    offset: usize,
    component_type: usize,
    normalized: bool,
) -> Result<f32, GltfLoadError> {
    match component_type {
        5120 => bytes
            .get(offset)
            .copied()
            .map(|value| {
                let value = i8::from_le_bytes([value]);
                if normalized {
                    (f32::from(value) / 127.0).max(-1.0)
                } else {
                    f32::from(value)
                }
            })
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5121 => bytes
            .get(offset)
            .copied()
            .map(|value| {
                if normalized {
                    f32::from(value) / 255.0
                } else {
                    f32::from(value)
                }
            })
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5122 => read_i16_le(bytes, offset).map(|value| {
            if normalized {
                (f32::from(value) / 32767.0).max(-1.0)
            } else {
                f32::from(value)
            }
        }),
        5123 => read_u16_le(bytes, offset).map(|value| {
            if normalized {
                f32::from(value) / 65535.0
            } else {
                f32::from(value)
            }
        }),
        5126 => read_f32_le(bytes, offset),
        _ => Err(GltfLoadError::Unsupported(
            "unsupported mesh attribute component type",
        )),
    }
}

fn read_joint_component(
    bytes: &[u8],
    offset: usize,
    component_type: usize,
) -> Result<usize, GltfLoadError> {
    match component_type {
        5121 => bytes
            .get(offset)
            .copied()
            .map(usize::from)
            .ok_or(GltfLoadError::BufferOutOfBounds),
        5123 => read_u16_le(bytes, offset).map(usize::from),
        _ => Err(GltfLoadError::Unsupported(
            "unsupported JOINTS_0 component type",
        )),
    }
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, GltfLoadError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(GltfLoadError::BufferOutOfBounds)?;
    Ok(f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, GltfLoadError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(GltfLoadError::BufferOutOfBounds)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Result<i16, GltfLoadError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(GltfLoadError::BufferOutOfBounds)?;
    Ok(i16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, GltfLoadError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(GltfLoadError::BufferOutOfBounds)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn decode_data_uri_base64(uri: &str) -> Result<Option<Vec<u8>>, GltfLoadError> {
    Ok(decode_data_uri_base64_with_mime(uri)?.map(|(_, bytes)| bytes))
}

fn decode_data_uri_base64_with_mime(
    uri: &str,
) -> Result<Option<(Option<String>, Vec<u8>)>, GltfLoadError> {
    let Some(data) = uri.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((metadata, payload)) = data.split_once(',') else {
        return Err(GltfLoadError::InvalidField("buffers.uri"));
    };
    if !metadata.ends_with(";base64") {
        return Err(GltfLoadError::Unsupported(
            "only base64 data URI buffers are supported",
        ));
    }
    let mime_type = metadata
        .strip_suffix(";base64")
        .filter(|mime_type| !mime_type.is_empty())
        .map(str::to_owned);
    decode_base64(payload).map(|bytes| Some((mime_type, bytes)))
}

fn decode_base64(source: &str) -> Result<Vec<u8>, GltfLoadError> {
    let mut output = Vec::new();
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0;

    for byte in source.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        chunk[chunk_len] = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return Err(GltfLoadError::InvalidField("buffers.uri")),
        };
        chunk_len += 1;

        if chunk_len == 4 {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                output.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                output.push((chunk[2] << 6) | chunk[3]);
            }
            chunk_len = 0;
        }
    }

    if chunk_len != 0 {
        return Err(GltfLoadError::InvalidField("buffers.uri"));
    }

    Ok(output)
}

fn image_label(mime_type: Option<&str>, name: Option<&str>) -> String {
    if let Some(name) = name {
        if name.rsplit_once('.').is_some() {
            return name.to_owned();
        }
    }

    let extension = match mime_type {
        Some("image/png") => "png",
        Some("image/jpeg") => "jpg",
        Some("image/bmp" | "image/x-ms-bmp") => "bmp",
        Some("image/tga" | "image/x-tga" | "image/x-targa") => "tga",
        Some("image/webp") => "webp",
        Some("image/ktx2" | "image/x-ktx2") => "ktx2",
        _ => "bin",
    };
    name.map_or_else(
        || format!("embedded.{extension}"),
        |name| format!("{name}.{extension}"),
    )
}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn get(&self, key: &str) -> Option<&JsonValue> {
        let Self::Object(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find(|(entry_key, _)| entry_key == key)
            .map(|(_, value)| value)
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            bytes: source.as_bytes(),
            index: 0,
        }
    }

    fn parse(mut self) -> Result<JsonValue, GltfLoadError> {
        let value = self.parse_value()?;
        self.skip_ws();
        if self.index != self.bytes.len() {
            return Err(self.error("trailing characters"));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, GltfLoadError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            _ => Err(self.error("expected JSON value")),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, GltfLoadError> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.consume(b'}') {
            return Ok(JsonValue::Object(entries));
        }

        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.consume(b'}') {
                break;
            }
            self.expect(b',')?;
        }

        Ok(JsonValue::Object(entries))
    }

    fn parse_array(&mut self) -> Result<JsonValue, GltfLoadError> {
        self.expect(b'[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.consume(b']') {
            return Ok(JsonValue::Array(values));
        }

        loop {
            self.skip_ws();
            values.push(self.parse_value()?);
            self.skip_ws();
            if self.consume(b']') {
                break;
            }
            self.expect(b',')?;
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, GltfLoadError> {
        self.expect(b'"')?;
        let mut value = Vec::new();

        while let Some(byte) = self.next() {
            match byte {
                b'"' => {
                    return String::from_utf8(value)
                        .map_err(|_| self.error("invalid utf-8 string"));
                }
                b'\\' => match self.next() {
                    Some(b'"') => value.push(b'"'),
                    Some(b'\\') => value.push(b'\\'),
                    Some(b'/') => value.push(b'/'),
                    Some(b'b') => value.push(b'\x08'),
                    Some(b'f') => value.push(b'\x0c'),
                    Some(b'n') => value.push(b'\n'),
                    Some(b'r') => value.push(b'\r'),
                    Some(b't') => value.push(b'\t'),
                    Some(b'u') => push_json_unicode_escape(self, &mut value)?,
                    _ => return Err(self.error("invalid string escape")),
                },
                0..=0x1f => return Err(self.error("control character in string")),
                0x20..=0x7f => value.push(byte),
                _ => {
                    let mut utf8 = [0u8; 4];
                    let utf8_len =
                        utf8_sequence_len(byte).ok_or(self.error("invalid utf-8 string"))?;
                    utf8[0] = byte;
                    for slot in utf8.iter_mut().take(utf8_len).skip(1) {
                        let continuation = self.next().ok_or(self.error("unterminated string"))?;
                        if continuation & 0b1100_0000 != 0b1000_0000 {
                            return Err(self.error("invalid utf-8 string"));
                        }
                        *slot = continuation;
                    }
                    std::str::from_utf8(&utf8[..utf8_len])
                        .map_err(|_| self.error("invalid utf-8 string"))?;
                    value.extend_from_slice(&utf8[..utf8_len]);
                }
            }
        }

        Err(self.error("unterminated string"))
    }

    fn parse_number(&mut self) -> Result<f64, GltfLoadError> {
        let start = self.index;
        self.consume(b'-');
        self.consume_digits();
        if self.consume(b'.') {
            self.consume_digits();
        }
        if self.consume(b'e') || self.consume(b'E') {
            let _ = self.consume(b'+') || self.consume(b'-');
            self.consume_digits();
        }
        let source = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| self.error("invalid number"))?;
        source.parse().map_err(|_| self.error("invalid number"))
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.index += 1;
        }
    }

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), GltfLoadError> {
        for expected in literal {
            self.expect(*expected)?;
        }
        Ok(())
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), GltfLoadError> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(self.error("unexpected character"))
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn error(&self, reason: &str) -> GltfLoadError {
        GltfLoadError::Json(format!("{reason} at byte {}", self.index))
    }
}

fn utf8_sequence_len(first_byte: u8) -> Option<usize> {
    match first_byte {
        0x00..=0x7f => Some(1),
        0b1100_0000..=0b1101_1111 => Some(2),
        0b1110_0000..=0b1110_1111 => Some(3),
        0b1111_0000..=0b1111_0111 => Some(4),
        _ => None,
    }
}

fn push_json_unicode_escape(
    parser: &mut JsonParser<'_>,
    output: &mut Vec<u8>,
) -> Result<(), GltfLoadError> {
    let code_unit = parse_json_hex_escape(parser)?;
    let scalar = if (0xd800..=0xdbff).contains(&code_unit) {
        parser.expect(b'\\')?;
        parser.expect(b'u')?;
        let low = parse_json_hex_escape(parser)?;
        if !(0xdc00..=0xdfff).contains(&low) {
            return Err(parser.error("invalid unicode surrogate pair"));
        }
        0x1_0000 + (((u32::from(code_unit) - 0xd800) << 10) | (u32::from(low) - 0xdc00))
    } else if (0xdc00..=0xdfff).contains(&code_unit) {
        return Err(parser.error("invalid unicode surrogate pair"));
    } else {
        u32::from(code_unit)
    };

    let character = char::from_u32(scalar).ok_or(parser.error("invalid unicode escape"))?;
    let mut utf8 = [0u8; 4];
    output.extend_from_slice(character.encode_utf8(&mut utf8).as_bytes());
    Ok(())
}

fn parse_json_hex_escape(parser: &mut JsonParser<'_>) -> Result<u16, GltfLoadError> {
    let mut value = 0u16;
    for _ in 0..4 {
        let digit = parser
            .next()
            .ok_or(parser.error("unterminated unicode escape"))?;
        let nibble = match digit {
            b'0'..=b'9' => digit - b'0',
            b'a'..=b'f' => digit - b'a' + 10,
            b'A'..=b'F' => digit - b'A' + 10,
            _ => return Err(parser.error("invalid unicode escape")),
        };
        value = (value << 4) | u16::from(nibble);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gltf_loader_imports_triangle_mesh_and_pbr_material() {
        let gltf = minimal_gltf("mesh.bin");
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(asset.materials.len(), 1);
        assert_eq!(asset.materials[0].material.tint, [0.8, 0.2, 0.1, 1.0]);
        assert_eq!(asset.materials[0].material.roughness, 0.35);
        assert_eq!(asset.materials[0].material.metallic, 0.25);
        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("albedo.bmp")
        );
        assert_eq!(
            asset.materials[0]
                .metallic_roughness_texture_path
                .as_deref(),
            Some("surface.bmp")
        );
        assert_eq!(
            asset.materials[0].normal_texture_path.as_deref(),
            Some("normal.bmp")
        );
        assert_eq!(
            asset.materials[0].emissive_texture_path.as_deref(),
            Some("emissive.bmp")
        );
        assert_eq!(
            asset.materials[0].occlusion_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
        assert_eq!(asset.materials[0].material.normal_scale, 0.5);
        assert_eq!(asset.materials[0].material.emissive, [0.1, 0.2, 0.3]);
        assert_eq!(asset.materials[0].material.occlusion_strength, 0.35);
        assert_eq!(asset.materials[0].material.alpha_cutoff, 0.42);
        assert_eq!(asset.materials[0].material.blend_mode, BlendMode::Opaque);
        assert!(asset.materials[0].material.double_sided);
        assert_eq!(asset.primitives.len(), 1);
        assert_eq!(asset.primitives[0].node_index, None);
        assert_eq!(asset.primitives[0].material_index, Some(0));
        assert_eq!(asset.primitives[0].mesh.vertex_count(), 3);
        assert_eq!(asset.primitives[0].mesh.indices(), &[0, 1, 2]);
        assert_eq!(asset.primitives[0].mesh.vertices()[1].uv, [1.0, 0.0]);
        assert_eq!(
            asset.primitives[0].mesh.vertices()[1].tangent,
            [1.0, 0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn gltf_loader_generates_normals_when_attribute_is_missing() {
        let gltf = minimal_gltf_without_normals("missing_normals.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "missing_normals.bin").then(mesh_bin_without_normals)
        })
        .unwrap();
        let vertices = asset.primitives[0].mesh.vertices();

        assert_vec3_close(vertices[0].normal, [0.0, -1.0, 0.0]);
        assert_vec3_close(vertices[1].normal, [0.0, -1.0, 0.0]);
        assert_vec3_close(vertices[2].normal, [0.0, -1.0, 0.0]);
    }

    #[test]
    fn gltf_loader_rejects_invalid_material_alpha_mode() {
        let gltf =
            minimal_gltf("mesh.bin").replace(r#""alphaMode": "MASK""#, r#""alphaMode": "CUTOUT""#);
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("materials.alphaMode"));
    }

    #[test]
    fn gltf_loader_rejects_missing_asset_header() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#"  "asset": { "version": "2.0" },
"#,
            "",
        );
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |_| panic!("buffer should not load"))
                .unwrap_err();

        assert_eq!(error, GltfLoadError::MissingField("asset"));
    }

    #[test]
    fn gltf_loader_rejects_unsupported_asset_version() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#""asset": { "version": "2.0" }"#,
            r#""asset": { "version": "1.0" }"#,
        );
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |_| panic!("buffer should not load"))
                .unwrap_err();

        assert_eq!(
            error,
            GltfLoadError::Unsupported("only glTF asset version 2.0 is supported")
        );
    }

    #[test]
    fn gltf_loader_rejects_newer_asset_min_version() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#""asset": { "version": "2.0" }"#,
            r#""asset": { "version": "2.0", "minVersion": "2.1" }"#,
        );
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |_| panic!("buffer should not load"))
                .unwrap_err();

        assert_eq!(
            error,
            GltfLoadError::Unsupported("glTF asset minVersion is newer than 2.0")
        );
    }

    #[test]
    fn gltf_loader_rejects_unsupported_required_extension() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#""asset": { "version": "2.0" },"#,
            r#""asset": { "version": "2.0" },
  "extensionsRequired": ["KHR_draco_mesh_compression"],"#,
        );
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |_| panic!("buffer should not load"))
                .unwrap_err();

        assert_eq!(
            error,
            GltfLoadError::UnsupportedRequiredExtension("KHR_draco_mesh_compression".to_owned())
        );
    }

    #[test]
    fn gltf_loader_accepts_supported_required_extensions() {
        let gltf = minimal_gltf("mesh.bin")
            .replace(
                r#""asset": { "version": "2.0" },"#,
                r#""asset": { "version": "2.0" },
  "extensionsRequired": ["KHR_materials_unlit"],"#,
            )
            .replace(
                r#""doubleSided": true,"#,
                r#""doubleSided": true,
    "extensions": { "KHR_materials_unlit": {} },"#,
            );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert!(asset.materials[0].material.unlit);
    }

    #[test]
    fn gltf_loader_accepts_supported_required_extensions_across_features() {
        let gltf = minimal_gltf("mesh.bin")
            .replace(
                r#""asset": { "version": "2.0" },"#,
                r#""asset": { "version": "2.0" },
  "extensionsRequired": [
    "EXT_mesh_gpu_instancing",
    "KHR_lights_punctual",
    "KHR_materials_clearcoat",
    "KHR_materials_ior",
    "KHR_materials_transmission",
    "KHR_texture_transform"
  ],"#,
            )
            .replace(
                r#""buffers": [{ "uri": "mesh.bin", "byteLength": 114 }],"#,
                r#""extensions": {
    "KHR_lights_punctual": {
      "lights": [{
        "type": "point",
        "color": [1.0, 0.5, 0.25],
        "intensity": 2.0,
        "range": 3.0
      }]
    }
  },
  "buffers": [{ "uri": "mesh.bin", "byteLength": 102 }],"#,
            )
            .replace(
                r#""textures": [{"source": 0}],"#,
                r#""textures": [{"source": 0}],"#,
            )
            .replace(
                r#""nodes": [{ "mesh": 0 }],"#,
                r#""nodes": [{
    "mesh": 0,
    "extensions": {
      "KHR_lights_punctual": { "light": 0 },
      "EXT_mesh_gpu_instancing": {
        "attributes": {
          "TRANSLATION": 4
        }
      }
    }
  }],"#,
            )
            .replace(
                r#""bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 72, "byteLength": 24 },
    { "buffer": 0, "byteOffset": 96, "byteLength": 6 }
  ],"#,
                r#""bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 72, "byteLength": 24 },
    { "buffer": 0, "byteOffset": 96, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 102, "byteLength": 12 }
  ],"#,
            )
            .replace(
                r#""accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" },
    { "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],"#,
                r#""accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" },
    { "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 4, "componentType": 5126, "count": 1, "type": "VEC3" }
  ],"#,
            )
            .replace(
                r#""baseColorTexture": { "index": 0 }"#,
                r#""baseColorTexture": {
        "index": 0,
        "extensions": {
          "KHR_texture_transform": {
            "offset": [0.25, 0.5],
            "scale": [1.5, 2.0]
          }
        }
      }"#,
            )
            .replace(
                r#""doubleSided": true,"#,
                r#""doubleSided": true,
    "extensions": {
      "KHR_materials_clearcoat": { "clearcoatFactor": 0.8 },
      "KHR_materials_transmission": { "transmissionFactor": 0.35 },
      "KHR_materials_ior": { "ior": 1.4 }
    },"#,
            );
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "mesh.bin").then(|| {
                let mut bytes = mesh_bin();
                for value in [2.0f32, 3.0, 4.0] {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                bytes
            })
        })
        .unwrap();

        assert_eq!(asset.primitives.len(), 1);
        assert_eq!(asset.materials[0].material.clearcoat, 0.8);
        assert_eq!(asset.materials[0].material.transmission, 0.35);
        assert_eq!(asset.materials[0].material.ior, 1.4);
        assert_eq!(
            asset.materials[0]
                .material
                .base_color_texture_transform
                .offset,
            [0.25, 0.5]
        );
        assert_eq!(
            asset.materials[0]
                .material
                .base_color_texture_transform
                .scale,
            [1.5, 2.0]
        );
    }

    #[test]
    fn gltf_loader_rejects_mismatched_normal_accessor_count() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#"{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }"#,
            r#"{ "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3" }"#,
        );
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("attributes.NORMAL"));
    }

    #[test]
    fn gltf_loader_rejects_mismatched_color_accessor_count() {
        let gltf = minimal_gltf_with_vertex_colors("colors.bin").replace(
            r#"{ "bufferView": 1, "componentType": 5121, "count": 3, "type": "VEC4" }"#,
            r#"{ "bufferView": 1, "componentType": 5121, "count": 2, "type": "VEC4" }"#,
        );
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "colors.bin").then(vertex_color_bin)
        })
        .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("attributes.COLOR_0"));
    }

    #[test]
    fn gltf_loader_rejects_out_of_bounds_indices() {
        let gltf = minimal_gltf("mesh.bin");
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "mesh.bin").then(mesh_bin_with_out_of_bounds_index)
        })
        .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("primitives.indices"));
    }

    #[test]
    fn gltf_loader_rejects_out_of_bounds_material_index() {
        let gltf = minimal_gltf("mesh.bin").replace(r#""material": 0"#, r#""material": 1"#);
        let error =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("primitives.material"));
    }

    #[test]
    fn gltf_loader_triangulates_triangle_strip_primitives() {
        let gltf = minimal_gltf_with_primitive_mode("mode.bin", 5);
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "mode.bin").then(primitive_mode_mesh_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives[0].mesh.vertex_count(), 4);
        assert_eq!(asset.primitives[0].mesh.indices(), &[0, 1, 2, 2, 1, 3]);
    }

    #[test]
    fn gltf_loader_triangulates_triangle_fan_primitives() {
        let gltf = minimal_gltf_with_primitive_mode("mode.bin", 6);
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "mode.bin").then(primitive_mode_mesh_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives[0].mesh.vertex_count(), 4);
        assert_eq!(asset.primitives[0].mesh.indices(), &[0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn gltf_loader_prefers_ext_texture_webp_source() {
        let gltf = minimal_gltf_with_texture_source_extension(
            "mesh.bin",
            "EXT_texture_webp",
            "albedo.webp",
        );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("albedo.webp")
        );
        assert_eq!(asset.materials[0].base_color_texture_data, None);
    }

    #[test]
    fn gltf_loader_prefers_khr_texture_basisu_source() {
        let gltf = minimal_gltf_with_texture_source_extension(
            "mesh.bin",
            "KHR_texture_basisu",
            "albedo.ktx2",
        );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("albedo.ktx2")
        );
        assert_eq!(asset.materials[0].base_color_texture_data, None);
    }

    #[test]
    fn gltf_image_label_maps_ktx2_mime_alias_to_ktx2_extension() {
        assert_eq!(
            image_label(Some("image/x-ktx2"), Some("albedo")),
            "albedo.ktx2"
        );
        assert_eq!(image_label(Some("image/x-ktx2"), None), "embedded.ktx2");
    }

    #[test]
    fn gltf_loader_imports_vertex_colors() {
        let gltf = minimal_gltf_with_vertex_colors("colors.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "colors.bin").then(vertex_color_bin)
        })
        .unwrap();
        let vertices = asset.primitives[0].mesh.vertices();

        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0]);
        assert_eq!(vertices[0].alpha, 1.0);
        assert!((vertices[1].color[1] - (128.0 / 255.0)).abs() < 0.0001);
        assert!((vertices[1].alpha - (128.0 / 255.0)).abs() < 0.0001);
        assert!((vertices[2].color[2] - (64.0 / 255.0)).abs() < 0.0001);
        assert_eq!(vertices[2].alpha, 1.0);
    }

    #[test]
    fn gltf_loader_imports_common_material_extensions() {
        let gltf = minimal_gltf("mesh.bin")
            .replace(
                r#""textures": ["#,
                r#""samplers": [{
    "magFilter": 9729,
    "minFilter": 9984,
    "wrapS": 33648,
    "wrapT": 33071
  }],
  "textures": ["#,
            )
            .replace(r#""source": "#, r#""sampler": 0, "source": "#)
            .replace(
                r#""doubleSided": true,"#,
                r#""doubleSided": true,
    "extensions": {
      "KHR_materials_clearcoat": {
        "clearcoatFactor": 0.8,
        "clearcoatRoughnessFactor": 0.25,
        "clearcoatTexture": {
          "index": 0,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.01, 0.02] }
          }
        },
        "clearcoatRoughnessTexture": {
          "index": 1,
          "extensions": {
            "KHR_texture_transform": { "rotation": 0.1, "scale": [1.1, 1.2] }
          }
        },
        "clearcoatNormalTexture": {
          "index": 2,
          "scale": 0.62,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.15, 0.16], "rotation": 0.17, "scale": [1.05, 1.06] }
          }
        }
      },
      "KHR_materials_sheen": {
        "sheenColorFactor": [0.1, 0.2, 0.3],
        "sheenRoughnessFactor": 0.45,
        "sheenColorTexture": {
          "index": 3,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.03, 0.04], "scale": [1.3, 1.4] }
          }
        },
        "sheenRoughnessTexture": {
          "index": 4,
          "extensions": {
            "KHR_texture_transform": { "rotation": 0.2 }
          }
        }
      },
      "KHR_materials_transmission": {
        "transmissionFactor": 0.6,
        "transmissionTexture": {
          "index": 0,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.05, 0.06], "rotation": 0.3, "scale": [1.5, 1.6] }
          }
        }
      },
      "KHR_materials_ior": { "ior": 1.33 },
      "KHR_materials_emissive_strength": { "emissiveStrength": 2.5 }
      ,
      "KHR_materials_specular": {
        "specularFactor": 0.7,
        "specularColorFactor": [0.8, 0.9, 1.0],
        "specularTexture": {
          "index": 4,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.07, 0.08], "scale": [1.7, 1.8] }
          }
        },
        "specularColorTexture": {
          "index": 3,
          "extensions": {
            "KHR_texture_transform": { "rotation": 0.4, "scale": [1.9, 2.0] }
          }
        }
      },
      "KHR_materials_anisotropy": {
        "anisotropyStrength": 0.55,
        "anisotropyRotation": 0.25,
        "anisotropyTexture": {
          "index": 1,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.09, 0.1], "rotation": 0.5, "scale": [2.1, 2.2] }
          }
        }
      },
      "KHR_materials_iridescence": {
        "iridescenceFactor": 0.4,
        "iridescenceIor": 1.45,
        "iridescenceThicknessMinimum": 120.0,
        "iridescenceThicknessMaximum": 380.0,
        "iridescenceTexture": {
          "index": 2,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.11, 0.12], "scale": [2.3, 2.4] }
          }
        },
        "iridescenceThicknessTexture": {
          "index": 1,
          "extensions": {
            "KHR_texture_transform": { "rotation": 0.6, "scale": [2.5, 2.6] }
          }
        }
      },
      "KHR_materials_volume": {
        "thicknessFactor": 0.35,
        "attenuationColor": [0.7, 0.8, 0.9],
        "attenuationDistance": 2.5,
        "thicknessTexture": {
          "index": 4,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.13, 0.14], "rotation": 0.7, "scale": [2.7, 2.8] }
          }
        }
      },
      "KHR_materials_dispersion": {
        "dispersion": 0.12
      }
    },"#,
            );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();
        let material = asset.materials[0].material;

        assert_eq!(material.clearcoat, 0.8);
        assert_eq!(material.clearcoat_roughness, 0.25);
        assert_eq!(material.sheen_color, [0.1, 0.2, 0.3]);
        assert_eq!(material.sheen_roughness, 0.45);
        assert_eq!(material.transmission, 0.6);
        assert_eq!(material.ior, 1.33);
        assert_eq!(material.emissive_strength, 2.5);
        assert_eq!(material.specular_factor, 0.7);
        assert_eq!(material.specular_color, [0.8, 0.9, 1.0]);
        assert_eq!(material.anisotropy_strength, 0.55);
        assert_eq!(material.anisotropy_rotation, 0.25);
        assert_eq!(material.iridescence_factor, 0.4);
        assert_eq!(material.iridescence_ior, 1.45);
        assert_eq!(material.iridescence_thickness_min, 120.0);
        assert_eq!(material.iridescence_thickness_max, 380.0);
        assert_eq!(material.thickness_factor, 0.35);
        assert_eq!(material.attenuation_color, [0.7, 0.8, 0.9]);
        assert_eq!(material.attenuation_distance, 2.5);
        assert_eq!(material.dispersion, 0.12);
        assert_eq!(
            material.clearcoat_texture_transform,
            TextureTransform::new([0.01, 0.02], 0.0, [1.0, 1.0], 0)
        );
        assert_eq!(
            material.clearcoat_roughness_texture_transform,
            TextureTransform::new([0.0, 0.0], 0.1, [1.1, 1.2], 0)
        );
        assert_eq!(
            material.clearcoat_normal_texture_transform,
            TextureTransform::new([0.15, 0.16], 0.17, [1.05, 1.06], 0)
        );
        assert_eq!(material.clearcoat_normal_scale, 0.62);
        assert_eq!(
            material.sheen_color_texture_transform,
            TextureTransform::new([0.03, 0.04], 0.0, [1.3, 1.4], 0)
        );
        assert_eq!(
            material.sheen_roughness_texture_transform,
            TextureTransform::new([0.0, 0.0], 0.2, [1.0, 1.0], 0)
        );
        assert_eq!(
            material.transmission_texture_transform,
            TextureTransform::new([0.05, 0.06], 0.3, [1.5, 1.6], 0)
        );
        assert_eq!(
            material.specular_texture_transform,
            TextureTransform::new([0.07, 0.08], 0.0, [1.7, 1.8], 0)
        );
        assert_eq!(
            material.specular_color_texture_transform,
            TextureTransform::new([0.0, 0.0], 0.4, [1.9, 2.0], 0)
        );
        assert_eq!(
            material.anisotropy_texture_transform,
            TextureTransform::new([0.09, 0.1], 0.5, [2.1, 2.2], 0)
        );
        assert_eq!(
            material.iridescence_texture_transform,
            TextureTransform::new([0.11, 0.12], 0.0, [2.3, 2.4], 0)
        );
        assert_eq!(
            material.iridescence_thickness_texture_transform,
            TextureTransform::new([0.0, 0.0], 0.6, [2.5, 2.6], 0)
        );
        assert_eq!(
            material.thickness_texture_transform,
            TextureTransform::new([0.13, 0.14], 0.7, [2.7, 2.8], 0)
        );
        let extension_sampler = TextureSampler::new(
            TextureAddressMode::MirrorRepeat,
            TextureAddressMode::ClampToEdge,
            TextureFilterMode::Linear,
            TextureFilterMode::Nearest,
            TextureFilterMode::Nearest,
        );
        assert_eq!(material.texture_samplers.clearcoat, extension_sampler);
        assert_eq!(
            material.texture_samplers.clearcoat_normal,
            extension_sampler
        );
        assert_eq!(material.texture_samplers.sheen_color, extension_sampler);
        assert_eq!(material.texture_samplers.transmission, extension_sampler);
        assert_eq!(material.texture_samplers.specular, extension_sampler);
        assert_eq!(material.texture_samplers.anisotropy, extension_sampler);
        assert_eq!(material.texture_samplers.iridescence, extension_sampler);
        assert_eq!(material.texture_samplers.thickness, extension_sampler);
        assert_eq!(material.blend_mode, BlendMode::AlphaBlend);
        assert!(!material.depth_write);
        assert_eq!(
            asset.materials[0].clearcoat_texture_path.as_deref(),
            Some("albedo.bmp")
        );
        assert_eq!(
            asset.materials[0]
                .clearcoat_roughness_texture_path
                .as_deref(),
            Some("surface.bmp")
        );
        assert_eq!(
            asset.materials[0].clearcoat_normal_texture_path.as_deref(),
            Some("normal.bmp")
        );
        assert_eq!(
            asset.materials[0].sheen_color_texture_path.as_deref(),
            Some("emissive.bmp")
        );
        assert_eq!(
            asset.materials[0].sheen_roughness_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
        assert_eq!(
            asset.materials[0].transmission_texture_path.as_deref(),
            Some("albedo.bmp")
        );
        assert_eq!(
            asset.materials[0].specular_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
        assert_eq!(
            asset.materials[0].specular_color_texture_path.as_deref(),
            Some("emissive.bmp")
        );
        assert_eq!(
            asset.materials[0].anisotropy_texture_path.as_deref(),
            Some("surface.bmp")
        );
        assert_eq!(
            asset.materials[0].iridescence_texture_path.as_deref(),
            Some("normal.bmp")
        );
        assert_eq!(
            asset.materials[0]
                .iridescence_thickness_texture_path
                .as_deref(),
            Some("surface.bmp")
        );
        assert_eq!(
            asset.materials[0].thickness_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
    }

    #[test]
    fn gltf_loader_imports_specular_glossiness_material_extension() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#""doubleSided": true,"#,
            r#""doubleSided": true,
    "extensions": {
      "KHR_materials_pbrSpecularGlossiness": {
        "diffuseFactor": [0.2, 0.4, 0.6, 0.5],
        "specularFactor": [0.3, 0.5, 0.7],
        "glossinessFactor": 0.25,
        "diffuseTexture": { "index": 3 },
        "specularGlossinessTexture": {
          "index": 4,
          "extensions": {
            "KHR_texture_transform": { "offset": [0.2, 0.3], "rotation": 0.4, "scale": [0.5, 0.6] }
          }
        }
      }
    },"#,
        );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();
        let material = asset.materials[0].material;

        assert_eq!(material.tint, [0.2, 0.4, 0.6, 0.5]);
        assert_eq!(material.metallic, 0.0);
        assert_eq!(material.roughness, 0.75);
        assert_eq!(material.specular_factor, 1.0);
        assert_eq!(material.specular_color, [0.3, 0.5, 0.7]);
        assert!(material.specular_glossiness_workflow);
        assert_eq!(
            material.metallic_roughness_texture_transform,
            TextureTransform::new([0.2, 0.3], 0.4, [0.5, 0.6], 0)
        );
        assert_eq!(
            material.specular_texture_transform,
            TextureTransform::new([0.2, 0.3], 0.4, [0.5, 0.6], 0)
        );
        assert_eq!(
            material.specular_color_texture_transform,
            TextureTransform::new([0.2, 0.3], 0.4, [0.5, 0.6], 0)
        );
        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("emissive.bmp")
        );
        assert_eq!(
            asset.materials[0]
                .metallic_roughness_texture_path
                .as_deref(),
            Some("occlusion.bmp")
        );
        assert_eq!(
            asset.materials[0].specular_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
        assert_eq!(
            asset.materials[0].specular_color_texture_path.as_deref(),
            Some("occlusion.bmp")
        );
    }

    #[test]
    fn gltf_loader_imports_unlit_material_flag() {
        let gltf = minimal_gltf("mesh.bin").replace(
            r#""doubleSided": true,"#,
            r#""doubleSided": true,
    "extensions": { "KHR_materials_unlit": {} },"#,
        );
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();
        let material = asset.materials[0].material;

        assert!(material.unlit);
        assert_eq!(material.tint, [0.8, 0.2, 0.1, 1.0]);
        assert_eq!(material.alpha_cutoff, 0.42);
        assert!(material.double_sided);
    }

    #[test]
    fn gltf_loader_imports_base_color_texture_transform_and_second_uv_set() {
        let gltf = minimal_gltf_with_texture_transform("uv1.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "uv1.bin").then(mesh_bin_with_uv1)
        })
        .unwrap();

        assert_eq!(
            asset.materials[0].material.base_color_texture_transform,
            TextureTransform::new([0.25, 0.5], 0.75, [2.0, 3.0], 1)
        );
        assert_eq!(
            asset.materials[0]
                .material
                .metallic_roughness_texture_transform,
            TextureTransform::new([0.0, 0.0], 0.0, [1.0, 1.0], 1)
        );
        assert_eq!(
            asset.materials[0].material.normal_texture_transform,
            TextureTransform::new([0.1, 0.2], 0.25, [0.5, 0.75], 1)
        );
        assert_eq!(
            asset.materials[0].material.emissive_texture_transform,
            TextureTransform::new([0.3, 0.4], 0.0, [1.5, 1.25], 0)
        );
        assert_eq!(
            asset.materials[0].material.occlusion_texture_transform,
            TextureTransform::new([0.5, 0.6], 0.5, [0.8, 0.9], 1)
        );
        let sampler = TextureSampler::new(
            TextureAddressMode::ClampToEdge,
            TextureAddressMode::MirrorRepeat,
            TextureFilterMode::Nearest,
            TextureFilterMode::Nearest,
            TextureFilterMode::Linear,
        );
        assert_eq!(
            asset.materials[0].material.texture_samplers.base_color,
            sampler
        );
        assert_eq!(
            asset.materials[0]
                .material
                .texture_samplers
                .metallic_roughness,
            sampler
        );
        assert_eq!(asset.materials[0].material.texture_samplers.normal, sampler);
        assert_eq!(
            asset.materials[0].material.texture_samplers.emissive,
            sampler
        );
        assert_eq!(
            asset.materials[0].material.texture_samplers.occlusion,
            sampler
        );
        assert_eq!(asset.primitives[0].mesh.vertices()[0].uv, [0.0, 0.0]);
        assert_eq!(asset.primitives[0].mesh.vertices()[1].uv, [1.0, 0.0]);
        assert_eq!(asset.primitives[0].mesh.vertices()[0].uv1, [0.1, 0.2]);
        assert_eq!(asset.primitives[0].mesh.vertices()[1].uv1, [0.3, 0.4]);
    }

    #[test]
    fn gltf_loader_imports_normalized_texcoord_accessors() {
        let gltf = minimal_gltf_with_normalized_texcoords("normalized_uv.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "normalized_uv.bin").then(mesh_bin_with_normalized_uvs)
        })
        .unwrap();
        let vertices = asset.primitives[0].mesh.vertices();

        assert_vec2_close(vertices[0].uv, [0.0, 0.0]);
        assert_vec2_close(vertices[1].uv, [1.0, 0.0]);
        assert_vec2_close(vertices[2].uv, [128.0 / 255.0, 1.0]);
        assert_vec2_close(vertices[0].uv1, [1.0, 0.0]);
        assert_vec2_close(vertices[1].uv1, [0.0, 32768.0 / 65535.0]);
        assert_vec2_close(vertices[2].uv1, [16384.0 / 65535.0, 1.0]);
    }

    #[test]
    fn gltf_loader_imports_quantized_position_normal_and_tangent_accessors() {
        let gltf = minimal_gltf_with_quantized_vertex_attributes("quantized.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "quantized.bin").then(mesh_bin_with_quantized_attributes)
        })
        .unwrap();
        let vertices = asset.primitives[0].mesh.vertices();

        assert_vec3_close(vertices[0].position, [0.0, 0.0, 0.0]);
        assert_vec3_close(vertices[1].position, [1.0, 0.0, 0.0]);
        assert_vec3_close(vertices[2].position, [0.0, 1.0, 0.0]);
        assert_vec3_close(vertices[0].normal, [0.0, 0.0, 1.0]);
        assert_vec3_close(
            [
                vertices[0].tangent[0],
                vertices[0].tangent[1],
                vertices[0].tangent[2],
            ],
            [1.0, 0.0, 0.0],
        );
        assert_eq!(vertices[0].tangent[3], -1.0);
    }

    #[test]
    fn gltf_loader_applies_sparse_accessor_overlays() {
        let gltf = minimal_gltf_with_sparse_positions("sparse.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "sparse.bin").then(mesh_bin_with_sparse_positions)
        })
        .unwrap();
        let vertices = asset.primitives[0].mesh.vertices();

        assert_vec3_close(vertices[0].position, [0.0, 0.0, 0.0]);
        assert_vec3_close(vertices[1].position, [2.0, 3.0, 4.0]);
        assert_vec3_close(vertices[2].position, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn gltf_loader_supports_base64_data_uri_buffers() {
        let uri = format!(
            "data:application/octet-stream;base64,{}",
            encode_base64(&mesh_bin())
        );
        let gltf = minimal_gltf(&uri);
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |_| None).unwrap();

        assert_eq!(asset.primitives[0].mesh.vertex_count(), 3);
    }

    #[test]
    fn gltf_loader_supports_glb_binary_chunk_buffers() {
        let glb = glb_bytes(&minimal_glb_gltf(), &mesh_bin());
        let asset = GltfAsset::from_glb_bytes(&glb).unwrap();

        assert_eq!(asset.materials.len(), 1);
        assert_eq!(asset.primitives.len(), 1);
        assert_eq!(asset.primitives[0].mesh.vertex_count(), 3);
        assert_eq!(asset.primitives[0].mesh.indices(), &[0, 1, 2]);
    }

    #[test]
    fn gltf_loader_applies_scene_node_hierarchy_transforms() {
        let gltf = minimal_gltf_with_nodes("mesh.bin");
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(asset.primitives.len(), 1);
        assert_eq!(asset.primitives[0].node_index, Some(1));
        assert_vec3_close(
            asset.primitives[0]
                .model_matrix
                .transform_point3([0.0, 0.0, 0.0]),
            [1.0, 2.0, 0.0],
        );
        assert_vec3_close(
            asset.primitives[0]
                .model_matrix
                .transform_point3([1.0, 0.0, 0.0]),
            [1.0, 4.0, 0.0],
        );
    }

    #[test]
    fn gltf_loader_imports_ext_mesh_gpu_instancing_transforms() {
        let gltf = minimal_gltf_with_gpu_instancing("instanced.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "instanced.bin").then(instanced_mesh_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives.len(), 2);
        assert_eq!(asset.primitives[0].node_index, Some(0));
        assert_eq!(asset.primitives[1].node_index, Some(0));
        assert_vec3_close(
            asset.primitives[0]
                .model_matrix
                .transform_point3([1.0, 0.0, 0.0]),
            [2.0, 2.0, 3.0],
        );
        assert_vec3_close(
            asset.primitives[1]
                .model_matrix
                .transform_point3([1.0, 0.0, 0.0]),
            [1.0, 5.0, 3.0],
        );
    }

    #[test]
    fn gltf_loader_imports_ext_mesh_gpu_instancing_normalized_rotation() {
        let gltf = minimal_gltf_with_gpu_instancing_normalized_rotation("instanced_rotation.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "instanced_rotation.bin").then(instanced_mesh_normalized_rotation_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives.len(), 2);
        assert_vec3_close(
            asset.primitives[0]
                .model_matrix
                .transform_point3([1.0, 0.0, 0.0]),
            [1.0, 0.0, 0.0],
        );
        assert_vec3_close(
            asset.primitives[1]
                .model_matrix
                .transform_point3([1.0, 0.0, 0.0]),
            [0.0, 1.0, 0.0],
        );
    }

    #[test]
    fn gltf_loader_imports_punctual_lights_from_active_scene_nodes() {
        let gltf = minimal_gltf_with_punctual_lights("mesh.bin");
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(asset.lights.len(), 3);
        let point = asset.lights[0];
        assert_eq!(point.node_index, 1);
        assert_eq!(point.kind, GltfPunctualLightKind::Point);
        assert_eq!(point.color, [1.0, 0.5, 0.25]);
        assert_eq!(point.intensity, 3.0);
        assert_eq!(point.range, Some(8.0));
        assert_vec3_close(point.position, [1.0, 2.0, 3.0]);

        let spot = asset.lights[1];
        assert_eq!(spot.kind, GltfPunctualLightKind::Spot);
        assert_vec3_close(spot.direction, [-1.0, 0.0, 0.0]);
        assert_eq!(spot.range, Some(6.0));
        assert!((spot.inner_cone_angle - 0.2).abs() < 0.0001);
        assert!((spot.outer_cone_angle - 0.6).abs() < 0.0001);

        let directional = asset.lights[2];
        assert_eq!(directional.kind, GltfPunctualLightKind::Directional);
        assert_eq!(directional.color, [0.8, 0.9, 1.0]);
        assert_eq!(directional.intensity, 2.0);
        assert_vec3_close(directional.direction, [-1.0, 0.0, 0.0]);

        let lighting = asset.punctual_lighting().unwrap();
        assert_eq!(lighting.ambient_intensity, 0.0);
        assert_eq!(lighting.directional.color, [0.8, 0.9, 1.0]);
        assert_eq!(lighting.directional.intensity, 2.0);
        assert_vec3_close(lighting.directional.direction, [-1.0, 0.0, 0.0]);
        assert_eq!(lighting.point_lights().len(), 1);
        assert_eq!(lighting.point_lights()[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(lighting.point_lights()[0].range, 8.0);
        assert_eq!(lighting.spot_lights().len(), 1);
        assert_vec3_close(lighting.spot_lights()[0].direction, [-1.0, 0.0, 0.0]);
        assert!((lighting.spot_lights()[0].inner_angle_radians - 0.2).abs() < 0.0001);
        assert!((lighting.spot_lights()[0].outer_angle_radians - 0.6).abs() < 0.0001);
    }

    #[test]
    fn gltf_loader_imports_cameras_from_active_scene_nodes() {
        let gltf = minimal_gltf_with_cameras("mesh.bin");
        let asset =
            GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(asset.cameras.len(), 2);
        let perspective = asset.cameras[0];
        assert_eq!(perspective.node_index, 1);
        assert_eq!(perspective.camera_index, 0);
        assert_vec3_close(perspective.position, [1.0, 2.0, 3.0]);
        assert_vec3_close(perspective.right, [0.0, 0.0, -1.0]);
        assert_vec3_close(perspective.up, [0.0, 1.0, 0.0]);
        assert_vec3_close(perspective.forward, [-1.0, 0.0, 0.0]);
        assert_eq!(
            perspective.projection,
            GltfCameraProjection::Perspective {
                aspect_ratio: Some(2.0),
                vertical_fov_radians: 1.0,
                near: 0.25,
                far: Some(50.0),
            }
        );

        let Camera::View(camera) = asset.default_camera(1.5).unwrap() else {
            panic!("expected imported view camera");
        };
        assert_vec3_close(camera.position, [1.0, 2.0, 3.0]);
        assert_vec3_close(camera.forward, [-1.0, 0.0, 0.0]);
        assert!((camera.transparent_sort_depth([-4.0, 2.0, 3.0]) - 5.0).abs() < 0.0001);

        let orthographic = asset.cameras[1];
        assert_eq!(orthographic.node_index, 2);
        assert_eq!(
            orthographic.projection,
            GltfCameraProjection::Orthographic {
                xmag: 4.0,
                ymag: 2.0,
                near: 0.1,
                far: 12.0,
            }
        );
    }

    #[test]
    fn gltf_loader_allows_camera_only_scenes() {
        let asset = GltfAsset::from_gltf_str_with_buffers(camera_only_gltf(), |_| None).unwrap();

        assert!(asset.primitives.is_empty());
        assert!(asset.materials.is_empty());
        assert_eq!(asset.cameras.len(), 1);
        assert_eq!(asset.cameras[0].node_index, 0);
        assert_eq!(
            asset.cameras[0].projection,
            GltfCameraProjection::Perspective {
                aspect_ratio: None,
                vertical_fov_radians: 0.75,
                near: 0.1,
                far: None,
            }
        );
    }

    #[test]
    fn gltf_loader_imports_embedded_image_buffer_view() {
        let image_bytes = bmp_32_top_down_1x1();
        let gltf = minimal_gltf_with_embedded_image("mesh_and_image.bin", image_bytes.len());
        let mut bytes = mesh_bin();
        bytes.extend_from_slice(&image_bytes);
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "mesh_and_image.bin").then(|| bytes.clone())
        })
        .unwrap();

        let image = asset.materials[0].base_color_texture_data.as_ref().unwrap();
        assert_eq!(asset.materials[0].base_color_texture_path, None);
        assert_eq!(image.label, "albedo.bmp");
        assert_eq!(image.mime_type.as_deref(), Some("image/bmp"));
        assert_eq!(image.bytes, image_bytes);
    }

    #[test]
    fn gltf_image_label_maps_tga_mime_types_to_tga_extension() {
        assert_eq!(image_label(Some("image/tga"), Some("albedo")), "albedo.tga");
        assert_eq!(
            image_label(Some("image/x-tga"), Some("albedo")),
            "albedo.tga"
        );
        assert_eq!(
            image_label(Some("image/x-targa"), Some("albedo")),
            "albedo.tga"
        );
    }

    #[test]
    fn gltf_loader_parses_unescaped_utf8_string_fields() {
        let gltf = r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "mesh.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "images": [{ "uri": "贴图.png" }],
  "textures": [{ "source": 0 }],
  "materials": [{
    "pbrMetallicRoughness": {
      "baseColorTexture": { "index": 0 }
    }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }]
}"#;
        let asset =
            GltfAsset::from_gltf_str_with_buffers(gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("贴图.png")
        );
    }

    #[test]
    fn gltf_loader_parses_unicode_escape_string_fields() {
        let gltf = r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "mesh.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "images": [{ "uri": "\u8d34\u56fe\ud83d\ude80.png" }],
  "textures": [{ "source": 0 }],
  "materials": [{
    "pbrMetallicRoughness": {
      "baseColorTexture": { "index": 0 }
    }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }]
}"#;
        let asset =
            GltfAsset::from_gltf_str_with_buffers(gltf, |uri| (uri == "mesh.bin").then(mesh_bin))
                .unwrap();

        assert_eq!(
            asset.materials[0].base_color_texture_path.as_deref(),
            Some("贴图🚀.png")
        );
    }

    #[test]
    fn gltf_loader_applies_mesh_default_morph_weights() {
        let gltf = minimal_gltf_with_morph_target("morph.bin", r#""weights": [0.5],"#, "");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "morph.bin").then(morph_mesh_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives[0].morph_target_count, 1);
        assert_eq!(
            asset.primitives[0].mesh.vertices()[0].position,
            [0.0, 0.0, 0.5]
        );
        assert_eq!(
            asset.primitives[0].mesh.vertices()[1].position,
            [1.0, 0.0, 0.5]
        );
    }

    #[test]
    fn gltf_loader_applies_node_morph_weights_override() {
        let gltf = minimal_gltf_with_morph_target(
            "morph.bin",
            r#""weights": [0.0],"#,
            r#"
  "nodes": [{ "mesh": 0, "weights": [1.0] }],
  "scenes": [{ "nodes": [0] }],
  "scene": 0,"#,
        );
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "morph.bin").then(morph_mesh_bin)
        })
        .unwrap();

        assert_eq!(
            asset.primitives[0].mesh.vertices()[0].position,
            [0.0, 0.0, 1.0]
        );
        assert_eq!(
            asset.primitives[0].mesh.vertices()[2].position,
            [0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn gltf_loader_applies_morph_target_tangent_deltas() {
        let gltf = minimal_gltf_with_morph_tangents("morph_tangent.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "morph_tangent.bin").then(morph_tangent_mesh_bin)
        })
        .unwrap();
        let tangent = asset.primitives[0].mesh.vertices()[0].tangent;

        assert_vec3_close(
            [tangent[0], tangent[1], tangent[2]],
            [0.8944272, 0.4472136, 0.0],
        );
        assert_eq!(tangent[3], 1.0);
    }

    #[test]
    fn gltf_loader_applies_static_skinning() {
        let gltf = minimal_gltf_with_skin("skin.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "skin.bin").then(skin_mesh_bin)
        })
        .unwrap();

        assert_eq!(asset.primitives[0].skin_joint_count, 1);
        assert_eq!(
            asset.primitives[0].mesh.vertices()[0].position,
            [0.0, 0.0, 1.0]
        );
        assert_eq!(
            asset.primitives[0].mesh.vertices()[1].position,
            [1.0, 0.0, 1.0]
        );
        assert_eq!(
            asset.primitives[0].mesh.vertices()[2].position,
            [0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn gltf_loader_rejects_skin_missing_joint_attributes() {
        let gltf = minimal_gltf_with_skin("skin.bin").replace(
            r#""attributes": { "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 }"#,
            r#""attributes": { "POSITION": 0 }"#,
        );
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "skin.bin").then(skin_mesh_bin)
        })
        .unwrap_err();

        assert_eq!(error, GltfLoadError::MissingField("attributes.JOINTS_0"));
    }

    #[test]
    fn gltf_loader_rejects_joint_attributes_without_weights() {
        let gltf = minimal_gltf_with_skin("skin.bin").replace(
            r#""attributes": { "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 }"#,
            r#""attributes": { "POSITION": 0, "JOINTS_0": 2 }"#,
        );
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "skin.bin").then(skin_mesh_bin)
        })
        .unwrap_err();

        assert_eq!(error, GltfLoadError::MissingField("attributes.WEIGHTS_0"));
    }

    #[test]
    fn gltf_loader_rejects_skin_joint_indices_outside_skin_joints() {
        let gltf = minimal_gltf_with_skin("skin.bin");
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "skin.bin").then(skin_mesh_bin_with_out_of_bounds_joint)
        })
        .unwrap_err();

        assert_eq!(error, GltfLoadError::InvalidField("attributes.JOINTS_0"));
    }

    #[test]
    fn gltf_loader_imports_and_samples_node_translation_animation() {
        let gltf = minimal_gltf_with_translation_animation("animated.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "animated.bin").then(animated_triangle_bin)
        })
        .unwrap();

        assert_eq!(asset.animations.len(), 1);
        assert_eq!(asset.animations[0].name.as_deref(), Some("move"));
        assert_eq!(asset.animations[0].duration, 2.0);
        let sample = asset.animations[0].sample(1.0);

        assert_eq!(sample.len(), 1);
        assert_eq!(sample[0].target_node, 0);
        assert_eq!(sample[0].path, GltfAnimationPath::Translation);
        assert_eq!(
            sample[0].value,
            GltfAnimationValue::Translation([2.0, 0.0, 0.0])
        );
        assert_eq!(
            asset.animations[0].sample(3.0)[0].value,
            GltfAnimationValue::Translation([4.0, 0.0, 0.0])
        );
    }

    #[test]
    fn gltf_loader_rejects_out_of_bounds_animation_target_node() {
        let gltf = minimal_gltf_with_translation_animation("animated.bin").replace(
            r#""target": { "node": 0, "path": "translation" }"#,
            r#""target": { "node": 1, "path": "translation" }"#,
        );
        let error = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "animated.bin").then(animated_triangle_bin)
        })
        .unwrap_err();

        assert_eq!(
            error,
            GltfLoadError::InvalidField("animations.channels.target.node")
        );
    }

    #[test]
    fn gltf_loader_samples_normalized_short_rotation_animation() {
        let gltf = minimal_gltf_with_normalized_rotation_animation("rotation.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "rotation.bin").then(normalized_rotation_animation_bin)
        })
        .unwrap();

        let sample = asset.animations[0].sample(2.0);
        assert_eq!(sample[0].path, GltfAnimationPath::Rotation);
        let GltfAnimationValue::Rotation(value) = sample[0].value else {
            panic!("expected rotation sample");
        };

        assert_vec4_close(value, [0.0, 0.0, 0.7071, 0.7071]);
    }

    #[test]
    fn gltf_loader_samples_cubic_spline_translation_animation() {
        let gltf = minimal_gltf_with_cubic_translation_animation("cubic.bin");
        let asset = GltfAsset::from_gltf_str_with_buffers(&gltf, |uri| {
            (uri == "cubic.bin").then(cubic_animated_triangle_bin)
        })
        .unwrap();

        assert_eq!(
            asset.animations[0].channels[0].sampler.interpolation,
            GltfAnimationInterpolation::CubicSpline
        );
        let sample = asset.animations[0].sample(0.5);
        let GltfAnimationValue::Translation(value) = sample[0].value else {
            panic!("expected translation sample");
        };

        assert_vec3_close(value, [0.625, 0.0, 0.0]);
    }

    #[test]
    fn gltf_animation_mixer_advances_looping_layers() {
        let animations = vec![translation_animation_clip(2.0)];
        let mut mixer = GltfAnimationMixer::new().with_layer(GltfAnimationLayer::new(0));

        let samples = mixer.advance(&animations, 1.5);

        assert_eq!(mixer.layers()[0].time, 0.5);
        assert_eq!(
            samples[0].value,
            GltfAnimationValue::Translation([1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn gltf_animation_mixer_blends_layers_by_node_and_path() {
        let animations = vec![
            translation_animation_clip(2.0),
            translation_animation_clip(6.0),
        ];
        let mixer = GltfAnimationMixer::new()
            .with_layer(GltfAnimationLayer::new(0).with_time(1.0).with_weight(1.0))
            .with_layer(GltfAnimationLayer::new(1).with_time(1.0).with_weight(1.0));

        let samples = mixer.sample(&animations);

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].target_node, 0);
        assert_eq!(samples[0].path, GltfAnimationPath::Translation);
        assert_eq!(
            samples[0].value,
            GltfAnimationValue::Translation([4.0, 0.0, 0.0])
        );
    }

    fn minimal_gltf(uri: &str) -> String {
        minimal_gltf_with_buffer(&format!(r#"{{ "uri": "{uri}", "byteLength": 102 }}"#))
    }

    fn minimal_glb_gltf() -> String {
        minimal_gltf_with_buffer(r#"{ "byteLength": 102 }"#)
    }

    fn minimal_gltf_with_buffer(buffer: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{buffer}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }},
    {{ "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [
    {{ "uri": "albedo.bmp" }},
    {{ "uri": "surface.bmp" }},
    {{ "uri": "normal.bmp" }},
    {{ "uri": "emissive.bmp" }},
    {{ "uri": "occlusion.bmp" }}
  ],
  "textures": [
    {{ "source": 0 }},
    {{ "source": 1 }},
    {{ "source": 2 }},
    {{ "source": 3 }},
    {{ "source": 4 }}
  ],
  "materials": [{{
    "normalTexture": {{ "index": 2, "scale": 0.5 }},
    "emissiveTexture": {{ "index": 3 }},
    "emissiveFactor": [0.1, 0.2, 0.3],
    "occlusionTexture": {{ "index": 4, "strength": 0.35 }},
    "doubleSided": true,
    "alphaMode": "MASK",
    "alphaCutoff": 0.42,
    "pbrMetallicRoughness": {{
      "baseColorFactor": [0.8, 0.2, 0.1, 1.0],
      "metallicFactor": 0.25,
      "roughnessFactor": 0.35,
      "baseColorTexture": {{ "index": 0 }},
      "metallicRoughnessTexture": {{ "index": 1 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
      "indices": 3,
      "material": 0
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_texture_source_extension(
        uri: &str,
        extension_name: &str,
        texture_uri: &str,
    ) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 102 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }},
    {{ "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [
    {{ "uri": "fallback.png" }},
    {{ "uri": "{texture_uri}" }}
  ],
  "textures": [
    {{
      "source": 0,
      "extensions": {{ "{extension_name}": {{ "source": 1 }} }}
    }}
  ],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
      "indices": 3,
      "material": 0
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_primitive_mode(uri: &str, mode: usize) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 56 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 48 }},
    {{ "buffer": 0, "byteOffset": 48, "byteLength": 8 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 4, "type": "SCALAR" }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "mode": {mode}
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_without_normals(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 42 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_nodes(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 102 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "nodes": [
    {{ "translation": [1.0, 0.0, 0.0], "children": [1] }},
    {{
      "mesh": 0,
      "translation": [0.0, 2.0, 0.0],
      "rotation": [0.0, 0.0, 0.70710677, 0.70710677],
      "scale": [2.0, 2.0, 2.0]
    }}
  ],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_gpu_instancing(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "extensionsUsed": ["EXT_mesh_gpu_instancing"],
  "buffers": [{{ "uri": "{uri}", "byteLength": 116 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 60, "byteLength": 32 }},
    {{ "buffer": 0, "byteOffset": 92, "byteLength": 24 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC4" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3" }}
  ],
  "nodes": [{{
    "mesh": 0,
    "translation": [1.0, 2.0, 3.0],
    "extensions": {{
      "EXT_mesh_gpu_instancing": {{
        "attributes": {{
          "TRANSLATION": 1,
          "ROTATION": 2,
          "SCALE": 3
        }}
      }}
    }}
  }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }}
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_gpu_instancing_normalized_rotation(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "extensionsUsed": ["EXT_mesh_gpu_instancing"],
  "buffers": [{{ "uri": "{uri}", "byteLength": 52 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 16 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5122, "count": 2, "type": "VEC4", "normalized": true }}
  ],
  "nodes": [{{
    "mesh": 0,
    "extensions": {{
      "EXT_mesh_gpu_instancing": {{
        "attributes": {{
          "ROTATION": 1
        }}
      }}
    }}
  }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }}
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_punctual_lights(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 102 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "extensions": {{
    "KHR_lights_punctual": {{
      "lights": [
        {{
          "type": "point",
          "color": [1.0, 0.5, 0.25],
          "intensity": 3.0,
          "range": 8.0
        }},
        {{
          "type": "spot",
          "color": [0.2, 0.4, 1.0],
          "intensity": 5.0,
          "range": 6.0,
          "spot": {{ "innerConeAngle": 0.2, "outerConeAngle": 0.6 }}
        }},
        {{
          "type": "directional",
          "color": [0.8, 0.9, 1.0],
          "intensity": 2.0
        }}
      ]
    }}
  }},
  "nodes": [
    {{ "mesh": 0 }},
    {{
      "translation": [1.0, 2.0, 3.0],
      "extensions": {{ "KHR_lights_punctual": {{ "light": 0 }} }}
    }},
    {{
      "translation": [0.0, 1.0, 0.0],
      "rotation": [0.0, 0.70710677, 0.0, 0.70710677],
      "extensions": {{ "KHR_lights_punctual": {{ "light": 1 }} }}
    }},
    {{
      "translation": [0.0, 0.0, 1.0],
      "rotation": [0.0, 0.70710677, 0.0, 0.70710677],
      "extensions": {{ "KHR_lights_punctual": {{ "light": 2 }} }}
    }}
  ],
  "scenes": [{{ "nodes": [0, 1, 2, 3] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_cameras(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 102 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "cameras": [
    {{
      "type": "perspective",
      "perspective": {{
        "aspectRatio": 2.0,
        "yfov": 1.0,
        "znear": 0.25,
        "zfar": 50.0
      }}
    }},
    {{
      "type": "orthographic",
      "orthographic": {{
        "xmag": 4.0,
        "ymag": 2.0,
        "znear": 0.1,
        "zfar": 12.0
      }}
    }}
  ],
  "nodes": [
    {{ "mesh": 0 }},
    {{
      "camera": 0,
      "translation": [1.0, 2.0, 3.0],
      "rotation": [0.0, 0.70710677, 0.0, 0.70710677]
    }},
    {{ "camera": 1, "translation": [0.0, 4.0, 0.0] }},
    {{ "camera": 0, "translation": [100.0, 0.0, 0.0] }}
  ],
  "scenes": [{{ "nodes": [0, 1, 2] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_texture_transform(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 126 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 120, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC2" }},
    {{ "bufferView": 4, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "uri": "atlas.png" }}],
  "samplers": [{{
    "magFilter": 9728,
    "minFilter": 9986,
    "wrapS": 33071,
    "wrapT": 33648
  }}],
  "textures": [{{ "source": 0, "sampler": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{
        "index": 0,
        "texCoord": 0,
        "extensions": {{
          "KHR_texture_transform": {{
            "offset": [0.25, 0.5],
            "rotation": 0.75,
            "scale": [2.0, 3.0],
            "texCoord": 1
          }}
        }}
      }},
      "metallicRoughnessTexture": {{
        "index": 0,
        "texCoord": 1
      }}
    }},
    "normalTexture": {{
      "index": 0,
      "extensions": {{
        "KHR_texture_transform": {{
          "offset": [0.1, 0.2],
          "rotation": 0.25,
          "scale": [0.5, 0.75],
          "texCoord": 1
        }}
      }}
    }},
    "emissiveTexture": {{
      "index": 0,
      "extensions": {{
        "KHR_texture_transform": {{
          "offset": [0.3, 0.4],
          "scale": [1.5, 1.25]
        }}
      }}
    }},
    "occlusionTexture": {{
      "index": 0,
      "strength": 0.35,
      "extensions": {{
        "KHR_texture_transform": {{
          "offset": [0.5, 0.6],
          "rotation": 0.5,
          "scale": [0.8, 0.9],
          "texCoord": 1
        }}
      }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{
        "POSITION": 0,
        "NORMAL": 1,
        "TEXCOORD_0": 2,
        "TEXCOORD_1": 3
      }},
      "indices": 4,
      "material": 0
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_normalized_texcoords(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 96 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 72, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 78, "byteLength": 12 }},
    {{ "buffer": 0, "byteOffset": 90, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC2", "normalized": true }},
    {{ "bufferView": 3, "componentType": 5123, "count": 3, "type": "VEC2", "normalized": true }},
    {{ "bufferView": 4, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{
        "POSITION": 0,
        "NORMAL": 1,
        "TEXCOORD_0": 2,
        "TEXCOORD_1": 3
      }},
      "indices": 4
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_quantized_vertex_attributes(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "extensionsUsed": ["KHR_mesh_quantization"],
  "buffers": [{{ "uri": "{uri}", "byteLength": 52 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 18 }},
    {{ "buffer": 0, "byteOffset": 18, "byteLength": 9 }},
    {{ "buffer": 0, "byteOffset": 28, "byteLength": 24 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5122, "count": 3, "type": "VEC3", "normalized": true }},
    {{ "bufferView": 1, "componentType": 5120, "count": 3, "type": "VEC3", "normalized": true }},
    {{ "bufferView": 2, "componentType": 5122, "count": 3, "type": "VEC4", "normalized": true }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "NORMAL": 1, "TANGENT": 2 }}
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_sparse_positions(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 80 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
    {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 66, "byteLength": 1 }},
    {{ "buffer": 0, "byteOffset": 68, "byteLength": 12 }}
  ],
  "accessors": [
    {{
      "componentType": 5126,
      "count": 3,
      "type": "VEC3",
      "sparse": {{
        "count": 1,
        "indices": {{ "bufferView": 3, "componentType": 5121 }},
        "values": {{ "bufferView": 4 }}
      }}
    }},
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
    {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
      "indices": 3
    }}]
  }}]
}}"#
        )
    }

    fn camera_only_gltf() -> &'static str {
        r#"{
  "asset": { "version": "2.0" },
  "cameras": [{
    "type": "perspective",
    "perspective": {
      "yfov": 0.75,
      "znear": 0.1
    }
  }],
  "nodes": [{ "camera": 0, "translation": [0.0, 1.0, 2.0] }],
  "scenes": [{ "nodes": [0] }],
  "scene": 0
}"#
    }

    fn minimal_gltf_with_vertex_colors(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 54 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 12 }},
    {{ "buffer": 0, "byteOffset": 48, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5121, "count": 3, "type": "VEC4" }},
    {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "COLOR_0": 1 }},
      "indices": 2
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_embedded_image(uri: &str, image_len: usize) -> String {
        let total_len = 102 + image_len;
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": {total_len} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 102, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/bmp", "name": "albedo" }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_morph_target(uri: &str, mesh_weights: &str, node_block: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 138 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 102, "byteLength": 36 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3" }}
  ],
{node_block}
  "meshes": [{{
    {mesh_weights}
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "targets": [{{ "POSITION": 2 }}]
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_morph_tangents(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 128 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 44, "byteLength": 48 }},
    {{ "buffer": 0, "byteOffset": 92, "byteLength": 36 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC3" }}
  ],
  "meshes": [{{
    "weights": [0.5],
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "TANGENT": 2 }},
      "indices": 1,
      "targets": [{{ "TANGENT": 3 }}]
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_skin(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 166 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": 12 }},
    {{ "buffer": 0, "byteOffset": 54, "byteLength": 48 }},
    {{ "buffer": 0, "byteOffset": 102, "byteLength": 64 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC4" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4" }},
    {{ "bufferView": 4, "componentType": 5126, "count": 1, "type": "MAT4" }}
  ],
  "nodes": [
    {{ "mesh": 0, "skin": 0 }},
    {{ "translation": [0.0, 0.0, 2.0] }}
  ],
  "skins": [{{ "joints": [1], "inverseBindMatrices": 4 }}],
  "scenes": [{{ "nodes": [0, 1] }}],
  "scene": 0,
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_translation_animation(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 76 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 44, "byteLength": 8 }},
    {{ "buffer": 0, "byteOffset": 52, "byteLength": 24 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 2, "type": "SCALAR" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3" }}
  ],
  "nodes": [{{ "mesh": 0 }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "animations": [{{
    "name": "move",
    "samplers": [{{ "input": 2, "output": 3, "interpolation": "LINEAR" }}],
    "channels": [{{ "sampler": 0, "target": {{ "node": 0, "path": "translation" }} }}]
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_cubic_translation_animation(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 124 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 44, "byteLength": 8 }},
    {{ "buffer": 0, "byteOffset": 52, "byteLength": 72 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 2, "type": "SCALAR" }},
    {{ "bufferView": 3, "componentType": 5126, "count": 6, "type": "VEC3" }}
  ],
  "nodes": [{{ "mesh": 0 }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "animations": [{{
    "name": "smooth_move",
    "samplers": [{{ "input": 2, "output": 3, "interpolation": "CUBICSPLINE" }}],
    "channels": [{{ "sampler": 0, "target": {{ "node": 0, "path": "translation" }} }}]
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn minimal_gltf_with_normalized_rotation_animation(uri: &str) -> String {
        format!(
            r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "{uri}", "byteLength": 68 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 44, "byteLength": 8 }},
    {{ "buffer": 0, "byteOffset": 52, "byteLength": 16 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }},
    {{ "bufferView": 2, "componentType": 5126, "count": 2, "type": "SCALAR" }},
    {{ "bufferView": 3, "componentType": 5122, "count": 2, "type": "VEC4", "normalized": true }}
  ],
  "nodes": [{{ "mesh": 0 }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0,
  "animations": [{{
    "name": "spin",
    "samplers": [{{ "input": 2, "output": 3, "interpolation": "LINEAR" }}],
    "channels": [{{ "sampler": 0, "target": {{ "node": 0, "path": "rotation" }} }}]
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1
    }}]
  }}]
}}"#
        )
    }

    fn mesh_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn mesh_bin_with_out_of_bounds_index() -> Vec<u8> {
        let mut bytes = mesh_bin();
        bytes[100..102].copy_from_slice(&3u16.to_le_bytes());
        bytes
    }

    fn mesh_bin_without_normals() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn primitive_mode_mesh_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [
            0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2, 3] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn instanced_mesh_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.70710677, 0.70710677] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1.0f32, 1.0, 1.0, 2.0, 1.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn instanced_mesh_normalized_rotation_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0i16, 0, 0, 32767, 0, 0, 23170, 23170] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn mesh_bin_with_uv1() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn mesh_bin_with_normalized_uvs() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0, 255, 0, 128, 255]);
        for value in [65535u16, 0, 0, 32768, 16384, 65535] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn mesh_bin_with_quantized_attributes() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0i16, 0, 0, 32767, 0, 0, 0, 32767, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0, 127, 0, 0, 127, 0, 0, 127]);
        bytes.push(0);
        for _ in 0..3 {
            for value in [32767i16, 0, 0, -32768] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    fn mesh_bin_with_sparse_positions() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(1);
        bytes.push(0);
        for value in [2.0f32, 3.0, 4.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn base_animated_triangle_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0; 2]);
        for value in [0.0f32, 2.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn animated_triangle_bin() -> Vec<u8> {
        let mut bytes = base_animated_triangle_bin();
        for value in [0.0f32, 0.0, 0.0, 4.0, 0.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn cubic_animated_triangle_bin() -> Vec<u8> {
        let mut bytes = base_animated_triangle_bin();
        for value in [
            0.0f32, 0.0, 0.0, // in tangent 0
            0.0, 0.0, 0.0, // value 0
            0.0, 0.0, 0.0, // out tangent 0
            0.0, 0.0, 0.0, // in tangent 1
            4.0, 0.0, 0.0, // value 1
            0.0, 0.0, 0.0, // out tangent 1
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn normalized_rotation_animation_bin() -> Vec<u8> {
        let mut bytes = base_animated_triangle_bin();
        for value in [0i16, 0, 0, 32767, 0, 0, 23170, 23170] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn translation_animation_clip(end_x: f32) -> GltfAnimation {
        GltfAnimation {
            name: None,
            duration: 1.0,
            channels: vec![GltfAnimationChannel {
                target_node: 0,
                path: GltfAnimationPath::Translation,
                sampler: GltfAnimationSampler {
                    input: vec![0.0, 1.0],
                    output: GltfAnimationOutput::Translations(vec![
                        [0.0, 0.0, 0.0],
                        [end_x, 0.0, 0.0],
                    ]),
                    in_tangents: None,
                    out_tangents: None,
                    interpolation: GltfAnimationInterpolation::Linear,
                },
            }],
        }
    }

    fn morph_mesh_bin() -> Vec<u8> {
        let mut bytes = mesh_bin();
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn morph_tangent_mesh_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
        for _ in 0..3 {
            for value in [1.0f32, 0.0, 0.0, 1.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for value in [0.0f32, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn vertex_color_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[255, 0, 0, 255, 0, 128, 0, 128, 0, 0, 64, 255]);
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn skin_mesh_bin() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for _ in 0..3 {
            for value in [1.0f32, 0.0, 0.0, 0.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for col in Mat4::translation([0.0, 0.0, -1.0]).to_cols_array() {
            for value in col {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    fn skin_mesh_bin_with_out_of_bounds_joint() -> Vec<u8> {
        let mut bytes = skin_mesh_bin();
        bytes[42] = 1;
        bytes
    }

    fn encode_base64(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::new();

        for chunk in bytes.chunks(3) {
            let first = chunk[0];
            let second = chunk.get(1).copied().unwrap_or(0);
            let third = chunk.get(2).copied().unwrap_or(0);
            let packed = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;

            encoded.push(TABLE[((packed >> 18) & 0x3f) as usize] as char);
            encoded.push(TABLE[((packed >> 12) & 0x3f) as usize] as char);
            encoded.push(if chunk.len() >= 2 {
                TABLE[((packed >> 6) & 0x3f) as usize] as char
            } else {
                '='
            });
            encoded.push(if chunk.len() == 3 {
                TABLE[(packed & 0x3f) as usize] as char
            } else {
                '='
            });
        }

        encoded
    }

    fn glb_bytes(json_source: &str, binary_chunk: &[u8]) -> Vec<u8> {
        let mut json = json_source.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }

        let mut binary = binary_chunk.to_vec();
        while binary.len() % 4 != 0 {
            binary.push(0);
        }

        let total_len = 12 + 8 + json.len() + 8 + binary.len();
        let mut bytes = Vec::with_capacity(total_len);
        bytes.extend_from_slice(&0x4654_6c67u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(total_len as u32).to_le_bytes());
        bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
        bytes.extend_from_slice(&json);
        bytes.extend_from_slice(&(binary.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0x004e_4942u32.to_le_bytes());
        bytes.extend_from_slice(&binary);
        bytes
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for index in 0..3 {
            assert!((actual[index] - expected[index]).abs() < 0.0001);
        }
    }

    fn assert_vec2_close(actual: [f32; 2], expected: [f32; 2]) {
        for index in 0..2 {
            assert!((actual[index] - expected[index]).abs() < 0.0001);
        }
    }

    fn assert_vec4_close(actual: [f32; 4], expected: [f32; 4]) {
        for index in 0..4 {
            assert!((actual[index] - expected[index]).abs() < 0.0001);
        }
    }

    fn bmp_32_top_down_1x1() -> Vec<u8> {
        let pixel_data_len = 4u32;
        let file_size = 54 + pixel_data_len;
        let mut bytes = Vec::with_capacity(file_size as usize);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&54u32.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&(-1i32).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&pixel_data_len.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes.extend_from_slice(&[30, 20, 10, 40]);
        bytes
    }
}
