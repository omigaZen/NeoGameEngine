use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl DirectionalLight {
    pub const DEFAULT: Self = Self {
        direction: [0.35, 0.75, 0.55],
        color: [1.0, 1.0, 1.0],
        intensity: 0.75,
    };

    pub const fn new(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            direction,
            color,
            intensity,
        }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalShadow {
    pub enabled: bool,
    pub map_size: u32,
    pub projection_size: f32,
    pub near: f32,
    pub far: f32,
    pub strength: f32,
    pub bias: f32,
    pub cascade_count: usize,
    pub cascade_max_distance: f32,
    pub cascade_split_lambda: f32,
}

impl DirectionalShadow {
    pub const DISABLED: Self = Self {
        enabled: false,
        map_size: 1024,
        projection_size: 8.0,
        near: -10.0,
        far: 10.0,
        strength: 0.0,
        bias: 0.0015,
        cascade_count: 1,
        cascade_max_distance: 20.0,
        cascade_split_lambda: 0.5,
    };

    pub const fn enabled(
        map_size: u32,
        projection_size: f32,
        near: f32,
        far: f32,
        strength: f32,
        bias: f32,
    ) -> Self {
        Self {
            enabled: true,
            map_size,
            projection_size,
            near,
            far,
            strength,
            bias,
            cascade_count: 1,
            cascade_max_distance: projection_size,
            cascade_split_lambda: 0.5,
        }
    }

    pub const fn with_cascades(
        mut self,
        cascade_count: usize,
        max_distance: f32,
        split_lambda: f32,
    ) -> Self {
        self.cascade_count = cascade_count;
        self.cascade_max_distance = max_distance;
        self.cascade_split_lambda = split_lambda;
        self
    }
}

impl Default for DirectionalShadow {
    fn default() -> Self {
        Self::DISABLED
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentLight {
    pub diffuse_color: [f32; 3],
    pub diffuse_intensity: f32,
    pub specular_color: [f32; 3],
    pub specular_intensity: f32,
    pub texture: Option<TextureHandle>,
    pub background_intensity: f32,
}

impl EnvironmentLight {
    pub const DISABLED: Self = Self {
        diffuse_color: [1.0, 1.0, 1.0],
        diffuse_intensity: 0.0,
        specular_color: [1.0, 1.0, 1.0],
        specular_intensity: 0.0,
        texture: None,
        background_intensity: 0.0,
    };

    pub const fn new(
        diffuse_color: [f32; 3],
        diffuse_intensity: f32,
        specular_color: [f32; 3],
        specular_intensity: f32,
    ) -> Self {
        Self {
            diffuse_color,
            diffuse_intensity,
            specular_color,
            specular_intensity,
            texture: None,
            background_intensity: 0.0,
        }
    }

    pub const fn with_texture(mut self, texture: TextureHandle) -> Self {
        self.texture = Some(texture);
        self
    }

    pub const fn with_background_intensity(mut self, intensity: f32) -> Self {
        self.background_intensity = intensity;
        self
    }
}

impl Default for EnvironmentLight {
    fn default() -> Self {
        Self::DISABLED
    }
}

pub const MAX_POINT_LIGHTS: usize = 4;
pub const MAX_SPOT_LIGHTS: usize = 4;
pub const MAX_DIRECTIONAL_SHADOW_CASCADES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointShadow {
    pub enabled: bool,
    pub map_size: u32,
    pub near: f32,
    pub far: f32,
    pub strength: f32,
    pub bias: f32,
}

impl PointShadow {
    pub const DISABLED: Self = Self {
        enabled: false,
        map_size: 512,
        near: 0.05,
        far: 10.0,
        strength: 0.0,
        bias: 0.0015,
    };

    pub const fn enabled(map_size: u32, near: f32, far: f32, strength: f32, bias: f32) -> Self {
        Self {
            enabled: true,
            map_size,
            near,
            far,
            strength,
            bias,
        }
    }
}

impl Default for PointShadow {
    fn default() -> Self {
        Self::DISABLED
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub shadow: PointShadow,
}

impl PointLight {
    pub const DEFAULT: Self = Self {
        position: [0.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
        range: 1.0,
        shadow: PointShadow::DISABLED,
    };

    pub const fn new(position: [f32; 3], color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            range,
            shadow: PointShadow::DISABLED,
        }
    }

    pub const fn with_shadow(mut self, shadow: PointShadow) -> Self {
        self.shadow = shadow;
        self
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotShadow {
    pub enabled: bool,
    pub map_size: u32,
    pub near: f32,
    pub far: f32,
    pub strength: f32,
    pub bias: f32,
}

impl SpotShadow {
    pub const DISABLED: Self = Self {
        enabled: false,
        map_size: 1024,
        near: 0.05,
        far: 10.0,
        strength: 0.0,
        bias: 0.0015,
    };

    pub const fn enabled(map_size: u32, near: f32, far: f32, strength: f32, bias: f32) -> Self {
        Self {
            enabled: true,
            map_size,
            near,
            far,
            strength,
            bias,
        }
    }
}

impl Default for SpotShadow {
    fn default() -> Self {
        Self::DISABLED
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotLight {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub inner_angle_radians: f32,
    pub outer_angle_radians: f32,
    pub shadow: SpotShadow,
}

impl SpotLight {
    pub const DEFAULT: Self = Self {
        position: [0.0, 0.0, 0.0],
        direction: [0.0, -1.0, 0.0],
        color: [1.0, 1.0, 1.0],
        intensity: 0.0,
        range: 1.0,
        inner_angle_radians: 0.35,
        outer_angle_radians: 0.7,
        shadow: SpotShadow::DISABLED,
    };

    pub const fn new(
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_angle_radians: f32,
        outer_angle_radians: f32,
    ) -> Self {
        Self {
            position,
            direction,
            color,
            intensity,
            range,
            inner_angle_radians,
            outer_angle_radians,
            shadow: SpotShadow::DISABLED,
        }
    }

    pub const fn with_shadow(mut self, shadow: SpotShadow) -> Self {
        self.shadow = shadow;
        self
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderLighting {
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub directional: DirectionalLight,
    pub directional_shadow: DirectionalShadow,
    pub environment: EnvironmentLight,
    pub point_lights: [PointLight; MAX_POINT_LIGHTS],
    pub point_light_count: usize,
    pub spot_lights: [SpotLight; MAX_SPOT_LIGHTS],
    pub spot_light_count: usize,
}

impl RenderLighting {
    pub const DEFAULT: Self = Self {
        ambient_color: [1.0, 1.0, 1.0],
        ambient_intensity: 0.25,
        directional: DirectionalLight::DEFAULT,
        directional_shadow: DirectionalShadow::DISABLED,
        environment: EnvironmentLight::DISABLED,
        point_lights: [PointLight::DEFAULT; MAX_POINT_LIGHTS],
        point_light_count: 0,
        spot_lights: [SpotLight::DEFAULT; MAX_SPOT_LIGHTS],
        spot_light_count: 0,
    };

    pub const fn new(
        ambient_color: [f32; 3],
        ambient_intensity: f32,
        directional: DirectionalLight,
    ) -> Self {
        Self {
            ambient_color,
            ambient_intensity,
            directional,
            directional_shadow: DirectionalShadow::DISABLED,
            environment: EnvironmentLight::DISABLED,
            point_lights: [PointLight::DEFAULT; MAX_POINT_LIGHTS],
            point_light_count: 0,
            spot_lights: [SpotLight::DEFAULT; MAX_SPOT_LIGHTS],
            spot_light_count: 0,
        }
    }

    pub fn with_point_lights(mut self, point_lights: &[PointLight]) -> Self {
        self.point_lights = [PointLight::DEFAULT; MAX_POINT_LIGHTS];
        self.point_light_count = point_lights.len().min(MAX_POINT_LIGHTS);

        for (target, source) in self
            .point_lights
            .iter_mut()
            .zip(point_lights.iter().take(MAX_POINT_LIGHTS))
        {
            *target = *source;
        }

        self
    }

    pub fn point_lights(&self) -> &[PointLight] {
        &self.point_lights[..self.point_light_count.min(MAX_POINT_LIGHTS)]
    }

    pub const fn with_directional_shadow(mut self, shadow: DirectionalShadow) -> Self {
        self.directional_shadow = shadow;
        self
    }

    pub const fn with_environment(mut self, environment: EnvironmentLight) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_spot_lights(mut self, spot_lights: &[SpotLight]) -> Self {
        self.spot_lights = [SpotLight::DEFAULT; MAX_SPOT_LIGHTS];
        self.spot_light_count = spot_lights.len().min(MAX_SPOT_LIGHTS);

        for (target, source) in self
            .spot_lights
            .iter_mut()
            .zip(spot_lights.iter().take(MAX_SPOT_LIGHTS))
        {
            *target = *source;
        }

        self
    }

    pub fn spot_lights(&self) -> &[SpotLight] {
        &self.spot_lights[..self.spot_light_count.min(MAX_SPOT_LIGHTS)]
    }
}

impl Default for RenderLighting {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_lighting_clamps_point_lights_to_supported_capacity() {
        let lights = [
            PointLight::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0, 2.0),
            PointLight::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0, 2.0),
            PointLight::new([2.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0, 2.0),
            PointLight::new([3.0, 0.0, 0.0], [1.0, 1.0, 0.0], 1.0, 2.0),
            PointLight::new([4.0, 0.0, 0.0], [0.0, 1.0, 1.0], 1.0, 2.0),
        ];

        let lighting = RenderLighting::DEFAULT.with_point_lights(&lights);

        assert_eq!(lighting.point_lights().len(), MAX_POINT_LIGHTS);
        assert_eq!(lighting.point_lights()[0], lights[0]);
        assert_eq!(
            lighting.point_lights()[MAX_POINT_LIGHTS - 1],
            lights[MAX_POINT_LIGHTS - 1]
        );
    }

    #[test]
    fn render_lighting_clamps_spot_lights_to_supported_capacity() {
        let lights = [
            SpotLight::new(
                [0.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [1.0, 0.0, 0.0],
                1.0,
                2.0,
                0.2,
                0.4,
            ),
            SpotLight::new(
                [1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                2.0,
                0.2,
                0.4,
            ),
            SpotLight::new(
                [2.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                1.0,
                2.0,
                0.2,
                0.4,
            ),
            SpotLight::new(
                [3.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                1.0,
                2.0,
                0.2,
                0.4,
            ),
            SpotLight::new(
                [4.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 1.0, 1.0],
                1.0,
                2.0,
                0.2,
                0.4,
            ),
        ];

        let lighting = RenderLighting::DEFAULT.with_spot_lights(&lights);

        assert_eq!(lighting.spot_lights().len(), MAX_SPOT_LIGHTS);
        assert_eq!(lighting.spot_lights()[0], lights[0]);
        assert_eq!(
            lighting.spot_lights()[MAX_SPOT_LIGHTS - 1],
            lights[MAX_SPOT_LIGHTS - 1]
        );
    }

    #[test]
    fn point_light_carries_shadow_settings() {
        let shadow = PointShadow::enabled(1024, 0.1, 8.0, 0.55, 0.003);
        let light =
            PointLight::new([0.0, 1.0, 2.0], [1.0, 0.8, 0.6], 1.25, 6.0).with_shadow(shadow);

        assert_eq!(PointLight::DEFAULT.shadow, PointShadow::DISABLED);
        assert_eq!(light.shadow, shadow);
    }

    #[test]
    fn spot_light_carries_shadow_settings() {
        let shadow = SpotShadow::enabled(2048, 0.1, 12.0, 0.7, 0.004);
        let light = SpotLight::new(
            [0.0, 2.0, 1.0],
            [0.0, -1.0, -0.2],
            [1.0, 1.0, 1.0],
            1.0,
            8.0,
            0.25,
            0.65,
        )
        .with_shadow(shadow);

        assert_eq!(SpotLight::DEFAULT.shadow, SpotShadow::DISABLED);
        assert_eq!(light.shadow, shadow);
    }

    #[test]
    fn render_lighting_carries_directional_shadow_settings() {
        let shadow = DirectionalShadow::enabled(2048, 12.0, -20.0, 30.0, 0.65, 0.002)
            .with_cascades(4, 40.0, 0.65);
        let lighting = RenderLighting::DEFAULT.with_directional_shadow(shadow);

        assert_eq!(
            RenderLighting::DEFAULT.directional_shadow,
            DirectionalShadow::DISABLED
        );
        assert_eq!(lighting.directional_shadow, shadow);
        assert_eq!(lighting.directional_shadow.cascade_count, 4);
        assert_eq!(lighting.directional_shadow.cascade_max_distance, 40.0);
        assert_eq!(lighting.directional_shadow.cascade_split_lambda, 0.65);
    }

    #[test]
    fn render_lighting_carries_environment_lighting_settings() {
        let environment = EnvironmentLight::new([0.2, 0.3, 0.4], 0.5, [0.8, 0.9, 1.0], 1.25);
        let lighting = RenderLighting::DEFAULT.with_environment(environment);

        assert_eq!(
            RenderLighting::DEFAULT.environment,
            EnvironmentLight::DISABLED
        );
        assert_eq!(lighting.environment, environment);
        assert_eq!(lighting.environment.background_intensity, 0.0);
    }

    #[test]
    fn environment_lighting_can_reference_texture() {
        let texture = TextureHandle::new(3, 7);
        let environment =
            EnvironmentLight::new([1.0, 1.0, 1.0], 0.5, [1.0, 1.0, 1.0], 0.8).with_texture(texture);

        assert_eq!(environment.texture, Some(texture));
    }

    #[test]
    fn environment_lighting_can_enable_visible_background() {
        let environment = EnvironmentLight::new([1.0, 1.0, 1.0], 0.5, [1.0, 1.0, 1.0], 0.8)
            .with_background_intensity(0.35);

        assert_eq!(environment.background_intensity, 0.35);
    }
}
