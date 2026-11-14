# Архитектура Soldatik3D

Документ описывает устройство движка, поток кадра, централизованные `World` и
камеры, консоль, оба графических бэкенда и порядок добавления новых бэкендов
и игр.

---

## 1. Принципы

1. **Игра зависит только от фасада.** `examples/demo` использует crate
   `runtime`; `winit`, Vulkan, OpenGL и внутренние крейты не попадают в её
   манифест.
2. **Направление зависимостей строгое.** Нижние слои не знают о верхних:
   `scene` не знает о платформе, `vulkan` и `opengl` — о `winit`.
3. **Сценарий отделён от реализации.** Игра описывает *что* нарисовать
   (`Scene`, `Camera`, `FrameData`), бэкенд решает *как*.
4. **Один бэкенд — один крейт.** Vulkan живёт в `vulkan`, OpenGL — в `opengl`.
   Новый API добавляется отдельным крейтом и реализацией одного трейта.
5. **Камеры и мир принадлежат движку.** Игра не создаёт рендерер и не хранит
   матрицы кадра: она мутирует `World`, а рантайм рендерит через активную
   камеру.
6. **Явное владение ресурсами.** Окно живёт дольше рендерера; графические
   объекты уничтожаются до устройства/контекста, устройство — до instance.

---

## 2. Крейты

| Крейт | Ответственность | Ключевые типы |
| --- | --- | --- |
| `assets` | CPU-ассеты без зависимости от GPU | `TextureData`, `font_atlas_rgba8`, `generate_environment_cubemap` |
| `scene` | Мир, камеры, геометрия, свет, материалы | `World`, `Camera`, `CameraRig`, `CameraId`, `Scene`, `Mesh`, `Vertex`, `Aabb`, `Material`, `Light`, `LightUniformBuffer` |
| `ui` | Backend-neutral список вершин UI | `UiDrawList`, `UiVertex` |
| `render` | Контракт рендерера и общие типы кадра | `Renderer`, `BackendKind`, `WindowSurface`, `CameraFrame`, `FrameData`, `DebugView`, `RenderStats` |
| `platform` | Окно, ввод, native handles и определение платформы | `PlatformWindow`, `Input`, `TargetOS`, `TargetArch`, `PlatformCapabilities`, `WindowHints` |
| `vulkan` | Vulkan 1.2, пайплайны, шейдеры, GPU-ресурсы | `VulkanRenderer` |
| `opengl` | OpenGL 3.3 core, FBO-пайплайн, GPU-ресурсы | `OpenGlRenderer` |
| `runtime` | Фасад, event loop, консоль, фабрика бэкендов | `run`, `Game`, `GameConfig`, `UpdateContext`, `CommandContext` |
| `demo` | Демонстрационная игра в `examples/demo` | `DemoGame`, `Player`, процедурный уровень |

Граф зависимостей:

```text
examples/demo ──► runtime ──► scene ──► assets
                     │           ▲
                     ├─► ui ─────┤
                     ├─► render  │
                     ├─► platform│
                     ├─► vulkan ─┘ (optional)
                     └─► opengl ── (optional)
```

`vulkan` и `opengl` зависят от `scene`, `assets`, `ui`, `render`, но не от
`platform` и не от `winit`. `runtime` — единственное место, которое знает
конкретные бэкенды; оба включены features по умолчанию:

```toml
[features]
default = ["vulkan", "opengl"]
vulkan = ["dep:vulkan"]
opengl = ["dep:opengl"]
```

Сборка без графического API возможна через `default-features = false`; фабрика
вернёт понятную ошибку, если выбранный бэкенд не скомпилирован.

---

## 3. Поток кадра

1. `winit` присылает события в `runtime::app::App`.
2. `App` обновляет `Input`: зажатые клавиши, дельту мыши, колесо, позицию
   курсора и буфер текстового ввода.
3. На `RedrawRequested` вычисляется `dt` (ограничен 0.1 с).
4. Текст, набранный между кадрами, передаётся консоли; при нажатии `Enter`
   команда выполняется до обновления игры.
5. `UiDrawList` очищается и получает актуальный размер viewport.
6. `Game::update(&mut UpdateContext)` обновляет геймплей, камеры и рисует UI.
7. Консоль дорисовывает свои строки поверх UI игры.
8. Рантайм берёт `world.active_camera()` и строит `CameraFrame` с матрицами
   для текущего aspect.
9. `Renderer::render(&FrameData)` записывает и отправляет GPU-работу.
10. `Input::begin_frame()` сбрасывает одноразовые состояния, окно запрашивает
    следующий кадр.

Рантайм не знает, какой бэкенд выбран: он работает с `Box<dyn Renderer>`.
Рендерер создаётся один раз в `runtime::backend::create_renderer` из
`GameConfig::backend` и `WindowSurface`.

### Инициализация и завершение

- `PlatformWindow` создаёт окно и отдаёт `WindowSurface` (raw handles + размер).
- `create_renderer` создаёт выбранный бэкенд; сцена из `World` загружается в
  GPU один раз.
- После инициализации выполняются команды из `SOLDATIK3D_COMMANDS`; при
  `SOLDATIK3D_CONSOLE=1` открывается консоль.
- При закрытии окна или достижении `SOLDATIK3D_MAX_FRAMES` вызывается
  `shutdown()`: сначала `Renderer::wait_idle()`, затем рендерер уничтожается,
  и только потом окно. Этот порядок обязателен для Vulkan/Wayland.

---

## 4. Мир и камеры

`World` объединяет сцену и `CameraRig`:

```rust
pub struct World {
    scene: Scene,
    cameras: CameraRig,
}
```

`Scene` (меши, материал, свет, shadow settings) загружается в GPU при создании
рендерера и дальше не меняется. Камеры, наоборот, доступны игре каждый кадр:

```rust
let id = world.spawn_camera("pool", Camera::at(Vec3::new(70.0, -3.0, 60.0)));
world.select_camera(id);
world.active_camera_mut().unwrap().camera.yaw = 0.0;
```

`CameraRig` гарантирует:

- `CameraId` стабилен и не переиспользуется после удаления;
- имена уникальны: повторный `spawn("view")` даёт `view.2`;
- первая добавленная камера становится активной;
- `select_next` / `select_previous` переключаются по кругу;
- удаление активной камеры выбирает следующую;
- последнюю камеру удалить нельзя.

`Camera` содержит только позицию, углы, FOV и диапазон; логика движения
(гравитация, коллизии, полёт) остаётся в игре. Демо синхронизирует камеру
`player` с контроллером, только когда она активна. Все остальные камеры —
статические пресеты.

Рантайм передаёт бэкенду уже готовые `view` и `projection`: бэкенд не знает,
сколько камер в мире и какая из них игровая.

---

## 5. Контракт рендерера

```rust
pub trait Renderer {
    fn backend(&self) -> BackendKind;
    fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()>;
    fn render(&mut self, frame: &FrameData<'_>) -> anyhow::Result<()>;
    fn wait_idle(&self) -> anyhow::Result<()>;
    fn stats(&self) -> RenderStats;
    fn extent(&self) -> (u32, u32);
    fn screenshot(&mut self) -> anyhow::Result<()>;
    fn set_vsync(&mut self, vsync: VsyncMode) -> anyhow::Result<()>;
    fn vsync(&self) -> VsyncMode;
}
```

- `resize` запоминает размер и пересоздаёт swapchain/FBO при необходимости;
  нулевой размер (свёрнутое окно) не приводит к невалидным ресурсам.
- `extent` возвращает текущий размер; `App` пропускает кадры при `0×0`.
- `stats` описывает последний отправленный кадр и попадает в `UpdateContext`.
- `WindowSurface` содержит `RawWindowHandle`, `RawDisplayHandle` и размер.
  Это единственная точка соприкосновения платформы и графического API.
- `FrameData` содержит `CameraFrame`, вершины UI и `DebugView`.
- `VsyncMode` — `On`, `Off`, `Adaptive`, `Mailbox` (`#[non_exhaustive]`).
  Vulkan выбирает `FIFO`/`IMMEDIATE`/`FIFO_RELAXED`/`MAILBOX`, OpenGL —
  `SwapInterval::Wait(1)`/`DontWait`. `GameConfig::vsync` и
  `SOLDATIK3D_VSYNC` задают стартовый режим, `Renderer::set_vsync`
  переключает его на ходу.
- `screenshot` запрашивает захват и сохраняет PNG в следующем кадре. Vulkan
  копирует уже приобретённое swapchain-изображение после ожидания frame
  fence и до `present`; OpenGL читает default framebuffer через
  `glReadPixels` до `swap_buffers`. Это гарантирует, что PNG содержит
  отрисованный кадр, а не пустую/старую поверхность.

`BackendKind` помечен `#[non_exhaustive]`; имена `vulkan`/`opengl` (и короткие
`vk`/`gl`) разбирает `BackendKind::from_name`, переменную окружения —
`BackendKind::from_env`.

---

## 6. Общий графический пайплайн

Оба бэкенда реализуют одинаковую последовательность проходов:

```text
shadow map 2048² (depth)
        │  barrier/glFinish-подобная синхронизация
        ▼
scene → HDR (RGBA16F) + depth
        ├──► SSAO (R8, half-res) → blur X → blur Y
        ├──► bright pass → bloom blur X → blur Y (half-res)
        ▼
composite: HDR × SSAO + bloom → ACES → gamma
        ▼
UI overlay → swapchain/default framebuffer (UNORM)
```

- До восьми точечных источников попадают в UBO.
- GLSL-блок света имеет layout std140: **два отдельных массива**
  `vec4 position_radius[8]` и `vec4 color_intensity[8]`. Rust-структура
  `scene::LightUniformBuffer` повторяет это расположение; межслойное
  чередование `position/color` ломает чтение.
- Основной directional light с ортографической shadow map передаётся
  push-константами (Vulkan) или uniform-ами (OpenGL).
- `DebugView` переключает lit / normals / depth в forward-проходе.
- UI рисуется в физических пикселях поверх LDR-результата.

### Vulkan

- `ash` + `ash-window`; surface создаётся из raw handles окна.
- `shaderc` компилирует встроенные GLSL-шейдеры при запуске из
  `crates/vulkan/shaders/`.
- Swapchain — `B8G8R8A8_UNORM` (fallback на RGBA/SRGB), tone mapping и гамма
  выполняются в composite-шейдере. Если surface поддерживает
  `TRANSFER_SRC`, swapchain создаётся с этим usage для скриншотов; иначе
  команда `screenshot` возвращает понятную ошибку.
- Скриншот копирует приобретённое изображение в host-visible буфер,
  дожидаясь frame fence до `present`. Строки Vulkan-изображения уже идут
  сверху вниз, поэтому вертикальный flip не нужен; выполняется только
  swizzle BGRA → RGBA.
- `MAX_FRAMES_IN_FLIGHT = 1`: один набор sync-объектов и command buffer'ов.
- Shadow и основной command buffer отправляются одним `queue_submit`; барьер
  в конце shadow-буфера делает карту теней видимой fragment-шейдеру.
- **Проекционные матрицы.** Vulkan использует Y-down clip space и depth
  `0..1`, OpenGL — Y-up и `-1..1`. Перед передачей в шейдеры применяется
  преобразование `flip Y` и `z' = 0.5·z + 0.5·w`. Важно менять **строки**
  матрицы, а не отдельные столбцы: clip-координаты вычисляются как
  `row · vertex`.
- Дескрипторные пулы уничтожаются раньше изображений и буферов, которые они
  описывают; порядок задан полями `VulkanRenderer`.
- UI-вершины обновляются каждый кадр в host-visible буфер; при переполнении
  буфер пересоздаётся с удвоенной ёмкостью.
- Сцена загружается один раз в immutable GPU-буферы (`GpuScene`).

### OpenGL

- `glutin` создаёт контекст на **существующем** raw handle winit-окна;
  `glow` предоставляет функции GL.
- На Linux используется EGL с fallback на GLX, на Windows — WGL/EGL, на
  Android — EGL/OpenGL ES 3.0. macOS не поддерживается.
- Пайплайн зеркалит Vulkan через FBO:
  - shadow map — depth texture + depth-only FBO;
  - HDR — `RGBA16F` color + depth texture; на Android при отсутствии
    `EXT_color_buffer_half_float`/`EXT_color_buffer_float` используется
    `RGBA8`;
  - SSAO — half-res `R8` FBO и два ping-pong FBO для blur;
  - bloom — half-res HDR FBO и два ping-pong FBO для blur;
  - composite — default framebuffer;
  - UI — отдельный VAO/VBO и шейдер.
- При resize все size-dependent FBO и текстуры пересоздаются.
- Шейдеры — GLSL 330 core, встроены через `include_str!` и компилируются
  драйвером при старте. На Android загрузчик заменяет `#version` на
  `#version 300 es` и добавляет precision-квалификаторы; на desktop исходники
  передаются без изменений.
- Проекционные матрицы используются напрямую: `Camera::projection_matrix`
  уже возвращает OpenGL-совместимый clip space. UI-шейдер инвертирует Y, так
  как NDC OpenGL направлен вверх; атлас шрифта при этом по V **не**
  переворачивается.
- Шрифт загружается с `NEAREST`-фильтрацией, чтобы bitmap-глифы оставались
  чёткими.
- Скриншот (`screenshot`) запрашивается в консоли и выполняется в следующем
  кадре до `swap_buffers`: `glReadPixels` читает default framebuffer, затем
  строки переворачиваются из нижне-левой ориентации GL в верхне-левую
  ориентацию PNG.

---

## 7. Ассеты, UI и шрифт

`assets` не зависит от графического API. Бэкенды загружают одни и те же
данные:

- `TextureData` — RGBA8-текстура материала;
- `font_atlas_rgba8()` — атлас 128×128, ячейки 8×8, символы `32..126`;
- `generate_environment_cubemap(size)` — процедурное окружение для IBL.

Метрики шрифта (`FONT_ATLAS_SIZE`, `FONT_CELL_SIZE`, `FONT_GLYPH_WIDTH`,
`FONT_GLYPH_HEIGHT`, `FONT_GLYPH_COLUMNS`) экспортируются из `assets`, а `ui`
использует их для UV. Сэмплируется только область глифа 5×7 внутри ячейки,
поэтому текст не «расползается» из-за прозрачных полей.

Встроенный шрифт содержит заглавные и строчные буквы `a-z`, цифры и
ASCII-пунктуацию; область глифа — 5×7 внутри ячейки 8×8. Этого достаточно
для консоли и отладочного UI. Кириллица и другие алфавиты требуют отдельного
атласа.

`UiDrawList` хранит только вершины (`position`, `color`, `uv`). Виджеты,
панели и кнопки строятся поверх него в игре или в консоли рантайма. Каждый
бэкенд загружает вершины в свой буфер и рисует одним draw call.

---

## 8. Консоль и команды

Консоль — часть рантайма, а не игры. Она владеет текстом, историей, курсором
и отрисовкой, но не знает игровых команд. Встроенные команды собраны в
`CommandRegistry` с описаниями и tab-автодополнением; неизвестные имена
делегируются в `Game::command`.

Встроенные команды:

- `help`, `clear`, `quit` / `exit`, `backend`, `screenshot`;
- `camera.list`, `camera.select`, `camera.next`, `camera.prev`;
- `camera.spawn`, `camera.remove`, `camera.rename`, `camera.info`,
  `camera.set`;
- `vsync.on`, `vsync.off`, `vsync.adaptive`, `vsync.mailbox`,
  `vsync.toggle`, `vsync.current`.

Игра описывает свои команды в `Game::command_help` и обрабатывает их в
`Game::command`, получая `CommandContext { name, args, world }`. Возврат
`Some(Ok(..))` печатает ответ, `Some(Err(..))` — ошибку, `None` — «unknown
command».

Консоль открывается по `F1` или `` ` ``, освобождает курсор и забирает
текстовый ввод. `Enter` выполняет строку, `↑`/`↓` листают историю, `Tab`
дополняет команду по префиксу, `Esc` закрывает консоль и возвращает захват
курсора, если он был включён.

---

## 9. Добавление бэкенда

На примере нового API:

1. Создать `crates/<api>`, зависящий от `scene`, `assets`, `ui`, `render` и
   выбранной обёртки над API. Не подключать `winit`.
2. Реализовать `render::Renderer`. Конструктор принимает `WindowSurface` и
   `&Scene`, как `VulkanRenderer::new` и `OpenGlRenderer::new`.
3. Добавить вариант в `BackendKind`:

   ```rust
   pub enum BackendKind {
       Vulkan,
       OpenGl,
       // новый вариант
   }
   ```

4. Добавить опциональную зависимость и feature в `crates/runtime/Cargo.toml`:

   ```toml
   [features]
   my_api = ["dep:my_api"]

   [dependencies]
   my_api = { path = "../my_api", optional = true }
   ```

5. Добавить arm в `runtime/src/backend.rs`:

   ```rust
   #[cfg(feature = "my_api")]
   BackendKind::MyApi => Ok(Box::new(my_api::MyApiRenderer::new(surface, scene)?)),
   ```

6. Игра выбирает бэкенд без изменения кода:

   ```rust
   fn config(&self) -> GameConfig {
       GameConfig::default().with_backend(BackendKind::MyApi)
   }
   ```

Fallback-arm в фабрике вернёт понятную ошибку, если бэкенд не включён в
сборку.

---

## 10. Добавление игры

1. Создать крейт, который зависит **только** от `runtime`:

   ```toml
   [dependencies]
   runtime = { path = "../soldatik3D/crates/runtime" }
   anyhow = "1"
   ```

2. Реализовать `Game`:

   ```rust
   use runtime::{glam::Vec3, run, Camera, CameraRig, Game, GameConfig, Scene, UpdateContext, World};

   struct MyGame;

   impl Game for MyGame {
       fn config(&self) -> GameConfig {
           GameConfig::default()
       }

       fn create_world(&mut self) -> World {
           let scene = Scene::new(Default::default());
           let mut cameras = CameraRig::empty();
           cameras.spawn("player", Camera::at(Vec3::new(0.0, 2.0, 8.0)));
           cameras.spawn("overview", Camera::at(Vec3::new(20.0, 25.0, 30.0)));
           World::with_cameras(scene, cameras)
       }

       fn update(&mut self, ctx: &mut UpdateContext<'_>) {
           let camera = ctx.world.active_camera_mut().unwrap();
           camera.camera.yaw += ctx.dt;
       }
   }

   fn main() -> anyhow::Result<()> {
       run(MyGame)
   }
   ```

3. Использовать реэкспорты фасада: `runtime::glam`, `runtime::winit`,
   `runtime::Input`, `runtime::UiDrawList` и типы из `runtime::prelude`.
4. Не подключать `winit`, `ash`, `glow` или бэкенды напрямую — это нарушит
   переносимость и смену графического API.
5. При необходимости добавить команды через `Game::command_help` и
   `Game::command`; камеры и мир уже доступны в `CommandContext`.

---

## 11. Соглашения и ограничения

**Соглашения**

- Публичные типы документируются коротко и по делу; платформенные типы не
  протекают в `scene` или `ui`.
- Ошибки инициализации и кадра возвращаются как `anyhow::Result`.
- Логи бэкендов имеют префикс `[engine-vulkan]` / `[engine-opengl]`,
  рантайма — `[engine]`.
- Форматирование: `cargo fmt --all`; линт:
  `cargo clippy --workspace --all-targets` без предупреждений.

**Текущие ограничения**

- Один материал и одна сцена на всё время жизни рендерера; динамическая
  подгрузка мешей отсутствует.
- `MAX_FRAMES_IN_FLIGHT = 1`; многокадровая подготовка — следующий шаг.
- Нет MSAA, TAA и постэффектов, зависящих от времени (motion blur).
- SSAO считается в половинном разрешении и умножает весь HDR-результат;
  разделение direct/ambient — запланировано.
- UI — immediate-mode список вершин без виджет-фреймворка.
- Встроенный шрифт только ASCII (заглавные и строчные); кириллица требует
  своего атласа.
- Видимого skybox нет: процедурное окружение используется только для IBL.

**Диагностика**

- `SOLDATIK3D_BACKEND=vulkan|opengl` — выбор бэкенда.
- `SOLDATIK3D_VSYNC=on|off|adaptive|mailbox` — режим VSync.
- `SOLDATIK3D_MAX_FRAMES=<n>` — корректно отрендерить `n` кадров и выйти
  (smoke-тест, в том числе для CI).
- `SOLDATIK3D_COMMANDS="cmd1;cmd2"` — выполнить команды консоли после старта.
- `SOLDATIK3D_CONSOLE=1` — открыть консоль при старте.
- `SOLDATIK3D_DEBUG_VIEW=normals|depth` — стартовый режим отладки демо-игры.
- `SOLDATIK3D_VALIDATION=1` — включить Vulkan validation layers.
- `F12` или команда `screenshot` — сохранить PNG текущего кадра.
- В debug-сборках Vulkan включается `VK_LAYER_KHRONOS_validation`, если слой
  установлен.

---

## 12. Платформы, CI и релизы

Поддерживаются только Linux, Windows и Android. `platform` не содержит
веток macOS/iOS/WebAssembly, а `TargetOS` ограничен этими тремя вариантами
плюс `Unknown`.

GitHub Actions:

- `.github/workflows/ci.yml` — на каждый push и pull request собирает движок
  (`runtime`) и игру (`demo`) **отдельными Cargo-командами**, запускает
  clippy/тесты/rustdoc и упаковывает артефакты.
- `.github/workflows/release.yml` — при публикации GitHub Release (включая
  prerelease) собирает release-версии для Linux, Windows (MSVC) и Android
  (arm64-v8a + armeabi-v7a) и прикрепляет архивы к релизу.

Android собирается с OpenGL ES 3.0. Нативные зависимости Vulkan-крейта
(`ash`, `shaderc`) находятся под `cfg(not(target_os = "android"))`, поэтому
`cargo build --workspace` под Android не собирает `shaderc`. Ветка Vulkan в
`runtime::backend` также включается только для не-Android целей. Демо
собирается как `cdylib` (`libdemo.so`) и экспортирует `android_main` для
`android.app.NativeActivity`; отдельного Android-исполняемого файла нет.

Подробности о job'ах, NDK, локальной кросс-компиляции и переменных
окружения — в [`docs/CI.md`](CI.md).
