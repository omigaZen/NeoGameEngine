use std::fmt;

use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureTransform {
    pub offset: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub tex_coord: u32,
}

impl TextureTransform {
    pub const IDENTITY: Self = Self {
        offset: [0.0, 0.0],
        rotation: 0.0,
        scale: [1.0, 1.0],
        tex_coord: 0,
    };

    pub const fn new(offset: [f32; 2], rotation: f32, scale: [f32; 2], tex_coord: u32) -> Self {
        Self {
            offset,
            rotation,
            scale,
            tex_coord,
        }
    }
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureAddressMode {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFilterMode {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSampler {
    pub address_mode_u: TextureAddressMode,
    pub address_mode_v: TextureAddressMode,
    pub address_mode_w: TextureAddressMode,
    pub mag_filter: TextureFilterMode,
    pub min_filter: TextureFilterMode,
    pub mipmap_filter: TextureFilterMode,
}

impl TextureSampler {
    pub const DEFAULT: Self = Self {
        address_mode_u: TextureAddressMode::Repeat,
        address_mode_v: TextureAddressMode::Repeat,
        address_mode_w: TextureAddressMode::Repeat,
        mag_filter: TextureFilterMode::Linear,
        min_filter: TextureFilterMode::Linear,
        mipmap_filter: TextureFilterMode::Linear,
    };

    pub const fn new(
        address_mode_u: TextureAddressMode,
        address_mode_v: TextureAddressMode,
        mag_filter: TextureFilterMode,
        min_filter: TextureFilterMode,
        mipmap_filter: TextureFilterMode,
    ) -> Self {
        Self {
            address_mode_u,
            address_mode_v,
            address_mode_w: TextureAddressMode::Repeat,
            mag_filter,
            min_filter,
            mipmap_filter,
        }
    }
}

impl Default for TextureSampler {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialTextureSamplers {
    pub base_color: TextureSampler,
    pub metallic_roughness: TextureSampler,
    pub normal: TextureSampler,
    pub emissive: TextureSampler,
    pub occlusion: TextureSampler,
    pub clearcoat: TextureSampler,
    pub clearcoat_roughness: TextureSampler,
    pub clearcoat_normal: TextureSampler,
    pub sheen_color: TextureSampler,
    pub sheen_roughness: TextureSampler,
    pub transmission: TextureSampler,
    pub specular: TextureSampler,
    pub specular_color: TextureSampler,
    pub anisotropy: TextureSampler,
    pub iridescence: TextureSampler,
    pub iridescence_thickness: TextureSampler,
    pub thickness: TextureSampler,
}

impl MaterialTextureSamplers {
    pub const DEFAULT: Self = Self {
        base_color: TextureSampler::DEFAULT,
        metallic_roughness: TextureSampler::DEFAULT,
        normal: TextureSampler::DEFAULT,
        emissive: TextureSampler::DEFAULT,
        occlusion: TextureSampler::DEFAULT,
        clearcoat: TextureSampler::DEFAULT,
        clearcoat_roughness: TextureSampler::DEFAULT,
        clearcoat_normal: TextureSampler::DEFAULT,
        sheen_color: TextureSampler::DEFAULT,
        sheen_roughness: TextureSampler::DEFAULT,
        transmission: TextureSampler::DEFAULT,
        specular: TextureSampler::DEFAULT,
        specular_color: TextureSampler::DEFAULT,
        anisotropy: TextureSampler::DEFAULT,
        iridescence: TextureSampler::DEFAULT,
        iridescence_thickness: TextureSampler::DEFAULT,
        thickness: TextureSampler::DEFAULT,
    };
}

impl Default for MaterialTextureSamplers {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Opaque
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub tint: [f32; 4],
    pub base_color_texture: Option<TextureHandle>,
    pub base_color_texture_transform: TextureTransform,
    pub metallic_roughness_texture: Option<TextureHandle>,
    pub metallic_roughness_texture_transform: TextureTransform,
    pub normal_texture: Option<TextureHandle>,
    pub normal_texture_transform: TextureTransform,
    pub emissive_texture: Option<TextureHandle>,
    pub emissive_texture_transform: TextureTransform,
    pub occlusion_texture: Option<TextureHandle>,
    pub occlusion_texture_transform: TextureTransform,
    pub clearcoat_texture: Option<TextureHandle>,
    pub clearcoat_texture_transform: TextureTransform,
    pub clearcoat_roughness_texture: Option<TextureHandle>,
    pub clearcoat_roughness_texture_transform: TextureTransform,
    pub clearcoat_normal_texture: Option<TextureHandle>,
    pub clearcoat_normal_texture_transform: TextureTransform,
    pub sheen_color_texture: Option<TextureHandle>,
    pub sheen_color_texture_transform: TextureTransform,
    pub sheen_roughness_texture: Option<TextureHandle>,
    pub sheen_roughness_texture_transform: TextureTransform,
    pub transmission_texture: Option<TextureHandle>,
    pub transmission_texture_transform: TextureTransform,
    pub specular_texture: Option<TextureHandle>,
    pub specular_texture_transform: TextureTransform,
    pub specular_color_texture: Option<TextureHandle>,
    pub specular_color_texture_transform: TextureTransform,
    pub anisotropy_texture: Option<TextureHandle>,
    pub anisotropy_texture_transform: TextureTransform,
    pub iridescence_texture: Option<TextureHandle>,
    pub iridescence_texture_transform: TextureTransform,
    pub iridescence_thickness_texture: Option<TextureHandle>,
    pub iridescence_thickness_texture_transform: TextureTransform,
    pub thickness_texture: Option<TextureHandle>,
    pub thickness_texture_transform: TextureTransform,
    pub texture_samplers: MaterialTextureSamplers,
    pub normal_scale: f32,
    pub clearcoat_normal_scale: f32,
    pub emissive: [f32; 3],
    pub occlusion_strength: f32,
    pub alpha_cutoff: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    pub sheen_color: [f32; 3],
    pub sheen_roughness: f32,
    pub transmission: f32,
    pub ior: f32,
    pub emissive_strength: f32,
    pub specular_factor: f32,
    pub specular_color: [f32; 3],
    pub anisotropy_strength: f32,
    pub anisotropy_rotation: f32,
    pub iridescence_factor: f32,
    pub iridescence_ior: f32,
    pub iridescence_thickness_min: f32,
    pub iridescence_thickness_max: f32,
    pub thickness_factor: f32,
    pub attenuation_color: [f32; 3],
    pub attenuation_distance: f32,
    pub dispersion: f32,
    pub specular_glossiness_workflow: bool,
    pub unlit: bool,
    pub blend_mode: BlendMode,
    pub depth_write: bool,
    pub double_sided: bool,
}

impl Material {
    pub const WHITE: Self = Self {
        tint: [1.0, 1.0, 1.0, 1.0],
        base_color_texture: None,
        base_color_texture_transform: TextureTransform::IDENTITY,
        metallic_roughness_texture: None,
        metallic_roughness_texture_transform: TextureTransform::IDENTITY,
        normal_texture: None,
        normal_texture_transform: TextureTransform::IDENTITY,
        emissive_texture: None,
        emissive_texture_transform: TextureTransform::IDENTITY,
        occlusion_texture: None,
        occlusion_texture_transform: TextureTransform::IDENTITY,
        clearcoat_texture: None,
        clearcoat_texture_transform: TextureTransform::IDENTITY,
        clearcoat_roughness_texture: None,
        clearcoat_roughness_texture_transform: TextureTransform::IDENTITY,
        clearcoat_normal_texture: None,
        clearcoat_normal_texture_transform: TextureTransform::IDENTITY,
        sheen_color_texture: None,
        sheen_color_texture_transform: TextureTransform::IDENTITY,
        sheen_roughness_texture: None,
        sheen_roughness_texture_transform: TextureTransform::IDENTITY,
        transmission_texture: None,
        transmission_texture_transform: TextureTransform::IDENTITY,
        specular_texture: None,
        specular_texture_transform: TextureTransform::IDENTITY,
        specular_color_texture: None,
        specular_color_texture_transform: TextureTransform::IDENTITY,
        anisotropy_texture: None,
        anisotropy_texture_transform: TextureTransform::IDENTITY,
        iridescence_texture: None,
        iridescence_texture_transform: TextureTransform::IDENTITY,
        iridescence_thickness_texture: None,
        iridescence_thickness_texture_transform: TextureTransform::IDENTITY,
        thickness_texture: None,
        thickness_texture_transform: TextureTransform::IDENTITY,
        texture_samplers: MaterialTextureSamplers::DEFAULT,
        normal_scale: 1.0,
        clearcoat_normal_scale: 1.0,
        emissive: [0.0, 0.0, 0.0],
        occlusion_strength: 1.0,
        alpha_cutoff: 0.0,
        roughness: 0.65,
        metallic: 0.0,
        clearcoat: 0.0,
        clearcoat_roughness: 0.0,
        sheen_color: [0.0, 0.0, 0.0],
        sheen_roughness: 0.0,
        transmission: 0.0,
        ior: 1.5,
        emissive_strength: 1.0,
        specular_factor: 1.0,
        specular_color: [1.0, 1.0, 1.0],
        anisotropy_strength: 0.0,
        anisotropy_rotation: 0.0,
        iridescence_factor: 0.0,
        iridescence_ior: 1.3,
        iridescence_thickness_min: 100.0,
        iridescence_thickness_max: 400.0,
        thickness_factor: 0.0,
        attenuation_color: [1.0, 1.0, 1.0],
        attenuation_distance: 0.0,
        dispersion: 0.0,
        specular_glossiness_workflow: false,
        unlit: false,
        blend_mode: BlendMode::Opaque,
        depth_write: true,
        double_sided: false,
    };

    pub const fn new(tint: [f32; 4]) -> Self {
        Self {
            tint,
            base_color_texture: None,
            base_color_texture_transform: TextureTransform::IDENTITY,
            metallic_roughness_texture: None,
            metallic_roughness_texture_transform: TextureTransform::IDENTITY,
            normal_texture: None,
            normal_texture_transform: TextureTransform::IDENTITY,
            emissive_texture: None,
            emissive_texture_transform: TextureTransform::IDENTITY,
            occlusion_texture: None,
            occlusion_texture_transform: TextureTransform::IDENTITY,
            clearcoat_texture: None,
            clearcoat_texture_transform: TextureTransform::IDENTITY,
            clearcoat_roughness_texture: None,
            clearcoat_roughness_texture_transform: TextureTransform::IDENTITY,
            clearcoat_normal_texture: None,
            clearcoat_normal_texture_transform: TextureTransform::IDENTITY,
            sheen_color_texture: None,
            sheen_color_texture_transform: TextureTransform::IDENTITY,
            sheen_roughness_texture: None,
            sheen_roughness_texture_transform: TextureTransform::IDENTITY,
            transmission_texture: None,
            transmission_texture_transform: TextureTransform::IDENTITY,
            specular_texture: None,
            specular_texture_transform: TextureTransform::IDENTITY,
            specular_color_texture: None,
            specular_color_texture_transform: TextureTransform::IDENTITY,
            anisotropy_texture: None,
            anisotropy_texture_transform: TextureTransform::IDENTITY,
            iridescence_texture: None,
            iridescence_texture_transform: TextureTransform::IDENTITY,
            iridescence_thickness_texture: None,
            iridescence_thickness_texture_transform: TextureTransform::IDENTITY,
            thickness_texture: None,
            thickness_texture_transform: TextureTransform::IDENTITY,
            texture_samplers: MaterialTextureSamplers::DEFAULT,
            normal_scale: 1.0,
            clearcoat_normal_scale: 1.0,
            emissive: [0.0, 0.0, 0.0],
            occlusion_strength: 1.0,
            alpha_cutoff: 0.0,
            roughness: 0.65,
            metallic: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            sheen_color: [0.0, 0.0, 0.0],
            sheen_roughness: 0.0,
            transmission: 0.0,
            ior: 1.5,
            emissive_strength: 1.0,
            specular_factor: 1.0,
            specular_color: [1.0, 1.0, 1.0],
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            iridescence_factor: 0.0,
            iridescence_ior: 1.3,
            iridescence_thickness_min: 100.0,
            iridescence_thickness_max: 400.0,
            thickness_factor: 0.0,
            attenuation_color: [1.0, 1.0, 1.0],
            attenuation_distance: 0.0,
            dispersion: 0.0,
            specular_glossiness_workflow: false,
            unlit: false,
            blend_mode: BlendMode::Opaque,
            depth_write: true,
            double_sided: false,
        }
    }

    pub const fn textured(tint: [f32; 4], texture: TextureHandle) -> Self {
        Self::opaque_textured(tint, texture)
    }

    pub const fn opaque_textured(tint: [f32; 4], texture: TextureHandle) -> Self {
        Self {
            tint,
            base_color_texture: Some(texture),
            base_color_texture_transform: TextureTransform::IDENTITY,
            metallic_roughness_texture: None,
            metallic_roughness_texture_transform: TextureTransform::IDENTITY,
            normal_texture: None,
            normal_texture_transform: TextureTransform::IDENTITY,
            emissive_texture: None,
            emissive_texture_transform: TextureTransform::IDENTITY,
            occlusion_texture: None,
            occlusion_texture_transform: TextureTransform::IDENTITY,
            clearcoat_texture: None,
            clearcoat_texture_transform: TextureTransform::IDENTITY,
            clearcoat_roughness_texture: None,
            clearcoat_roughness_texture_transform: TextureTransform::IDENTITY,
            clearcoat_normal_texture: None,
            clearcoat_normal_texture_transform: TextureTransform::IDENTITY,
            sheen_color_texture: None,
            sheen_color_texture_transform: TextureTransform::IDENTITY,
            sheen_roughness_texture: None,
            sheen_roughness_texture_transform: TextureTransform::IDENTITY,
            transmission_texture: None,
            transmission_texture_transform: TextureTransform::IDENTITY,
            specular_texture: None,
            specular_texture_transform: TextureTransform::IDENTITY,
            specular_color_texture: None,
            specular_color_texture_transform: TextureTransform::IDENTITY,
            anisotropy_texture: None,
            anisotropy_texture_transform: TextureTransform::IDENTITY,
            iridescence_texture: None,
            iridescence_texture_transform: TextureTransform::IDENTITY,
            iridescence_thickness_texture: None,
            iridescence_thickness_texture_transform: TextureTransform::IDENTITY,
            thickness_texture: None,
            thickness_texture_transform: TextureTransform::IDENTITY,
            texture_samplers: MaterialTextureSamplers::DEFAULT,
            normal_scale: 1.0,
            clearcoat_normal_scale: 1.0,
            emissive: [0.0, 0.0, 0.0],
            occlusion_strength: 1.0,
            alpha_cutoff: 0.0,
            roughness: 0.65,
            metallic: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            sheen_color: [0.0, 0.0, 0.0],
            sheen_roughness: 0.0,
            transmission: 0.0,
            ior: 1.5,
            emissive_strength: 1.0,
            specular_factor: 1.0,
            specular_color: [1.0, 1.0, 1.0],
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            iridescence_factor: 0.0,
            iridescence_ior: 1.3,
            iridescence_thickness_min: 100.0,
            iridescence_thickness_max: 400.0,
            thickness_factor: 0.0,
            attenuation_color: [1.0, 1.0, 1.0],
            attenuation_distance: 0.0,
            dispersion: 0.0,
            specular_glossiness_workflow: false,
            unlit: false,
            blend_mode: BlendMode::Opaque,
            depth_write: true,
            double_sided: false,
        }
    }

    pub const fn alpha_blended_textured(tint: [f32; 4], texture: TextureHandle) -> Self {
        Self {
            tint,
            base_color_texture: Some(texture),
            base_color_texture_transform: TextureTransform::IDENTITY,
            metallic_roughness_texture: None,
            metallic_roughness_texture_transform: TextureTransform::IDENTITY,
            normal_texture: None,
            normal_texture_transform: TextureTransform::IDENTITY,
            emissive_texture: None,
            emissive_texture_transform: TextureTransform::IDENTITY,
            occlusion_texture: None,
            occlusion_texture_transform: TextureTransform::IDENTITY,
            clearcoat_texture: None,
            clearcoat_texture_transform: TextureTransform::IDENTITY,
            clearcoat_roughness_texture: None,
            clearcoat_roughness_texture_transform: TextureTransform::IDENTITY,
            clearcoat_normal_texture: None,
            clearcoat_normal_texture_transform: TextureTransform::IDENTITY,
            sheen_color_texture: None,
            sheen_color_texture_transform: TextureTransform::IDENTITY,
            sheen_roughness_texture: None,
            sheen_roughness_texture_transform: TextureTransform::IDENTITY,
            transmission_texture: None,
            transmission_texture_transform: TextureTransform::IDENTITY,
            specular_texture: None,
            specular_texture_transform: TextureTransform::IDENTITY,
            specular_color_texture: None,
            specular_color_texture_transform: TextureTransform::IDENTITY,
            anisotropy_texture: None,
            anisotropy_texture_transform: TextureTransform::IDENTITY,
            iridescence_texture: None,
            iridescence_texture_transform: TextureTransform::IDENTITY,
            iridescence_thickness_texture: None,
            iridescence_thickness_texture_transform: TextureTransform::IDENTITY,
            thickness_texture: None,
            thickness_texture_transform: TextureTransform::IDENTITY,
            texture_samplers: MaterialTextureSamplers::DEFAULT,
            normal_scale: 1.0,
            clearcoat_normal_scale: 1.0,
            emissive: [0.0, 0.0, 0.0],
            occlusion_strength: 1.0,
            alpha_cutoff: 0.0,
            roughness: 0.65,
            metallic: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            sheen_color: [0.0, 0.0, 0.0],
            sheen_roughness: 0.0,
            transmission: 0.0,
            ior: 1.5,
            emissive_strength: 1.0,
            specular_factor: 1.0,
            specular_color: [1.0, 1.0, 1.0],
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            iridescence_factor: 0.0,
            iridescence_ior: 1.3,
            iridescence_thickness_min: 100.0,
            iridescence_thickness_max: 400.0,
            thickness_factor: 0.0,
            attenuation_color: [1.0, 1.0, 1.0],
            attenuation_distance: 0.0,
            dispersion: 0.0,
            specular_glossiness_workflow: false,
            unlit: false,
            blend_mode: BlendMode::AlphaBlend,
            depth_write: false,
            double_sided: false,
        }
    }

    pub const fn solid(tint: [f32; 4]) -> Self {
        Self::new(tint)
    }

    pub const fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    pub const fn with_depth_write(mut self, depth_write: bool) -> Self {
        self.depth_write = depth_write;
        self
    }

    pub const fn with_double_sided(mut self, double_sided: bool) -> Self {
        self.double_sided = double_sided;
        self
    }

    pub const fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }

    pub const fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic;
        self
    }

    pub const fn with_surface(mut self, roughness: f32, metallic: f32) -> Self {
        self.roughness = roughness;
        self.metallic = metallic;
        self
    }

    pub const fn with_clearcoat(mut self, clearcoat: f32, roughness: f32) -> Self {
        self.clearcoat = clearcoat;
        self.clearcoat_roughness = roughness;
        self
    }

    pub const fn with_clearcoat_normal_scale(mut self, scale: f32) -> Self {
        self.clearcoat_normal_scale = scale;
        self
    }

    pub const fn with_sheen(mut self, color: [f32; 3], roughness: f32) -> Self {
        self.sheen_color = color;
        self.sheen_roughness = roughness;
        self
    }

    pub const fn with_transmission(mut self, transmission: f32) -> Self {
        self.transmission = transmission;
        if transmission > 0.0 {
            self.blend_mode = BlendMode::AlphaBlend;
            self.depth_write = false;
        }
        self
    }

    pub const fn with_ior(mut self, ior: f32) -> Self {
        self.ior = ior;
        self
    }

    pub const fn with_emissive_strength(mut self, strength: f32) -> Self {
        self.emissive_strength = strength;
        self
    }

    pub const fn with_specular(mut self, factor: f32, color: [f32; 3]) -> Self {
        self.specular_factor = factor;
        self.specular_color = color;
        self
    }

    pub const fn with_anisotropy(mut self, strength: f32, rotation: f32) -> Self {
        self.anisotropy_strength = strength;
        self.anisotropy_rotation = rotation;
        self
    }

    pub const fn with_iridescence(
        mut self,
        factor: f32,
        ior: f32,
        thickness_min: f32,
        thickness_max: f32,
    ) -> Self {
        self.iridescence_factor = factor;
        self.iridescence_ior = ior;
        self.iridescence_thickness_min = thickness_min;
        self.iridescence_thickness_max = thickness_max;
        self
    }

    pub const fn with_volume(
        mut self,
        thickness_factor: f32,
        attenuation_color: [f32; 3],
        attenuation_distance: f32,
    ) -> Self {
        self.thickness_factor = thickness_factor;
        self.attenuation_color = attenuation_color;
        self.attenuation_distance = attenuation_distance;
        if thickness_factor > 0.0 {
            self.blend_mode = BlendMode::AlphaBlend;
            self.depth_write = false;
        }
        self
    }

    pub const fn with_dispersion(mut self, dispersion: f32) -> Self {
        self.dispersion = dispersion;
        self
    }

    pub const fn with_specular_glossiness_workflow(mut self, enabled: bool) -> Self {
        self.specular_glossiness_workflow = enabled;
        self
    }

    pub const fn with_unlit(mut self, unlit: bool) -> Self {
        self.unlit = unlit;
        self
    }

    pub const fn with_texture_samplers(mut self, samplers: MaterialTextureSamplers) -> Self {
        self.texture_samplers = samplers;
        self
    }

    pub const fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness_texture = Some(texture);
        self
    }

    pub const fn with_base_color_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.base_color_texture_transform = transform;
        self
    }

    pub const fn with_metallic_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.metallic_roughness_texture_transform = transform;
        self
    }

    pub const fn with_normal_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.normal_texture_transform = transform;
        self
    }

    pub const fn with_emissive_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.emissive_texture_transform = transform;
        self
    }

    pub const fn with_occlusion_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.occlusion_texture_transform = transform;
        self
    }

    pub const fn with_clearcoat_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.clearcoat_texture_transform = transform;
        self
    }

    pub const fn with_clearcoat_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.clearcoat_roughness_texture_transform = transform;
        self
    }

    pub const fn with_clearcoat_normal_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.clearcoat_normal_texture_transform = transform;
        self
    }

    pub const fn with_sheen_color_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.sheen_color_texture_transform = transform;
        self
    }

    pub const fn with_sheen_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.sheen_roughness_texture_transform = transform;
        self
    }

    pub const fn with_transmission_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.transmission_texture_transform = transform;
        self
    }

    pub const fn with_specular_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.specular_texture_transform = transform;
        self
    }

    pub const fn with_specular_color_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.specular_color_texture_transform = transform;
        self
    }

    pub const fn with_anisotropy_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.anisotropy_texture_transform = transform;
        self
    }

    pub const fn with_iridescence_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.iridescence_texture_transform = transform;
        self
    }

    pub const fn with_iridescence_thickness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.iridescence_thickness_texture_transform = transform;
        self
    }

    pub const fn with_thickness_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.thickness_texture_transform = transform;
        self
    }

    pub const fn with_normal_texture(mut self, texture: TextureHandle, scale: f32) -> Self {
        self.normal_texture = Some(texture);
        self.normal_scale = scale;
        self
    }

    pub const fn with_emissive(mut self, emissive: [f32; 3]) -> Self {
        self.emissive = emissive;
        self
    }

    pub const fn with_emissive_texture(mut self, texture: TextureHandle) -> Self {
        self.emissive_texture = Some(texture);
        self
    }

    pub const fn with_occlusion_texture(mut self, texture: TextureHandle, strength: f32) -> Self {
        self.occlusion_texture = Some(texture);
        self.occlusion_strength = strength;
        self
    }

    pub const fn with_clearcoat_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_texture = Some(texture);
        self
    }

    pub const fn with_clearcoat_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_roughness_texture = Some(texture);
        self
    }

    pub const fn with_clearcoat_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_normal_texture = Some(texture);
        self
    }

    pub const fn with_sheen_color_texture(mut self, texture: TextureHandle) -> Self {
        self.sheen_color_texture = Some(texture);
        self
    }

    pub const fn with_sheen_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.sheen_roughness_texture = Some(texture);
        self
    }

    pub const fn with_transmission_texture(mut self, texture: TextureHandle) -> Self {
        self.transmission_texture = Some(texture);
        self
    }

    pub const fn with_specular_texture(mut self, texture: TextureHandle) -> Self {
        self.specular_texture = Some(texture);
        self
    }

    pub const fn with_specular_color_texture(mut self, texture: TextureHandle) -> Self {
        self.specular_color_texture = Some(texture);
        self
    }

    pub const fn with_anisotropy_texture(mut self, texture: TextureHandle) -> Self {
        self.anisotropy_texture = Some(texture);
        self
    }

    pub const fn with_iridescence_texture(mut self, texture: TextureHandle) -> Self {
        self.iridescence_texture = Some(texture);
        self
    }

    pub const fn with_iridescence_thickness_texture(mut self, texture: TextureHandle) -> Self {
        self.iridescence_thickness_texture = Some(texture);
        self
    }

    pub const fn with_thickness_texture(mut self, texture: TextureHandle) -> Self {
        self.thickness_texture = Some(texture);
        self
    }

    pub const fn with_alpha_cutoff(mut self, alpha_cutoff: f32) -> Self {
        self.alpha_cutoff = alpha_cutoff;
        self
    }

    pub const fn is_transparent(self) -> bool {
        matches!(self.blend_mode, BlendMode::AlphaBlend)
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::WHITE
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialLoadError {
    Empty,
    MaterialWithoutName { line: usize },
    PropertyBeforeMaterial { line: usize, property: String },
    MalformedLine { line: usize, reason: &'static str },
    InvalidNumber { line: usize, value: String },
}

impl fmt::Display for MaterialLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "MTL source did not contain any materials"),
            Self::MaterialWithoutName { line } => {
                write!(f, "line {line}: newmtl must include a material name")
            }
            Self::PropertyBeforeMaterial { line, property } => {
                write!(
                    f,
                    "line {line}: property '{property}' appears before newmtl"
                )
            }
            Self::MalformedLine { line, reason } => write!(f, "line {line}: {reason}"),
            Self::InvalidNumber { line, value } => {
                write!(f, "line {line}: invalid number '{value}'")
            }
        }
    }
}

impl std::error::Error for MaterialLoadError {}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedMaterial {
    pub name: String,
    pub material: Material,
    pub base_color_texture_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialLibrary {
    materials: Vec<NamedMaterial>,
}

impl MaterialLibrary {
    pub fn from_mtl_str(source: &str) -> Result<Self, MaterialLoadError> {
        let mut materials = Vec::new();
        let mut current: Option<NamedMaterial> = None;
        let mut explicit_roughness = false;

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.split_once('#').map_or(raw_line, |(line, _)| line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(kind) = parts.next() else {
                continue;
            };

            match kind {
                "newmtl" => {
                    if let Some(material) = current.take() {
                        materials.push(material);
                    }

                    let name = parts.collect::<Vec<_>>().join(" ");
                    if name.is_empty() {
                        return Err(MaterialLoadError::MaterialWithoutName { line: line_number });
                    }

                    current = Some(NamedMaterial {
                        name,
                        material: Material::default(),
                        base_color_texture_path: None,
                    });
                    explicit_roughness = false;
                }
                "Kd" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    let color = parse_vec3(parts, line_number)?;
                    material.material.tint[0] = color[0];
                    material.material.tint[1] = color[1];
                    material.material.tint[2] = color[2];
                }
                "d" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    let alpha = parse_one(parts, line_number, "expected dissolve alpha")?;
                    set_alpha(&mut material.material, alpha);
                }
                "Tr" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    let transparency = parse_one(parts, line_number, "expected transparency")?;
                    set_alpha(&mut material.material, 1.0 - transparency);
                }
                "Pr" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    material.material.roughness =
                        parse_one(parts, line_number, "expected roughness")?;
                    explicit_roughness = true;
                }
                "Pm" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    material.material.metallic =
                        parse_one(parts, line_number, "expected metallic")?;
                }
                "Ns" if !explicit_roughness => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    let exponent =
                        parse_one(parts, line_number, "expected specular exponent")?.max(0.0);
                    material.material.roughness = (2.0 / (exponent + 2.0)).sqrt();
                }
                "map_Kd" => {
                    let material = current_material_mut(&mut current, line_number, kind)?;
                    material.base_color_texture_path =
                        Some(parse_texture_path(parts, line_number)?);
                }
                _ => {}
            }
        }

        if let Some(material) = current {
            materials.push(material);
        }

        if materials.is_empty() {
            return Err(MaterialLoadError::Empty);
        }

        Ok(Self { materials })
    }

    pub fn material(&self, name: &str) -> Option<Material> {
        self.named_material(name).map(|material| material.material)
    }

    pub fn named_material(&self, name: &str) -> Option<&NamedMaterial> {
        self.materials.iter().find(|material| material.name == name)
    }

    pub fn entries(&self) -> &[NamedMaterial] {
        &self.materials
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

fn current_material_mut<'a>(
    current: &'a mut Option<NamedMaterial>,
    line: usize,
    property: &str,
) -> Result<&'a mut NamedMaterial, MaterialLoadError> {
    current
        .as_mut()
        .ok_or_else(|| MaterialLoadError::PropertyBeforeMaterial {
            line,
            property: property.to_owned(),
        })
}

fn set_alpha(material: &mut Material, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    material.tint[3] = alpha;
    if alpha < 1.0 {
        material.blend_mode = BlendMode::AlphaBlend;
        material.depth_write = false;
    } else {
        material.blend_mode = BlendMode::Opaque;
        material.depth_write = true;
    }
}

fn parse_vec3<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[f32; 3], MaterialLoadError> {
    let x = parse_number(required_part(parts.next(), line, "expected x")?, line)?;
    let y = parse_number(required_part(parts.next(), line, "expected y")?, line)?;
    let z = parse_number(required_part(parts.next(), line, "expected z")?, line)?;

    Ok([x, y, z])
}

fn parse_one<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line: usize,
    reason: &'static str,
) -> Result<f32, MaterialLoadError> {
    parse_number(required_part(parts.next(), line, reason)?, line)
}

fn parse_texture_path<'a>(
    parts: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<String, MaterialLoadError> {
    let tokens = parts.collect::<Vec<_>>();
    let Some(path) = tokens.last() else {
        return Err(MaterialLoadError::MalformedLine {
            line,
            reason: "expected texture path",
        });
    };

    Ok((*path).to_owned())
}

fn required_part<'a>(
    part: Option<&'a str>,
    line: usize,
    reason: &'static str,
) -> Result<&'a str, MaterialLoadError> {
    part.ok_or(MaterialLoadError::MalformedLine { line, reason })
}

fn parse_number(source: &str, line: usize) -> Result<f32, MaterialLoadError> {
    let value = source
        .parse::<f32>()
        .map_err(|_| MaterialLoadError::InvalidNumber {
            line,
            value: source.to_owned(),
        })?;

    if value.is_finite() {
        Ok(value)
    } else {
        Err(MaterialLoadError::InvalidNumber {
            line,
            value: source.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_defaults_to_non_metallic_moderately_rough_surface() {
        assert_eq!(Material::WHITE.roughness, 0.65);
        assert_eq!(Material::WHITE.metallic, 0.0);
        assert_eq!(Material::WHITE.metallic_roughness_texture, None);
        assert_eq!(Material::WHITE.normal_texture, None);
        assert_eq!(Material::WHITE.normal_scale, 1.0);
        assert_eq!(Material::WHITE.clearcoat_normal_scale, 1.0);
        assert_eq!(Material::WHITE.emissive_texture, None);
        assert_eq!(Material::WHITE.occlusion_texture, None);
        assert_eq!(Material::WHITE.clearcoat_texture, None);
        assert_eq!(Material::WHITE.clearcoat_roughness_texture, None);
        assert_eq!(Material::WHITE.clearcoat_normal_texture, None);
        assert_eq!(Material::WHITE.sheen_color_texture, None);
        assert_eq!(Material::WHITE.sheen_roughness_texture, None);
        assert_eq!(Material::WHITE.transmission_texture, None);
        assert_eq!(Material::WHITE.specular_texture, None);
        assert_eq!(Material::WHITE.specular_color_texture, None);
        assert_eq!(Material::WHITE.anisotropy_texture, None);
        assert_eq!(Material::WHITE.iridescence_texture, None);
        assert_eq!(Material::WHITE.iridescence_thickness_texture, None);
        assert_eq!(Material::WHITE.thickness_texture, None);
        assert_eq!(Material::WHITE.emissive, [0.0, 0.0, 0.0]);
        assert_eq!(Material::WHITE.occlusion_strength, 1.0);
        assert_eq!(Material::WHITE.alpha_cutoff, 0.0);
        assert_eq!(Material::WHITE.clearcoat, 0.0);
        assert_eq!(Material::WHITE.clearcoat_roughness, 0.0);
        assert_eq!(Material::WHITE.sheen_color, [0.0, 0.0, 0.0]);
        assert_eq!(Material::WHITE.sheen_roughness, 0.0);
        assert_eq!(Material::WHITE.transmission, 0.0);
        assert_eq!(Material::WHITE.ior, 1.5);
        assert_eq!(Material::WHITE.emissive_strength, 1.0);
        assert_eq!(Material::WHITE.specular_factor, 1.0);
        assert_eq!(Material::WHITE.specular_color, [1.0, 1.0, 1.0]);
        assert_eq!(Material::WHITE.anisotropy_strength, 0.0);
        assert_eq!(Material::WHITE.anisotropy_rotation, 0.0);
        assert_eq!(Material::WHITE.iridescence_factor, 0.0);
        assert_eq!(Material::WHITE.iridescence_ior, 1.3);
        assert_eq!(Material::WHITE.iridescence_thickness_min, 100.0);
        assert_eq!(Material::WHITE.iridescence_thickness_max, 400.0);
        assert_eq!(Material::WHITE.thickness_factor, 0.0);
        assert_eq!(Material::WHITE.attenuation_color, [1.0, 1.0, 1.0]);
        assert_eq!(Material::WHITE.attenuation_distance, 0.0);
        assert_eq!(Material::WHITE.dispersion, 0.0);
        assert!(!Material::WHITE.specular_glossiness_workflow);
        assert!(!Material::WHITE.unlit);
        assert!(!Material::WHITE.double_sided);
    }

    #[test]
    fn material_surface_builder_sets_roughness_and_metallic() {
        let material = Material::new([1.0, 0.8, 0.6, 1.0]).with_surface(0.25, 0.75);

        assert_eq!(material.roughness, 0.25);
        assert_eq!(material.metallic, 0.75);
    }

    #[test]
    fn material_can_be_marked_unlit() {
        let material = Material::new([0.2, 0.4, 0.6, 0.8]).with_unlit(true);

        assert!(material.unlit);
        assert_eq!(material.tint, [0.2, 0.4, 0.6, 0.8]);
        assert_eq!(material.blend_mode, BlendMode::Opaque);
        assert!(material.depth_write);
    }

    #[test]
    fn material_extension_builders_set_physical_terms() {
        let clearcoat_texture = TextureHandle::new(1, 2);
        let clearcoat_roughness_texture = TextureHandle::new(2, 3);
        let clearcoat_normal_texture = TextureHandle::new(3, 4);
        let sheen_color_texture = TextureHandle::new(4, 5);
        let sheen_roughness_texture = TextureHandle::new(5, 6);
        let transmission_texture = TextureHandle::new(6, 7);
        let specular_texture = TextureHandle::new(6, 7);
        let specular_color_texture = TextureHandle::new(7, 8);
        let anisotropy_texture = TextureHandle::new(8, 9);
        let iridescence_texture = TextureHandle::new(9, 10);
        let iridescence_thickness_texture = TextureHandle::new(10, 11);
        let thickness_texture = TextureHandle::new(11, 12);
        let material = Material::new([1.0, 0.8, 0.6, 1.0])
            .with_clearcoat(0.9, 0.2)
            .with_clearcoat_normal_scale(0.7)
            .with_clearcoat_texture(clearcoat_texture)
            .with_clearcoat_roughness_texture(clearcoat_roughness_texture)
            .with_clearcoat_normal_texture(clearcoat_normal_texture)
            .with_sheen([0.1, 0.2, 0.3], 0.45)
            .with_sheen_color_texture(sheen_color_texture)
            .with_sheen_roughness_texture(sheen_roughness_texture)
            .with_transmission(0.6)
            .with_transmission_texture(transmission_texture)
            .with_ior(1.33)
            .with_emissive_strength(2.5)
            .with_specular(0.7, [0.8, 0.9, 1.0])
            .with_specular_texture(specular_texture)
            .with_specular_color_texture(specular_color_texture)
            .with_anisotropy(0.55, 0.25)
            .with_anisotropy_texture(anisotropy_texture)
            .with_iridescence(0.4, 1.45, 120.0, 380.0)
            .with_iridescence_texture(iridescence_texture)
            .with_iridescence_thickness_texture(iridescence_thickness_texture)
            .with_volume(0.35, [0.7, 0.8, 0.9], 2.5)
            .with_thickness_texture(thickness_texture)
            .with_dispersion(0.12);

        assert_eq!(material.clearcoat, 0.9);
        assert_eq!(material.clearcoat_roughness, 0.2);
        assert_eq!(material.clearcoat_normal_scale, 0.7);
        assert_eq!(material.clearcoat_texture, Some(clearcoat_texture));
        assert_eq!(
            material.clearcoat_roughness_texture,
            Some(clearcoat_roughness_texture)
        );
        assert_eq!(
            material.clearcoat_normal_texture,
            Some(clearcoat_normal_texture)
        );
        assert_eq!(material.sheen_color, [0.1, 0.2, 0.3]);
        assert_eq!(material.sheen_roughness, 0.45);
        assert_eq!(material.sheen_color_texture, Some(sheen_color_texture));
        assert_eq!(
            material.sheen_roughness_texture,
            Some(sheen_roughness_texture)
        );
        assert_eq!(material.transmission, 0.6);
        assert_eq!(material.transmission_texture, Some(transmission_texture));
        assert_eq!(material.ior, 1.33);
        assert_eq!(material.emissive_strength, 2.5);
        assert_eq!(material.specular_factor, 0.7);
        assert_eq!(material.specular_color, [0.8, 0.9, 1.0]);
        assert_eq!(material.specular_texture, Some(specular_texture));
        assert_eq!(
            material.specular_color_texture,
            Some(specular_color_texture)
        );
        assert_eq!(material.anisotropy_strength, 0.55);
        assert_eq!(material.anisotropy_rotation, 0.25);
        assert_eq!(material.anisotropy_texture, Some(anisotropy_texture));
        assert_eq!(material.iridescence_factor, 0.4);
        assert_eq!(material.iridescence_texture, Some(iridescence_texture));
        assert_eq!(
            material.iridescence_thickness_texture,
            Some(iridescence_thickness_texture)
        );
        assert_eq!(material.iridescence_ior, 1.45);
        assert_eq!(material.iridescence_thickness_min, 120.0);
        assert_eq!(material.iridescence_thickness_max, 380.0);
        assert_eq!(material.thickness_factor, 0.35);
        assert_eq!(material.thickness_texture, Some(thickness_texture));
        assert_eq!(material.attenuation_color, [0.7, 0.8, 0.9]);
        assert_eq!(material.attenuation_distance, 2.5);
        assert_eq!(material.dispersion, 0.12);
        assert_eq!(material.blend_mode, BlendMode::AlphaBlend);
        assert!(!material.depth_write);
    }

    #[test]
    fn mtl_loader_imports_diffuse_alpha_and_surface_properties() {
        let library = MaterialLibrary::from_mtl_str(
            "\
newmtl glass
Kd 0.2 0.4 0.8
d 0.45
Pr 0.18
Pm 0.7
",
        )
        .unwrap();

        let material = library.material("glass").unwrap();

        assert_eq!(library.len(), 1);
        assert_eq!(material.tint, [0.2, 0.4, 0.8, 0.45]);
        assert_eq!(material.roughness, 0.18);
        assert_eq!(material.metallic, 0.7);
        assert_eq!(material.blend_mode, BlendMode::AlphaBlend);
        assert!(!material.depth_write);
    }

    #[test]
    fn mtl_loader_imports_base_color_texture_path() {
        let library = MaterialLibrary::from_mtl_str(
            "\
newmtl textured
map_Kd textures/albedo.png
",
        )
        .unwrap();

        assert_eq!(
            library
                .named_material("textured")
                .and_then(|material| material.base_color_texture_path.as_deref()),
            Some("textures/albedo.png")
        );
    }

    #[test]
    fn mtl_loader_maps_specular_exponent_to_roughness_when_pr_is_missing() {
        let library = MaterialLibrary::from_mtl_str(
            "\
newmtl glossy
Ns 30
",
        )
        .unwrap();

        let material = library.material("glossy").unwrap();

        assert!((material.roughness - 0.25).abs() < 0.0001);
    }
}
