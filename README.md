# soldatik3D

![Rust](https://img.shields.io/badge/Rust-2024-ed6b1f?logo=rust)
![Vulkan](https://img.shields.io/badge/Vulkan-1.2-b21b1b?logo=vulkan)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

**soldatik3D** — модульный игровой движок на Rust с Vulkan-рендерером, ориентированный
на реалистичную графику и высокую производительность. Репозиторий содержит сам движок
и демонстрационную игру, которая использует только его публичный API.

Движок разделён на независимые крейты: платформа, математика/сцена, контракт рендерера
и конкретный Vulkan-бэкенд. Это позволяет добавлять новые графические API и игры, не
меняя код уже написанных игр.

![Сцена демо-игры](docs/screenshot.jpg)

---

## Возможности

- **Forward PBR-рендеринг**: metallic/roughness, IBL, точечные источники света.
- **Тени**: ортографическая shadow map с фильтрацией и барьером видимости.
- **Постобработка**: SSAO, bright-pass bloom с разделяемым блюром, ACES tone mapping,
  гамма-коррекция. HDR-пайплайн использует `R16G16B16A16_SFLOAT`.
- **Отсечение по пирамиде видимости** для камеры и источника тени.
- **Immediate-mode UI** со встроенным атласом шрифта: панели, кнопки, перекрестие.
- **Кроссплатформенное окно и ввод** через `winit`, изолированные в `engine-platform`.
- **Смена бэкенда через конфигурацию**: `GameConfig::backend`, без изменения игры.
- **Демо-игра** с процедурным уровнем, освещением, коллизиями и FPS-контроллером.

---

## Структура репозитория

```text
soldatik3D/
├── crates/
│   ├── engine-core/      # математика, камера, сцена, материалы, свет, UI
│   ├── engine-render/    # Renderer trait, BackendKind, WindowSurface
│   ├── engine-platform/  # winit: окно, ввод, raw-handle для рендерера
│   ├── engine-vulkan/    # Vulkan-бэкенд и встроенные шейдеры
│   ├── engine/           # публичный фасад, Game trait, event loop, backend factory
│   └── game/             # демонстрационная игра
├── docs/ARCHITECTURE.md  # устройство движка, поток кадра, добавление бэкенда/игры
└── Cargo.toml            # workspace
```

Направление зависимостей строгое:

```text
game ──► engine ──► engine-platform ──► engine-render ──► engine-core
                 ├─► engine-render
                 └─► engine-vulkan (optional, feature "vulkan") ──► engine-render
```

`game` не зависит от `winit` и Vulkan напрямую: только от `engine`. Подробнее —
в [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## Требования

- **Rust** 1.88+ (edition 2024, let-chains).
- **Vulkan 1.2+** и установленный ICD-драйвер.
- **Vulkan loader** (`libvulkan`).
- Для сборки `shaderc` (компиляция GLSL во время запуска): C++17-компилятор, CMake и
  Python 3. В debug-сборках дополнительно используется слой
  `VK_LAYER_KHRONOS_validation`, если он установлен.

На Arch Linux:

```bash
sudo pacman -S rust vulkan-icd-loader vulkan-validation-layers cmake python
```

---

## Быстрый старт

```bash
git clone https://github.com/finesoldat1k/soldatik3D
cd soldatik3D
cargo run --release -p game
```

Для разработки используется обычная debug-сборка (в ней включён слой валидации):

```bash
cargo run -p game
```

Полезные команды:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo check --workspace
cargo doc --workspace --no-deps --open
```

---

## Управление в демо

| Клавиши | Действие |
| --- | --- |
| `WASD` | движение |
| `Shift` | бег |
| `Space` | прыжок (в полёте — вверх) |
| `Ctrl` | вниз в режиме полёта |
| `F` | включить/выключить полёт |
| `Мышь` | обзор |
| `Tab` | освободить/захватить курсор |
| `F2` / `F3` / `F4` | lit / normals / depth |
| `Esc` | освободить курсор или выйти |

---

## Переменные окружения

| Переменная | Назначение |
| --- | --- |
| `SOLDATIK3D_MAX_FRAMES=<n>` | отрендерить `n` кадров и корректно завершиться (smoke-тест) |
| `SOLDATIK3D_DEBUG_VIEW=normals\|depth` | стартовый debug-view демо-игры |
| `RUST_LOG` | зарезервировано для логирования |

Пример headless-проверки без ручного закрытия окна:

```bash
SOLDATIK3D_MAX_FRAMES=300 cargo run -p game
```

---

## Лицензия

Проект распространяется под лицензией **GNU General Public License v3.0 (GPLv3)**.
Подробности — в файле [LICENSE](LICENSE).
