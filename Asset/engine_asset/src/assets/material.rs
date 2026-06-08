use std::collections::HashMap;

use crate::{
    asset::{Asset, AssetDependencies, AssetDependencyReference, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    gpu_upload::{GpuResourceHandle, GpuUploadCommand, GpuUploadKind},
    handle::Handle,
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

use super::{shader::Shader, texture::Texture};

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: Option<String>,
    pub shader: Option<Handle<Shader>>,
    pub properties: MaterialProperties,
    pub textures: Vec<MaterialTextureBinding>,
    pub render_state: MaterialRenderState,
    pub gpu: Option<GpuResourceHandle>,
}

impl Asset for Material {
    const TYPE_NAME: &'static str = "Material";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0004);
}

impl AssetMemoryUsage for Material {
    fn cpu_bytes(&self) -> u64 {
        let custom = self
            .properties
            .custom
            .iter()
            .map(|(key, _)| key.len() as u64 + 16)
            .sum::<u64>();
        64 + custom + (self.textures.len() as u64 * 64)
    }
}

impl AssetDependencies for Material {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference)) {
        if let Some(shader) = &self.shader {
            visitor(AssetDependencyReference::from_handle(shader.untyped()));
        }
        for texture in &self.textures {
            visitor(AssetDependencyReference::from_handle(
                texture.texture.untyped(),
            ));
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialProperties {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub alpha_cutoff: Option<f32>,
    pub custom: HashMap<String, MaterialPropertyValue>,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            alpha_cutoff: None,
            custom: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialPropertyValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialTextureBinding {
    pub name: String,
    pub texture: Handle<Texture>,
    pub sampler: SamplerDesc,
    pub options: MaterialTextureOptions,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialTextureOptions {
    pub transform: MaterialTextureTransform,
    pub bump_scale: Option<f32>,
    pub color_remap: Option<[f32; 2]>,
    pub source_channel: Option<MaterialTextureChannel>,
    pub boost: Option<f32>,
    pub blend_u: Option<bool>,
    pub blend_v: Option<bool>,
    pub color_correction: Option<bool>,
    pub color_space: Option<MaterialTextureColorSpace>,
    pub projection: Option<MaterialTextureProjection>,
    pub texture_resolution: Option<u32>,
}

impl Default for MaterialTextureOptions {
    fn default() -> Self {
        Self {
            transform: MaterialTextureTransform::default(),
            bump_scale: None,
            color_remap: None,
            source_channel: None,
            boost: None,
            blend_u: None,
            blend_v: None,
            color_correction: None,
            color_space: None,
            projection: None,
            texture_resolution: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialTextureChannel {
    Red,
    Green,
    Blue,
    Matte,
    Luminance,
    Depth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialTextureColorSpace {
    Srgb,
    Linear,
    NonColor,
    Raw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialTextureProjection {
    Flat,
    Sphere,
    CubeTop,
    CubeBottom,
    CubeFront,
    CubeBack,
    CubeLeft,
    CubeRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialTextureTransform {
    pub offset: [f32; 3],
    pub scale: [f32; 3],
    pub turbulence: [f32; 3],
}

impl Default for MaterialTextureTransform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            turbulence: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplerDesc {
    pub filter: FilterMode,
    pub address: AddressMode,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            filter: FilterMode::Linear,
            address: AddressMode::Repeat,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressMode {
    Repeat,
    ClampToEdge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialRenderState {
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub depth_write: bool,
    pub depth_test: bool,
}

impl Default for MaterialRenderState {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            depth_write: true,
            depth_test: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

pub struct MaterialLoader;

impl MaterialLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MaterialLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for MaterialLoader {
    fn name(&self) -> &'static str {
        "MaterialLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["material", "mat"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Material::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
            message: format!("material source must be UTF-8: {error}"),
        })?;
        let mut material = Material {
            name: None,
            shader: None,
            properties: MaterialProperties::default(),
            textures: Vec::new(),
            render_state: MaterialRenderState::default(),
            gpu: None,
        };
        let mut texture_metadata = HashMap::<String, MaterialTextureMetadata>::new();
        for (line_index, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(AssetError::Decode {
                    message: format!("invalid material line {}", line_index + 1),
                });
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => material.name = Some(value.to_owned()),
                "shader" => material.shader = Some(ctx.dependency::<Shader>(value)),
                "base_color" => material.properties.base_color = parse_vec4(value, line_index)?,
                "metallic" => material.properties.metallic = parse_f32(value, line_index)?,
                "roughness" => material.properties.roughness = parse_f32(value, line_index)?,
                "emissive" => material.properties.emissive = parse_vec3(value, line_index)?,
                "alpha_cutoff" => {
                    material.properties.alpha_cutoff = Some(parse_f32(value, line_index)?)
                }
                "alpha_mode" => {
                    material.render_state.alpha_mode = parse_alpha_mode(value, line_index)?
                }
                "double_sided" => {
                    material.render_state.double_sided = parse_bool(value, line_index)?
                }
                "depth_write" => material.render_state.depth_write = parse_bool(value, line_index)?,
                "depth_test" => material.render_state.depth_test = parse_bool(value, line_index)?,
                key if key.starts_with("texture.") && key.contains(".sampler.") => {
                    let (name, field) = parse_texture_sampler_key(key, line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    apply_texture_sampler_field(&mut metadata.sampler, field, value, line_index)?;
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.sampler = metadata.sampler;
                    }
                }
                key if key.starts_with("texture.") && key.contains(".transform.") => {
                    let (name, field) = parse_texture_transform_key(key, line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    apply_texture_transform_field(
                        &mut metadata.options.transform,
                        field,
                        value,
                        line_index,
                    )?;
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".bump_scale") => {
                    let name = parse_texture_bump_scale_key(key, line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.bump_scale = Some(parse_f32(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".color_remap") => {
                    let name = parse_texture_option_key(key, ".color_remap", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.color_remap = Some(parse_vec2(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".source_channel") => {
                    let name = parse_texture_option_key(key, ".source_channel", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.source_channel =
                        Some(parse_texture_channel(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".boost") => {
                    let name = parse_texture_option_key(key, ".boost", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.boost = Some(parse_f32(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".blend_u") => {
                    let name = parse_texture_option_key(key, ".blend_u", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.blend_u = Some(parse_bool(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".blend_v") => {
                    let name = parse_texture_option_key(key, ".blend_v", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.blend_v = Some(parse_bool(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".color_correction") => {
                    let name = parse_texture_option_key(key, ".color_correction", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.color_correction = Some(parse_bool(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".color_space") => {
                    let name = parse_texture_option_key(key, ".color_space", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.color_space =
                        Some(parse_texture_color_space(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".projection") => {
                    let name = parse_texture_option_key(key, ".projection", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.projection =
                        Some(parse_texture_projection(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") && key.ends_with(".texture_resolution") => {
                    let name = parse_texture_option_key(key, ".texture_resolution", line_index)?;
                    let metadata = texture_metadata.entry(name.to_owned()).or_default();
                    metadata.options.texture_resolution =
                        Some(parse_texture_resolution(value, line_index)?);
                    if let Some(binding) = material
                        .textures
                        .iter_mut()
                        .rev()
                        .find(|binding| binding.name == name)
                    {
                        binding.options = metadata.options;
                    }
                }
                key if key.starts_with("texture.") => {
                    let name = key.trim_start_matches("texture.").to_owned();
                    let texture = ctx.dependency::<Texture>(value);
                    let metadata = texture_metadata.get(&name).copied().unwrap_or_default();
                    material.textures.push(MaterialTextureBinding {
                        name,
                        texture,
                        sampler: metadata.sampler,
                        options: metadata.options,
                    });
                }
                key if key.starts_with("custom.") => {
                    let (name, value) = parse_custom_material_property(key, value, line_index)?;
                    material.properties.custom.insert(name.to_owned(), value);
                }
                other => {
                    material.properties.custom.insert(
                        other.to_owned(),
                        MaterialPropertyValue::Float(parse_f32(value, line_index)?),
                    );
                }
            }
        }
        let upload = GpuUploadCommand {
            id: ctx.id(),
            asset_type: Material::TYPE_ID,
            kind: GpuUploadKind::Material,
            label: Some(ctx.path().display_string()),
            metadata: crate::gpu_upload::GpuUploadMetadata::None,
            bytes: source.as_bytes().to_vec(),
        };
        Ok(LoadedAsset::new(material).with_gpu_upload(upload))
    }
}

pub(crate) fn validate_material_source(source: &str) -> Result<(), AssetError> {
    for (line_index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid material line {}", line_index + 1),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" | "shader" => {}
            "base_color" => {
                let _ = parse_vec4(value, line_index)?;
            }
            "metallic" | "roughness" | "alpha_cutoff" => {
                let _ = parse_f32(value, line_index)?;
            }
            "emissive" => {
                let _ = parse_vec3(value, line_index)?;
            }
            "alpha_mode" => {
                let _ = parse_alpha_mode(value, line_index)?;
            }
            "double_sided" | "depth_write" | "depth_test" => {
                let _ = parse_bool(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.contains(".sampler.") => {
                let (_, field) = parse_texture_sampler_key(key, line_index)?;
                let mut sampler = SamplerDesc::default();
                apply_texture_sampler_field(&mut sampler, field, value, line_index)?;
            }
            key if key.starts_with("texture.") && key.contains(".transform.") => {
                let (_, field) = parse_texture_transform_key(key, line_index)?;
                let mut transform = MaterialTextureTransform::default();
                apply_texture_transform_field(&mut transform, field, value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".bump_scale") => {
                let _ = parse_texture_bump_scale_key(key, line_index)?;
                let _ = parse_f32(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".color_remap") => {
                let _ = parse_texture_option_key(key, ".color_remap", line_index)?;
                let _ = parse_vec2(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".source_channel") => {
                let _ = parse_texture_option_key(key, ".source_channel", line_index)?;
                let _ = parse_texture_channel(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".boost") => {
                let _ = parse_texture_option_key(key, ".boost", line_index)?;
                let _ = parse_f32(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".blend_u") => {
                let _ = parse_texture_option_key(key, ".blend_u", line_index)?;
                let _ = parse_bool(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".blend_v") => {
                let _ = parse_texture_option_key(key, ".blend_v", line_index)?;
                let _ = parse_bool(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".color_correction") => {
                let _ = parse_texture_option_key(key, ".color_correction", line_index)?;
                let _ = parse_bool(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".color_space") => {
                let _ = parse_texture_option_key(key, ".color_space", line_index)?;
                let _ = parse_texture_color_space(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".projection") => {
                let _ = parse_texture_option_key(key, ".projection", line_index)?;
                let _ = parse_texture_projection(value, line_index)?;
            }
            key if key.starts_with("texture.") && key.ends_with(".texture_resolution") => {
                let _ = parse_texture_option_key(key, ".texture_resolution", line_index)?;
                let _ = parse_texture_resolution(value, line_index)?;
            }
            key if key.starts_with("texture.") => {}
            key if key.starts_with("custom.") => {
                let _ = parse_custom_material_property(key, value, line_index)?;
            }
            _ => {
                let _ = parse_f32(value, line_index)?;
            }
        }
    }
    Ok(())
}

pub fn canonical_material_runtime_bytes(bytes: &[u8]) -> Result<Vec<u8>, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("material source must be UTF-8: {error}"),
    })?;
    validate_material_source(source)?;
    Ok(canonical_material_source_text(source).into_bytes())
}

pub fn canonical_material_source_text(source_text: &str) -> String {
    let mut lines = Vec::new();
    for line in source_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("# mtllib ") {
            lines.push(line.to_owned());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            lines.push(format!("{}={}", key.trim(), value.trim()));
        }
    }
    let mut canonical = lines.join("\n");
    if !canonical.is_empty() {
        canonical.push('\n');
    }
    canonical
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MaterialTextureMetadata {
    sampler: SamplerDesc,
    options: MaterialTextureOptions,
}

fn parse_f32(value: &str, line_index: usize) -> Result<f32, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid float on line {}: {error}", line_index + 1),
    })
}

fn parse_bool(value: &str, line_index: usize) -> Result<bool, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid bool on line {}: {error}", line_index + 1),
    })
}

fn parse_i32(value: &str, line_index: usize) -> Result<i32, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid int on line {}: {error}", line_index + 1),
    })
}

fn parse_custom_material_property<'a>(
    key: &'a str,
    value: &str,
    line_index: usize,
) -> Result<(&'a str, MaterialPropertyValue), AssetError> {
    let suffix = key.trim_start_matches("custom.");
    if suffix.is_empty() {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material custom property key on line {}",
                line_index + 1
            ),
        });
    }
    let Some((name, value_type)) = suffix.rsplit_once('.') else {
        return Ok((
            suffix,
            MaterialPropertyValue::Float(parse_f32(value, line_index)?),
        ));
    };
    if name.is_empty() || value_type.is_empty() {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material custom property key on line {}",
                line_index + 1
            ),
        });
    }
    let value = match value_type {
        "float" => MaterialPropertyValue::Float(parse_f32(value, line_index)?),
        "vec2" => MaterialPropertyValue::Vec2(parse_vec2(value, line_index)?),
        "vec3" => MaterialPropertyValue::Vec3(parse_vec3(value, line_index)?),
        "vec4" => MaterialPropertyValue::Vec4(parse_vec4(value, line_index)?),
        "int" => MaterialPropertyValue::Int(parse_i32(value, line_index)?),
        "bool" => MaterialPropertyValue::Bool(parse_bool(value, line_index)?),
        other => {
            return Err(AssetError::Decode {
                message: format!(
                    "unknown material custom property type `{other}` on line {}",
                    line_index + 1
                ),
            })
        }
    };
    Ok((name, value))
}

fn parse_alpha_mode(value: &str, line_index: usize) -> Result<AlphaMode, AssetError> {
    match value {
        "opaque" => Ok(AlphaMode::Opaque),
        "mask" => Ok(AlphaMode::Mask),
        "blend" => Ok(AlphaMode::Blend),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material alpha mode `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_texture_sampler_key<'a>(
    key: &'a str,
    line_index: usize,
) -> Result<(&'a str, &'a str), AssetError> {
    let suffix = key.trim_start_matches("texture.");
    let Some((name, field)) = suffix.split_once(".sampler.") else {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture sampler key on line {}",
                line_index + 1
            ),
        });
    };
    if name.is_empty() || field.is_empty() {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture sampler key on line {}",
                line_index + 1
            ),
        });
    }
    Ok((name, field))
}

fn parse_texture_transform_key<'a>(
    key: &'a str,
    line_index: usize,
) -> Result<(&'a str, &'a str), AssetError> {
    let suffix = key.trim_start_matches("texture.");
    let Some((name, field)) = suffix.split_once(".transform.") else {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture transform key on line {}",
                line_index + 1
            ),
        });
    };
    if name.is_empty() || field.is_empty() {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture transform key on line {}",
                line_index + 1
            ),
        });
    }
    Ok((name, field))
}

fn parse_texture_bump_scale_key<'a>(
    key: &'a str,
    line_index: usize,
) -> Result<&'a str, AssetError> {
    parse_texture_option_key(key, ".bump_scale", line_index)
}

fn parse_texture_option_key<'a>(
    key: &'a str,
    option_suffix: &str,
    line_index: usize,
) -> Result<&'a str, AssetError> {
    let suffix = key.trim_start_matches("texture.");
    let Some(name) = suffix.strip_suffix(option_suffix) else {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture option key on line {}",
                line_index + 1
            ),
        });
    };
    if name.is_empty() {
        return Err(AssetError::Decode {
            message: format!(
                "invalid material texture option key on line {}",
                line_index + 1
            ),
        });
    }
    Ok(name)
}

fn apply_texture_sampler_field(
    sampler: &mut SamplerDesc,
    field: &str,
    value: &str,
    line_index: usize,
) -> Result<(), AssetError> {
    match field {
        "address" => {
            sampler.address = parse_sampler_address(value, line_index)?;
        }
        "filter" => {
            sampler.filter = parse_sampler_filter(value, line_index)?;
        }
        other => {
            return Err(AssetError::Decode {
                message: format!(
                    "unknown material texture sampler field `{other}` on line {}",
                    line_index + 1
                ),
            })
        }
    }
    Ok(())
}

fn apply_texture_transform_field(
    transform: &mut MaterialTextureTransform,
    field: &str,
    value: &str,
    line_index: usize,
) -> Result<(), AssetError> {
    match field {
        "offset" => {
            transform.offset = parse_vec3(value, line_index)?;
        }
        "scale" => {
            transform.scale = parse_vec3(value, line_index)?;
        }
        "turbulence" => {
            transform.turbulence = parse_vec3(value, line_index)?;
        }
        other => {
            return Err(AssetError::Decode {
                message: format!(
                    "unknown material texture transform field `{other}` on line {}",
                    line_index + 1
                ),
            })
        }
    }
    Ok(())
}

fn parse_sampler_address(value: &str, line_index: usize) -> Result<AddressMode, AssetError> {
    match value {
        "repeat" => Ok(AddressMode::Repeat),
        "clamp" | "clamp_to_edge" => Ok(AddressMode::ClampToEdge),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material sampler address `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_sampler_filter(value: &str, line_index: usize) -> Result<FilterMode, AssetError> {
    match value {
        "nearest" => Ok(FilterMode::Nearest),
        "linear" => Ok(FilterMode::Linear),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material sampler filter `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_texture_channel(
    value: &str,
    line_index: usize,
) -> Result<MaterialTextureChannel, AssetError> {
    match value {
        "r" | "red" => Ok(MaterialTextureChannel::Red),
        "g" | "green" => Ok(MaterialTextureChannel::Green),
        "b" | "blue" => Ok(MaterialTextureChannel::Blue),
        "m" | "matte" => Ok(MaterialTextureChannel::Matte),
        "l" | "luminance" => Ok(MaterialTextureChannel::Luminance),
        "z" | "depth" => Ok(MaterialTextureChannel::Depth),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material texture source channel `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_texture_projection(
    value: &str,
    line_index: usize,
) -> Result<MaterialTextureProjection, AssetError> {
    match value {
        "flat" => Ok(MaterialTextureProjection::Flat),
        "sphere" => Ok(MaterialTextureProjection::Sphere),
        "cube_top" => Ok(MaterialTextureProjection::CubeTop),
        "cube_bottom" => Ok(MaterialTextureProjection::CubeBottom),
        "cube_front" => Ok(MaterialTextureProjection::CubeFront),
        "cube_back" => Ok(MaterialTextureProjection::CubeBack),
        "cube_left" => Ok(MaterialTextureProjection::CubeLeft),
        "cube_right" => Ok(MaterialTextureProjection::CubeRight),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material texture projection `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_texture_color_space(
    value: &str,
    line_index: usize,
) -> Result<MaterialTextureColorSpace, AssetError> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
        .as_str()
    {
        "srgb" => Ok(MaterialTextureColorSpace::Srgb),
        "linear" => Ok(MaterialTextureColorSpace::Linear),
        "non_color" => Ok(MaterialTextureColorSpace::NonColor),
        "raw" => Ok(MaterialTextureColorSpace::Raw),
        other => Err(AssetError::Decode {
            message: format!(
                "invalid material texture color space `{other}` on line {}",
                line_index + 1
            ),
        }),
    }
}

fn parse_texture_resolution(value: &str, line_index: usize) -> Result<u32, AssetError> {
    let resolution = value.parse::<u32>().map_err(|error| AssetError::Decode {
        message: format!(
            "invalid material texture resolution `{value}` on line {}: {error}",
            line_index + 1
        ),
    })?;
    if resolution == 0 {
        return Err(AssetError::Decode {
            message: format!(
                "material texture resolution must be greater than zero on line {}",
                line_index + 1
            ),
        });
    }
    Ok(resolution)
}

fn parse_vec2(value: &str, line_index: usize) -> Result<[f32; 2], AssetError> {
    let parts = value
        .split(',')
        .map(str::trim)
        .map(|part| parse_f32(part, line_index))
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 2 {
        return Err(AssetError::Decode {
            message: format!("expected two values on line {}", line_index + 1),
        });
    }
    Ok([parts[0], parts[1]])
}

fn parse_vec3(value: &str, line_index: usize) -> Result<[f32; 3], AssetError> {
    let parts = value
        .split(',')
        .map(str::trim)
        .map(|part| parse_f32(part, line_index))
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 3 {
        return Err(AssetError::Decode {
            message: format!("expected three values on line {}", line_index + 1),
        });
    }
    Ok([parts[0], parts[1], parts[2]])
}

fn parse_vec4(value: &str, line_index: usize) -> Result<[f32; 4], AssetError> {
    let parts = value
        .split(',')
        .map(str::trim)
        .map(|part| parse_f32(part, line_index))
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 4 {
        return Err(AssetError::Decode {
            message: format!("expected four values on line {}", line_index + 1),
        });
    }
    Ok([parts[0], parts[1], parts[2], parts[3]])
}
