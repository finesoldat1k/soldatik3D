# Сборка и CI

Документ описывает, как Soldatik3D собирается локально и в GitHub Actions.
Каноническая конфигурация находится в
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) и
[`.github/workflows/release.yml`](../.github/workflows/release.yml).

---

## 1. Поддерживаемые платформы

| Платформа | Архитектура | Бэкенды в CI | Артефакты |
| --- | --- | --- | --- |
| Linux | x86_64 | Vulkan + OpenGL | `soldatik3d-engine-linux-x86_64.tar.gz`, `soldatik3d-demo-linux-x86_64.tar.gz` |
| Windows | x86_64 (MSVC) | Vulkan + OpenGL | `soldatik3d-engine-windows-x86_64.zip`, `soldatik3d-demo-windows-x86_64.zip` |
| Android | arm64-v8a, armeabi-v7a | OpenGL ES | `soldatik3d-engine-android-<abi>.tar.gz`, `soldatik3d-demo-android-<abi>.tar.gz` |

macOS, iOS и WebAssembly в проект не входят. Платформенные ветки для них
удалены из `platform`, чтобы не поддерживать неработающие конфигурации.

---

## 2. Workflow `CI`

Запускается на **каждый push** (в любую ветку), на pull request и вручную
через `workflow_dispatch`.

| Job | Что делает |
| --- | --- |
| `linux` | `cargo fmt --check`, clippy, сборка движка, сборка демо, тесты, rustdoc, упаковка артефактов |
| `windows` | clippy, сборка движка и демо (MSVC), тесты, упаковка артефактов |
| `android` | matrix из arm64-v8a и armeabi-v7a: NDK r26c, сборка движка и демо под OpenGL ES, упаковка артефактов |

Движок и игра всегда собираются **отдельными командами**:

```bash
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
```

`runtime` — это библиотека движка, `demo` — игра. На Android включён только
OpenGL ES, потому что Vulkan-бэкенд не компилируется под Android (см. раздел
5). Артефакты движка на Android — `libruntime.rlib`, игры — нативная
библиотека `libdemo.so` (native-activity `cdylib` с экспортом `android_main`),
а не самостоятельный исполняемый файл.

Rust-кэш (`Swatinem/rust-cache`) хранит `target/` между запусками. Первая
сборка может быть долгой из-за `shaderc-sys`: он собирает C++-часть shaderc
из исходников, если в системе не найдена готовая библиотека.

---

## 3. Workflow `Release`

Запускается при **публикации GitHub Release**, включая prerelease
(`release: types: [published]`), и вручную через `workflow_dispatch`.

Порядок:

1. Собираются release-версии движка и демо для Linux, Windows (MSVC) и
   Android (arm64-v8a + armeabi-v7a).
2. Артефакты складываются в `dist/` и загружаются как GitHub Actions
   artifacts.
3. Job `publish` скачивает все артефакты и прикрепляет архивы к тому же
   GitHub Release через `softprops/action-gh-release`.
4. Флаг `prerelease` берётся из события: prerelease остаётся prerelease.

Чтобы выпустить версию:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Затем создайте GitHub Release из тега и опубликуйте его (можно отметить
`Set as a pre-release`). Workflow сам разложит бинарники по ассетам релиза.

Для ручного запуска release-сборки без публикации используйте
`Actions -> Release -> Run workflow`; job `publish` при этом пропускается,
а артефакты остаются в Actions.

---

## 4. Локальная сборка

### Linux

Зависимости (Debian/Ubuntu):

```bash
sudo apt-get install -y \
  build-essential pkg-config cmake ninja-build git python3 \
  libvulkan-dev libshaderc-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libgl1-mesa-dev libegl1-mesa-dev libgles2-mesa-dev
```

```bash
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
cargo test --locked --workspace --all-features
```

`libshaderc-dev` не обязателен: без него `shaderc-sys` соберёт библиотеку из
исходников.

### Windows

Нужны Rust stable с toolchain MSVC, Visual Studio Build Tools (C++), CMake,
Ninja и Python 3. Проще всего запускать те же команды в
`x64 Native Tools Command Prompt for VS`:

```powershell
cargo build --locked -p runtime --all-features
cargo build --locked -p demo --all-features
cargo test --locked --workspace --all-features
```

Кросс-сборка из Linux под `x86_64-pc-windows-gnu` поддерживается скриптом
`cross_build.sh windows`, но она собирает только OpenGL-вариант: C++-часть
shaderc сложно гарантированно собрать любым MinGW-тулчейном. Каноническая
Windows-сборка — нативная MSVC в CI.

### Android

Нужен NDK r26c (или новее) и Rust-таргеты:

```bash
export ANDROID_NDK_HOME="$HOME/android-ndk-r26c"
rustup target add aarch64-linux-android armv7-linux-androideabi

./cross_build.sh android arm64-v8a
./cross_build.sh android armeabi-v7a
```

Скрипт выставляет `CARGO_TARGET_<TRIPLE>_LINKER`, `CC_<TRIPLE>` и
`AR_<TRIPLE>` на компиляторы NDK (`aarch64-linux-android29-clang` /
`armv7a-linux-androideabi29-clang`), а также
`CMAKE_POLICY_VERSION_MINIMUM=3.5` для совместимости vendored-сборки `shaderc`
с CMake 4, и собирает движок и демо отдельно.
Результаты — в `dist/android-<abi>/<triple>/`: `libruntime.rlib` и
`libdemo.so`.

Те же переменные можно выставить вручную:

```bash
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=\
  "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export AR_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
cargo build --locked -p runtime --no-default-features --features opengl \
  --target aarch64-linux-android
```

---

## 5. Особенности Android

- Используется `winit` с feature `android-native-activity`; это не требует
  Jetpack GameActivity и Gradle-плагина.
- Демо собрано как `cdylib` (`libdemo.so`) и экспортирует
  `#[unsafe(no_mangle)] pub fn android_main(app: AndroidApp)`. Хост
  `android.app.NativeActivity` загружает библиотеку через
  `android.app.lib_name` и вызывает эту функцию на главном Android-потоке.
  Отдельного ELF-исполняемого файла `demo` на Android больше нет.
- Контекст создаётся через EGL как **OpenGL ES 3.0**. Desktop-шейдеры
  (`#version 330 core`) при компиляции переводятся в `#version 300 es` с
  добавлением precision-квалификаторов; HDR-таргеты используют `RGBA16F`
  только при наличии `EXT_color_buffer_half_float`/`EXT_color_buffer_float`,
  иначе откатываются на `RGBA8`.
- В `crates/vulkan/Cargo.toml` нативные зависимости (`ash`, `shaderc`, `png`
  и т. д.) находятся под `cfg(not(target_os = "android"))`. Поэтому
  `cargo build --workspace` под Android не тянет `shaderc`.
- В `runtime` ветка Vulkan-бэкенда включается только для не-Android целей;
  по умолчанию на Android выбирается OpenGL. Явный
  `SOLDATIK3D_BACKEND=vulkan` на Android вернёт понятную ошибку «backend is
  not available».
- Release-артефакт Android — `libdemo.so`, а не APK. Чтобы запустить демо,
  нужен APK-хост: activity `android.app.NativeActivity` (или её наследник),
  `android:hasCode="false"` в манифесте, `libdemo.so` в `lib/<abi>/` и
  meta-data `android.app.lib_name="demo"`. Без такого хоста библиотеку
  загрузить нельзя.

---

## 6. Переменные CI и кросс-компиляции

| Переменная | Назначение |
| --- | --- |
| `CARGO_TARGET_<TRIPLE>_LINKER` | линкер для целевого триплета |
| `CC_<TRIPLE>` / `AR_<TRIPLE>` | C-компилятор и архиватор для build-скриптов |
| `SHADERC_LIB_DIR` | каталог с готовой `libshaderc_combined` (необязательно) |
| `VULKAN_SDK` | Vulkan SDK; `shaderc-sys` попробует взять библиотеку оттуда |

Имена переменных с целевым триплетом бывают в двух формах: с дефисами
(`CC_aarch64-linux-android`) и с подчёркиваниями
(`CC_aarch64_linux_android`); `cc-rs` понимает обе. Дефисную форму нельзя
`export`-ить обычным способом — используйте
`env 'CC_aarch64-linux-android=...'` или запись в `$GITHUB_ENV`, как это
делает workflow. `cross_build.sh` использует форму с подчёркиваниями.

---

## 7. Диагностика

- `shaderc-sys` долго собирается? Установите `libshaderc-dev` (Linux) или
  Vulkan SDK (Windows), либо дождитесь заполнения Rust-кэша.
- Windows-job падает на C++/CMake? Проверьте, что в образе есть Ninja и
  Python 3; workflow использует `windows-latest`.
- Android: `failed to find tool "aarch64-linux-android-clang"` означает, что
  не выставлен `CARGO_TARGET_..._LINKER`. Проверьте `ANDROID_NDK_HOME` и
  наличие `toolchains/llvm/prebuilt/linux-x86_64/bin`.
- Release не прикрепил файлы? У job `publish` должны быть права
  `contents: write`; они заданы на уровне workflow.
