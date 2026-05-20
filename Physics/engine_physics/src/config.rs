use crate::math::{Real, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsConfig {
    pub gravity: Vec3,
    pub timestep: TimestepConfig,
    pub solver: SolverConfig,
    pub sleeping: SleepingConfig,
    pub ccd: CcdConfig,
    pub events: EventConfig,
    pub debug: DebugConfig,
    pub determinism: DeterminismConfig,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            timestep: TimestepConfig::default(),
            solver: SolverConfig::default(),
            sleeping: SleepingConfig::default(),
            ccd: CcdConfig::default(),
            events: EventConfig::default(),
            debug: DebugConfig::default(),
            determinism: DeterminismConfig::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimestepConfig {
    pub fixed_dt: Real,
    pub max_frame_dt: Real,
    pub max_substeps: u32,
}

impl Default for TimestepConfig {
    fn default() -> Self {
        Self {
            fixed_dt: 1.0 / 60.0,
            max_frame_dt: 0.25,
            max_substeps: 5,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverConfig {
    pub velocity_iterations: u32,
    pub position_iterations: u32,
    pub stabilization_iterations: u32,
    pub allowed_linear_error: Real,
    pub prediction_distance: Real,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            velocity_iterations: 8,
            position_iterations: 3,
            stabilization_iterations: 1,
            allowed_linear_error: 0.001,
            prediction_distance: 0.002,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SleepingConfig {
    pub enabled: bool,
    pub linear_threshold: Real,
    pub angular_threshold: Real,
    pub minimum_sleep_time: Real,
}

impl Default for SleepingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            linear_threshold: 0.02,
            angular_threshold: 0.02,
            minimum_sleep_time: 0.5,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CcdConfig {
    pub enabled: bool,
    pub max_substeps: u32,
}

impl Default for CcdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_substeps: 1,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventConfig {
    pub collect_collision_events: bool,
    pub collect_sensor_events: bool,
    pub collect_contact_force_events: bool,
    pub max_events_per_tick: usize,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            collect_collision_events: true,
            collect_sensor_events: true,
            collect_contact_force_events: true,
            max_events_per_tick: 1024,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeterminismConfig {
    pub deterministic_ordering: bool,
    pub stable_event_sorting: bool,
}

impl Default for DeterminismConfig {
    fn default() -> Self {
        Self {
            deterministic_ordering: true,
            stable_event_sorting: true,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugConfig {
    pub enabled: bool,
    pub record_query_gizmos: bool,
    pub record_contact_points: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            record_query_gizmos: true,
            record_contact_points: true,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhysicsStepReport {
    pub tick: crate::id::PhysicsTick,
    pub dt: Real,
    pub active_bodies: usize,
    pub events_generated: usize,
    pub commands_applied: usize,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhysicsFrameReport {
    pub frame_index: u64,
    pub frame_dt: Real,
    pub steps_run: u32,
    pub dropped_steps: u32,
    pub accumulator: Real,
    pub interpolation_alpha: Real,
}
