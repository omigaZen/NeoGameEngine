# Engine Physics API 文档

> 版本：v0.1 草案  
> 目标：为 Rust 自研游戏引擎设计一套完整、可扩展、后端可替换的物理模块 API。  
> 推荐后端：Rapier，但公共 API 不暴露任何 Rapier 类型。

> 实现同步：当前仓库已新增 `Physics/engine_physics` crate，公开 API 使用 engine-facing math/ID/descriptor 类型，不暴露 Rapier 类型。默认功能包含 deterministic local 3D world 与真实 `backend_rapier` 适配层；`rapier_backend::RapierPhysicsBackend` 持有 Rapier pipeline/body/collider/joint/query 状态，但只通过引擎自有 ID、descriptor、event、query、snapshot 类型对外交互。覆盖状态与验证命令见 `docs/physics_goal_coverage_matrix.md` 和 `docs/physics_goal_audit_report.md`。

---

## 目录

1. [设计目标](#1-设计目标)
2. [模块结构](#2-模块结构)
3. [基础约定](#3-基础约定)
4. [PhysicsWorld](#4-physicsworld)
5. [配置 API](#5-配置-api)
6. [刚体 API](#6-刚体-api)
7. [碰撞体 API](#7-碰撞体-api)
8. [物理材质与过滤](#8-物理材质与过滤)
9. [命令缓冲 API](#9-命令缓冲-api)
10. [事件 API](#10-事件-api)
11. [查询 API](#11-查询-api)
12. [角色控制器 API](#12-角色控制器-api)
13. [关节 API](#13-关节-api)
14. [Debug Draw API](#14-debug-draw-api)
15. [Snapshot 与序列化 API](#15-snapshot-与序列化-api)
16. [ECS 集成 API](#16-ecs-集成-api)
17. [后端适配 API](#17-后端适配-api)
18. [Hooks 与自定义碰撞规则](#18-hooks-与自定义碰撞规则)
19. [使用示例](#19-使用示例)
20. [推荐目录结构](#20-推荐目录结构)
21. [推荐 Feature Flags](#21-推荐-feature-flags)
22. [MVP 实现顺序](#22-mvp-实现顺序)

---

# 1. 设计目标

这套 API 的核心目标是：**让游戏逻辑只和你的引擎物理接口对话，而不是直接依赖 Rapier、PhysX、Bullet 或其他后端类型。**

物理模块应该负责：

```text
刚体
碰撞体
物理材质
碰撞过滤
固定时间步
碰撞事件
触发器事件
接触力事件
raycast / shapecast / overlap 查询
角色控制器
关节
debug draw
snapshot / restore
后端适配
```

物理模块不应该负责：

```text
扣血
AI 决策
动画状态机
播放音效
粒子特效
任务系统
网络协议
渲染层级变换
```

物理模块只告诉玩法系统：

```text
谁撞了谁
在哪里撞了
受力多大
射线打到了什么
角色最终能移动多少
```

至于“撞了以后是扣血、开门、播放金属音、还是史莱姆开始怀疑人生”，交给 gameplay 系统。

---

# 2. 模块结构

推荐 crate 名：

```rust
engine_physics
```

推荐模块：

```rust
pub mod prelude;

pub mod world;
pub mod config;
pub mod id;
pub mod math;
pub mod body;
pub mod collider;
pub mod material;
pub mod filter;
pub mod command;
pub mod event;
pub mod query;
pub mod joint;
pub mod character;
pub mod debug;
pub mod mesh;
pub mod snapshot;
pub mod ecs;
pub mod backend;
pub mod error;
```

推荐导入方式：

```rust
use engine_physics::prelude::*;
```

`prelude` 建议导出：

```rust
pub use crate::world::PhysicsWorld;
pub use crate::config::*;
pub use crate::id::*;
pub use crate::math::*;
pub use crate::body::*;
pub use crate::collider::*;
pub use crate::material::*;
pub use crate::filter::*;
pub use crate::command::*;
pub use crate::event::*;
pub use crate::query::*;
pub use crate::joint::*;
pub use crate::character::*;
pub use crate::debug::*;
pub use crate::error::*;
```

---

# 3. 基础约定

## 3.1 数学类型

物理模块可以使用自己的 `engine_math`，也可以先基于 `glam`。

```rust
pub type Real = f32;

pub type Vec2 = glam::Vec2;
pub type Vec3 = glam::Vec3;
pub type Quat = glam::Quat;
```

3D Transform：

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self;

    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Self;
    pub fn from_translation(translation: Vec3) -> Self;
    pub fn from_rotation(rotation: Quat) -> Self;
    pub fn from_translation_rotation(translation: Vec3, rotation: Quat) -> Self;
    pub fn is_finite(&self) -> bool;
}
```

> 注意：物理模拟通常只应该使用 `translation + rotation`。非均匀缩放不要直接塞进物理世界，应该在创建 shape 时 bake 到形状参数里。

---

## 3.2 ID 类型

所有物理对象使用稳定 ID，不暴露后端 handle。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColliderId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JointId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControllerId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhysicsMeshId(u64);
```

建议内部编码：

```text
高 32 位：generation
低 32 位：index
```

公共方法：

```rust
impl BodyId {
    pub const INVALID: BodyId;

    pub fn is_valid(self) -> bool;
    pub fn raw(self) -> u64;
}

impl ColliderId {
    pub const INVALID: ColliderId;

    pub fn is_valid(self) -> bool;
    pub fn raw(self) -> u64;
}
```

---

## 3.3 用户数据

```rust
pub type EntityId = u64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicsUserData {
    pub entity: Option<EntityId>,
    pub layer_tag: u32,
    pub gameplay_tag: u32,
    pub payload: u64,
}
```

用途：

```text
entity       ：回指 ECS entity
layer_tag    ：例如 Player / Enemy / Projectile
gameplay_tag ：例如 Hurtbox / Hitbox / Pickup
payload      ：留给玩法系统塞额外数据
```

---

## 3.4 错误类型

```rust
pub type PhysicsResult<T> = Result<T, PhysicsError>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum PhysicsError {
    #[error("body not found: {0:?}")]
    BodyNotFound(BodyId),

    #[error("collider not found: {0:?}")]
    ColliderNotFound(ColliderId),

    #[error("joint not found: {0:?}")]
    JointNotFound(JointId),

    #[error("physics mesh not found: {0:?}")]
    MeshNotFound(PhysicsMeshId),

    #[error("invalid shape: {reason}")]
    InvalidShape { reason: String },

    #[error("invalid transform")]
    InvalidTransform,

    #[error("invalid parent body: {0:?}")]
    InvalidParent(BodyId),

    #[error("object already exists")]
    AlreadyExists,

    #[error("backend error: {0}")]
    Backend(String),
}
```

---

# 4. PhysicsWorld

`PhysicsWorld` 是物理模块的主入口。它负责：

```text
创建 / 删除刚体
创建 / 删除碰撞体
固定时间步模拟
事件缓存
查询接口
角色控制器
关节
Debug Draw
Snapshot / Restore
```

```rust
pub struct PhysicsWorld {
    // private
}
```

## 4.1 创建世界

```rust
impl PhysicsWorld {
    pub fn new(config: PhysicsConfig) -> Self;

    pub fn config(&self) -> &PhysicsConfig;
    pub fn config_mut(&mut self) -> &mut PhysicsConfig;

    pub fn gravity(&self) -> Vec3;
    pub fn set_gravity(&mut self, gravity: Vec3);

    pub fn tick(&self) -> PhysicsTick;
    pub fn frame_index(&self) -> u64;
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicsTick(pub u64);
```

---

## 4.2 时间步 API

```rust
impl PhysicsWorld {
    /// 推进一个固定物理 tick。
    ///
    /// 推荐 dt 使用 1.0 / 60.0 或 1.0 / 120.0。
    pub fn step_fixed(&mut self, dt: Real) -> PhysicsStepReport;

    /// 使用 accumulator 自动跑多个 fixed step。
    ///
    /// frame_dt 来自渲染帧 delta time。
    pub fn update_fixed(&mut self, frame_dt: Real) -> PhysicsFrameReport;

    /// 清空 accumulator。
    ///
    /// 用于切场景、暂停恢复、加载存档后避免补跑大量物理步。
    pub fn reset_accumulator(&mut self);

    pub fn interpolation_alpha(&self) -> Real;
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsStepReport {
    pub tick: PhysicsTick,
    pub dt: Real,
    pub active_bodies: usize,
    pub events_generated: usize,
    pub commands_applied: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PhysicsFrameReport {
    pub frame_index: u64,
    pub frame_dt: Real,
    pub steps_run: u32,
    pub dropped_steps: u32,
    pub accumulator: Real,
    pub interpolation_alpha: Real,
}
```

推荐固定时间步流程：

```text
1. 收集 gameplay 产生的物理命令
2. 应用创建 / 删除 / 设置速度 / 施力命令
3. 从 Scene/ECS 同步 kinematic 目标位置
4. 固定时间步 physics step
5. 收集 collision / sensor / contact force 事件
6. 把 dynamic body 的结果写回 Scene/ECS
7. 渲染时做插值
```

---

# 5. 配置 API

```rust
#[derive(Clone, Debug)]
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
    fn default() -> Self;
}
```

## 5.1 时间步配置

```rust
#[derive(Clone, Copy, Debug)]
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
```

## 5.2 求解器配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub velocity_iterations: u32,
    pub position_iterations: u32,
    pub stabilization_iterations: u32,
    pub allowed_linear_error: Real,
    pub prediction_distance: Real,
}
```

## 5.3 睡眠配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct SleepingConfig {
    pub enabled: bool,
    pub linear_threshold: Real,
    pub angular_threshold: Real,
    pub minimum_sleep_time: Real,
}
```

## 5.4 CCD 配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct CcdConfig {
    pub enabled: bool,
    pub max_substeps: u32,
}
```

## 5.5 事件配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct EventConfig {
    pub collect_collision_events: bool,
    pub collect_sensor_events: bool,
    pub collect_contact_force_events: bool,
    pub max_events_per_tick: usize,
}
```

## 5.6 确定性配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct DeterminismConfig {
    pub deterministic_ordering: bool,
    pub stable_event_sorting: bool,
}
```

## 5.7 Debug 配置

```rust
#[derive(Clone, Copy, Debug)]
pub struct DebugConfig {
    pub enabled: bool,
    pub record_query_gizmos: bool,
    pub record_contact_points: bool,
}
```

---

# 6. 刚体 API

## 6.1 BodyKind

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// 受力、重力、碰撞约束影响。
    Dynamic,

    /// 不移动，常用于地面、墙、静态关卡。
    Fixed,

    /// 游戏逻辑直接设置下一帧目标位置。
    KinematicPosition,

    /// 游戏逻辑设置速度，物理系统积分位置。
    KinematicVelocity,
}
```

推荐同步方向：

| BodyKind | 位置所有者 | 同步方向 |
|---|---|---|
| `Dynamic` | 物理引擎 | Physics → Scene |
| `Fixed` | 游戏 / 关卡 | Scene → Physics，通常只初始化一次 |
| `KinematicPosition` | 游戏逻辑 | Scene → Physics |
| `KinematicVelocity` | 游戏逻辑设置速度，物理积分 | Velocity → Physics |

---

## 6.2 BodyDesc

```rust
#[derive(Clone, Debug)]
pub struct BodyDesc {
    pub kind: BodyKind,
    pub transform: Transform,
    pub velocity: Velocity,
    pub mass: MassDesc,
    pub damping: Damping,
    pub gravity_scale: Real,
    pub lock_axes: LockedAxes,
    pub ccd_enabled: bool,
    pub can_sleep: bool,
    pub enabled: bool,
    pub user_data: PhysicsUserData,
    pub debug_name: Option<String>,
}
```

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub enum MassDesc {
    /// 由 attached colliders 的 density 自动计算。
    Auto,

    /// 显式质量。
    Explicit {
        mass: Real,
        center_of_mass: Vec3,
        principal_inertia: Vec3,
    },

    /// 无限质量，通常只给 Fixed 使用。
    Infinite,
}

#[derive(Clone, Copy, Debug)]
pub struct Damping {
    pub linear: Real,
    pub angular: Real,
}
```

```rust
bitflags::bitflags! {
    pub struct LockedAxes: u32 {
        const TRANSLATION_X = 1 << 0;
        const TRANSLATION_Y = 1 << 1;
        const TRANSLATION_Z = 1 << 2;
        const ROTATION_X    = 1 << 3;
        const ROTATION_Y    = 1 << 4;
        const ROTATION_Z    = 1 << 5;

        const TRANSLATION_ALL =
            Self::TRANSLATION_X.bits()
          | Self::TRANSLATION_Y.bits()
          | Self::TRANSLATION_Z.bits();

        const ROTATION_ALL =
            Self::ROTATION_X.bits()
          | Self::ROTATION_Y.bits()
          | Self::ROTATION_Z.bits();

        const ALL = Self::TRANSLATION_ALL.bits() | Self::ROTATION_ALL.bits();
    }
}
```

---

## 6.3 BodyDesc Builder

```rust
impl BodyDesc {
    pub fn dynamic() -> Self;
    pub fn fixed() -> Self;
    pub fn kinematic_position() -> Self;
    pub fn kinematic_velocity() -> Self;

    pub fn with_transform(self, transform: Transform) -> Self;
    pub fn with_translation(self, translation: Vec3) -> Self;
    pub fn with_rotation(self, rotation: Quat) -> Self;

    pub fn with_velocity(self, velocity: Velocity) -> Self;
    pub fn with_linear_velocity(self, linear: Vec3) -> Self;
    pub fn with_angular_velocity(self, angular: Vec3) -> Self;

    pub fn with_mass(self, mass: MassDesc) -> Self;
    pub fn with_damping(self, damping: Damping) -> Self;
    pub fn with_gravity_scale(self, scale: Real) -> Self;
    pub fn with_locked_axes(self, locked: LockedAxes) -> Self;
    pub fn with_ccd(self, enabled: bool) -> Self;
    pub fn with_sleeping(self, can_sleep: bool) -> Self;
    pub fn with_user_data(self, user_data: PhysicsUserData) -> Self;
    pub fn with_debug_name(self, name: impl Into<String>) -> Self;
}
```

---

## 6.4 创建 / 删除刚体

```rust
impl PhysicsWorld {
    pub fn create_body(&mut self, desc: BodyDesc) -> PhysicsResult<BodyId>;

    pub fn destroy_body(&mut self, body: BodyId) -> PhysicsResult<()>;

    /// 删除刚体，同时删除挂在它下面的 collider 和 joint。
    pub fn destroy_body_recursive(&mut self, body: BodyId) -> PhysicsResult<DestroyedObjects>;

    pub fn contains_body(&self, body: BodyId) -> bool;
}
```

```rust
#[derive(Clone, Debug, Default)]
pub struct DestroyedObjects {
    pub bodies: Vec<BodyId>,
    pub colliders: Vec<ColliderId>,
    pub joints: Vec<JointId>,
}
```

---

## 6.5 刚体状态查询

```rust
impl PhysicsWorld {
    pub fn body_kind(&self, body: BodyId) -> PhysicsResult<BodyKind>;

    pub fn body_transform(&self, body: BodyId) -> PhysicsResult<Transform>;
    pub fn body_previous_transform(&self, body: BodyId) -> PhysicsResult<Transform>;
    pub fn body_interpolated_transform(&self, body: BodyId, alpha: Real) -> PhysicsResult<Transform>;

    pub fn body_velocity(&self, body: BodyId) -> PhysicsResult<Velocity>;

    pub fn body_mass(&self, body: BodyId) -> PhysicsResult<Real>;
    pub fn body_center_of_mass(&self, body: BodyId) -> PhysicsResult<Vec3>;

    pub fn body_is_sleeping(&self, body: BodyId) -> PhysicsResult<bool>;
    pub fn body_is_enabled(&self, body: BodyId) -> PhysicsResult<bool>;

    pub fn body_user_data(&self, body: BodyId) -> PhysicsResult<PhysicsUserData>;
}
```

---

## 6.6 修改刚体

```rust
impl PhysicsWorld {
    /// 直接设置位置，相当于 teleport。
    ///
    /// Dynamic body 不建议频繁调用。Dynamic 运动优先用 force / impulse / velocity。
    pub fn set_body_transform(
        &mut self,
        body: BodyId,
        transform: Transform,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    /// 位置型 kinematic 推荐使用这个。
    pub fn set_next_kinematic_transform(
        &mut self,
        body: BodyId,
        next_transform: Transform,
    ) -> PhysicsResult<()>;

    pub fn set_body_velocity(
        &mut self,
        body: BodyId,
        velocity: Velocity,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn set_body_linear_velocity(
        &mut self,
        body: BodyId,
        linear: Vec3,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn set_body_angular_velocity(
        &mut self,
        body: BodyId,
        angular: Vec3,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn set_body_enabled(&mut self, body: BodyId, enabled: bool) -> PhysicsResult<()>;
    pub fn set_body_kind(&mut self, body: BodyId, kind: BodyKind) -> PhysicsResult<()>;

    pub fn wake_body(&mut self, body: BodyId) -> PhysicsResult<()>;
    pub fn sleep_body(&mut self, body: BodyId) -> PhysicsResult<()>;
}
```

---

## 6.7 力和冲量

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceMode {
    /// 连续力，受 dt 影响。
    Force,

    /// 瞬时冲量。
    Impulse,

    /// 忽略质量的连续加速度。
    Acceleration,

    /// 忽略质量的速度变化。
    VelocityChange,
}
```

```rust
impl PhysicsWorld {
    pub fn add_force(
        &mut self,
        body: BodyId,
        force: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn add_force_at_point(
        &mut self,
        body: BodyId,
        force: Vec3,
        world_point: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn add_torque(
        &mut self,
        body: BodyId,
        torque: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()>;

    pub fn clear_forces(&mut self, body: BodyId) -> PhysicsResult<()>;
}
```

---

# 7. 碰撞体 API

## 7.1 ColliderShape

```rust
#[derive(Clone, Debug)]
pub enum ColliderShape {
    Sphere {
        radius: Real,
    },

    Cuboid {
        half_extents: Vec3,
    },

    Capsule {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },

    Cylinder {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },

    Cone {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },

    ConvexHull {
        mesh: PhysicsMeshId,
    },

    TriMesh {
        mesh: PhysicsMeshId,
        flags: TriMeshFlags,
    },

    HeightField {
        mesh: PhysicsMeshId,
    },

    Compound {
        parts: Vec<CompoundShapePart>,
    },
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis3 {
    X,
    Y,
    Z,
}
```

```rust
#[derive(Clone, Debug)]
pub struct CompoundShapePart {
    pub local_transform: Transform,
    pub shape: Box<ColliderShape>,
}
```

```rust
bitflags::bitflags! {
    pub struct TriMeshFlags: u32 {
        const DOUBLE_SIDED = 1 << 0;
        const FIX_INTERNAL_EDGES = 1 << 1;
    }
}
```

---

## 7.2 Mesh 资源

为了避免每个 collider 都复制大网格，复杂 shape 通过 `PhysicsMeshId` 引用。

```rust
pub struct TriMeshDesc {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

pub struct ConvexMeshDesc {
    pub points: Vec<Vec3>,
}

pub struct HeightFieldDesc {
    pub heights: Vec<Real>,
    pub rows: u32,
    pub cols: u32,
    pub scale: Vec3,
}
```

```rust
impl PhysicsWorld {
    pub fn create_trimesh(&mut self, desc: TriMeshDesc) -> PhysicsResult<PhysicsMeshId>;
    pub fn create_convex_mesh(&mut self, desc: ConvexMeshDesc) -> PhysicsResult<PhysicsMeshId>;
    pub fn create_heightfield(&mut self, desc: HeightFieldDesc) -> PhysicsResult<PhysicsMeshId>;

    pub fn destroy_mesh(&mut self, mesh: PhysicsMeshId) -> PhysicsResult<()>;
    pub fn contains_mesh(&self, mesh: PhysicsMeshId) -> bool;
}
```

---

## 7.3 ColliderDesc

```rust
#[derive(Clone, Debug)]
pub struct ColliderDesc {
    pub shape: ColliderShape,
    pub local_transform: Transform,
    pub material: PhysicsMaterial,
    pub density: Real,
    pub filter: CollisionFilter,
    pub sensor: bool,
    pub enabled: bool,
    pub events: ActiveEvents,
    pub contact_skin: Real,
    pub user_data: PhysicsUserData,
    pub debug_name: Option<String>,
}
```

```rust
bitflags::bitflags! {
    pub struct ActiveEvents: u32 {
        const COLLISION_EVENTS     = 1 << 0;
        const SENSOR_EVENTS        = 1 << 1;
        const CONTACT_FORCE_EVENTS = 1 << 2;
    }
}
```

---

## 7.4 ColliderDesc Builder

```rust
impl ColliderDesc {
    pub fn new(shape: ColliderShape) -> Self;

    pub fn sphere(radius: Real) -> Self;
    pub fn cuboid(half_extents: Vec3) -> Self;
    pub fn capsule_y(half_height: Real, radius: Real) -> Self;
    pub fn trimesh(mesh: PhysicsMeshId) -> Self;
    pub fn convex_hull(mesh: PhysicsMeshId) -> Self;

    pub fn with_local_transform(self, transform: Transform) -> Self;
    pub fn with_material(self, material: PhysicsMaterial) -> Self;
    pub fn with_density(self, density: Real) -> Self;
    pub fn with_filter(self, filter: CollisionFilter) -> Self;
    pub fn with_sensor(self, sensor: bool) -> Self;
    pub fn with_events(self, events: ActiveEvents) -> Self;
    pub fn with_user_data(self, user_data: PhysicsUserData) -> Self;
    pub fn with_debug_name(self, name: impl Into<String>) -> Self;
}
```

---

## 7.5 创建 / 删除 Collider

```rust
impl PhysicsWorld {
    /// 创建独立 collider。
    ///
    /// 独立 collider 常用于纯 trigger、编辑器拾取区域、静态查询体。
    pub fn create_collider(&mut self, desc: ColliderDesc) -> PhysicsResult<ColliderId>;

    /// 创建并挂到刚体上。
    pub fn create_collider_with_parent(
        &mut self,
        parent: BodyId,
        desc: ColliderDesc,
    ) -> PhysicsResult<ColliderId>;

    pub fn attach_collider(
        &mut self,
        collider: ColliderId,
        parent: BodyId,
    ) -> PhysicsResult<()>;

    pub fn detach_collider(&mut self, collider: ColliderId) -> PhysicsResult<()>;

    pub fn destroy_collider(&mut self, collider: ColliderId) -> PhysicsResult<()>;

    pub fn contains_collider(&self, collider: ColliderId) -> bool;
}
```

---

## 7.6 Collider 查询 / 修改

```rust
impl PhysicsWorld {
    pub fn collider_parent(&self, collider: ColliderId) -> PhysicsResult<Option<BodyId>>;

    pub fn collider_shape(&self, collider: ColliderId) -> PhysicsResult<ColliderShape>;
    pub fn collider_world_transform(&self, collider: ColliderId) -> PhysicsResult<Transform>;
    pub fn collider_local_transform(&self, collider: ColliderId) -> PhysicsResult<Transform>;

    pub fn collider_material(&self, collider: ColliderId) -> PhysicsResult<PhysicsMaterial>;
    pub fn collider_filter(&self, collider: ColliderId) -> PhysicsResult<CollisionFilter>;
    pub fn collider_is_sensor(&self, collider: ColliderId) -> PhysicsResult<bool>;

    pub fn set_collider_shape(
        &mut self,
        collider: ColliderId,
        shape: ColliderShape,
    ) -> PhysicsResult<()>;

    pub fn set_collider_local_transform(
        &mut self,
        collider: ColliderId,
        transform: Transform,
    ) -> PhysicsResult<()>;

    pub fn set_collider_material(
        &mut self,
        collider: ColliderId,
        material: PhysicsMaterial,
    ) -> PhysicsResult<()>;

    pub fn set_collider_filter(
        &mut self,
        collider: ColliderId,
        filter: CollisionFilter,
    ) -> PhysicsResult<()>;

    pub fn set_collider_sensor(
        &mut self,
        collider: ColliderId,
        sensor: bool,
    ) -> PhysicsResult<()>;

    pub fn set_collider_enabled(
        &mut self,
        collider: ColliderId,
        enabled: bool,
    ) -> PhysicsResult<()>;
}
```

---

# 8. 物理材质与过滤

## 8.1 PhysicsMaterial

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsMaterial {
    pub friction: Real,
    pub restitution: Real,
    pub friction_combine: CombineRule,
    pub restitution_combine: CombineRule,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.5,
            restitution: 0.0,
            friction_combine: CombineRule::Average,
            restitution_combine: CombineRule::Average,
        }
    }
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombineRule {
    Average,
    Min,
    Multiply,
    Max,
}
```

---

## 8.2 InteractionGroups

```rust
pub type LayerMask = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InteractionGroups {
    pub memberships: LayerMask,
    pub filter: LayerMask,
}

impl InteractionGroups {
    pub const ALL: Self;
    pub const NONE: Self;

    pub fn new(memberships: LayerMask, filter: LayerMask) -> Self;

    pub fn can_interact_with(self, other: Self) -> bool {
        (self.memberships & other.filter) != 0
            && (other.memberships & self.filter) != 0
    }
}
```

---

## 8.3 CollisionFilter

```rust
#[derive(Clone, Copy, Debug)]
pub struct CollisionFilter {
    /// 控制是否生成 contact / sensor intersection。
    pub collision_groups: InteractionGroups,

    /// 控制是否进入 solver 产生接触力。
    pub solver_groups: InteractionGroups,
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            collision_groups: InteractionGroups::ALL,
            solver_groups: InteractionGroups::ALL,
        }
    }
}
```

推荐在游戏里定义：

```rust
pub mod physics_layers {
    pub const WORLD: u64      = 1 << 0;
    pub const PLAYER: u64     = 1 << 1;
    pub const ENEMY: u64      = 1 << 2;
    pub const PROJECTILE: u64 = 1 << 3;
    pub const TRIGGER: u64    = 1 << 4;
    pub const HITBOX: u64     = 1 << 5;
    pub const HURTBOX: u64    = 1 << 6;
}
```

示例：

```rust
let player_filter = CollisionFilter {
    collision_groups: InteractionGroups::new(
        physics_layers::PLAYER,
        physics_layers::WORLD | physics_layers::ENEMY | physics_layers::TRIGGER,
    ),
    solver_groups: InteractionGroups::ALL,
};
```

---

# 9. 命令缓冲 API

物理模块支持立即 API，也支持 command buffer。运行时建议用 command buffer，避免 ECS 遍历中途修改物理世界。

## 9.1 PhysicsCommand

```rust
#[derive(Clone, Debug)]
pub enum PhysicsCommand {
    CreateBody {
        id: BodyId,
        desc: BodyDesc,
    },

    DestroyBody {
        id: BodyId,
        recursive: bool,
    },

    CreateCollider {
        id: ColliderId,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    },

    DestroyCollider {
        id: ColliderId,
    },

    SetBodyTransform {
        id: BodyId,
        transform: Transform,
        wake_up: bool,
    },

    SetNextKinematicTransform {
        id: BodyId,
        transform: Transform,
    },

    SetBodyVelocity {
        id: BodyId,
        velocity: Velocity,
        wake_up: bool,
    },

    AddForce {
        id: BodyId,
        force: Vec3,
        mode: ForceMode,
        wake_up: bool,
    },

    AddForceAtPoint {
        id: BodyId,
        force: Vec3,
        point: Vec3,
        mode: ForceMode,
        wake_up: bool,
    },

    SetColliderEnabled {
        id: ColliderId,
        enabled: bool,
    },

    SetCollisionFilter {
        id: ColliderId,
        filter: CollisionFilter,
    },
}
```

---

## 9.2 PhysicsCommandBuffer

```rust
pub struct PhysicsCommandBuffer {
    // private
}

impl PhysicsCommandBuffer {
    pub fn new() -> Self;
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn clear(&mut self);

    pub fn push(&mut self, command: PhysicsCommand);

    pub fn create_body(&mut self, id: BodyId, desc: BodyDesc);
    pub fn destroy_body(&mut self, id: BodyId, recursive: bool);

    pub fn create_collider(
        &mut self,
        id: ColliderId,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    );

    pub fn destroy_collider(&mut self, id: ColliderId);

    pub fn set_body_transform(&mut self, id: BodyId, transform: Transform, wake_up: bool);
    pub fn set_next_kinematic_transform(&mut self, id: BodyId, transform: Transform);
    pub fn set_body_velocity(&mut self, id: BodyId, velocity: Velocity, wake_up: bool);

    pub fn add_force(&mut self, id: BodyId, force: Vec3, mode: ForceMode, wake_up: bool);
}
```

---

## 9.3 ID 预分配与应用命令

```rust
impl PhysicsWorld {
    pub fn reserve_body_id(&mut self) -> BodyId;
    pub fn reserve_collider_id(&mut self) -> ColliderId;
    pub fn reserve_joint_id(&mut self) -> JointId;

    pub fn apply_commands(&mut self, commands: &mut PhysicsCommandBuffer) -> PhysicsCommandReport;
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsCommandReport {
    pub applied: usize,
    pub failed: usize,
}
```

---

# 10. 事件 API

物理事件从后端事件转换为引擎自己的事件类型。

## 10.1 PhysicsEvent

```rust
#[derive(Clone, Debug)]
pub enum PhysicsEvent {
    CollisionStarted(CollisionEvent),
    CollisionStopped(CollisionEvent),

    SensorEntered(SensorEvent),
    SensorExited(SensorEvent),

    ContactForce(ContactForceEvent),

    BodyWokeUp(BodyId),
    BodyWentToSleep(BodyId),

    JointBroken(JointBrokenEvent),
}
```

```rust
#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub tick: PhysicsTick,
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub user_data_a: PhysicsUserData,
    pub user_data_b: PhysicsUserData,
}

#[derive(Clone, Debug)]
pub struct SensorEvent {
    pub tick: PhysicsTick,
    pub sensor: ColliderId,
    pub other: ColliderId,
    pub sensor_body: Option<BodyId>,
    pub other_body: Option<BodyId>,
}

#[derive(Clone, Debug)]
pub struct ContactForceEvent {
    pub tick: PhysicsTick,
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub total_force: Vec3,
    pub total_force_magnitude: Real,
    pub max_force_magnitude: Real,
}

#[derive(Clone, Debug)]
pub struct JointBrokenEvent {
    pub tick: PhysicsTick,
    pub joint: JointId,
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub impulse: Real,
}
```

---

## 10.2 事件读取

```rust
impl PhysicsWorld {
    pub fn events(&self) -> &[PhysicsEvent];

    pub fn drain_events(&mut self) -> impl Iterator<Item = PhysicsEvent> + '_;

    pub fn clear_events(&mut self);

    pub fn events_since(&self, cursor: &mut EventCursor) -> &[PhysicsEvent];
}
```

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct EventCursor {
    index: usize,
}
```

推荐用法：

```rust
let mut cursor = EventCursor::default();

for event in physics.events_since(&mut cursor) {
    match event {
        PhysicsEvent::SensorEntered(e) => {
            // 拾取物、门、剧情触发器
        }
        PhysicsEvent::CollisionStarted(e) => {
            // 撞击音效、伤害判定
        }
        _ => {}
    }
}
```

---

# 11. 查询 API

查询 API 用于：

```text
射线检测
shape cast
overlap
point projection
AABB 查询
接触点查询
```

后端可以使用 Rapier 的 query pipeline，但外部只使用你自己的接口。

---

## 11.1 查询入口

```rust
impl PhysicsWorld {
    pub fn query(&self) -> PhysicsQuery<'_>;

    /// 创建可跨线程使用的查询快照。
    ///
    /// 适合 AI、编辑器拾取、异步寻路预采样。
    pub fn query_snapshot(&self) -> PhysicsQuerySnapshot;
}
```

```rust
pub struct PhysicsQuery<'a> {
    // private
}

pub struct PhysicsQuerySnapshot {
    // private
}
```

---

## 11.2 QueryFilter

```rust
#[derive(Clone, Debug)]
pub struct QueryFilter {
    pub groups: InteractionGroups,
    pub include_sensors: bool,
    pub include_dynamic: bool,
    pub include_fixed: bool,
    pub include_kinematic: bool,
    pub exclude_body: Option<BodyId>,
    pub exclude_collider: Option<ColliderId>,
    pub max_results: Option<usize>,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            groups: InteractionGroups::ALL,
            include_sensors: false,
            include_dynamic: true,
            include_fixed: true,
            include_kinematic: true,
            exclude_body: None,
            exclude_collider: None,
            max_results: None,
        }
    }
}
```

---

## 11.3 Raycast

```rust
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub max_toi: Real,
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub normal: Vec3,
    pub toi: Real,
    pub user_data: PhysicsUserData,
}
```

```rust
impl<'a> PhysicsQuery<'a> {
    pub fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit>;

    pub fn cast_ray_all(
        &self,
        ray: Ray,
        filter: QueryFilter,
        hits: &mut Vec<RayHit>,
    ) -> usize;

    pub fn cast_ray_predicate<F>(
        &self,
        ray: Ray,
        filter: QueryFilter,
        predicate: F,
    ) -> Option<RayHit>
    where
        F: Fn(ColliderId, PhysicsUserData) -> bool;
}
```

---

## 11.4 Shape Cast

```rust
#[derive(Clone, Debug)]
pub struct ShapeCastInput {
    pub shape: ColliderShape,
    pub transform: Transform,
    pub translation: Vec3,
    pub max_toi: Real,
    pub stop_at_penetration: bool,
    pub target_distance: Real,
}

#[derive(Clone, Debug)]
pub struct ShapeCastHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub toi: Real,
    pub point1: Vec3,
    pub point2: Vec3,
    pub normal1: Vec3,
    pub normal2: Vec3,
    pub user_data: PhysicsUserData,
}
```

```rust
impl<'a> PhysicsQuery<'a> {
    pub fn cast_shape(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
    ) -> Option<ShapeCastHit>;

    pub fn cast_shape_all(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
        hits: &mut Vec<ShapeCastHit>,
    ) -> usize;
}
```

---

## 11.5 Overlap / Intersection

```rust
#[derive(Clone, Debug)]
pub struct OverlapInput {
    pub shape: ColliderShape,
    pub transform: Transform,
}

#[derive(Clone, Copy, Debug)]
pub struct OverlapHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub user_data: PhysicsUserData,
}
```

```rust
impl<'a> PhysicsQuery<'a> {
    pub fn overlap_shape(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize;

    pub fn overlap_aabb(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize;

    pub fn contains_point(
        &self,
        point: Vec3,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize;
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
```

---

## 11.6 Point Projection

```rust
#[derive(Clone, Copy, Debug)]
pub struct PointProjection {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub is_inside: bool,
    pub distance: Real,
}
```

```rust
impl<'a> PhysicsQuery<'a> {
    pub fn project_point(
        &self,
        point: Vec3,
        max_distance: Real,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<PointProjection>;
}
```

---

## 11.7 接触查询

```rust
#[derive(Clone, Debug)]
pub struct ContactPoint {
    pub local_point_a: Vec3,
    pub local_point_b: Vec3,
    pub world_point_a: Vec3,
    pub world_point_b: Vec3,
    pub normal: Vec3,
    pub penetration: Real,
    pub impulse: Real,
}

#[derive(Clone, Debug)]
pub struct ContactManifold {
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub contacts: Vec<ContactPoint>,
}
```

```rust
impl PhysicsWorld {
    pub fn contact_pair(
        &self,
        a: ColliderId,
        b: ColliderId,
    ) -> PhysicsResult<Option<ContactManifold>>;

    pub fn contacts_with_body(
        &self,
        body: BodyId,
        out: &mut Vec<ContactManifold>,
    ) -> PhysicsResult<usize>;

    pub fn contacts_with_collider(
        &self,
        collider: ColliderId,
        out: &mut Vec<ContactManifold>,
    ) -> PhysicsResult<usize>;
}
```

---

# 12. 角色控制器 API

角色控制器不建议做成普通 dynamic body 乱推。推荐：

```text
玩家 / NPC = kinematic capsule
移动 = desired_translation
阻挡 = shapecast / controller 修正
输出 = corrected_translation + grounded 信息
```

---

## 12.1 CharacterControllerDesc

```rust
#[derive(Clone, Debug)]
pub struct CharacterControllerDesc {
    pub up: Vec3,

    /// 角色与环境保留的小间隙。
    pub offset: Real,

    /// 最大可站立斜坡角度，单位弧度。
    pub max_slope_angle: Real,

    /// 小于该高度的台阶可以自动爬上。
    pub step_height: Real,

    /// 最大自动贴地距离。
    pub snap_to_ground_distance: Real,

    pub enable_slide: bool,
    pub enable_auto_step: bool,
    pub enable_snap_to_ground: bool,

    /// 是否对 dynamic body 施加推力。
    pub apply_impulses_to_dynamic_bodies: bool,

    pub max_iterations: u32,
}

impl Default for CharacterControllerDesc {
    fn default() -> Self;
}
```

---

## 12.2 创建 / 删除控制器

```rust
impl PhysicsWorld {
    pub fn create_character_controller(
        &mut self,
        desc: CharacterControllerDesc,
    ) -> CharacterControllerId;

    pub fn destroy_character_controller(
        &mut self,
        id: CharacterControllerId,
    ) -> PhysicsResult<()>;

    pub fn character_controller(
        &self,
        id: CharacterControllerId,
    ) -> PhysicsResult<&CharacterControllerDesc>;

    pub fn character_controller_mut(
        &mut self,
        id: CharacterControllerId,
    ) -> PhysicsResult<&mut CharacterControllerDesc>;
}
```

---

## 12.3 CharacterMoveInput / Output

```rust
#[derive(Clone, Debug)]
pub struct CharacterMoveInput {
    pub controller: CharacterControllerId,
    pub body: BodyId,
    pub collider: ColliderId,
    pub desired_translation: Vec3,
    pub dt: Real,
    pub filter: QueryFilter,
}

#[derive(Clone, Debug)]
pub struct CharacterMoveOutput {
    pub requested_translation: Vec3,
    pub corrected_translation: Vec3,
    pub final_transform: Transform,

    pub grounded: bool,
    pub ground_collider: Option<ColliderId>,
    pub ground_body: Option<BodyId>,
    pub ground_normal: Vec3,

    pub hit_wall: bool,
    pub hit_ceiling: bool,

    pub collisions: Vec<CharacterCollision>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct CharacterCollision {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub normal: Vec3,
    pub translation_remaining: Vec3,
}
```

---

## 12.4 移动 API

```rust
impl PhysicsWorld {
    /// 只计算，不写回 body。
    pub fn compute_character_movement(
        &self,
        input: CharacterMoveInput,
    ) -> PhysicsResult<CharacterMoveOutput>;

    /// 计算并写回 kinematic body。
    pub fn move_character(
        &mut self,
        input: CharacterMoveInput,
    ) -> PhysicsResult<CharacterMoveOutput>;
}
```

---

# 13. 关节 API

## 13.1 JointDesc

```rust
#[derive(Clone, Debug)]
pub enum JointDesc {
    Fixed(FixedJointDesc),
    Ball(BallJointDesc),
    Hinge(HingeJointDesc),
    Prismatic(PrismaticJointDesc),
    Distance(DistanceJointDesc),
    Generic(GenericJointDesc),
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct JointAnchor {
    pub local_anchor_a: Vec3,
    pub local_anchor_b: Vec3,
    pub local_axis_a: Vec3,
    pub local_axis_b: Vec3,
}
```

```rust
#[derive(Clone, Debug)]
pub struct FixedJointDesc {
    pub local_frame_a: Transform,
    pub local_frame_b: Transform,
}

#[derive(Clone, Debug)]
pub struct BallJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
}

#[derive(Clone, Debug)]
pub struct HingeJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
    pub motor: Option<JointMotor>,
}

#[derive(Clone, Debug)]
pub struct PrismaticJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
    pub motor: Option<JointMotor>,
}

#[derive(Clone, Debug)]
pub struct DistanceJointDesc {
    pub local_anchor_a: Vec3,
    pub local_anchor_b: Vec3,
    pub min_distance: Real,
    pub max_distance: Real,
}

#[derive(Clone, Debug)]
pub struct GenericJointDesc {
    pub local_frame_a: Transform,
    pub local_frame_b: Transform,
    pub locked_axes: JointLockedAxes,
    pub limits: Vec<JointAxisLimit>,
    pub motors: Vec<JointAxisMotor>,
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct JointLimits {
    pub min: Real,
    pub max: Real,
}

#[derive(Clone, Copy, Debug)]
pub struct JointMotor {
    pub target_velocity: Real,
    pub target_position: Option<Real>,
    pub stiffness: Real,
    pub damping: Real,
    pub max_force: Real,
}
```

占位类型：

```rust
#[derive(Clone, Copy, Debug)]
pub enum JointAxis {
    X,
    Y,
    Z,
    AngularX,
    AngularY,
    AngularZ,
}

bitflags::bitflags! {
    pub struct JointLockedAxes: u32 {
        const LIN_X = 1 << 0;
        const LIN_Y = 1 << 1;
        const LIN_Z = 1 << 2;
        const ANG_X = 1 << 3;
        const ANG_Y = 1 << 4;
        const ANG_Z = 1 << 5;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JointAxisLimit {
    pub axis: JointAxis,
    pub min: Real,
    pub max: Real,
}

#[derive(Clone, Copy, Debug)]
pub struct JointAxisMotor {
    pub axis: JointAxis,
    pub motor: JointMotor,
}
```

---

## 13.2 创建 / 删除 Joint

```rust
impl PhysicsWorld {
    pub fn create_joint(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        desc: JointDesc,
    ) -> PhysicsResult<JointId>;

    pub fn destroy_joint(&mut self, joint: JointId) -> PhysicsResult<()>;

    pub fn contains_joint(&self, joint: JointId) -> bool;

    pub fn joint_bodies(&self, joint: JointId) -> PhysicsResult<(BodyId, BodyId)>;

    pub fn set_joint_enabled(&mut self, joint: JointId, enabled: bool) -> PhysicsResult<()>;

    pub fn set_joint_motor(
        &mut self,
        joint: JointId,
        axis: JointAxis,
        motor: JointMotor,
    ) -> PhysicsResult<()>;

    pub fn set_joint_limits(
        &mut self,
        joint: JointId,
        axis: JointAxis,
        limits: JointLimits,
    ) -> PhysicsResult<()>;
}
```

---

# 14. Debug Draw API

物理 debug draw 应该尽早接入，否则后面会变成黑箱考古。

调试内容建议包括：

```text
collider wireframe
body center of mass
contact points
contact normals
raycast lines
sleeping / active body 状态
collision layer 名称
AABB
character controller ground normal
```

---

## 14.1 Debug Draw 选项

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsDebugDrawOptions {
    pub draw_bodies: bool,
    pub draw_colliders: bool,
    pub draw_aabbs: bool,
    pub draw_contacts: bool,
    pub draw_contact_normals: bool,
    pub draw_joints: bool,
    pub draw_sleeping: bool,
    pub draw_query_gizmos: bool,
    pub draw_names: bool,
}

impl Default for PhysicsDebugDrawOptions {
    fn default() -> Self;
}
```

---

## 14.2 PhysicsDebugRenderer Trait

```rust
pub trait PhysicsDebugRenderer {
    fn line(&mut self, from: Vec3, to: Vec3, style: DebugLineStyle);
    fn sphere(&mut self, center: Vec3, radius: Real, style: DebugShapeStyle);
    fn cuboid(&mut self, transform: Transform, half_extents: Vec3, style: DebugShapeStyle);
    fn capsule(
        &mut self,
        transform: Transform,
        axis: Axis3,
        half_height: Real,
        radius: Real,
        style: DebugShapeStyle,
    );
    fn text(&mut self, position: Vec3, text: &str);
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct DebugLineStyle {
    pub category: DebugDrawCategory,
    pub thickness: Real,
}

#[derive(Clone, Copy, Debug)]
pub struct DebugShapeStyle {
    pub category: DebugDrawCategory,
    pub wireframe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugDrawCategory {
    DynamicBody,
    FixedBody,
    KinematicBody,
    Sensor,
    Sleeping,
    Contact,
    Joint,
    Query,
}
```

---

## 14.3 Debug Draw 调用

```rust
impl PhysicsWorld {
    pub fn debug_draw(
        &self,
        renderer: &mut dyn PhysicsDebugRenderer,
        options: PhysicsDebugDrawOptions,
    );
}
```

---

# 15. Snapshot 与序列化 API

用于：

```text
存档
回放
编辑器撤销
网络同步
物理状态调试
```

```rust
#[derive(Clone, Debug)]
pub struct PhysicsSnapshot {
    pub tick: PhysicsTick,
    pub config: PhysicsConfig,
    pub bodies: Vec<BodySnapshot>,
    pub colliders: Vec<ColliderSnapshot>,
    pub joints: Vec<JointSnapshot>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct BodySnapshot {
    pub id: BodyId,
    pub desc: BodyDesc,
    pub transform: Transform,
    pub previous_transform: Transform,
    pub velocity: Velocity,
    pub sleeping: bool,
}

#[derive(Clone, Debug)]
pub struct ColliderSnapshot {
    pub id: ColliderId,
    pub parent: Option<BodyId>,
    pub desc: ColliderDesc,
}

#[derive(Clone, Debug)]
pub struct JointSnapshot {
    pub id: JointId,
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub desc: JointDesc,
}
```

```rust
impl PhysicsWorld {
    pub fn snapshot(&self) -> PhysicsSnapshot;

    pub fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()>;
}
```

---

# 16. ECS 集成 API

这个模块不要求你的引擎一定是 ECS，但如果是 ECS，可以用这组组件。

## 16.1 组件

```rust
#[derive(Clone, Debug)]
pub struct RigidBodyComponent {
    pub body: BodyId,
    pub desc: BodyDesc,
}

#[derive(Clone, Debug)]
pub struct ColliderComponent {
    pub collider: ColliderId,
    pub parent: Option<BodyId>,
    pub desc: ColliderDesc,
}

#[derive(Clone, Debug)]
pub struct JointComponent {
    pub joint: JointId,
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub desc: JointDesc,
}
```

---

## 16.2 Transform 同步

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsSyncMode {
    /// 物理结果写回 Transform。
    ///
    /// Dynamic body 推荐这个。
    PhysicsToTransform,

    /// Transform 写入物理。
    ///
    /// Fixed / Kinematic 推荐这个。
    TransformToPhysics,

    /// 双方不自动同步。
    Disabled,
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsSyncComponent {
    pub mode: PhysicsSyncMode,
    pub interpolate: bool,
}
```

推荐默认规则：

| BodyKind | 默认同步方向 |
|---|---|
| `Dynamic` | `PhysicsToTransform` |
| `Fixed` | `TransformToPhysics`，通常只初始化一次 |
| `KinematicPosition` | `TransformToPhysics` |
| `KinematicVelocity` | velocity 写入 physics，transform 由 physics 积分 |
| Sensor | 跟随 parent 或手动同步 |

---

## 16.3 推荐系统顺序

```text
InputSystem
GameplaySystem
PhysicsCommandBuildSystem
PhysicsApplyCommandsSystem
PrePhysicsTransformSyncSystem
PhysicsStepSystem
PhysicsEventDispatchSystem
PostPhysicsTransformSyncSystem
AnimationSystem
RenderSystem
```

其中：

```text
PrePhysicsTransformSyncSystem:
    把 kinematic / fixed transform 写进 physics

PhysicsStepSystem:
    fixed tick step

PostPhysicsTransformSyncSystem:
    把 dynamic body transform 写回 ECS Transform
```

---

# 17. 后端适配 API

一开始可以只实现 `RapierBackend`，但 public API 不暴露 Rapier。

```rust
pub trait PhysicsBackend {
    fn create_body(&mut self, id: BodyId, desc: BodyDesc) -> PhysicsResult<()>;
    fn destroy_body(&mut self, id: BodyId, recursive: bool) -> PhysicsResult<DestroyedObjects>;

    fn create_collider(
        &mut self,
        id: ColliderId,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    ) -> PhysicsResult<()>;

    fn destroy_collider(&mut self, id: ColliderId) -> PhysicsResult<()>;

    fn create_joint(
        &mut self,
        id: JointId,
        body_a: BodyId,
        body_b: BodyId,
        desc: JointDesc,
    ) -> PhysicsResult<()>;

    fn destroy_joint(&mut self, id: JointId) -> PhysicsResult<()>;

    fn step(&mut self, dt: Real, events: &mut Vec<PhysicsEvent>) -> PhysicsStepReport;

    fn query(&self) -> Box<dyn PhysicsQueryBackend + '_>;

    fn debug_draw(
        &self,
        renderer: &mut dyn PhysicsDebugRenderer,
        options: PhysicsDebugDrawOptions,
    );

    fn snapshot(&self) -> PhysicsSnapshot;
    fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()>;
}
```

```rust
pub trait PhysicsQueryBackend {
    fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit>;

    fn cast_shape(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
    ) -> Option<ShapeCastHit>;

    fn overlap_shape(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize;
}
```

后端模块：

```rust
#[cfg(feature = "backend_rapier")]
pub mod rapier_backend {
    pub struct RapierPhysicsBackend {
        // private
    }
}
```

---

# 18. Hooks 与自定义碰撞规则

用于：

```text
复杂阵营过滤
单向平台
接触点修改
冰面 / 泥地 / 弹簧地板
自定义 hitbox/hurtbox 规则
```

```rust
pub trait PhysicsHooks: Send + Sync + 'static {
    fn filter_collision_pair(&self, pair: CollisionPairInfo) -> CollisionDecision {
        CollisionDecision::UseDefault
    }

    fn modify_contacts(&self, context: &mut ContactModificationContext) {}
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct CollisionPairInfo {
    pub collider_a: ColliderId,
    pub collider_b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub user_data_a: PhysicsUserData,
    pub user_data_b: PhysicsUserData,
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionDecision {
    UseDefault,
    DisableCollision,
    DisableSolver,
}
```

```rust
pub struct ContactModificationContext<'a> {
    pub pair: CollisionPairInfo,
    pub contacts: &'a mut [ContactPoint],
    pub material: &'a mut PhysicsMaterial,
}
```

```rust
impl PhysicsWorld {
    pub fn set_hooks<H>(&mut self, hooks: H)
    where
        H: PhysicsHooks;

    pub fn clear_hooks(&mut self);
}
```

---

# 19. 使用示例

## 19.1 初始化世界

```rust
use engine_physics::prelude::*;

let mut physics = PhysicsWorld::new(PhysicsConfig {
    gravity: Vec3::new(0.0, -9.81, 0.0),
    ..Default::default()
});
```

---

## 19.2 创建地面

```rust
let ground_body = physics.create_body(
    BodyDesc::fixed()
        .with_translation(Vec3::new(0.0, -1.0, 0.0))
        .with_debug_name("Ground"),
)?;

let ground_collider = physics.create_collider_with_parent(
    ground_body,
    ColliderDesc::cuboid(Vec3::new(50.0, 1.0, 50.0))
        .with_material(PhysicsMaterial {
            friction: 0.8,
            restitution: 0.0,
            ..Default::default()
        }),
)?;
```

---

## 19.3 创建动态箱子

```rust
let box_body = physics.create_body(
    BodyDesc::dynamic()
        .with_translation(Vec3::new(0.0, 5.0, 0.0))
        .with_ccd(true)
        .with_debug_name("Falling Box"),
)?;

let box_collider = physics.create_collider_with_parent(
    box_body,
    ColliderDesc::cuboid(Vec3::splat(0.5))
        .with_density(1.0),
)?;
```

---

## 19.4 固定时间步

```rust
fn frame_update(physics: &mut PhysicsWorld, frame_dt: f32) {
    let report = physics.update_fixed(frame_dt);

    for event in physics.drain_events() {
        match event {
            PhysicsEvent::CollisionStarted(e) => {
                println!("collision: {:?} {:?}", e.a, e.b);
            }
            PhysicsEvent::SensorEntered(e) => {
                println!("sensor entered: {:?} {:?}", e.sensor, e.other);
            }
            _ => {}
        }
    }
}
```

---

## 19.5 Raycast

```rust
let ray = Ray {
    origin: camera_pos,
    direction: camera_forward,
    max_toi: 1000.0,
};

let hit = physics.query().cast_ray(
    ray,
    QueryFilter {
        include_sensors: false,
        ..Default::default()
    },
);

if let Some(hit) = hit {
    println!("hit collider {:?} at {:?}", hit.collider, hit.point);
}
```

---

## 19.6 角色移动

```rust
let controller = physics.create_character_controller(CharacterControllerDesc {
    up: Vec3::Y,
    step_height: 0.35,
    max_slope_angle: 50.0_f32.to_radians(),
    snap_to_ground_distance: 0.2,
    ..Default::default()
});

let output = physics.move_character(CharacterMoveInput {
    controller,
    body: player_body,
    collider: player_capsule,
    desired_translation: wish_dir * speed * dt,
    dt,
    filter: QueryFilter {
        exclude_body: Some(player_body),
        include_sensors: false,
        ..Default::default()
    },
})?;

if output.grounded {
    // 允许跳跃
}
```

---

# 20. 推荐目录结构

```text
src/
  physics/
    mod.rs
    prelude.rs

    world.rs
    config.rs
    id.rs
    math.rs
    error.rs

    body.rs
    collider.rs
    material.rs
    filter.rs
    mesh.rs

    command.rs
    event.rs
    query.rs
    joint.rs
    character.rs
    debug.rs
    snapshot.rs

    ecs/
      mod.rs
      components.rs
      systems.rs
      sync.rs

    backend/
      mod.rs
      trait.rs
      rapier.rs
```

---

# 21. 推荐 Feature Flags

```toml
[features]
default = ["3d", "backend_rapier"]

3d = []
2d = []

backend_rapier = ["dep:rapier3d"]
serde = ["dep:serde", "rapier3d?/serde-serialize"]
debug_draw = []
parallel = ["rapier3d?/parallel"]
deterministic = ["rapier3d?/enhanced-determinism"]
```

推荐依赖：

```toml
[dependencies]
glam = "*"
bitflags = "*"
thiserror = "*"

rapier3d = { version = "0.32.0", optional = true }
serde = { version = "*", features = ["derive"], optional = true }
```

> `*` 仅表示文档占位。实际项目中应该锁定明确版本。

---

# 22. MVP 实现顺序

第一版不要一次吞下一整只机械鲸。先实现最小闭环：

```text
1. fixed ground
2. dynamic cube / sphere
3. fixed timestep
4. dynamic transform 写回 scene
5. raycast
6. collision started / stopped event
7. debug draw collider
```

第一阶段 API：

```rust
PhysicsWorld::new
PhysicsWorld::create_body
PhysicsWorld::create_collider_with_parent
PhysicsWorld::destroy_body
PhysicsWorld::step_fixed
PhysicsWorld::update_fixed
PhysicsWorld::body_transform
PhysicsWorld::set_next_kinematic_transform
PhysicsWorld::set_body_velocity
PhysicsWorld::add_force
PhysicsWorld::events
PhysicsWorld::drain_events
PhysicsWorld::query
PhysicsQuery::cast_ray
PhysicsQuery::overlap_shape
PhysicsWorld::debug_draw
```

第二阶段再加：

```text
shape cast
character controller
joints
snapshot
ECS command buffer
hooks
mesh collider
contact manifold
```

---

# 总结

这份 API 的边界是：

```text
Game / ECS / Scene
      │
      ▼
PhysicsWorld
      │
      ├── Body / Collider / Joint API
      ├── Command Buffer
      ├── Fixed Step
      ├── Events
      ├── Queries
      ├── Character Controller
      ├── Debug Draw
      ├── Snapshot
      └── Backend Adapter
              │
              ▼
          Rapier / Custom / Other
```

最重要的原则：

```text
不要让游戏逻辑直接依赖后端物理引擎类型。
不要让动态刚体和场景 Transform 互相抢控制权。
不要用渲染帧 delta time 直接跑物理。
不要等到 bug 满天飞才做 debug draw。
```

物理模块应该像一台封装良好的地下蒸汽机：外面只看到稳定的接口，里面的齿轮可以轰鸣，但不要把热油喷到玩法代码脸上。
