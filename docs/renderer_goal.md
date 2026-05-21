# Renderer Goal: Complete Renderer Layer

## 目标执行口径（Goal 模式专用）

本文件在 Goal 模式执行时按本节 + 两个“当前生效”段落执行，不再按下列历史段落单独判断：

- 2026-05-20 最新权威口径（从本节到“完整 renderer 层最低 Definition of Done”）
- 2026-05-20 最高优先级目标锁定（`## 2026-05-20 最高优先级目标锁定：完整实现 renderer 层`）

执行判定规则（固定）：

- 可完成性只看 `docs/renderer_goal_coverage_matrix.md`；出现任何可实现、未接线、未验证或只靠替代语义的 `Partial`/`Stub`/`Missing`/`TODO`/`unsupported-only`/`headless-only`/`mock/helper-only`/`fixed-stat-only`/`support-matrix-only` 项，goal 均未完成。
- `Complete/Implemented` 必须同时具备：public API 可达性、真实执行路径、backend-wgpu 真实路径（能接线能力）或明确适用路径、错误路径、可观测输出、focused tests、文档同步。
- 只有真实仓库外边界可记为 `External Blocked`（并且必须有 capability gate、用户可见错误、测试与后续接入点）。

Goal 模式每轮优先处理：

- 优先级一：窗口/surface/frame output/RenderGraph/resource lifecycle/backend-wgpu 的 `Partial`/`Stub`/`Missing`。
- 优先级二：其余当前可实现能力。
- 每轮结尾同步更新 `docs/renderer_goal_coverage_matrix.md` 与该文件状态，并同步补齐错误路径与验证方式。

## 2026-05-20 强制验收改写：目标是完整 renderer 产品层，不是能力登记

本 goal 的目标明确固定为：实现完整的 NeoGameEngine renderer 层产品闭环。任何能力矩阵、support query、capability report、debug/stat/capture 字段、测试 helper、示例 smoke path 或文档解释，都只能作为实现证据，不能替代真实 renderer 层实现。

从本节起，以下规则覆盖所有历史记录：

- API 文档、public facade、backend API、examples、tests、stats/capture/debug/editor 输出中出现的 renderer 能力，默认都属于必须实现范围。
- 当前仓库内可实现、可接线或可通过 backend-wgpu 表达的能力，必须实现真实执行路径；不得仅登记为 `Partial`、`Stub`、`Missing`、unsupported-only、headless-only、mock/helper-only、label-only、fixed-stat-only 或 support-matrix-only。
- `engine_renderer` 必须作为用户唯一入口提供完整 renderer 产品层能力，包括资源生命周期、scene/view、RenderGraph/RHI、standard 3D frame path、surface/window output、frame output、stats、capture、debug/editor、profiling 和错误语义。
- backend-wgpu 必须承担当前 wgpu 可表达能力的真实实现，包括 resource、bind group、pipeline、pass、render target/surface、queue/submission、readback/export、profiling marker、barrier/lifetime 和错误路径；headless/RHI 只能作为补充验证，不能替代 backend-wgpu。
- 标准 3D renderer 必须形成端到端可运行路径：scene、camera/view、mesh、material、shader、texture、sampler、light、shadow、environment/IBL、opaque/transparent、post process、tonemap、surface/window 和 durable frame output 必须可通过 public API 使用。
- 只有仓库外 SDK、平台原生能力、当前 wgpu 不暴露能力或不可稳定模拟的硬件能力，才允许标为 `External Blocked`；每个外部阻塞仍必须具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。

完成判定只能是：用户可以只通过 `engine_renderer` public API，把 NeoGameEngine renderer 作为完整 renderer 产品层使用，并获得真实 frame output、真实资源生命周期、真实 RenderGraph/RHI 执行、真实 backend-wgpu/headless 行为、真实错误语义和真实可观测输出。

因此，后续工作不得以“新增支持矩阵”“暴露 capability query”“补充文档说明”“局部测试通过”“窗口 smoke 用例能启动”或“headless 路径可验证”声明本 goal 完成。只要能力矩阵或审计中仍有任一当前仓库可实现项未达到真实执行、错误路径、观测输出、focused tests 和同步文档闭环，本 goal 必须保持未完成。

## 2026-05-20 最新权威口径：本 goal 只接受完整 renderer 层完成

本文件从此以本节作为最高优先级执行口径。若后续历史段落、阶段记录、测试记录、示例记录、能力矩阵条目或审计说明与本节存在任何可解释冲突，一律以本节为准。

本 goal 的唯一完成对象是：完整实现 NeoGameEngine renderer 层。它不能被降级为 API 文档局部特性、某组测试、某个窗口 smoke 用例、某条 headless/RHI 路径、某个 backend-wgpu 子路径、某些 stats/capture/debug 字段、某批 feature gate 或某个 RenderGraph 显式导入导出切片。

完整 renderer 层必须作为产品层闭环交付，至少同时满足以下范围：

- Public facade：`engine_renderer` 对外 API 覆盖初始化、配置、capability、feature gate、资源创建/更新/销毁/查询、scene/view、frame lifecycle、RenderGraph、frame output、stats、capture、debug/editor、profiling 和错误语义。
- Resource lifecycle：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 等资源具备真实 lifecycle、handle generation、stale/destroyed/missing 错误、upload/readback、residency/streaming、delayed destroy、stats/capture/debug 可观测性。
- RenderGraph/RHI：custom pass、standard pass、resource import/export、public output promotion、read/write usage、dependency、barrier/lifetime、queue/submission、pipeline/cache、execution stats、extension failure behavior 和 backend 可观测输出形成真实可执行路径。
- Backend：backend-wgpu 对当前 wgpu 可表达且当前仓库架构可接线的能力必须有真实 resource、bind group、pipeline、pass、render target/surface、queue/submission、readback/export、profiling marker 和错误路径；headless/RHI 只能作为补充验证，不能替代 backend-wgpu 完成度。
- Standard 3D renderer：scene、camera/view、mesh、material、shader、texture、sampler、light、shadow、environment、opaque/transparent、post process、tonemap、surface/window、durable frame output 和 public observability 必须形成端到端可运行路径。
- Tooling and delivery：FrameStats、FrameCapture、graph stats、backend trace、debug/editor report、debug draw、profiling/capture hook、窗口化用例、RenderGraph 用例、headless/backend 测试、错误路径测试、能力矩阵、审计报告、API 设计文档和最终验收记录保持同一事实。

完成判定必须从 `docs/renderer_goal_coverage_matrix.md` 反推实现，而不是从已经实现的代码反推目标范围。矩阵必须覆盖 API 设计文档、public API、backend API、examples、tests、stats/capture/debug/editor 输出、本 goal 执行期间新增的能力和所有用户可见 renderer 语义。

能力状态只允许按以下规则解释：

- `Implemented`：必须同时具备 public API 可达性、真实执行路径、backend-wgpu 路径或明确适用的 headless/RHI 路径、错误路径、可观测输出、focused tests 和同步文档证据。
- `External Blocked`：只允许用于仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟的硬件能力；必须同时具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。
- `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper-only、fixed-stat-only、unsupported-only、backend-wgpu 未接线或仅文档承诺，均表示本 goal 未完成。

后续执行要求：

1. 每轮从能力矩阵选择当前仓库可实现的未完成项，不允许只补文档来缩小范围。
2. 每个实现切片必须同时补齐 public facade、真实 renderer/backend 路径、错误路径、观测输出和 focused tests。
3. 涉及窗口、surface、frame output、RenderGraph、resource lifecycle 或 backend-wgpu 的能力，必须优先闭合真实 renderer 路径。
4. 新增或变更的 public API、feature bit、stats/capture/debug 字段、example 行为、test helper 语义或 backend API，必须同步进入能力矩阵、审计报告和 API 文档。
5. 只有当用户可以只通过 `engine_renderer` public API 把 renderer 层作为完整产品层使用，并获得真实资源生命周期、真实 RenderGraph/RHI 执行、真实 backend-wgpu/headless 行为、真实标准 3D frame output、真实错误语义和真实可观测输出时，本 goal 才能关闭。

## 2026-05-20 最高优先级目标锁定：完整实现 renderer 层

本 goal 的当前唯一目标是：完整实现 NeoGameEngine renderer 层。后续任何代码、测试、示例、能力矩阵、审计报告或 API 文档更新，都必须服务于这个完整交付目标；不能再以某个局部 API、局部 backend path、局部 headless 路径、局部窗口示例、局部 frame output、局部 stats/capture 字段或某组测试通过作为 goal 完成结论。

完整 renderer 层必须作为一个产品层闭环交付，至少同时覆盖：

- `engine_renderer` public facade：初始化、配置、capability/feature gate、资源生命周期、scene/view、frame lifecycle、RenderGraph 提交、frame output、stats、capture、debug/editor、profiling 和错误语义。
- backend/RHI：backend-wgpu、headless/RHI、真实资源、bind group、pipeline、pass、render target/surface、queue/submission、resource import/export、barrier/lifetime、readback/export、profiling marker 和错误路径。
- 标准 3D renderer：scene、camera/view、mesh、buffer、texture、sampler、shader、material、pipeline、light/shadow/environment、opaque/transparent、post process、tonemap、surface/window 和 durable frame output。
- 资源系统：create/update/destroy/info/status、handle generation、stale/destroyed/missing 错误、upload/readback、residency/streaming、delayed destroy、stats/capture/debug 可观测性。
- RenderGraph：custom pass、standard pass、external resource import/export、public output promotion、read/write usage、dependency、queue、execution stats 和 extension failure behavior。
- 交付资产：窗口化用例、RenderGraph 用例、headless/backend 测试、错误路径测试、能力矩阵、审计报告、API 设计文档和最终验收记录。

完成判定必须从 `docs/renderer_goal_coverage_matrix.md` 反推实现，而不是从已经实现的代码反推目标范围。所有 public renderer 能力、API 文档承诺、feature bit、example 行为、test helper 语义、stats/capture/debug/editor 输出和 backend API，都必须进入能力矩阵并被分流为：

- `Implemented`：具备 public API 可达性、真实执行路径、错误路径、可观测输出、目标测试和同步文档证据。
- `External Blocked`：只允许用于仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力；必须同时具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。

除上述真实外部阻塞外，任何 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper-only、fixed-stat-only、unsupported-only 或 backend-wgpu 未接线项，都表示 goal 未完成。backend-wgpu 当前可表达且当前仓库架构可接线的能力必须实现真实 backend path；headless/RHI/bookkeeping 只能作为补充验证，不能替代 backend-wgpu 完成度。

后续执行要求：

1. 每轮先从能力矩阵选择当前仓库可实现的 `Partial`、`Stub` 或 `Missing` 项。
2. 实现必须同时补齐 public facade、backend/headless 或 RHI 执行路径、错误路径、观测输出和 focused tests。
3. 涉及窗口、surface、frame output、RenderGraph、resource lifecycle 或 backend-wgpu 的能力，必须优先闭合真实 renderer 路径，不允许只补文档或测试替身。
4. 每轮完成后同步更新能力矩阵、审计报告和 API 文档事实；新增 public API、feature bit、stats/capture/debug 字段、example 行为或测试语义必须立刻进入矩阵。
5. goal 只能在用户可以只通过 `engine_renderer` public API 把 renderer 层作为完整产品层使用，并获得真实资源生命周期、真实 RenderGraph/RHI 执行、真实 backend-wgpu/headless 行为、真实 frame output、真实错误语义和真实可观测输出时关闭。

## 2026-05-19 最高优先级交付合约：必须实现完整 renderer 层

本文件的最高优先级要求是：实现完整 NeoGameEngine renderer 层。若本文件后续任何段落、历史说明、阶段性总结或测试记录看起来允许只完成局部 API、局部示例、局部 RenderGraph/headless 能力、局部 stats/capture 字段或局部 backend path，则一律以后者为过程证据、以前述完整 renderer 层交付合约为准。

完整 renderer 层不能被拆成可单独关闭的子 goal。以下全部对象必须在同一套 public-facing 语义中闭合：

- `engine_renderer` public facade：配置、capability、feature gate、资源 API、scene/view、frame lifecycle、RenderGraph 执行、stats、capture、debug/editor、profiling 和错误模型。
- 资源层：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 的 create/update/destroy/info/status、handle generation、stale/destroyed/missing 错误、upload/readback、residency/streaming、delayed destroy 和观测输出。
- RenderGraph/RHI：custom pass、standard pass、resource import/export、read/write usage、dependency/barrier/lifetime、queue、pipeline/cache、backend submit、public output promotion 和执行统计。
- backend-wgpu：当前 wgpu 可表达且当前仓库架构可接线的真实 resource、bind group、pipeline、pass、target、surface/render target、submission、profiling/marker、readback/export 和错误路径。
- 标准 3D renderer：scene、camera/view、mesh/material/shader、light/shadow/environment/IBL、opaque/transparent、forward/deferred 或等价路径、post process、tonemap、frame output 和窗口化运行路径。
- tooling 与文档：FrameStats、FrameCapture、graph stats、backend trace、debug/editor report、debug draw、profiling、external capture hook、examples、tests、API 设计文档、能力矩阵和审计报告。

完成度判定必须从能力矩阵反推代码，而不是从已完成代码反推 goal。执行顺序是：

1. `docs/renderer_goal_coverage_matrix.md` 必须覆盖 API 设计文档、public API、backend API、examples、tests、stats/capture/debug/editor 输出中所有 renderer 能力。
2. 每个 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper-only、fixed-stat-only、unsupported-only 或 backend-wgpu 未接线项，必须被实现为真实路径，或被登记为真实外部阻塞。
3. 真实外部阻塞只允许用于仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力；并且必须具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。
4. 每个标为 `Implemented` 的能力必须同时具备 public API 可达性、真实执行路径、错误路径、可观测输出、目标测试和同步文档证据。
5. backend-wgpu 可表达且当前架构可接线的能力必须有 backend-wgpu 真实路径；headless/RHI/bookkeeping 只能作为补充验证，不能替代 backend-wgpu 完成度。

本 goal 只能在用户可以只通过 `engine_renderer` public API 把 renderer 层作为完整产品层使用时关闭。只要仍有任一当前仓库可实现能力未达到真实执行、错误处理、观测、测试和文档一致性闭环，本 goal 必须保持未完成。

## 2026-05-19 强制修订：完整 renderer 层是唯一完成对象

本 goal 的完成对象只能是完整 NeoGameEngine renderer 层，不能缩减为 API 文档中的若干条目、单个窗口用例、单个 backend/headless 路径、局部 RenderGraph 能力、局部统计字段或某组测试通过。

后续执行必须把以下内容视为同一验收范围：`engine_renderer` public facade、资源生命周期、RenderGraph/RHI、backend-wgpu、headless 验证、标准 3D frame path、frame output、stats、capture、debug/editor/profiling、examples、tests、API 设计文档、能力矩阵和审计报告。

任何已经公开、承诺、演示或被测试依赖的 renderer 能力，都必须纳入完整 renderer 层验收。仓库内当前可实现或可接线的能力必须具备真实执行路径、public API 可达性、错误路径、观测输出和测试证据；wgpu 可表达且当前架构可接线的能力必须具备 backend-wgpu 真实路径，headless/bookkeeping/mock/helper/固定统计/label-only 不能替代 backend-wgpu 完成度。

只要能力矩阵、审计报告、API 文档、代码、测试或示例中仍存在未解释的 `Partial`、`Stub`、`Missing`、TODO、unsupported-only、headless-only、mock/helper-only、fixed-stat-only、label-only 或 backend-wgpu 未接线项，本 goal 必须保持未完成。

外部阻塞只能用于真实仓库外边界，例如仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力。每个外部阻塞仍必须具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点，并且不能被计入 `Implemented`。

## 强制目标锁定：实现完整 renderer 层（2026-05-19）

本 goal 的交付目标已锁定为：实现完整的 NeoGameEngine renderer 层。后续所有代码、测试、示例和文档工作都必须服务于这个目标，不能再按局部 API、局部示例、局部 backend、局部 headless 验证或局部统计字段来声明完成。

完整 renderer 层是一个产品级闭环，必须同时覆盖：

- `engine_renderer` public facade 的创建、更新、销毁、查询、frame lifecycle、RenderGraph 执行、scene/view 配置、stats、capture、debug/editor、profiling 和错误语义。
- renderer 资源生命周期，包括 mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 的 generation/stale/destroyed/missing 处理、upload/readback、residency、streaming、delayed destroy、stats、capture 和 debug 可观测性。
- RenderGraph/RHI 的 custom pass、standard pass、resource import/export、read/write usage、dependency、barrier/lifetime、queue、pipeline/cache、backend submit 和 public observability。
- backend-wgpu 的真实 wgpu resource、bind group、pipeline、pass、target、surface/render target、submission、profiling/marker、错误路径和运行状态观测；headless 只能作为无设备验证替身，不能替代 backend-wgpu 完成度。
- 标准 3D renderer 的 scene/view/camera、mesh/material/shader、light/shadow/environment、opaque/transparent/deferred 或 forward 路径、post process、tonemap、frame output 和观测面。
- examples、tests、usecase、helper、API 文档、能力矩阵、审计报告、stats、capture、debug/editor 输出中已经公开、承诺、演示或依赖的全部 renderer 语义。

完成判断只能采用一个标准：用户是否可以只通过 `engine_renderer` public API，把 NeoGameEngine renderer 层作为完整 renderer 产品层使用，并获得真实 frame output、真实资源生命周期、真实 RenderGraph/RHI 执行、真实 backend-wgpu/headless 行为、真实错误语义和真实可观测输出。

只要存在任一当前仓库可实现能力仍处于 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper-only、fixed-stat-only、unsupported-only 或 backend-wgpu 未接线状态，本 goal 必须保持未完成。外部阻塞只能用于仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力，并且必须同时具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。

## 本轮修订：完整 renderer 层是唯一交付目标（2026-05-19）

本 goal 的执行目标明确调整为：实现完整的 NeoGameEngine renderer 层。它不是 API 文档特性清单的局部补齐，不是窗口 smoke demo，不是 headless/RHI 替身验证，也不是 `engine_renderer` 单 crate 的 facade 完成。

完整 renderer 层必须覆盖并闭合 public facade、资源生命周期、RenderGraph/RHI、backend-wgpu、headless 验证、标准 3D frame path、frame output、stats、capture、debug/editor report、examples、tests 和 docs。任何已经在 API 文档、public API、feature bit、backend API、example、test、helper、stats、capture 或 debug/editor 输出中出现的 renderer 能力，均自动纳入本 goal 验收范围。

后续执行必须按以下强制规则判断完成度：

- 仓库内当前可以实现或接线的能力，必须实现为真实 renderer 路径，并具备 public API 可达性、错误路径、测试、示例或可观测证据。
- wgpu 当前可以表达且现有架构可以接入的能力，必须有 backend-wgpu 真实路径；headless、bookkeeping、mock、固定统计、固定 pass label 或 helper 行为不能替代 backend-wgpu 完成度。
- 标准 3D renderer 必须形成端到端 frame 闭环：scene/view/camera、mesh/material/shader、light/environment、render target、standard passes、post process、frame output 和观测面必须能通过 public facade 使用。
- RenderGraph/RHI 必须形成端到端执行闭环：custom pass、standard pass、resource import/export、read/write usage、dependency、barrier/lifetime、queue、pipeline/cache、backend submit 和观测输出必须真实可验证。
- 资源系统必须形成完整生命周期闭环：create/update/destroy/info/status、handle generation/stale/destroyed/missing error、upload/readback、residency/streaming、stats/capture/debug 必须一致。
- 只有仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力，才允许登记为外部阻塞；外部阻塞仍必须有 capability gate、用户可见错误、测试、矩阵说明和后续接入点。
- 能力矩阵、审计报告、API 文档、代码、测试和示例必须保持同一事实；任一 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper-only、fixed-stat-only、unsupported-only 或 backend-wgpu 未接线项都会阻止 goal 完成。

因此，后续不能再用“API 文档特性已经部分实现”“局部测试通过”“某个示例能启动”“headless 路径可验证”作为 goal 完成结论。最终完成结论只能是：用户可以只通过 `engine_renderer` public API，把 NeoGameEngine renderer 层作为完整 renderer 产品层使用，并获得真实 frame output、真实资源生命周期、真实 backend-wgpu/headless/RHI 行为、真实错误语义和真实可观测输出。

## 当前执行指令（2026-05-19）

本 goal 已正式收敛为一个不可拆分的交付目标：实现完整的 NeoGameEngine renderer 层。

## 唯一权威口径：完整 renderer 层，不是特性子集（2026-05-19）

本 goal 的唯一完成对象是完整的 renderer 层产品交付。任何局部实现、局部测试、局部示例、能力矩阵整理、API facade 补齐、backend path 接线或外部阻塞登记，都只能作为过程证据，不能替代完整 renderer 层完成结论。

完整 renderer 层必须同时覆盖并闭合以下全部范围：

- `engine_renderer` public facade 暴露的全部能力和错误语义。
- `render_wgpu` 中当前 wgpu 可表达、当前仓库架构可接线的真实后端路径。
- headless/RHI、`engine_render`、`Graphics` 中已经存在或被 facade 依赖的 renderer 行为。
- `docs/rust_3d_renderer_api_design.md`、能力矩阵、审计报告、examples、tests、usecase、helper、stats、capture、debug/editor 输出中已经承诺、演示或依赖的全部 renderer 语义。
- 本 goal 执行期间新增的任何 public API、feature bit、backend path、example 行为、统计字段、capture 字段、debug/editor 字段、测试语义和文档承诺。

完成声明必须满足同一条判断：用户是否可以只通过 `engine_renderer` public API，把 NeoGameEngine renderer 层作为完整 renderer 产品层使用，并获得真实资源生命周期、真实 RenderGraph/RHI 执行、真实 backend-wgpu/headless 行为、真实 frame output、真实错误语义和真实可观测输出。

只要存在以下任一情况，本 goal 必须保持未完成：

- 任一当前仓库可实现能力仍停留在 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only、mock/helper、固定统计、空 hook、unsupported-only gate 或 backend-wgpu 未接线状态。
- 任一 public facade 能力无法触达真实 renderer 语义，或只能通过内部 helper、测试替身、手写 debug 输出、固定 pass label 证明。
- 任一 `RendererFeature`、stats、capture、debug/editor 字段、example 或测试所表达的能力没有真实代码路径、错误路径、测试覆盖和可观测输出。
- backend-wgpu 对 wgpu 可表达、当前仓库架构可接线的能力没有真实 resource、bind group、pipeline、pass、target、submission、surface/render target、profiling/marker 或错误路径。
- API 文档、能力矩阵、审计报告、代码实现、测试断言和示例行为之间仍存在完成口径不一致。

外部阻塞只能用于记录真实边界，不能用于缩小本 goal 范围或宣布完整完成。只有依赖仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力的项，才允许登记为外部阻塞；并且必须同时具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。外部阻塞项仍必须在最终结论中显式列出，不能被计入 `Implemented`。

## 强制范围修订：必须实现完整 renderer 层（2026-05-19）

本文件的 goal 不是“补齐若干 API 文档特性”，也不是“把当前容易接线的 renderer 子集做完”。本 goal 的强制范围是完整 renderer 层产品交付。

以下输入全部是本 goal 的验收范围，不能被单独裁剪：

- `docs/rust_3d_renderer_api_design.md` 中定义或承诺的 renderer 层能力。
- 当前 `engine_renderer` public facade 已经暴露的 API、feature bit、stats、capture、debug/editor 输出和错误类型。
- 当前 `render_wgpu`、headless/RHI、`engine_render`、`Graphics` 中已经存在或已经被 facade 依赖的 renderer 行为。
- 当前 examples、tests、usecase、helper 中已经演示或依赖的 renderer 语义。
- 本 goal 执行期间新增的任何 public API、backend path、example 行为、统计字段、capture 字段、debug/editor 字段和文档承诺。

完整 renderer 层要求把上述输入全部闭合到可运行、可验证、可观测的实现状态。任何能力如果仍只有 facade 形状、headless bookkeeping、label-only pass、固定统计、mock/helper 行为、空 hook、TODO、unsupported-only gate、文档未来描述或 backend-wgpu 未接线路径，都必须在矩阵中保留为未完成项，不能作为 goal 完成依据。

`当前仓库可实现` 不能解释为缩小范围。它只用于区分执行方式：

- 仓库内可以通过 `engine_renderer`、`render_wgpu`、headless/RHI、`engine_render` 或 `Graphics` 闭合的能力，必须实现为真实路径，并具备错误路径、测试、示例或观测证据。
- 依赖仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力的能力，不能标为 `Implemented`；只能标为明确外部阻塞，并且必须同时具备 capability gate、用户可见错误、测试、能力矩阵说明和后续接入点。
- 外部阻塞不会让对应能力变成已实现；它只说明为什么该项不能在当前仓库内完成。完整 renderer 层最终结论必须显式列出这些阻塞和用户可见行为。

因此，后续执行必须以“完整 renderer 层缺口清零”为目标：能力矩阵、审计报告、API 文档、代码、测试和示例中只要还有未解释的 `Partial`、`Stub`、`Missing`、TODO、label-only、headless-only 或 backend-wgpu 未接线行为，本 goal 就保持未完成。

后续所有实现、测试、示例、审计和文档更新都必须服务于同一个完成口径：用户可以只通过 `engine_renderer` public facade，把 renderer 层作为完整产品层使用，并获得真实的 backend-wgpu/headless/RHI 执行行为、真实资源生命周期、真实 frame output、真实错误语义和真实可观测输出。

以下事项不能再作为 goal 完成口径：

- 只实现 API 文档中的若干条目。
- 只补 facade 形状、feature bit、stats 字段、debug 输出或能力矩阵。
- 只让 headless、mock、label-only pass、固定统计或测试 helper 通过。
- 只让某个窗口示例、某个 crate、某个 backend pass 或某组局部测试通过。
- 把当前仓库可实现但尚未接 backend-wgpu/public facade/tests/examples/docs 的能力标成外部阻塞。

完整 renderer 层必须形成以下端到端闭环：

- Public facade 闭环：创建、更新、销毁、查询、scene/view 配置、frame lifecycle、RenderGraph 执行、stats/capture/debug/profiling 查询都必须通过公开 API 可用。
- Resource 闭环：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 必须具备 lifecycle、generation/stale handle、upload/update、destroy、residency、stats/capture/debug 语义。
- Backend 闭环：backend-wgpu 必须承载当前 wgpu 可表达、当前仓库架构可接线的真实 resource、bind group、pipeline、pass、target、submission、surface/render target、profiling/marker 和错误路径；headless 只能作为无设备验证替身。
- RenderGraph/RHI 闭环：custom pass、standard pass、resource import/export、read/write usage、dependency、barrier/lifetime、queue、pipeline/cache、backend submit 必须真实可执行、可验证、可观测。
- Standard 3D renderer 闭环：scene、camera/view、mesh/material/shader、lighting/shadow/environment、opaque/transparent/deferred 或 forward 路径、post process、tonemap/frame output 必须形成窗口化可运行路径。
- Observability 闭环：FrameStats、FrameCapture、graph stats、backend trace、debug/editor report、debug draw、profiling、external capture hook 和 unsupported reporting 必须反映真实运行状态。
- Examples/tests/docs 闭环：公开示例必须走 public facade；测试必须覆盖成功路径、错误路径、capability/unsupported 路径、resource lifetime 和观测面；API 文档、能力矩阵、审计报告、代码、测试和示例必须保持同一事实。

本 goal 只能在全部 renderer 层能力完成分流后关闭：仓库内可实现能力必须从 `Partial`、`Stub`、`Missing` 收敛到有真实执行路径、有错误路径、有测试、有示例或观测证据、有一致文档的 `Implemented`；仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力只能保留为明确外部阻塞，并且必须同时具备 capability gate、用户可见错误、测试、矩阵说明和后续接入点。未分流、未解释或只是暂未实现的能力，一律阻止 goal 完成。

## 目标

本 goal 的唯一交付对象是：完整、可运行、可验证、可观测的 NeoGameEngine renderer 层。

任何阶段性工作，包括补 API facade、补示例、补统计字段、补 headless 替身、补 capability gate、补能力矩阵或补审计报告，都只能作为中间证据，不能作为 goal 完成口径。

## Goal 锁定：完整 renderer 层交付（2026-05-19）

本 goal 已调整为“实现完整 renderer 层”，不是“实现 API 文档中的若干特性”、不是“让某个示例能跑”、也不是“让某个 crate 的局部测试通过”。

完整 renderer 层必须按产品交付口径验收：

- Public facade、backend-wgpu、headless/RHI、RenderGraph、资源系统、标准 3D 管线、frame/debug/capture/profiling、examples、tests、docs 必须形成同一套真实语义。
- 用户只通过 `engine_renderer` public API 就能完成 renderer 层承诺的创建、更新、销毁、查询、提交 frame、执行 graph、配置 scene/view、读取 stats/capture/debug 的端到端流程。
- backend-wgpu 必须作为真实图形后端承载所有当前 wgpu 能表达、当前仓库架构能接线的能力；headless 只能作为无设备验证替身，不能替代 backend-wgpu 完成度。
- 所有已经公开、已经文档承诺、已经示例演示、已经测试依赖或已经 stats/capture/debug 暴露的 renderer 能力，都自动纳入本 goal，不允许因为不在某个局部清单中就排除。
- `docs/rust_3d_renderer_api_design.md` 是最低能力基线，不是范围上限；代码中已经存在的 public API、feature bit、backend API、example 行为和测试 helper 同样是验收输入。
- 任一能力只有 facade 形状、固定 label、固定统计、headless bookkeeping、mock 行为、空 hook、TODO 或未接 backend-wgpu 路径，都不能标为完成。
- 任一当前仓库可实现能力仍为 `Partial`、`Stub`、`Missing`、unsupported-only 或文档未来项时，本 goal 必须保持未完成。
- 只有仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力、硬件不可稳定模拟能力这类真实外部边界，才允许保留为外部阻塞；并且必须有 capability gate、用户可见错误、测试、矩阵说明和后续接入点。

完整 renderer 层的必交付能力包包括：

- Renderer facade：配置、capability、feature gate、surface/headless/backend 初始化、frame lifecycle、error model。
- Resource layer：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 的 lifecycle、update、destroy、stale handle、generation、residency、upload、stats/capture/debug。
- Scene/View layer：retained scene、object transform/material/visibility/layer/bounds、camera/view/render target/viewport/scissor、culling/sorting/batching/instancing。
- Material/Shader/Pipeline layer：standard material、material template、shader reflection、variant、hot reload 语义、pipeline key/cache、render state。
- Standard 3D renderer：depth prepass、gbuffer/deferred 或明确 forward 等价路径、shadow、opaque、transparent、environment/IBL、post process、tonemap、frame output。
- RenderGraph/RHI：custom pass、standard pass、resource import/export、read/write usage、dependency、barrier/lifetime、queue、backend submit、pipeline/cache。
- Backend-wgpu：真实 wgpu resource、bind group、pipeline、pass、target、submission、surface/render target、profiling/timestamp/marker、错误路径和可观测 stats。
- Tooling/observability：FrameStats、FrameCapture、graph stats、backend trace、debug/editor report、debug draw、profiling、external capture/RenderDoc hook、unsupported reporting。
- Examples/tests/docs：窗口化标准 3D 用例、RenderGraph custom pass 用例、资源生命周期用例、material/shader/pipeline 用例、capture/debug/stats 用例；并且全部与文档矩阵和测试事实一致。

因此，之后任何“完成”判断必须回答同一个问题：用户是否已经可以把 NeoGameEngine renderer 层作为完整 renderer 产品层使用。只要答案是否定的，本 goal 就不能关闭。

## 最新硬性口径（2026-05-19）

- `docs/rust_3d_renderer_api_design.md` 是 renderer 层最低验收基线和能力主索引，不是范围上限。
- 本 goal 不接受按单个子系统、单个 crate、单个 backend、单个示例或单个能力项声明完成；只有完整 renderer 层端到端闭环才允许声明完成。
- “完整 renderer 层”必须同时覆盖 public facade、资源生命周期、RenderGraph/RHI、backend-wgpu/headless、标准 3D 管线、frame observability、debug/editor/profiling/capture、examples、tests 和 docs。
- “完整 renderer 层”不是 capability 列表完整、API 形状完整或测试样例可跑；它要求每个承诺能力都有真实 renderer 行为、真实状态变化、真实错误语义、真实观测输出和对应验收证据。
- 当前 public renderer API、backend API、examples、tests、helper、stats、capture、debug/editor 输出中已经暴露或承诺的 renderer 能力，都必须反向纳入本 goal。
- 能力矩阵、审计报告、API 文档、代码、测试或示例中的任何 `Partial`、`Stub`、`Missing`、TODO、空 hook、label-only pass、假统计、headless-only 语义或 backend-wgpu 未接线行为，都代表本 goal 未完成。
- 凡是当前 `engine_renderer`、`render_wgpu`、`engine_render`、`Graphics`、backend-wgpu、headless/RHI 架构可以闭合的能力，最终必须实现为真实执行路径，并具备错误路径、测试和可观测输出。
- 只有确实依赖仓库外 SDK、平台原生后端、当前 wgpu 不暴露能力或不可稳定模拟硬件能力的项，才允许作为外部阻塞保留。
- 外部阻塞也必须有 capability gate、用户可见错误、测试、文档矩阵说明和后续接入点。
- 完成声明必须同时满足代码路径、错误路径、测试、示例、frame stats/capture/debug 可观测性、能力矩阵和 API 文档一致性。

## 完整 renderer 层强制解释（2026-05-19）

本 goal 的实现目标必须按“renderer 层产品化交付”理解，而不是按“补齐文档条目”理解。

- 本 goal 是全量交付门禁，不是分阶段里程碑清单；任意单轮实现、单个 crate 修复、单个 backend pass 接线、单个示例可运行或单个测试集通过，都只能证明局部进度，不能降低最终范围。
- “完整 renderer 层”必须覆盖当前仓库已经公开、文档已经承诺、示例已经演示、测试已经依赖、stats/capture/debug 已经暴露的全部 renderer 能力；不能只选择 API 文档中的稳定子集，也不能只选择当前最容易实现的子系统。
- 完整性判断以用户可通过 public facade 真实使用为准：如果能力只能通过内部 helper、headless bookkeeping、测试替身、固定 label、固定统计、手写 debug 输出或未接 backend-wgpu 的路径观察到，则该能力仍未完成。
- backend-wgpu 是本 goal 的主要真实后端验收对象；凡是 wgpu 能表达且当前仓库架构能接线的 renderer 能力，最终必须在 backend-wgpu 中形成真实资源、真实 pass、真实 pipeline、真实 submission、真实错误路径和真实观测输出。
- headless/RHI 结果只能作为语义验证和无设备测试证据，不能替代窗口化/surface/backend-wgpu 路径完成度，也不能作为标准 3D renderer 完整性的最终证明。
- 标准 3D renderer 必须按完整帧管线交付：资源上传、scene/view/camera、material/shader/pipeline、lighting/shadow/environment、opaque/transparent/deferred 或 forward 路径、post process、frame output、stats/capture/debug 必须闭合；缺任一可实现环节都不能声明完整。
- RenderGraph/RHI 必须从 facade 到 backend submit 闭合；只存在 graph 节点、pass label、依赖记录或 headless 执行，不等于 backend-wgpu graph 能力完成。
- 能力矩阵中的每个 `Partial`、`Stub`、`Missing` 都是阻止本 goal 完成的硬门禁；只有被证明为仓库外 SDK、平台原生后端、wgpu 暂不暴露或不可稳定模拟硬件能力的项，才允许作为外部阻塞保留。
- 若某项被保留为外部阻塞，必须同时具备：明确不可实现原因、capability gate、用户可见 unsupported error、测试覆盖、文档矩阵说明、后续接入点；否则仍按未完成处理。
- 每轮实现后必须更新文档事实，不能让 API 文档、能力矩阵、审计报告、代码、测试或示例之间出现不同完成口径。

- Public API 必须完整：用户只通过 `engine_renderer` public facade 就能创建、更新、销毁、查询和渲染资源，并拿到一致的错误、状态、统计、capture 和 debug 输出。
- Backend-wgpu 必须完整：凡是 wgpu 可以表达的资源、pipeline、pass、graph、upload、profiling、timestamp、surface、render target 和 submission 行为，都必须走真实 backend 路径；不能用 headless、mock、bookkeeping 或 label-only pass 替代。
- Headless 必须定位清楚：headless 只能用于无设备测试、RHI 语义验证和 CI 替身；headless 通过不代表 backend-wgpu 完整。
- RenderGraph/RHI 必须完整：custom pass、standard pass、resource import/export、read/write usage、barrier/lifetime、queue、dependency、pipeline/cache 和 backend submit 必须能被验证和观测。
- 标准 3D 管线必须完整：scene、camera/view、mesh/material/shader、light/shadow/environment、animation/LOD、render targets、standard passes、post process、frame output 必须形成端到端可运行路径；只输出 pass 名称、计数或固定 stats 不算实现。
- Resource layer 必须完整：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 等资源必须覆盖 lifecycle、generation/stale handle、destroy queued/destroyed、upload/update、residency/streaming、delayed destroy、stats/capture/debug。
- Tooling 必须完整：profiling、GPU markers、frame capture、RenderDoc/external debugger hook、debug draw、editor report、feature gate、unsupported error 必须和真实运行状态一致。
- Examples 必须完整：公开示例必须走 public facade，覆盖窗口化标准 3D renderer、RenderGraph custom pass、资源生命周期、material/shader/pipeline、capture/debug/stats；示例不能绕过 facade 伪造 renderer 能力。
- Tests 必须完整：每个能力至少覆盖成功路径、错误路径、unsupported/capability path、resource lifetime path、frame stats/capture/debug 观测面；最终必须运行完整相关 crate 验收。
- Docs 必须完整：API 文档、能力矩阵、审计报告、代码实现、测试断言和示例行为必须保持同一事实；任何一处仍写未来能力、占位能力或未接线能力，goal 未完成。

Capability gate 只能用于表达真实能力边界，不能替代实现。若某能力在 API 文档中被承诺且当前仓库可实现，则必须实现；若不可实现，必须证明其依赖仓库外 SDK、平台原生后端、当前 wgpu 缺口或不可稳定模拟硬件能力，并同时提供 gate、错误路径、测试、文档说明和后续接入点。

## 范围

主要入口：`Render/engine_renderer`

根据完整 renderer 层闭环需要，以下目录同属本 goal 范围：

- `Render/render_wgpu`
- `Render/engine_render`
- `Graphics`
- 相关 examples、tests、docs

不要无关扩散到游戏逻辑、ECS 主世界、物理、窗口事件循环等非 renderer 边界。若 API 文档中的能力跨越多个 crate，必须补齐 renderer 层所需的最小闭环，而不是只在 `engine_renderer` 中保留 facade 形状。

## 完整 renderer 层最低 Definition of Done

完整 renderer 层只有在以下条件同时满足时才算完成：

- 全量范围闭环：`docs/rust_3d_renderer_api_design.md`、当前 public API、backend API、feature bits、examples、tests、stats/capture/debug/editor 输出中承诺或暴露的 renderer 能力全部进入矩阵，并且除明确外部阻塞外全部达到 `Implemented`。
- 真实执行闭环：每个 `Implemented` 能力都必须至少有一条 public facade 到 backend/headless 或 backend-wgpu 的真实执行路径；对于 wgpu 可表达能力，必须有 backend-wgpu 真实路径，不能只依赖 headless。
- 验收证据闭环：每个 `Implemented` 能力必须同时有代码路径、错误路径、测试、可观测输出和文档证据；缺任一项都不能标为 `Implemented`。
- 文档闭环：API 设计文档、能力矩阵、审计报告和最终总结对每个 renderer 能力项给出同一状态、同一证据和同一剩余风险。
- Facade 闭环：用户通过 public renderer API 可以完成 renderer 层承诺的资源创建、更新、销毁、查询、frame 提交、graph 执行、scene/view 配置和调试输出，不需要绕过 facade 调 backend 私有接口。
- Backend 闭环：backend-wgpu 对所有 wgpu 可表达能力都有真实执行路径；headless 只作为无设备/测试替身，不能替代 wgpu 路径完成度。
- Resource 闭环：所有 renderer 资源具备 handle generation、stale/destroyed/missing 错误、生命周期、upload/update、delayed destroy、stats/capture/debug 可见状态。
- Graph/RHI 闭环：RenderGraph 和 RHI 必须能表达并验证 pass 顺序、queue、read/write、resource lifetime、barrier、pipeline/cache、custom pass 和 backend 提交行为。
- Frame 闭环：frame output、FrameStats、FrameCapture、graph stats、backend trace、debug/editor report 必须能观察到真实 renderer 行为，而不是空字段、固定值或 label-only 数据。
- 标准 3D 闭环：scene、camera/view、mesh、material、shader、light、pipeline、render target、standard passes、post process 和 output 形成端到端可运行路径。
- Tooling 闭环：profiling、capture、debug draw、editor hooks、feature gates、unsupported errors 和 capability reporting 与实际实现一致。
- Example 闭环：公开示例必须走 public renderer API，覆盖窗口化标准 3D renderer 用例和 RenderGraph custom pass 用例；示例构建或启动失败则 goal 未完成。
- Test 闭环：每个能力项至少有相关正常路径、错误路径、capability/unsupported 路径和观测面测试；整个 goal 完成前必须通过相关 crate 的完整验收命令。

## 完整 renderer 层交付合约

- Facade 合约：所有 public renderer API 必须能触达真实语义，不能只在内部 helper、示例私有代码或 headless bookkeeping 中成立。
- 资源合约：mesh、buffer、texture、sampler、shader、material、pipeline、scene、view、render target 等资源必须覆盖 create/update/destroy/info/status、stale handle、generation mismatch、destroyed resource、错误返回和可观测状态。
- 后端合约：backend-wgpu 必须承载 wgpu 能表达的真实执行路径；headless 只能作为测试/无窗口替身，不能替代 backend-wgpu 完整实现。
- RenderGraph/RHI 合约：custom pass、standard pass、resource read/write、queue、dependency、barrier/lifetime、pipeline/cache、upload/streaming/delayed destroy 必须形成可执行和可验证语义。
- Frame 合约：每个 frame 级能力必须能通过 frame output、FrameStats、FrameCapture、graph stats、backend trace、resource state 或 debug/editor report 观察到。
- 标准 3D 管线合约：scene/view/camera/light/material/shader/pipeline/pass/output 必须形成端到端路径；pass 名称、空统计或 label-only pass 不算实现。
- 工具链合约：profiling、capture、debug draw、editor hook、feature gate、错误处理必须和真实 renderer 状态一致，不能报告假数据或未来行为。
- 示例合约：面向用户的 renderer 能力必须至少有公开 API 用例覆盖；窗口化和 RenderGraph custom pass 示例不能绕过 renderer facade 伪造完成度。
- 文档合约：API 文档、能力矩阵、审计报告、代码实现、测试断言、示例行为必须一致；任意一处仍是未来描述、占位语义或未接线能力，goal 未完成。

## 必须先做的事情

1. 建立完整文档能力矩阵，覆盖 API 文档所有 renderer 章节。
2. 对每一项标注 `Implemented`、`Partial`、`Stub` 或 `Missing`。
3. 明确列出剩余缺口，并按 renderer 完整性优先级排序。
4. 每轮实现前，从能力矩阵选择一个可以被代码和测试闭合的 `Partial`、`Stub` 或 `Missing` 项。
5. 每轮实现后，更新能力矩阵中的证据、剩余缺口和实际验证命令。
6. 如果发现 API 文档中有能力未进入矩阵，必须先补矩阵项并按 `Missing` 处理。
7. 如果发现矩阵把当前架构可实现能力标成外部阻塞，必须修正矩阵并实现。
8. 如果发现代码里新增了 public API、feature bit、stats 字段、capture 字段、debug/editor 字段、example 行为或测试 helper，必须同步补进矩阵并给出完成状态。
9. 如果某能力只有 facade bookkeeping、headless 语义、label-only graph pass、固定统计或 unsupported gate，不能标为 `Implemented`。
10. 如果某能力可以通过 backend-wgpu、现有 RHI、现有资源系统或现有 graph 架构实现，不能保留为外部阻塞。

能力矩阵必须覆盖：

- 顶层 Renderer API
- Handle / resource manager
- Mesh / Buffer API
- Texture / Sampler API
- Shader / reflection / hot reload / variants
- Material / material template / render state
- Scene / object / retained mode update
- Camera / View / render target
- Light / shadow / environment / IBL
- Animation / skinning / morph / LOD
- Frame API / frame stats / frame capture
- RenderGraph API / custom pass
- 标准 3D RenderGraph
- RHI API / backend abstraction
- Pipeline / pipeline key / cache
- GPU memory / upload / streaming / delayed destroy
- ECS extract integration boundary
- Debug draw / editor API
- Profiling / capture / stats
- Error handling
- Feature flags / stability tiers
- Complete usage examples

## 实现优先级

1. Renderer facade 完整性：`RendererConfig`、`RendererCaps`、feature gate、surface/headless/backend 初始化、resource create/update/destroy/info/status、frame begin/finish 生命周期。
2. Resource 与 upload 完整性：mesh、buffer、texture、sampler、staging/upload、streaming priority、residency、delayed destroy、stats。
3. Scene 与 View 完整性：retained scene、object transform/material/visibility/layer/bounds、camera/view/render target/viewport/scissor、culling/sorting/batching/instancing。
4. Material / Shader / Pipeline 完整性：standard material、material template、shader reflection、shader variants、feature set、pipeline key/cache、hot reload 语义。
5. 标准 3D 管线完整性：forward/deferred、depth prepass、shadow passes、transparent pass、post process、tonemap、bloom、TAA、environment、IBL。
6. RenderGraph / RHI 完整性：custom render/compute pass、graph resource lifetime、pass dependency/barrier/queue、RHI command abstraction、backend-wgpu 真实执行路径。
7. 高级能力完整性：GPU culling、indirect draw、multi-draw indirect、async compute、bindless textures、virtual texturing、meshlet/mesh shader、ray tracing、variable rate shading。
8. 工具链与可观测性：frame stats、GPU markers/profiling、frame capture、debug draw/editor hooks、RenderDoc integration hooks。
9. 示例与验收：renderer facade usecase、窗口化标准 3D renderer usecase、RenderGraph custom pass usecase；示例必须走公开 API，不允许绕过 renderer facade 证明 facade 能力。

## 实现要求

- 以真实 renderer 层语义为准，不接受纯占位实现。
- 不要为了通过测试写只满足断言的假实现；测试必须观察真实状态变化、资源依赖、frame output 或后端行为。
- 不要删除已有测试意图；应在现有测试基础上增强。
- 不要用 panic 处理 public API 错误；除非是不变量破坏，public API 应返回 `Result`。
- 所有 feature-gated 能力必须同时覆盖支持路径与不支持路径。
- 所有资源 handle 必须覆盖 stale handle、generation mismatch、destroyed resource 等错误路径。
- 所有 frame 级功能必须能通过 FrameStats、FrameCapture、graph stats 或 backend trace 观察到。
- 所有 custom pass / graph extension 必须验证 resource dependency、queue、read/write usage 和执行顺序。
- backend 缺失不能作为最终完成理由；只能作为矩阵中明确登记的外部阻塞项。
- 对已经声明支持的 feature，必须存在可执行路径；否则应改为不支持并返回明确错误。
- 对已经声明存在的标准 3D pass，必须至少具备 renderer 层可观测的资源读写、依赖、输出或 stats 语义；不能只有 label。
- 对 backend-wgpu 可以实现的功能，优先补真实 wgpu 路径；headless/bookkeeping 语义只能作为测试替身，不能代表完整 backend 实现。
- 对高级 GPU 特性，如果当前 wgpu/硬件能力不足，必须通过 capability gate、错误路径、测试和文档矩阵同时闭合。

## 测试要求

- 每完成一个能力项，必须补测试。
- 每个能力至少覆盖正常路径、capability gate、unsupported path、stale/missing resource、graph dependency/queue/resource usage、frame stats/frame output/capture 观测面。
- 必须运行改动涉及的精确测试。
- 完成整个 goal 前必须运行并通过 `cargo test -p engine_renderer`。
- 如果涉及 `render_wgpu`、`engine_render` 或 `Graphics`，必须运行对应 crate 的最小相关测试或构建验证。
- 示例验收至少包含构建验证；窗口化示例如本机 GUI 可用，还应实际启动一次并说明结果。
- 最终输出必须包含实际执行过的测试命令清单和结果摘要。

## 完成门禁

本 goal 未完成，如果存在任一情况：

- 仍有任何当前仓库可实现的 renderer 能力没有接入 public facade、backend-wgpu/headless/RHI 执行路径、错误路径、测试和 frame/debug/capture 可观测输出。
- 仍有任何 API 文档、能力矩阵、审计报告、示例、测试或代码注释把 renderer 能力描述为未来工作、占位能力、模拟能力、label-only 能力或未接线能力。
- 仍有任何 `Implemented` 项的证据只能证明 facade 形状、headless 替身、固定统计、固定 pass label、mock 行为或内部 helper 行为，而不能证明真实 renderer 行为。
- 只完成了 renderer 层中的某一个子系统，例如只完成资源、只完成 backend tombstone、只完成 frame stats、只完成示例或只完成文档。
- renderer 层没有完整覆盖 API 文档、当前 public renderer API、backend-wgpu/headless 执行路径、示例和测试中已经声明或依赖的 renderer 能力。
- renderer 层还没有达到公开 API、内部 RHI、backend-wgpu/headless、资源生命周期、RenderGraph、标准 3D pass、frame stats/capture/trace、debug/editor、examples 和 tests 的完整闭环。
- `docs/rust_3d_renderer_api_design.md` 中任一 renderer 层能力仍没有代码路径、测试覆盖和可观测输出。
- 当前代码库中已经存在的 renderer public API、backend API、example 或测试 helper 语义没有进入能力矩阵、实现验证和文档说明。
- 能力矩阵中存在未解释的 `Partial`、`Stub` 或 `Missing`。
- 某能力可以由当前 `engine_renderer`、`render_wgpu`、`engine_render`、`Graphics` 或 headless/RHI 架构实现，却被登记为外部阻塞。
- backend-wgpu 路径没有承载 API 文档中可由 wgpu 表达的真实语义。
- 示例、测试或文档仍通过绕过 renderer facade 的方式证明 facade 能力。
- frame stats、capture、trace、resource state、pipeline cache 或 graph stats 仍只报告假数据、空数据或 label-only 数据。
- 任一 public `RendererFeature` 声明 supported，但对应能力只存在 facade 形状、测试替身、headless 行为、统计镜像或未接 backend-wgpu 的语义。
- 任一 public `RendererFeature` 声明 unsupported，但实际只是当前未排期、未接线或未写测试，而不是仓库外 SDK、平台后端、wgpu 缺口或硬件不可稳定模拟造成的真实外部阻塞。
- 任一窗口化、surface、standard 3D 或 RenderGraph 示例不能通过 public renderer facade 构建真实 frame output。
- 任一 backend-wgpu 可实现路径仍依赖 headless 测试结果、固定 pass label、固定 stats 或手写 debug output 证明完成度。

最终完成时：

- 最终声明只能针对“完整 renderer 层”，不能针对某个已闭合的局部能力替代本 goal 完成结论。
- 能力矩阵中不允许存在 `Stub` 或 `Missing`。
- `Partial` 只能用于明确超出当前仓库可实现边界的外部依赖项，例如真实 RenderDoc SDK 集成、平台原生 Vulkan/Metal/D3D12 后端、当前 wgpu 不暴露的 backend 查询能力、不可稳定模拟的设备级能力。
- 每个保留的 `Partial` 必须说明不可完成原因、当前显式 gate、用户可观察行为和后续落地入口。
- API 文档、能力矩阵、代码实现、测试断言和示例行为必须互相一致。

## 最终输出要求

最终请给出：

1. 完整文档能力矩阵。
2. `Implemented`、`Partial`、`Stub`、`Missing` 清单。
3. 本轮实际实现的能力项。
4. 修改的核心文件。
5. 新增/修改的测试。
6. 实际执行过的测试命令。
7. 仍未完成的能力及阻塞原因。
8. 和 API 文档仍不一致的地方。
9. 下一轮建议。

## 完成禁区

- 不能只因为测试通过就宣称完整，必须同时满足 API 文档矩阵。
- 不能只因为某能力有 capability gate 就宣称实现，除非 API 文档允许该能力作为可选特性且不支持路径完整。
- 不能把 `render_wgpu`、RHI、pipeline cache、upload、capture、profiling 中的真实执行缺口掩盖为 facade 完成。
- 不能忽略 `Stub` / `Missing` 项提交最终结论。
- 不能把“目前没有后端”作为 renderer 层永久完成理由；必须要么实现当前可用 backend-wgpu/headless 闭环，要么在矩阵中明确外部阻塞。

## 执行风格

- 直接动手，不要停在分析。
- 先读必要文件，再连续实现。
- 先闭合 renderer 基础语义，再推进高级图形能力。
- 优先小步可验证闭环，不做无测试的大面积重构。
- 如果发现已有实现和 API 文档冲突，以 API 文档为准修正。

## Renderer layer completion requirement (2026-05-19 18:00:29 +08:00)

The renderer goal is full renderer-layer implementation, not a smoke demo or partial preview. Completion requires every public renderer feature described by docs/rust_3d_renderer_api_design.md to have production-facing behavior, deterministic observability, and targeted validation.

Required completion bar:
- Render lifecycle: instance/device/swapchain or headless setup, frame begin/end, resize, device-loss handling, capture, stats, debug reports.
- Public frame outputs: durable public output resources, render-target writeback, texture-view subresource handling, capture/debug/stat provenance.
- Render graph: node/resource scheduling, explicit imports/exports, external resource promotion, pass observability, and extension output visibility.
- Scene rendering: mesh/material/texture/sampler resources, transforms, visibility, layers, cameras, lights, environment, exposure, and LOD-driven resource selection.
- Render paths and features: clear, forward/forward-plus/deferred/headless behavior, quality overrides, post-process feature preview/observability, and feature fallback reporting.
- Asset/resource layer: creation, update, upload/readback, lifetime, alias safety, labels, and error diagnostics.
- Extensibility: custom graph/pass hooks, exported resources, debug names, and safe behavior when extensions fail.
- Validation: each completed slice must add or update focused tests and record coverage in docs/renderer_goal_coverage_matrix.md plus residual gaps in docs/renderer_goal_audit_report.md.

A slice is not complete until its public API behavior is implemented, observable from FrameStats/debug/capture where applicable, and covered by a targeted test. The overall goal must remain open until the coverage matrix shows no renderer API feature as pending or partial without an accepted follow-up.

## Goal scope reaffirmation - complete renderer layer only (2026-05-19)

This goal remains open until the renderer layer is complete as a product layer, not merely until individual API-document features have facade coverage.

The required implementation target is the full renderer layer across public facade, resource lifecycle, RenderGraph/RHI, backend-wgpu, headless validation, standard 3D frame path, frame outputs, stats, capture, debug/editor reporting, examples, tests, and documentation. API-document feature coverage, coverage-matrix entries, audit notes, helper methods, capability gates, public metadata, or explicit-graph compatibility paths are process evidence only.

Completion still requires every repository-implementable renderer capability to have:

- public `engine_renderer` API reachability;
- real renderer behavior, not only labels, bookkeeping, mocks, fixed stats, or unsupported-only gates;
- backend-wgpu implementation when wgpu can express the behavior and the current architecture can connect it;
- headless/RHI semantics where useful for deterministic tests, without treating headless as a substitute for backend-wgpu;
- user-visible error behavior for stale, destroyed, unsupported, invalid, or capability-gated paths;
- FrameStats, FrameCapture, graph stats, debug/editor report, resource info, or other public observability where the behavior is frame/resource-visible;
- focused tests for success, failure/unsupported, lifetime, and observability paths;
- synchronized facts in the API design document, coverage matrix, audit report, examples, and code comments.

The goal must not be closed while any current repository-implementable item remains `Partial`, `Stub`, `Missing`, TODO, label-only, headless-only, mock/helper-only, fixed-stat-only, unsupported-only, or backend-wgpu-unwired. External blockers must be limited to true repository-external boundaries such as SDK integration, platform-native backends, wgpu-unexposed functionality, or hardware behavior that cannot be stably simulated; each such blocker must still have a capability gate, user-visible error, tests, matrix entry, and follow-up integration point.

## 附录：执行日志（历史与证据，非完成规则）

## 2026-05-20 renderer goal update: graph texture descriptor support query

完整 renderer 层要求继续保留为最终目标；本次补齐的是 graph texture descriptor 创建前的显式能力查询，而不是缩小范围。

- 新增 `GraphTextureDescSupport`，用于在调用 graph texture 创建 API 前报告 renderer graph 当前是否支持给定 `TextureDesc`。
- `RenderGraphBuilder::texture_desc_support(&TextureDesc)` 会返回 supported 状态、unsupported reason，以及 dimension/size/mip/samples/format 诊断字段。
- `RenderGraphBuilder::try_create_texture_from_desc` 复用同一套校验逻辑，当前只接受 native graph 已完整覆盖的 D1、D2、flattened D2Array、D3、Cube 或 CubeArray、1 mip、1 sample 纹理。
- 旧 `create_texture_from_desc` 保留兼容但已标记 deprecated；完整 renderer 层后续应继续补 native array/mip/MSAA/depth graph resource，而不是依赖旧投影行为。
- focused validation: `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed.

## 2026-05-20 execution note: cross-layer partial region export status corrected

Focused validation confirmed that public imported layered texture cross-layer partial-region export execution/readback is already implemented on both headless/RHI and backend-wgpu RGBA8 paths. The coverage matrix has been corrected so this stale `Remaining scope` item no longer blocks the goal incorrectly.

Validation:
- `cargo test -p engine_renderer cross_layer -- --nocapture` passed, 2 passed.

The complete renderer goal remains open for native graph-created multi-shape resources, readback-backed surface graph export promotion plus direct swapchain export capability gate, native MSAA graph execution, persistent backend-resident synchronization, and any other current `Partial`/`Missing` items not yet closed by real implementation plus evidence.


## 2026-05-20 execution note: graph-created D1 transient promotion

The safe graph-created texture descriptor path now supports valid D1 descriptors in addition to single-layer D2 descriptors. `try_create_texture_from_desc` records renderer descriptor metadata, and transient graph export promotion uses that metadata to create a durable public D1 texture handle instead of collapsing the result to D2.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d1_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for native graph-created MSAA, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.


## 2026-05-20 execution note: graph-created D2Array transient promotion

The safe graph-created texture descriptor path now supports flattened D2Array descriptors. RHI execution creates a flattened 2D backing texture, export metadata records the flattened RHI height, and public promotion restores the durable `TextureDimension::D2Array` descriptor and complete layer metadata.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d2_array_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for native graph-created MSAA, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.


## 2026-05-20 execution note: graph-created D3/Cube/CubeArray transient promotion

The safe graph-created texture descriptor path now supports D3, Cube, and CubeArray descriptors in addition to D1, D2, and D2Array. These resources use the current flattened RHI backing representation during graph execution and are promoted back to durable public textures with their original public dimensions and complete subresource metadata.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d3_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_cube_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_cube_array_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for graph-created MSAA textures, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.


## 2026-05-20 execution note: graph-created packed mip-chain transient promotion

The safe graph-created texture descriptor path now supports one-sample packed mip-chain transients. Headless/RHI coverage includes D1, D2, D2Array, D3, Cube, and CubeArray. Backend-wgpu coverage verifies a D2 packed mip-chain transient through the real `Renderer::execute_graph_to_resources` wgpu path. Promoted public textures retain packed mip-chain bytes and complete subresource metadata.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_promotes_graph_created_mip_chain_texture_descs -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_promotes_graph_created_mip_chain_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for custom MSAA resolve validation evidence, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.


## 2026-05-20 execution note: graph-created MSAA transient resolve promotion

The graph-created texture path now accepts D2 one-mip MSAA descriptors, RHI texture descriptors carry sample count, backend-wgpu creates native multisampled textures, and RGBA8 MSAA exports resolve through a temporary single-sample texture before public promotion. Promoted public textures preserve the original sample count in export and texture metadata.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_promotes_graph_created_msaa_texture_desc -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer msaa -- --nocapture` passed, 7 passed.

The complete renderer goal remains open for custom MSAA resolve validation evidence, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.

## 2026-05-20 execution note: RHI texture sample-count observability

`RhiDevice` now exposes `texture_samples`, and both headless and backend-wgpu RHI devices report the stored/native texture sample count. This closes the RHI observability side of graph-created MSAA texture work, while custom resolve validation evidence remains pending.

Validation:
- `cargo test -p engine_renderer headless_rhi_texture_samples_are_queryable -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer wgpu_rhi_texture_samples_are_queryable -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for custom MSAA resolve validation evidence, readback-backed surface graph exports plus direct swapchain export capability gate, and persistent backend-resident synchronization.

## 2026-05-20 execution note: persistent backend-wgpu graph texture import cache

Backend-wgpu graph execution now uses a persistent `WgpuRhiDevice` state owned by `WgpuRendererRuntime`, and public texture graph imports are cached by `TextureHandle`. Compatible imports reuse the same backend RHI texture across graph executions; when the public texture revision changes, bytes are re-synchronized into the existing backend allocation.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_texture_import_cache -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 17 passed.

The complete renderer goal remains open for persistent graph buffer import caching/revisions, cache eviction policy, readback-backed surface graph exports plus direct swapchain export capability gate, and custom MSAA resolve validation evidence.

## 2026-05-20 execution note: persistent backend-wgpu graph buffer import cache

Persistent backend-wgpu graph import synchronization now covers both textures and buffers. Public buffers carry revisions, and compatible graph imports reuse the same backend RHI buffer allocation while re-synchronizing represented byte ranges after public updates.

Validation:
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_buffer_import_cache -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_texture_import_cache -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 18 passed.

The complete renderer goal remains open for readback-backed surface graph exports plus direct swapchain export capability gate and custom MSAA resolve validation evidence.

## 2026-05-20 execution note: persistent backend-wgpu graph import cache eviction

Persistent backend-wgpu graph import cache retirement now follows public resource lifetime. Destroying a public texture removes its cached graph RHI texture import entry, and destroying a public buffer removes its cached graph RHI buffer import entry, preventing stale backend graph imports from surviving after public handle destruction.

Validation:
- Added `destroying_public_graph_import_resources_evicts_persistent_import_cache` to cover texture and buffer import cache population followed by public `destroy()` eviction.
- `cargo test -p engine_renderer destroying_public_graph_import_resources_evicts_persistent_import_cache -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for direct readback-backed surface graph exports plus direct swapchain export capability gate, custom resolve support-query validation evidence, and persistent backend-resident synchronization.

## 2026-05-20 execution note: readback-backed surface main-color graph export promotion

Backend-owned surface frames now remap exported standard-frame `main_color` graph resources to the durable backend surface readback texture when a backend surface public frame output is materialized. The graph export source is recorded as `BackendMainSurfaceReadback` or `BackendSurfaceReadback`, separating true surface-readback-backed graph promotion from ordinary offscreen transient promotion.

Validation:
- Added `backend_surface_readback_replaces_main_color_graph_export_handle` to cover replacing a promoted `main_color` graph export handle with the backend surface readback public texture and provenance.
- `cargo test -p engine_renderer backend_surface_readback_replaces_main_color_graph_export_handle -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for direct/non-readback surface or swapchain graph export mechanisms and custom resolve support-query validation evidence.

## 2026-05-20 execution note: RHI graphics pipeline MSAA sample counts

`RhiGraphicsPipelineDesc` now carries `sample_count`, backend-wgpu render pipeline creation uses that value in `wgpu::MultisampleState`, and both headless and backend-wgpu RHI render-pass validation reject color/depth targets whose sample count does not match the graphics pipeline. This enables graph/RHI passes to create programmable pipelines that target graph-created MSAA textures instead of being limited to single-sample pipeline descriptors.

Validation:
- Added `headless_rhi_graphics_pipeline_sample_count_matches_render_targets`.
- Added `wgpu_rhi_graphics_pipeline_sample_count_matches_msaa_target`.
- `cargo test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture` passed, 2 passed.

The complete renderer goal remains open for direct/non-readback surface or swapchain graph export coverage and custom resolve support-query validation evidence.

## 2026-05-20 execution note: explicit RGBA8 MSAA resolve API

`RhiDevice::resolve_texture_rgba8(source, target)` now exposes an explicit renderer-layer MSAA resolve operation. The API validates a multisampled RGBA8 render-attachment source and a same-sized single-sample RGBA8 render-attachment target. Backend-wgpu performs a native render-pass resolve into the target texture, while headless RHI copies the deterministic resolved payload for semantic validation. `PassContext::resolve_rhi_texture_rgba8(source, target)` makes the operation callable from RenderGraph passes.

Validation:
- Added `headless_rhi_resolves_rgba8_msaa_texture_explicitly`.
- Added `wgpu_rhi_resolves_rgba8_msaa_texture_explicitly`.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and direct/non-readback surface or swapchain graph export coverage.

## 2026-05-20 execution note: indexed-sample custom RGBA8 MSAA resolve

`RhiResolveMode` now distinguishes `Average`, `FirstSample`, and `Sample(u32)` resolve modes. `RhiDevice::resolve_texture_rgba8_with_mode(source, target, mode)` and `PassContext::resolve_rhi_texture_rgba8_with_mode(source, target, mode)` expose the mode-selectable resolve path to RHI and RenderGraph callbacks. Backend-wgpu implements indexed-sample resolve with a compute shader that reads `texture_multisampled_2d<f32>` at the requested sample index and writes the target storage texture; `FirstSample` is the sample-0 shortcut, while `Average` continues to use native render-pass resolve.

Validation:
- Added `headless_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode`, including non-zero `Sample(2)` and out-of-range sample-index validation.
- Added `wgpu_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode`, including non-zero `Sample(2)` and out-of-range sample-index validation.
- Added `graph_pass_context_resolves_msaa_texture_with_first_sample_mode`.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.
- `cargo test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 3 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and direct/non-readback surface or swapchain graph export coverage.

## 2026-05-20 execution note: backend-wgpu custom WGSL RGBA8 MSAA resolve shader

`RhiResolveShaderDesc` and `RhiDevice::resolve_texture_rgba8_with_shader(source, target, shader)` now expose a user-supplied WGSL custom resolve path for backend-wgpu. The shader ABI binds the multisampled RGBA8 source as `@group(0) @binding(0) texture_multisampled_2d<f32>` and the single-sample RGBA8 target as `@group(0) @binding(1) texture_storage_2d<rgba8unorm, write>`. Backend-wgpu validates the source/target shape and usage, creates a compute pipeline, binds the source/target views, and dispatches over the target extent. Headless RHI rejects the shader path with `UnsupportedFeature(BackendWgpu)` instead of pretending to execute WGSL.

Validation:
- Added `headless_rhi_rejects_custom_resolve_shader`.
- Added `wgpu_rhi_resolves_rgba8_msaa_texture_with_custom_shader`.
- Added `graph_pass_context_resolves_msaa_texture_with_custom_wgsl_shader`.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.
- `cargo test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 3 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and direct/non-readback surface or swapchain graph export coverage.

## 2026-05-20 execution note: backend-wgpu custom WGSL RGBA16F MSAA resolve shader

`RhiDevice::resolve_texture_rgba16f_with_shader(source, target, shader)` and `PassContext::resolve_rhi_texture_rgba16f_with_shader(source, target, shader)` now extend the user-supplied WGSL resolve path to RGBA16F/HDR textures. Backend-wgpu validates a multisampled RGBA16F sampled source and a same-sized single-sample RGBA16F storage target, then dispatches the caller WGSL shader with the same source/target binding ABI as RGBA8 except the target storage format is `rgba16float`.

Validation:
- Added `wgpu_rhi_resolves_rgba16f_msaa_texture_with_custom_shader`.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and direct/non-readback surface or swapchain graph export coverage.

## 2026-05-20 execution note: backend-wgpu custom WGSL RGBA32F MSAA resolve shader

`RhiDevice::resolve_texture_rgba32f_with_shader(source, target, shader)` and `PassContext::resolve_rhi_texture_rgba32f_with_shader(source, target, shader)` now extend the custom WGSL resolve ABI to RGBA32F textures. Backend-wgpu validates a multisampled RGBA32F sampled source and a same-sized single-sample RGBA32F storage target, then dispatches the caller WGSL shader with `rgba32float` storage output.

Validation:
- Added `wgpu_rhi_resolves_rgba32f_msaa_texture_with_custom_shader`.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed; the native execution branch is gated on wgpu guaranteed-format MSAA support so unsupported adapters validate the capability path without entering invalid native texture creation.

The complete renderer goal remains open for custom resolve support-query validation evidence and direct/non-readback surface or swapchain graph export coverage.

## 2026-05-20 execution note: surface graph export unsupported provenance

Surface `main_color` graph exports now distinguish readback-backed durable promotion from unsupported backend surface output. When a backend-owned surface cannot materialize a public frame output because readback is unsupported, disabled, or unavailable, the matching `main_color` graph export is marked unpromoted and receives `BackendSurfaceReadbackUnsupported`, `BackendSurfaceReadbackDisabled`, or `BackendSurfaceReadbackUnavailable` provenance. Imported-public graph export counts now key off `RendererGraphExportSource::ImportedPublic` instead of treating every unpromoted export as an import, so unsupported surface graph exports are not misclassified as public imports.

Validation:
- Added `backend_surface_readback_unsupported_marks_main_color_graph_export_unpromoted`.
- `cargo test -p engine_renderer backend_surface_readback_unsupported_marks_main_color_graph_export_unpromoted -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and any platform-specific direct swapchain export mechanisms beyond readback-backed or explicitly unsupported provenance.

## 2026-05-20 execution note: surface graph export support query

`Renderer::surface_graph_export_support()` now exposes a public support query for surface graph exports. It reports that direct swapchain image graph export is not supported by the current renderer path, while also reporting whether readback-backed surface graph export is supported and enabled. This gives tools a stable branch point instead of inferring support from frame-output side effects.

Validation:
- Added `surface_graph_export_support_reports_direct_swapchain_export_unsupported`.
- `cargo test -p engine_renderer surface_graph_export_support_reports_direct_swapchain_export_unsupported -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for custom resolve support-query validation evidence and any future platform-specific direct swapchain export mechanisms beyond readback-backed materialization plus explicit unsupported provenance.

## 2026-05-20 execution note: focused graph/RHI validation sweep

The focused MSAA resolve, graph callback, persistent import-cache eviction, and surface graph-export tests now have execution evidence. Additional stabilization in this pass preserved graph-created texture descriptor usage, submitted pending backend-wgpu graph command buffers before immediate RHI callbacks, validated backend-wgpu MSAA texture sample-count support before native texture creation, fixed packed mip-chain metadata after environment bake, prevented internal frame-output materialization from polluting upload stats, and tightened the game-layer prelude boundary regression test.

Validation:
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `cargo test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture` passed, 2 passed.
- `cargo test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.
- `cargo test -p engine_renderer graph_ -- --nocapture` passed, 133 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.
- `cargo test -p render_wgpu -- --test-threads=1` passed, 41 unit tests plus 1 integration test plus doc-tests.

The default parallel `engine_renderer` test harness hit a Windows `STATUS_ACCESS_VIOLATION` after many backend-wgpu tests; the serial full-suite run above passed. The complete renderer goal remains open for direct native surface/swapchain graph export capability gate, backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage.

## 2026-05-20 execution note: Depth32F custom MSAA resolve shader

`RhiDevice::resolve_texture_depth32f_with_shader(source, target, shader)` now extends custom MSAA resolve coverage to Depth32Float. Backend-wgpu binds the source as `texture_depth_multisampled_2d` and runs the caller fragment entry over a fullscreen pass that writes `@builtin(frag_depth)` into a single-sample Depth32Float render target. `PassContext::resolve_rhi_texture_depth32f_with_shader` exposes the same path inside RenderGraph callbacks, and headless RHI reports the shader path as explicitly unsupported.

Validation:
- Added `wgpu_rhi_resolves_depth32f_msaa_texture_with_custom_shader`.
- Added `graph_pass_context_resolves_depth32f_msaa_texture_with_custom_wgsl_shader`.
- Extended `headless_rhi_rejects_custom_resolve_shader` to cover the depth shader path.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `cargo test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

The complete renderer goal remains open for direct native surface/swapchain graph export capability gate, backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage.

## 2026-05-20 execution note: 8-bit sRGB/BGRA custom MSAA resolve shader

`RhiDevice::resolve_texture_8bit_color_with_shader(source, target, shader)` now covers custom MSAA resolves for the current public 8-bit color formats, including `Rgba8UnormSrgb` and `Bgra8UnormSrgb`. Backend-wgpu runs the caller fragment entry over a fullscreen pass, binding the multisampled source as `texture_multisampled_2d<f32>` at group 0 binding 0 and writing the single-sample target through a color render attachment. `PassContext::resolve_rhi_texture_8bit_color_with_shader` exposes the same fragment-resolve ABI inside RenderGraph callbacks, and headless RHI reports the path as explicitly unsupported.

Validation:
- Added `wgpu_rhi_resolves_rgba8_srgb_msaa_texture_with_custom_fragment_shader`.
- Added `wgpu_rhi_resolves_bgra8_srgb_msaa_texture_with_custom_fragment_shader`.
- Added `graph_pass_context_resolves_srgb_msaa_texture_with_custom_fragment_shader`.
- Extended `headless_rhi_rejects_custom_resolve_shader` to cover the 8-bit fragment shader path.
- `cargo test -p engine_renderer srgb_msaa_texture_with_custom_fragment_shader -- --nocapture` passed, 3 passed.
- `cargo test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `cargo test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

The complete renderer goal remains open for direct native surface/swapchain graph export capability gate, backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage. Current public `TextureFormat` custom resolve coverage is closed; future non-public or newly added formats remain future work.

## 2026-05-20 execution note: custom MSAA resolve support matrix

`RhiCustomResolveSupport` now exposes the renderer-layer custom resolve capability matrix. Backend-wgpu reports support for RGBA8 storage-compute, RGBA16F storage-compute, RGBA32F storage-compute, 8-bit color fragment-output, and Depth32Float fragment-depth custom resolve paths. Headless RHI reports the same paths as unsupported because it does not execute user-supplied WGSL. `RhiDevice::custom_resolve_support()` and `PassContext::rhi_custom_resolve_support()` make this query available before a graph pass chooses a custom resolve path.

Evidence added in this slice:

- Added `rhi_custom_resolve_support_reports_supported_paths`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for direct/non-readback platform swapchain graph export mechanisms and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.





## 2026-05-20 execution note: cooperative background resource retirement startup

`Renderer::start_background_resource_retirement()` is no longer an unsupported-only API. It enables a cooperative background retirement state, immediately performs one retirement tick through the existing submission-boundary/backend tombstone retirement path, and exposes active state through `Renderer::background_resource_retirement_active()`, `Renderer::stop_background_resource_retirement()`, `ResourceRetirementStats::background_retirement_active`, and `MemoryStats::background_retirement_active`. The public feature bit and `RendererFeature::BackgroundResourceRetirement` now report supported facade semantics. `RendererFeature::NonblockingResourceRetirementPoll` is now conditionally supported: true nonblocking completion polling is available when a live tracked submission completion tracker exists, while fallback queue-empty polling remains the only path when no tracker is active.

Evidence added in this slice:

- Updated `background_resource_retirement_can_be_started_and_observed`.
- Updated feature-info and feature-audit expectations for the now-supported cooperative retirement startup path.
- `cargo test -p engine_renderer background_resource_retirement -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer renderer_feature -- --nocapture` passed, 4 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for cross-thread direct renderer/wgpu mutation, true nonblocking per-submission backend completion queries, direct/non-readback platform swapchain graph export mechanisms, and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.


Implementation refinement: the cooperative service owns a lightweight scheduler thread. The worker sets atomic retirement-tick requests, while `begin_frame()` and `poll_resource_retirements()` consume those requests on the renderer thread before running the existing upload/submission-boundary/backend tombstone retirement logic. This gives the API a real start/stop worker lifecycle without moving renderer arenas or wgpu runtime objects across threads.

## 2026-05-20 execution note: backend-wgpu native pipeline replacement tombstones

Replacing cached backend-wgpu native reflected pipeline objects for an existing `PipelineKey` now moves the previous shader module, layout objects, bind groups, owned buffers, render-pipeline reference, and fence metadata into the backend-owned tombstone queue instead of dropping them immediately. Structural render-pipeline cache entries remain live when the replacement or another entry still references the same native render pipeline, while the old per-material/per-key backend objects retire through the existing backend tombstone polling path.

Evidence added in this slice:

- Added `wgpu_native_pipeline_replacement_enters_backend_tombstone`.
- `cargo test -p engine_renderer wgpu_native_pipeline_replacement_enters_backend_tombstone -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer backend_wgpu::tests -- --test-threads=1` passed, 41 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for direct/non-readback platform swapchain graph export mechanisms, true backend fence objects or nonblocking per-submission completion queries beyond wgpu's queue-empty fallback, and any remaining backend-owned resource classes not yet covered by tombstones.

## 2026-05-20 execution note: pipeline cache backend coverage sync

Renderer facade pipeline cache entries now synchronize their `has_backend_object` state with active backend-wgpu native pipeline objects when cache usage stats refresh. `Renderer::pipeline_cache_backend_coverage()` returns a structured `PipelineCacheBackendCoverage` report with total/ready/used entries, backend-object-backed entries, missing backend-object counts, missing keys, and an aggregate `complete` flag. This closes the previous observability weakness where tools had to infer the facade/backend cache gap from raw counters alone.

Evidence added in this slice:

- Updated pipeline cache warmup coverage assertions for `PipelineCacheBackendCoverage`.
- `cargo test -p engine_renderer pipeline -- --nocapture` passed, 18 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

`RendererFeature::CompleteBackendPipelineCache` remains unsupported until all facade-created pipeline entries are backed by native backend objects in real rendering paths; the missing keys are now directly queryable.

## 2026-05-20 execution note: post-process backend coverage artifact

`FramePostProcessBackendCoverage` now maps declared `FramePostProcessOutput` entries to backend-native post-process labels. `FrameStats::post_process_backend_coverage()`, `FrameDebugReport::post_process_backend_coverage()`, and `FrameCapture::post_process_backend_coverage()` report total/covered/missing post-process outputs, the backend labels used for matching, covered pass labels, missing pass labels, and an aggregate completeness flag. Dynamic backend-wgpu labels such as `Neo Bloom Fxaa Taa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass` are mapped back to semantic outputs including bloom, FXAA, TAA, motion blur, SSR, depth of field, tonemap, and color grading.

Evidence added in this slice:

- Added `post_process_backend_coverage_maps_dynamic_native_labels`.
- `cargo test -p engine_renderer post_process_backend_coverage -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for production-grade implementations behind some sampled post-process branches, direct/non-readback platform swapchain graph export mechanisms, true nonblocking backend completion queries, and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: post-process support matrix

`PostProcessSupport` now exposes the renderer-layer post-process implementation matrix. `Renderer::post_process_support()` reports facade-only support for headless renderers and backend-wgpu sampled-minimal support when a backend-wgpu runtime is active. Each effect entry reports the semantic effect, backend visibility, implementation level, backend label token, production readiness, and the remaining production limitation. This covers HDR, bloom, TAA, FXAA, SSAO, SSR, depth of field, motion blur, tonemap, and color grading.

Evidence added in this slice:

- Added `post_process_support_distinguishes_backend_visible_from_production_ready`.
- `cargo test -p engine_renderer post_process_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open because these branches are explicitly sampled-minimal, not production-complete effect pipelines. Remaining production work is now queryable per effect instead of being represented only as broad `post-process family` wording.

## 2026-05-20 execution note: deformation support matrix

`DeformationSupport` now exposes renderer-layer deformation support as a product-facing matrix instead of relying only on `RendererFeature::BackendGpuDeformation`. `Renderer::deformation_support()` reports skeletal animation, morph targets, LOD selection, motion vectors, and backend GPU deformation separately, including implementation level and limitation text. This makes it explicit that retained skeletal/morph state and frame deformation/motion-vector observability are supported, while true backend GPU skinning/morph deformation buffers and submission remain an unsupported backend path.

Evidence added in this slice:

- Added `deformation_support_distinguishes_facade_outputs_from_backend_gpu_path`.
- `cargo test -p engine_renderer deformation_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for true backend GPU deformation execution and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: lighting and IBL support matrix

`RendererLightingSupport` now exposes light/shadow/environment support as a product-facing matrix. `Renderer::lighting_support()` reports retained lights, shadow mapping, environment IBL, backend IBL convolution, and runtime environment capture separately, including implementation level and limitation text. This makes it explicit that retained light/environment descriptors and graph/frame observability are supported, while backend-real IBL convolution and runtime cubemap/probe capture remain separate unsupported backend paths.

Evidence added in this slice:

- Added `lighting_support_distinguishes_retained_lighting_from_backend_ibl_convolution`.
- `cargo test -p engine_renderer lighting_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for backend-real IBL convolution/capture execution and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: frame capture support matrix

`FrameCaptureSupport` now aggregates frame-capture backend support. `Renderer::frame_capture_support()` exposes internal capture availability, registered external-hook backends, native-SDK-blocked backends, unavailable backends, per-backend info, and a `complete_native_sdk_integration` flag. This keeps internal capture and callback handoff separate from native RenderDoc/external-debugger SDK integration, which remains unavailable unless supplied through registered hooks.

Evidence added in this slice:

- Added `frame_capture_support_distinguishes_internal_hooks_and_native_sdk_blockers`.
- `cargo test -p engine_renderer frame_capture_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for real native RenderDoc/external-debugger SDK loading/capture calls and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: debug tooling support matrix

`DebugToolingSupport` now exposes debug/editor tooling support as a product-facing matrix. `Renderer::debug_tooling_support()` reports debug draw commands, picking readback, frame debug reports, frame capture, and native frame debugger capture separately, including implementation level and limitation text. This makes it explicit that debug draw, picking, frame-debug snapshots, internal capture, and external-hook capture handoff are supported, while native RenderDoc/external-debugger SDK capture remains an explicit unsupported SDK integration point.

Evidence added in this slice:

- Added `debug_tooling_support_keeps_native_debugger_sdk_blocker_explicit`.
- `cargo test -p engine_renderer debug_tooling_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for native debugger SDK loading/capture and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: resource lifecycle support matrix

`ResourceLifecycleSupport` now exposes resource lifecycle coverage per resource class. `Renderer::resource_lifecycle_support()` reports create/update/destroy support, generation/stale-handle error coverage, upload/readback applicability, residency support, stats/capture/debug observability, backend residency level, and limitation text for mesh, buffer, texture, sampler, shader, material, material template, scene, view, render target, camera, environment, graph extension, skeleton instance, morph weights, LOD group, and pipeline cache entry resources. Backend-wgpu-active renderers mark backend-resident classes as `BackendPersistentPartial`, making persistent backend synchronization gaps explicit instead of hiding them behind broad lifecycle wording.

Evidence added in this slice:

- Added `resource_lifecycle_support_reports_per_class_lifecycle_and_backend_gaps`.
- `cargo test -p engine_renderer resource_lifecycle_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for complete backend-resident dirty synchronization, direct swapchain graph export, true nonblocking backend completion queries, and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: backend synchronization support matrix

`BackendSynchronizationSupport` now exposes renderer/backend retirement and synchronization capability as a product-facing matrix. `Renderer::backend_synchronization_support()` reports submission-boundary retirement, backend tombstone retirement, queue-empty fallback polling, true nonblocking submission-index polling, and background retirement scheduling separately, including implementation level, active background scheduler state, and limitation text. This keeps the current queue-empty fallback and scheduler-thread safe-point model distinct from the activated nonblocking path, which is available when a tracked submission boundary exists and may still fall back when no completion tracker is present.

Evidence added in this slice:

- Added `backend_synchronization_support_reports_polling_and_scheduler_limits`.
- `cargo test -p engine_renderer backend_synchronization_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Additional evidence in this slice:

- Added `backend_synchronization_support_reports_true_nonblocking_after_tracked_completion` to assert that sync support transitions from fallback-only to true nonblocking behavior once a tracked completion boundary is observed.
- `cargo test -p engine_renderer backend_synchronization_support_reports_true_nonblocking_after_tracked_completion -- --nocapture` passed, 1 passed.

The complete renderer goal remains open for true nonblocking per-submission backend completion queries, complete backend-resident dirty synchronization, direct swapchain graph export, and any remaining matrix item that is still `Partial`, `Stub`, `Missing`, unsupported-only without a real external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: RenderGraph support matrix public boundary

`Renderer::render_graph_support()` now exposes a product-facing RenderGraph capability matrix for public buffer import/export, public D2 texture import/export, packed mip compatibility, flattened layer compatibility, graph-created D2 transient promotion, graph-created MSAA resolve promotion, custom MSAA resolve PassContext integration, persistent backend import cache, readback-backed surface graph export, and direct swapchain graph export.

Evidence added in this slice:

- Added `render_graph_support_reports_backend_and_swapchain_boundaries`.
- `cargo test -p engine_renderer render_graph_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

This does not complete the renderer goal. The matrix makes remaining RenderGraph boundaries explicit: persistent backend import cache depends on an active backend-wgpu runtime, direct swapchain graph export remains unsupported, and the full renderer layer still requires all current `Partial`, `Stub`, `Missing`, unsupported-only, backend-incomplete, and support-matrix-only items to be converted into real execution paths or true external blockers.

## 2026-05-20 execution note: feature support matrix public boundary

`RendererFeatureSupportMatrix` now exposes the feature/stability surface as a product-facing support matrix instead of relying only on aggregate audit counts. `Renderer::feature_support_matrix()` returns all public feature infos grouped by `RendererFeatureTier` and `RendererFeatureImplementation`, including explicit `supported_non_backend_real_features`, `all_supported_features_backend_real`, and `all_unsupported_features_explained` fields. This keeps backend-real, facade-semantic, graph-semantic, config-gated, and reserved capabilities separate, so graph/facade semantics are queryable as non-backend-real rather than being implied as complete backend execution.

Evidence added in this slice:

- Added `renderer_feature_support_matrix_distinguishes_backend_real_from_facade_and_graph_semantics`.
- `cargo test -p engine_renderer renderer_feature_support_matrix -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open for direct/non-readback platform swapchain graph export, true nonblocking per-submission backend completion queries, complete backend-resident dirty synchronization, backend-real conversion of remaining graph-semantic standard/advanced passes where required, and any other matrix item that remains `Partial`, `Stub`, `Missing`, unsupported-only without a true external blocker, or not yet backed by code/tests/docs evidence.

## 2026-05-20 execution note: backend material resource dependency invalidation

Texture updates, generated mip changes, texture destruction, sampler destruction, material destruction, material parameter removal, and material parameter replacement now drive backend dependency invalidation instead of relying on later passive replacement. The renderer resolves materials that reference a texture or sampler, unregisters backend-wgpu material texture/sampler bindings when applicable, and invalidates affected backend native pipeline objects so old bind groups and external resource bindings enter the existing backend tombstone retirement path.

Evidence added in this slice:

- Added `material_dependency_lookup_tracks_texture_and_sampler_users` to cover dependency discovery for standard material texture slots plus reflected material texture/sampler parameters.
- Covered by `cargo test -p engine_renderer -- --test-threads=1`, which passed 408 tests plus doc-tests.

The complete renderer goal remains open. This closes one backend-resident lifecycle gap for material-bound texture/sampler resources, but full native multi-subresource texture synchronization, persistent buffer dirty-range synchronization, direct swapchain graph export, true nonblocking backend completion queries, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: backend material resource binding observability

`Renderer::backend_material_resource_stats()` now exposes backend material resource binding counts through `BackendMaterialResourceStats`. Backend-wgpu reports live material texture bindings, sampler bindings, total bindings, and backend-active state from its material external resource registry. Headless renderers report the default inactive/zero state.

Evidence added in this slice:

- Added `WgpuMaterialExternalResourceStats` in backend-wgpu.
- Added `BackendMaterialResourceStats` and `Renderer::backend_material_resource_stats()` in the renderer facade.
- Added `backend_material_resource_stats_reports_headless_inactive`.
- `cargo test -p engine_renderer backend_material_resource_stats -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This adds public observability for the material-bound backend resource lifecycle path, but it does not complete all backend resource residency, dirty synchronization, direct swapchain graph export, true nonblocking backend completion, or production-grade standard renderer paths.

## 2026-05-20 execution note: backend material resource stats in frame/debug/capture outputs

`BackendMaterialResourceStats` is now carried through frame observability. `FrameStats` stores the per-frame backend material resource snapshot, `FrameDebugReport::from_stats` preserves it for editor/debug consumers, `FrameCapture` stores it alongside pipeline cache data, and `FrameCaptureResourceDump` includes the current backend material resource stats in resource dumps. `Renderer::apply_frame_instrumentation` fills the snapshot from `Renderer::backend_material_resource_stats()` so all frame construction paths share the same source.

Evidence added in this slice:

- Added `backend_material_resources` to `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.
- Added `frame_debug_report_preserves_backend_material_resource_stats`.
- `cargo test -p engine_renderer frame_debug_report_preserves_backend_material_resource_stats -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This closes the public frame/debug/capture observability gap for material-bound backend resources, but complete backend dirty synchronization, direct swapchain graph export, true nonblocking backend completion queries, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: material backend support matrix

`MaterialBackendSupport` now exposes the material backend coverage boundary directly. `Renderer::material_backend_support()` reports facade standard/custom material support, shader-reflection/schema validation and diagnostics, backend-wgpu reflected custom-material native draw support, backend texture/sampler material binding support, and the still-unsupported complete dynamic material-template backend pipeline path. This keeps reflected wgpu custom-material coverage separate from the broader dynamic material-template backend integration gap.

Evidence added in this slice:

- Added `MaterialBackendFeature`, `MaterialBackendImplementationLevel`, `MaterialBackendFeatureSupport`, `MaterialBackendSupport`, and `Renderer::material_backend_support()`.
- Added `material_backend_support_distinguishes_facade_reflected_backend_and_dynamic_template_gap`.
- `cargo test -p engine_renderer material_backend_support -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. The new matrix is an explicit support boundary, not complete backend material execution: complete dynamic material-template backend pipeline layouts/bind groups, direct swapchain graph export, true nonblocking backend completion queries, and other remaining `Partial` rows still need real implementation or explicit external blockers.

## 2026-05-20 execution note: graph RHI import cache dirty-state observability

`Renderer::graph_rhi_import_cache_stats()` now exposes `RendererGraphRhiImportCacheStats` for persistent graph RHI public-resource import caches. The report includes texture entries, buffer entries, total entries, stale texture entries, stale buffer entries, total stale entries, and an aggregate synchronized flag. This keeps the existing persistent cache behavior intact: public buffer/texture updates can be detected as stale revisions before the next import synchronizes the cached RHI resource.

Evidence added in this slice:

- Added `RendererGraphRhiImportCacheStats`.
- Added `Renderer::graph_rhi_import_cache_stats()`.
- Added `graph_rhi_import_cache_stats_reports_stale_public_revisions`.
- `cargo test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This improves backend graph import dirty-sync observability, but direct swapchain graph export, true nonblocking backend completion queries, full multi-subresource backend residency synchronization, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: graph RHI import cache stats in frame/debug/capture outputs

`RendererGraphRhiImportCacheStats` is now part of the standard frame observability chain. `FrameStats` stores the graph RHI import cache snapshot, `FrameDebugReport::from_stats` preserves it for editor/debug consumers, `FrameCapture` stores it alongside pipeline/material backend stats, and `FrameCaptureResourceDump` includes the current graph import cache dirty-state report. `Renderer::apply_frame_instrumentation` fills the field from `Renderer::graph_rhi_import_cache_stats()`.

Evidence added in this slice:

- Added `graph_rhi_import_cache` to `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.
- Added `frame_debug_report_preserves_graph_rhi_import_cache_stats`.
- `cargo test -p engine_renderer frame_debug_report_preserves_graph_rhi_import_cache_stats -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This closes graph import cache dirty-state observability across stats/debug/capture, but direct swapchain graph export, true nonblocking backend completion queries, full backend residency synchronization, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: graph import cache stale byte/range accounting

`RendererGraphRhiImportCacheStats` now reports the dirty synchronization footprint of stale persistent graph RHI import cache entries. In addition to stale texture/buffer entry counts, it includes stale texture bytes, stale buffer represented range count, stale buffer bytes, and total stale bytes. These values describe how much public resource data the next graph import synchronization must push back into cached RHI resources.

Evidence added in this slice:

- Extended `RendererGraphRhiImportCacheStats` with stale byte/range counters.
- Updated `graph_rhi_import_cache_stats_reports_stale_public_revisions` expectations for stale texture bytes, buffer ranges, buffer bytes, and total stale bytes.
- `cargo test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This improves dirty-sync observability for persistent graph imports, but does not close direct swapchain graph export, true nonblocking backend completion queries, full backend residency synchronization, or production-complete standard renderer paths.

## 2026-05-20 execution note: pipeline cache backend coverage in frame/debug/capture outputs

`PipelineCacheBackendCoverage` is now carried through the standard frame observability chain. `FrameStats` stores the per-frame facade/backend pipeline object coverage snapshot, `FrameDebugReport::from_stats` preserves it for editor/debug consumers, `FrameCapture` stores it with the capture payload, and `FrameCaptureResourceDump` includes the current coverage report. `Renderer::apply_frame_instrumentation` fills the field from `Renderer::pipeline_cache_backend_coverage()`.

Evidence added in this slice:

- Added `pipeline_cache_backend_coverage` to `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.
- Added `frame_debug_report_preserves_pipeline_cache_backend_coverage`.
- `cargo test -p engine_renderer frame_debug_report_preserves_pipeline_cache_backend_coverage -- --nocapture` passed, 1 passed.

The complete renderer goal remains open. This closes a pipeline-cache observability gap, but it does not make every facade pipeline entry backend-backed, and direct swapchain graph export, true nonblocking backend completion queries, full backend residency synchronization, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: pipeline cache backend coverage missing-entry classification

`PipelineCacheBackendCoverage` now classifies missing backend pipeline objects with more detail. In addition to total missing backend object entries and used missing entries, it reports ready missing backend object entries and unused missing backend object entries. This lets debug/editor tools distinguish a complete coverage failure that affected the current frame from a ready-but-unused facade/backend cache gap.

Evidence added in this slice:

- Added `ready_missing_backend_object_entries` and `unused_missing_backend_object_entries` to `PipelineCacheBackendCoverage`.
- Updated focused coverage expectations for missing backend object classification.
- `cargo test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture` passed, 1 passed.

The complete renderer goal remains open. This improves pipeline cache diagnostics, but it does not make every facade pipeline entry backend-backed, and direct swapchain graph export, true nonblocking backend completion queries, full backend residency synchronization, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: sampler info and destroyed texture-view output coverage

The texture/sampler API now includes direct sampler descriptor inspection. `SamplerInfo` and `Renderer::sampler_info()` expose the retained sampler descriptor plus current status for live sampler payloads, while `Renderer::resource_status()` remains the public status surface after destruction moves the sampler into `DestroyQueued` without keeping the descriptor payload. Public texture-view frame output also has explicit destroyed-target coverage: a view targeting a destroyed texture now remains locked by a focused test before any public frame-output writeback can materialize stale bytes.

Evidence added in this slice:

- Added `SamplerInfo` and `Renderer::sampler_info()`, exported through the renderer prelude.
- Added `sampler_info_reports_desc_status_and_destroyed_payload_boundary`.
- Added `texture_view_frame_output_rejects_destroyed_target_texture`.
- `cargo test -p engine_renderer sampler_info_reports_desc_status_and_destroyed_payload_boundary -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer texture_view_frame_output_rejects_destroyed_target_texture -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This closes a small sampler inspection gap and strengthens destroyed texture-view frame-output evidence, but full backend-resident resource synchronization, direct swapchain graph export, true nonblocking backend completion queries, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: backend submission completion report

`BackendSubmissionCompletionReport` now exposes renderer/backend completion-polling state. `Renderer::backend_submission_completion_report()` reports backend-active state, queue-empty poll support, last poll queue-empty result, whether the last poll used the queue-empty fallback, whether a submission index was recorded, whether true nonblocking submission-index polling is supported, and limitation text. The report is also carried through `FrameStats`, `FrameDebugReport`, and `FrameCapture` so debug/editor tools and captures can inspect completion behavior without relying only on resource-retirement counters.

Evidence added in this slice:

- Added `BackendSubmissionCompletionReport`.
- Added `Renderer::backend_submission_completion_report()`.
- Added `backend_submission_completion_report_exposes_nonblocking_limit`.
- Added `frame_debug_report_preserves_backend_submission_completion_report`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit` and `frame_debug_report_preserves_backend_submission_completion_report`.

The complete renderer goal remains open. This makes backend completion limitations explicit and observable, but true nonblocking per-submission backend completion queries are currently only conditionally available when a tracker is active; direct swapchain graph export, full backend residency synchronization, and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: backend submission completion in resource dumps

`FrameCaptureResourceDump` now includes `BackendSubmissionCompletionReport`, matching the `FrameStats`, `FrameDebugReport`, and `FrameCapture` propagation added earlier. Resource dumps therefore preserve backend-active state, queue-empty fallback use, submission-index recording state, and the true-nonblocking-completion limitation alongside resource inventory and backend pipeline/material/graph cache diagnostics.

Evidence added in this slice:

- Added `backend_submission_completion` to `FrameCaptureResourceDump`.
- Extended `frame_debug_report_preserves_backend_submission_completion_report` to assert resource-dump propagation.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_backend_submission_completion_report`.

The complete renderer goal remains open. This completes the observability propagation for the current backend completion report, but true nonblocking per-submission backend completion queries remain conditionally available while a completion tracker is active; direct swapchain graph export, full backend residency synchronization, and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: backend submission completion in resource retirement stats

`ResourceRetirementStats` now includes `backend_submission_completion: BackendSubmissionCompletionReport`. `Renderer::poll_resource_retirements()` therefore returns the same backend completion boundary that frame stats, debug reports, captures, and resource dumps expose: backend-active state, queue-empty fallback state, submission-index recording state, and true-nonblocking-completion support/limitation.

Evidence added in this slice:

- Added `backend_submission_completion` to `ResourceRetirementStats`.
- Added `resource_retirement_stats_preserve_backend_submission_completion_report`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `resource_retirement_stats_preserve_backend_submission_completion_report`.

The complete renderer goal remains open. Resource-retirement observability is now consistent, but true nonblocking per-submission backend completion queries remain conditionally available while a completion tracker is active; direct swapchain graph export, full backend residency synchronization, and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: backend completion report tombstone wait/retire counters

`BackendSubmissionCompletionReport` now includes backend tombstone pressure fields: pending tombstones, tombstones waiting for submission-index retirement, tombstones waiting for queue-empty fallback retirement, retired tombstones in the last poll, retired-after-queue-empty state, and retired-after-completed-submission-index state. The report now links completion polling semantics directly to backend resource retirement state.

Evidence added in this slice:

- Extended `BackendSubmissionCompletionReport` with tombstone wait/retire counters.
- Updated `Renderer::backend_submission_completion_report()` to fill those fields from `BackendResourceRetirementStats`.
- Updated `backend_submission_completion_report_exposes_nonblocking_limit` expectations.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit`.

The complete renderer goal remains open. This improves backend synchronization observability, but true nonblocking per-submission backend completion queries remain conditionally available while a completion tracker is active; direct swapchain graph export, full backend residency synchronization, and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: external render target destroyed attachment validation

External render targets are now covered for destroyed attachment handles at frame time. A `RenderTarget::External` descriptor can remain live after a color or depth attachment texture is destroyed, but `Frame::render_view()` revalidates the descriptor and returns the destroyed texture handle as an invalid texture instead of allowing stale offscreen output.

Evidence added in this slice:

- Added `external_render_target_rejects_destroyed_attachment_at_frame_time`.
- Added `external_render_target_rejects_destroyed_depth_attachment_at_frame_time`.
- `cargo test -p engine_renderer external_render_target_rejects_destroyed_attachment_at_frame_time -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer external_render_target_rejects_destroyed_depth_attachment_at_frame_time -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

The complete renderer goal remains open. This closes another specialized destroyed frame-output path, but full backend-resident synchronization, direct swapchain graph export, conditionally available true nonblocking backend completion, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: render_view rejects destroyed plain texture render targets

`Frame::render_view()` now has explicit coverage for `RenderTarget::Texture` paths in addition to texture-view cases. Destroying a texture target after `RenderTarget::Texture` creation now causes `Frame::render_view()` to reject the stale handle with `RendererError::InvalidHandle` while preserving delayed-destroy queue semantics for the handle.

Evidence added in this slice:

- Added `render_view_rejects_destroyed_texture_target`.
- `cargo test -p engine_renderer render_view_rejects_destroyed_texture_target -- --nocapture` passed, 1 passed.

The complete renderer goal remains open. This further broadens stale-handle rejection coverage for render target variants, but full backend-resident synchronization, direct swapchain graph export, conditionally available true nonblocking backend completion, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: build_view_graph_stats rejects destroyed render targets

`FrameGraph` prebuild validation now rejects stale texture targets before planning graph execution. Destroyed `RenderTarget::Texture` and texture-view descriptors now return `RendererError::InvalidHandle` through `build_view_graph_stats(...)`, preventing stale render targets from entering graph compile paths.

Evidence added in this slice:

- Added `build_view_graph_stats_rejects_destroyed_texture_target`.
- Added `build_view_graph_stats_rejects_destroyed_texture_view_target`.
- `cargo test -p engine_renderer build_view_graph_stats_rejects_destroyed_texture_target -- --nocapture` passed, 1 passed.
- `cargo test -p engine_renderer build_view_graph_stats_rejects_destroyed_texture_view_target -- --nocapture` passed, 1 passed.

The complete renderer goal remains open. This broadens graph prebuild stale-handle rejection coverage for texture-target variants, while full backend-resident synchronization, direct swapchain graph export, conditionally available true nonblocking backend completion, and production-complete standard renderer paths remain open.

## 2026-05-20 execution note: explicit nonblocking backend completion poll error/success path

`Renderer::poll_backend_submission_completion_nonblocking()` now provides an explicit public API for true nonblocking backend submission completion polling. The current renderer returns `RendererError::Validation` with user-visible limitation text when no active nonblocking per-submission completion tracker is available, and returns `Ok` once runtime polling observes/retains an active completion tracker. This turns the previous report-only limitation into a callable capability gate and error path.

Evidence added in this slice:

- Added `Renderer::poll_backend_submission_completion_nonblocking()`.
- Added `nonblocking_backend_submission_completion_poll_reports_user_visible_error`.
- Added `nonblocking_backend_submission_completion_poll_can_be_supported_after_real_submission`.
- Added `nonblocking_backend_submission_completion_poll_reports_user_visible_error_without_trackers`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `nonblocking_backend_submission_completion_poll_reports_user_visible_error`, `nonblocking_backend_submission_completion_poll_can_be_supported_after_real_submission`, and `nonblocking_backend_submission_completion_poll_reports_user_visible_error_without_trackers`.

The complete renderer goal remains open. This closes the error-only public gate by actually advancing backend retirement polling before reporting, and now returns `Ok` once an active nonblocking submission-index tracker is present (for example, immediately after a real tracked submission/tombstone pair). `Direct swapchain graph export`, `full backend residency synchronization`, and `production-complete standard renderer paths` also remain open.

## 2026-05-20 execution note: explicit direct swapchain graph export gate

`Renderer::require_direct_swapchain_graph_export_supported()` now provides a public capability gate for direct swapchain image graph export. Current renderer paths return `RendererError::Validation` with the same user-visible limitation text exposed by `Renderer::surface_graph_export_support()`. This turns the previous support-matrix-only unsupported state into a callable error path.

Evidence added in this slice:

- Added `Renderer::require_direct_swapchain_graph_export_supported()`.
- Added `direct_swapchain_graph_export_gate_returns_user_visible_error`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `direct_swapchain_graph_export_gate_returns_user_visible_error`.

The complete renderer goal remains open. This closes public gate/error semantics for direct swapchain graph export, but it does not implement native swapchain image export itself; true nonblocking backend completion remains conditionally available by tracker presence, and full backend residency synchronization and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: surface graph export support in frame/debug/capture outputs

`RendererSurfaceGraphExportSupport` is now part of the standard frame observability chain. `FrameStats` stores the surface graph export support snapshot, `FrameDebugReport::from_stats` preserves it, `FrameCapture` stores it, and `FrameCaptureResourceDump` includes it. `Renderer::apply_frame_instrumentation` fills the field from `Renderer::surface_graph_export_support()`.

Evidence added in this slice:

- Added `surface_graph_export` to `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.
- Added `frame_debug_report_preserves_surface_graph_export_support`.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_surface_graph_export_support`.

The complete renderer goal remains open. This makes direct/readback-backed surface graph export support consistently observable, but native direct swapchain image graph export remains unsupported; true nonblocking backend completion remains conditionally available by tracker presence, while full backend residency synchronization and production-complete standard renderer paths also remain open.

## 2026-05-20 execution note: RenderGraph support matrix in frame/debug/capture outputs

`RendererRenderGraphSupport` is now part of the standard frame observability chain. `FrameStats` stores the render-graph support matrix, `FrameDebugReport::from_stats` preserves it, `FrameCapture` stores it, and `FrameCaptureResourceDump` includes it. `Renderer::apply_frame_instrumentation` fills the field from `Renderer::render_graph_support()`.

Evidence added in this slice:

- Added `render_graph_support` to `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.
- Added `Default` for `RendererRenderGraphSupport` so default frame/capture payloads remain constructible.
- Added `frame_debug_report_preserves_render_graph_support_matrix`.
- `cargo test -p engine_renderer frame_debug_report_preserves_render_graph_support_matrix -- --nocapture` passed.

The complete renderer goal remains open. RenderGraph support state is now consistently observable, including the unsupported direct swapchain graph export capability, but native direct swapchain image export, true nonblocking backend completion, full backend residency synchronization, and production-complete standard renderer paths remain open.

## 2026-05-21 execution note: surface runtime consistency and completion-tracker boundary enforcement

`Renderer::with_surface()` now validates supplied window/display handles before delegating to backend creation and verifies runtime surface/depth format consistency before runtime adoption. This closes startup mismatches where configured caps drift from actual runtime capabilities.

`Renderer::supports_nonblocking_resource_retirement_poll()`/`RendererFeature::NonblockingResourceRetirementPoll` now reflect runtime completion-tracker availability, and `Renderer::poll_backend_submission_completion_nonblocking()` returns a user-visible validation error when no tracker exists, while succeeding once an active completion-index tracker is present.

Evidence added in this slice:

- Added `validate_surface_window_handles` checks.
- Added `validate_surface_runtime_formats` checks in `WgpuRendererRuntime::with_surface`.
- Added completion tracker reuse for repeated identical submission index tombstones.
- Added focused tests for tracker gating and surface consistency.
- `cargo test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `with_surface_requires_backend_wgpu_if_unavailable`, `with_surface_validates_window_handles_for_surface_creation`, `validate_surface_runtime_formats_rejects_configured_color_format_mismatch`, `validate_surface_runtime_formats_rejects_configured_depth_format_mismatch`, `feature_support_reflects_nonblocking_completion_tracker_state`, and `nonblocking_backend_submission_completion_poll_can_be_supported_after_real_submission`.

The complete renderer goal remains open. Native direct swapchain image graph export, production-complete backend-resident synchronization, and broader standard renderer real-backend completion paths remain open.

## 本轮窗口/Surface 句柄验证证据补充（2026-05-21）

- 能力项：窗口/Surface 初始化语义与错误路径。
- 本轮实现：新增测试 `with_surface_validates_display_handles_for_surface_creation`（`backend-wgpu` 下运行），补齐 `Renderer::with_surface` 对 `HasDisplayHandle` 不可用情况的独立验证。
- 该用例使用 `DummySurfaceWindowWithoutDisplay` 同时提供有效 `RawWindowHandle` 的 `WindowHandle`，但对 `HasDisplayHandle` 返回 `HandleError::Unavailable`。
- 断言结果：`Renderer::with_surface` 返回 `RendererError::Validation`，且错误消息包含 `display`。
- 这形成了窗口句柄无效分支之外的独立错误路径，避免将 display 依赖缺失误判为窗口句柄错误。
- 该轮附加验证：新增测试 `with_surface_short_circuits_display_validation_on_window_handle_error`，构建窗口句柄不可用但 display 跟踪桩，确认 `with_surface` 在窗口句柄阶段失败时不会继续调用 display 校验。若 display 被误调用会触发桩的 panic，帮助防止错误分支错配。
- 该轮附加验证：新增测试 `with_surface_invokes_window_handle_validation_before_display_validation`，使用顺序计数桩确认当 `display` 不可用时仍需先进行 `window_handle()` 查询，并且窗口与 display 查询各发生一次。
