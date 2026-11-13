# soldatik3D

![Rust](https://img.shields.io/badge/Rust-2024-ed6b1f?logo=rust)
![Vulkan](https://img.shields.io/badge/Vulkan-1.2-b21b1b?logo=vulkan)
![OpenGL](https://img.shields.io/badge/OpenGL-3.3%20core-5586a4?logo=opengl)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

**soldatik3D** — модульный игровой движок на Rust с двумя полноценными
графическими бэкендами: Vulkan 1.2 и OpenGL 3.3 core. Репозиторий содержит
сам движок и демонстрационную игру, которая использует только публичный API
`runtime` и не знает, какой графический API выбран.

Движок собран из небольших крейтов с одной ответственностью: ассеты, сцена и
камеры, UI, контракт рендерера, платформа, два бэкенда и рантайм-фасад.
Демо лежит вне `crates/`, поэтому каталог `crates/` содержит только код
движка.

![Сцена демо-игры](docs/screenshot.jpg)

---

## Возможности

- **Два бэкенда с одинаковым результатом**: Vulkan 1.2 (`ash` + `shaderc`) и
  OpenGL 3.3 core (`glutin` + `glow`). Переключение одной переменной окружения
  или одним полем `GameConfig`.
- **Forward PBR-рендеринг**: metallic/roughness, IBL из процедурного
  environment cubemap, до восьми точечных источников света.
- **Тени**: ортографическая shadow map 2048² с фильтрацией.
- **Постобработка**: SSAO половинного разрешения, bright-pass bloom с
  разделяемым гауссовым блюром, ACES tone mapping и гамма-коррекция.
  HDR-пайплайн использует `R16G16B16A16_SFLOAT`.
- **Отсечение по пирамиде видимости** для камеры и источника тени.
- **Камеры как часть движка**: `World` и `CameraRig` позволяют спавнить
  именованные камеры, выбирать активную, переключать следующую/предыдущую и
  удалять их. Рантайм всегда рендерит через активную камеру.
- **Встроенная консоль**: команды камер, диагностика, история ввода и
  расширение командами игры. Открывается по `F1` или `` ` ``.
- **Immediate-mode UI** со встроенным bitmap-шрифтом: панели, кнопки, статус,
  перекрестие.
- **Кроссплатформенное окно и ввод** через `winit`, изолированные в
  `platform`; бэкенды получают только raw handles и размер поверхности.
- **Демо-игра** с процедурным уровнем, семью камерами, освещением,
  коллизиями, FPS-контроллером и debug-видами.

---

## Структура репозитория

```text
soldatik3D/
├── crates/
│   ├── assets/       # текстуры, bitmap-шрифт, environment cubemap
│   ├── scene/        # Camera, CameraRig, World, Mesh, Material, Light, Scene
│   ├── ui/           # UiDrawList и UiVertex для immediate-mode UI
│   ├── render/       # Renderer, BackendKind, FrameData, WindowSurface
│   ├── platform/     # winit: окно, ввод, raw handles
│   ├── vulkan/       # Vulkan-бэкенд, пайплайны и GLSL-шейдеры
│   ├── opengl/       # OpenGL-бэкенд, FBO-пайплайн и GLSL-шейдеры
│   └── runtime/      # фасад: Game trait, event loop, World, консоль, фабрика
├── examples/
│   └── demo/         # демонстрационная игра (package `demo`)
├── docs/
│   ├── ARCHITECTURE.md
│   ├── screenshot.jpg
│   └── console.jpg
└── Cargo.toml        # workspace
```

Граф зависимостей:

```text
examples/demo ──► runtime ──► scene ──► assets
                     │           ▲
                     ├─► ui ─────┤
                     ├─► render  │
                     ├─► platform│
                     ├─► vulkan ─┘ (optional feature)
                     └─► opengl ── (optional feature)
```

`demo` зависит только от `runtime` и `anyhow`. `vulkan` и `opengl` не знают о
`winit` и получают `render::WindowSurface` с raw handles. Оба бэкенда
реализуют один трейт `render::Renderer`. Подробности —
в [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## Требования

- **Rust 1.88+** (edition 2024).
- **Vulkan 1.2+**, Vulkan loader и установленный ICD-драйвер.
- Для Vulkan-бэкенда `shaderc` компилирует GLSL во время запуска: нужны
  C++17-компилятор, CMake и Python 3. В debug-сборках подключается
  `VK_LAYER_KHRONOS_validation`, если слой установлен.
- **OpenGL 3.3 core** и рабочий EGL/GLX (Linux), WGL (Windows) или CGL
  (macOS). На Linux достаточно драйвера Mesa или проприетарного драйвера.

Arch Linux:

```bash
sudo pacman -S rust vulkan-icd-loader vulkan-validation-layers cmake python mesa
```

---

## Быстрый старт

```bash
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D

# debug-сборка, оба бэкенда включены
cargo run -p demo

# release-сборка
cargo run --release -p demo
```

### Выбор графического бэкенда

По умолчанию используется Vulkan. Бэкенд можно выбрать переменной окружения:

```bash
SOLDATIK3D_BACKEND=vulkan cargo run -p demo
SOLDATIK3D_BACKEND=opengl cargo run -p demo
```

Или в коде игры:

```rust
use runtime::{BackendKind, GameConfig};

let config = GameConfig::default().with_backend(BackendKind::OpenGl);
```

`runtime` включает оба бэкенда через features `vulkan` и `opengl` (обе
включены по умолчанию). Если движок используется как зависимость, ненужные
бэкенды отключаются в манифесте:

```toml
runtime = { path = "../soldatik3D/crates/runtime", default-features = false, features = ["opengl"] }
```

Выбранный, но не скомпилированный бэкенд даёт понятную ошибку при запуске.

---

## API движка

Игра реализует `Game`, создаёт `World` и получает его каждый кадр. Камеры
спавнятся и выбираются через движок; рендер, ввод, окно и консоль остаются
внутри `runtime`.

```rust
use runtime::{glam::Vec3, run, Camera, CameraRig, Game, GameConfig, Scene, UpdateContext, World};

struct MyGame;

impl Game for MyGame {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("My game")
    }

    fn create_world(&mut self) -> World {
        let scene = Scene::new(Default::default());
        let mut cameras = CameraRig::empty();
        cameras.spawn("player", Camera::at(Vec3::new(0.0, 2.0, 8.0)));
        cameras.spawn("overview", Camera::at(Vec3::new(20.0, 25.0, 30.0)));
        World::with_cameras(scene, cameras)
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        // ctx.world.active_camera() / active_camera_mut()
        // ctx.world.spawn_camera(...), select_camera(...), select_next_camera()
    }
}

fn main() -> anyhow::Result<()> {
    run(MyGame)
}
```

`UpdateContext` также содержит `dt`, `input`, `ui`, `stats`, `viewport` и
`console_open`. Для собственных команд игра реализует `Game::command` и
`Game::command_help`.

### Камеры и консоль

| Команда | Действие |
| --- | --- |
| `camera.list` | список камер с ID, именами и позициями |
| `camera.select <name\|id>` | сделать камеру активной (`overview`, `cam-6`, `6`) |
| `camera.next` / `camera.prev` | переключить камеру по кругу |
| `camera.spawn <name> [x y z] [yaw] [pitch]` | создать камеру |
| `camera.remove <name\|id>` | удалить камеру (кроме последней) |
| `camera.rename <name>` | переименовать активную камеру |
| `camera.info` | позиция, углы, FOV и диапазон активной камеры |
| `camera.set x y z [yaw] [pitch]` | переместить активную камеру |
| `help`, `clear`, `quit`, `backend` | справка, очистка, выход, текущий бэкенд |

Демо добавляет команды `debug.lit`, `debug.normals`, `debug.depth`,
`debug.cycle`, `player.camera`, `player.teleport`, `player.fly` и
`player.reset`.

![Консоль и список камер](docs/console.jpg)

---

## Управление в демо

| Клавиши | Действие |
| --- | --- |
| `WASD` | движение |
| `Shift` | бег |
| `Space` | прыжок, в полёте — вверх |
| `Ctrl` | вниз в режиме полёта |
| `F` | включить/выключить полёт |
| `Мышь` | обзор |
| `Tab` | освободить/захватить курсор |
| `F1` / `` ` `` | открыть/закрыть консоль |
| `Enter` | выполнить команду |
| `↑` / `↓` | история команд |
| `Esc` | закрыть консоль, освободить курсор или выйти |
| `F2` / `F3` / `F4` | lit / normals / depth |

Игрок управляет камерой `player`, только когда она активна. Через консоль
можно переключиться на пресеты `corridor`, `left-hall`, `right-hall`, `pool`,
`final-hall` и `overview`.

---

## Переменные окружения

| Переменная | Назначение |
| --- | --- |
| `SOLDATIK3D_BACKEND=vulkan\|opengl` | выбрать графический бэкенд |
| `SOLDATIK3D_MAX_FRAMES=<n>` | отрендерить `n` кадров и корректно завершиться |
| `SOLDATIK3D_COMMANDS="cmd1;cmd2"` | выполнить команды консоли после старта |
| `SOLDATIK3D_CONSOLE=1` | открыть консоль при старте |
| `SOLDATIK3D_DEBUG_VIEW=normals\|depth` | стартовый debug-вид демо |

Пример автоматической проверки камер и консоли:

```bash
SOLDATIK3D_BACKEND=opengl \
SOLDATIK3D_CONSOLE=1 \
SOLDATIK3D_COMMANDS="camera.list;camera.select overview;camera.info" \
SOLDATIK3D_MAX_FRAMES=60 \
cargo run -p demo
```

---

## Разработка

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo doc --workspace --no-deps --open
```

Smoke-тест обоих бэкендов без ручного закрытия окна:

```bash
SOLDATIK3D_BACKEND=vulkan SOLDATIK3D_MAX_FRAMES=3 cargo run -p demo
SOLDATIK3D_BACKEND=opengl SOLDATIK3D_MAX_FRAMES=3 cargo run -p demo
```

Шейдеры Vulkan компилируются `shaderc` при запуске из `crates/vulkan/shaders/`.
OpenGL-шейдеры лежат в `crates/opengl/shaders/` и также встроены в бинарник.

---

## Лицензия

Проект распространяется под лицензией **GNU General Public License v3.0
(GPLv3)**. Подробности — в файле [LICENSE](LICENSE).
