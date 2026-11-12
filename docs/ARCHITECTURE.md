# Архитектура Soldatik3D

Документ описывает устройство движка, поток кадра, владение шейдерами и порядок
добавления новых бэкендов и игр.

---

## 1. Принципы

1. **Игра зависит только от фасада.** `game` использует crate `engine`; `winit`,
   Vulkan и внутренние крейты не попадают в её манифест.
2. **Направление зависимостей строгое.** Нижние слои не знают о верхних:
   `engine-core` не знает о платформе, `engine-vulkan` — о winit.
3. **Сценарий отделён от реализации.** Игра описывает *что* нарисовать
   (`Scene`, `Camera`, `FrameData`), бэкенд решает *как*.
4. **Один бэкенд — один крейт.** Vulkan живёт в `engine-vulkan`. OpenGL или
   другой API добавляются отдельным крейтом и реализацией одного трейта.
5. **Явное владение ресурсами.** Окно живёт дольше рендерера; Vulkan-объекты
   уничтожаются до устройства, устройство — до instance.

---

## 2. Крейты

| Крейт | Ответственность | Ключевые типы |
| --- | --- | --- |
| `engine-core` | Платформенно-независимые данные и математика | `Camera`, `Scene`, `Mesh`, `Material`, `Light`, `FrameData`, `UiDrawList` |
| `engine-render` | Контракт рендерера и смена бэкенда | `Renderer`, `BackendKind`, `WindowSurface` |
| `engine-platform` | Окно, ввод, native-handle | `PlatformWindow`, `Input` |
| `engine-vulkan` | Vulkan 1.2, пайплайны, шейдеры, ресурсы GPU | `VulkanRenderer` |
| `engine` | Фасад, `Game` trait, event loop, backend factory | `run`, `Game`, `GameConfig`, `UpdateContext` |
| `game` | Демо-игра | `DemoGame`, `Player`, `world` |

Граф зависимостей:

```text
                         ┌──────────────┐
                         │     game     │
                         └──────┬───────┘
                                │ только `engine`
                         ┌──────▼───────┐
                         │    engine    │
                         └──┬────┬────┬─┘
              ┌─────────────┘    │    └──────────────┐
      ┌───────▼────────┐  ┌──────▼───────┐  ┌────────▼────────┐
      │ engine-platform│  │ engine-render│  │ engine-vulkan   │
      └───┬────────┬───┘  └──────┬───────┘  └───────┬─────────┘
          │        └─────────────┼──────────────────┘
   ┌──────▼──────────────────────▼───────┐
   │              engine-core            │
   └─────────────────────────────────────┘
```

`engine-vulkan` — опциональная зависимость фасада. Feature `vulkan` включён по
умолчанию; сборка `--no-default-features` даёт фасад без графического API
(полезно для инструментов и альтернативных бэкендов).

---

## 3. Поток кадра

1. `winit` присылает события в `engine::app::App`.
2. `App` обновляет `Input`: зажатые клавиши, дельту мыши, колесо, позицию курсора.
3. На `RedrawRequested` вычисляется `dt` (ограничен 0.1 с).
4. `UiDrawList` очищается и получает актуальный размер viewport.
5. `Game::update(&mut UpdateContext)` обновляет геймплей и рисует UI.
6. `Game::camera()` превращается в `CameraFrame` с матрицами для текущего aspect.
7. `Renderer::render(&FrameData)` записывает и отправляет GPU-работу.
8. `Input::begin_frame()` сбрасывает одноразовые состояния, окно запрашивает
   следующий кадр.

Рантайм не знает, какой бэкенд выбран: он работает с `Box<dyn Renderer>`.
Рендерер создаётся один раз в `engine/src/backend.rs` из
`GameConfig::backend` и `WindowSurface`.

### Владение и завершение

- `PlatformWindow` создаёт окно и отдаёт `WindowSurface` (raw handles + размер).
- `VulkanRenderer` создаёт surface, swapchain и все GPU-ресурсы.
- При закрытии окна или `SOLDATIK3D_MAX_FRAMES` вызывается `shutdown()`:
  сначала `Renderer::wait_idle()`, затем рендерер уничтожается, и только потом
  окно. Этот порядок обязателен для Vulkan/Wayland.

---

## 4. Контракт рендерера

```rust
pub trait Renderer {
    fn backend(&self) -> BackendKind;
    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()>;
    fn render(&mut self, frame: &FrameData<'_>) -> anyhow::Result<()>;
    fn wait_idle(&self) -> anyhow::Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
}
```

- `resize` только запоминает размер и пересоздаёт swapchain при необходимости;
  нулевой размер (свёрнутое окно) не приводит к созданию невалидного swapchain.
- `extent` возвращает запрошенный размер; `App` пропускает кадры при `0×0`.
- `stats` описывает последний отправленный кадр и попадает в `UpdateContext`.
- `WindowSurface` содержит `RawWindowHandle`, `RawDisplayHandle` и размер.
  Это единственная точка соприкосновения платформы и графического API.

---

## 5. Vulkan-пайплайн

### Порядок проходов

```text
shadow map 2048² (depth)
        │  barrier DEPTH_STENCIL_ATTACHMENT_WRITE → SHADER_READ
        ▼
scene → HDR (RGBA16F) + depth
        ├──► SSAO (R8, half-res) → blur X → blur Y
        ├──► bright pass → bloom blur X → blur Y (half-res)
        ▼
composite: HDR × SSAO + bloom → ACES → gamma
        ▼
UI overlay → swapchain (UNORM)
```

- Shadow и основной command buffer отправляются одним `queue_submit`.
  Барьер в конце shadow-буфера делает карту теней видимой fragment-шейдеру.
- `MAX_FRAMES_IN_FLIGHT = 1`: один набор sync-объектов и command buffer'ов.
  Это упрощает синхронизацию; увеличение константы требует per-frame ресурсов.
- Swapchain создаётся в `B8G8R8A8_UNORM` (fallback — `R8G8B8A8_UNORM`/SRGB),
  потому что tone mapping и гамма выполняются в composite-шейдере.
- NDC Vulkan инвертирован по Y: проекционные матрицы умножаются на `y = -y`
  перед передачей в шейдеры.

### Свет

- До 8 ближайших точечных источников попадают в UBO.
- GLSL-блок `LightBuffer` имеет layout std140: **два отдельных массива**
  `vec4 position_radius[8]` и `vec4 color_intensity[8]`. Rust-структура
  `LightUniformBuffer` обязана повторять это расположение. Межслойное
  чередование `position/color` ломает чтение и даёт цветные артефакты.
- Основной directional light с ортографической shadow map передаётся push-константами.

### Ресурсы

- Сцена загружается один раз в immutable GPU-буферы (`GpuScene`).
- UI-вершины обновляются каждый кадр в host-visible буфер; при переполнении
  буфер пересоздаётся с удвоенной ёмкостью.
- Дескрипторные пулы уничтожаются раньше изображений и буферов, которые они
  описывают; порядок задан полями `VulkanRenderer`.

---

## 6. Шейдеры

- Встроенные шейдеры бэкенда лежат в `crates/engine-vulkan/shaders/`.
  Они компилируются `shaderc` во время запуска из `include_str!`, поэтому
  правка GLSL требует перезапуска приложения.
- Это шейдеры движка: PBR, тени, SSAO, bloom, composite, UI. Игра не обязана
  их знать.
- Игровые шейдеры (кастомные материалы/эффекты) в будущем добавляются в
  `game/shaders/` и подключаются через публичный API пайплайнов. До появления
  такого API демо использует встроенный PBR-пайплайн.

---

## 7. Добавление бэкенда

На примере OpenGL:

1. Создать `crates/engine-opengl`, зависящий от `engine-core`, `engine-render`
   и выбранной обёртки над OpenGL (например, `glow` + `glutin`).
2. Реализовать `engine_render::Renderer`. Конструктор принимает
   `WindowSurface` и `&Scene`, как `VulkanRenderer::new`.
3. Добавить вариант в `BackendKind`:

   ```rust
   pub enum BackendKind {
       Vulkan,
       OpenGl,
   }
   ```

4. Добавить опциональную зависимость и feature в `crates/engine/Cargo.toml`:

   ```toml
   [features]
   opengl = ["dep:engine-opengl"]

   [dependencies]
   engine-opengl = { path = "../engine-opengl", optional = true }
   ```

5. Добавить arm в `engine/src/backend.rs`:

   ```rust
   #[cfg(feature = "opengl")]
   BackendKind::OpenGl => Ok(Box::new(engine_opengl::OpenGlRenderer::new(surface, scene)?)),
   ```

6. Игра выбирает бэкенд без изменения кода:

   ```rust
   fn config(&self) -> GameConfig {
       GameConfig::default().with_backend(BackendKind::OpenGl)
   }
   ```

Fallback-arm в фабрике вернёт понятную ошибку, если выбранный бэкенд не
включён в сборку.

---

## 8. Добавление игры

1. Создать крейт, который зависит **только** от `engine`:

   ```toml
   [dependencies]
   engine = { path = "../engine" }
   anyhow = "1"
   ```

2. Реализовать `Game`:

   ```rust
   use engine::{run, Camera, Game, GameConfig, Scene, UpdateContext};

   struct MyGame {
       camera: Camera,
   }

   impl Game for MyGame {
       fn config(&self) -> GameConfig {
           GameConfig::default()
       }

       fn create_scene(&mut self) -> Scene {
           Scene::new(Default::default())
       }

       fn camera(&self) -> Camera {
           self.camera
       }

       fn update(&mut self, _ctx: &mut UpdateContext<'_>) {}
   }

   fn main() -> anyhow::Result<()> {
       run(MyGame { camera: Camera::default() })
   }
   ```

3. Использовать реэкспорты фасада: `engine::glam`, `engine::winit`,
   `engine::Input`, `engine::UiDrawList` и типы из `engine::prelude`.
4. Не подключать `winit`, `ash` или `engine-vulkan` напрямую — это нарушит
   переносимость и смену бэкенда.

---

## 9. Соглашения и ограничения

**Соглашения**

- Публичные типы документируются коротко и по делу; платформенные типы не
  протекают в `engine-core`.
- Ошибки инициализации и кадра возвращаются как `anyhow::Result`.
- Логи бэкенда имеют префикс `[engine-vulkan]`, рантайма — `[engine]`.
- Форматирование: `cargo fmt --all`; линт: `cargo clippy --workspace --all-targets`
  без предупреждений.

**Текущие ограничения**

- Один материал и одна сцена на всё время жизни рендерера; динамическая
  подгрузка мешей отсутствует.
- `MAX_FRAMES_IN_FLIGHT = 1`; многокадровая подготовка — следующий шаг.
- Нет MSAA и постэффектов, зависящих от времени (motion blur, TAA).
- SSAO считается в половинном разрешении и умножает весь HDR-результат;
  разделение direct/ambient — запланировано.
- UI — immediate-mode список вершин без виджет-фреймворка.

**Диагностика**

- `SOLDATIK3D_MAX_FRAMES=<n>` — корректно отрендерить `n` кадров и выйти
  (smoke-тест, в том числе для CI).
- `SOLDATIK3D_DEBUG_VIEW=normals|depth` — стартовый режим отладки демо-игры.
- В debug-сборках включается `VK_LAYER_KHRONOS_validation`, если слой установлен.
