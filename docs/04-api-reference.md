# 4. Справочник API

Публичный интерфейс tvision-gui небольшой — всё остальное приходит из
tvision-rs.

```rust
pub use tvision;                 // сам крейт tvision-rs
pub use tvision::*;              // всё его содержимое

pub fn run<F, R>(cfg: GuiConfig, app_main: F) -> R;
pub fn set_title(title: &str);
pub fn set_screen_size(cols: u16, rows: u16);

pub struct GuiBackend;           // impl tvision::platform::Backend
pub trait ProgramExt;            // Program::with_window, with_window_config
pub struct GuiConfig;
pub const VGA_PALETTE: [u32; 16];
```

---

## `run`

```rust
pub fn run<F, R>(cfg: GuiConfig, app_main: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static;
```

Запускает программу в графическом окне.

1. Создаёт цикл событий окна в **текущем** потоке (он должен быть главным).
2. Запускает `app_main` в новом потоке с именем `tvision-app`.
3. Создаёт окно (пока скрытое) и сообщает размер экрана первому
   `GuiBackend::new()`, вызванному внутри `app_main`.
4. Показывает окно, когда программа нарисует первый кадр.
5. Когда `app_main` завершается (в том числе паникой), закрывает окно и
   возвращает её результат. Паника из `app_main` пробрасывается дальше.

Тип результата любой: обычно `std::io::Result<()>`.

Ограничения:

* вызывать один раз за время работы процесса и только из главного потока
  (на macOS это требование ОС; цикл событий winit нельзя создать дважды);
* внутри `app_main` создаётся одна программа (`Program`) на окно;
* замыкание и всё, что оно захватывает, должны быть `Send`.

Ошибки:

* нет графического сеанса (например, Linux по SSH без X11 и Wayland) — цикл
  событий создать нельзя, `run` завершается паникой с сообщением
  «tvision-gui: не удалось создать цикл событий окна». Если программа должна
  работать и в таком окружении, проверьте его заранее и запустите
  терминальную версию (см. [раздел 2](02-porting.md#программа-которая-умеет-и-окно-и-терминал));
* цикл событий есть, но окно создать не удалось — `GuiBackend::new()` внутри
  `app_main` возвращает `io::Error` с описанием, `run` возвращает результат
  `app_main`.

---

## Трейт `ProgramExt`

Дополняет `tvision::Program` конструкторами для окна — парой к
`Program::with_terminal`.

```rust
pub trait ProgramExt: Sized {
    fn with_window(menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self>;
    fn with_window_config(cfg: GuiConfig, menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self>;
}
impl ProgramExt for Program { ... }
```

| Метод | Что делает |
|-------|-----------|
| `Program::with_window(menu, status)` | то же, что `Program::new(Box::new(GuiBackend::new()?), menu, status)` |
| `Program::with_window_config(cfg, menu, status)` | то же через `GuiBackend::with_config(cfg)` |

Ошибки — те же, что у `GuiBackend::new()`.

---

## `GuiBackend`

```rust
pub struct GuiBackend { /* скрытые поля */ }

impl GuiBackend {
    pub fn new() -> io::Result<Self>;
    pub fn with_config(cfg: GuiConfig) -> io::Result<Self>;
    pub fn set_config(cfg: GuiConfig);
}

impl tvision::platform::Backend for GuiBackend { ... }
impl Drop for GuiBackend { ... }
```

«Экран» Turbo Vision в окне. Передаётся в `Program::new` вместо терминала.

### `GuiBackend::new()`

Подключается к окну и ждёт, пока оно будет создано (функция возвращается,
когда известен размер экрана в символах).

* Внутри `run` — подключается к окну, которое создал `run`.
* Без `run` на Windows и Linux — запускает поток `tvision-window` со своим
  циклом событий и открывает окно с настройками из `set_config` (или
  `GuiConfig::default()`).
* Без `run` на macOS — возвращает ошибку `ErrorKind::Unsupported`.

Ошибки: не удалось создать цикл событий или окно; окно закрылось до запуска
программы.

### `GuiBackend::with_config(cfg)`

Если программа не обёрнута в `run`, запоминает `cfg` (как `set_config`) и
вызывает `new()`. Внутри `run` настройки игнорируются.

### `GuiBackend::set_config(cfg)`

Задаёт настройки окна, которое откроет следующий `new()` без `run`.

### Поведение методов `Backend`

| Метод | В окне |
|-------|--------|
| `size()` | текущий размер сетки в символах |
| `read(timeout)` | ждёт событие окна (клавиша, мышь, изменение размера) не дольше `timeout` |
| `flush(screen, cursor, insert)` | передаёт кадр окну; одинаковые кадры подряд не передаются |
| `suspend()` / `resume()` | прячут и снова показывают окно (для `dos_shell`) |
| `system_clipboard()` | `true` — включается системный буфер обмена tvision-rs |

Если окно уже уничтожено, а программа продолжает читать события, `read`
возвращает Alt+X (просьба выйти); если программа не выходит долго
(около 50 попыток), процесс завершается.

При уничтожении (`Drop`) `GuiBackend` закрывает окно.

---

## `set_title`

```rust
pub fn set_title(title: &str);
```

Меняет заголовок окна. Можно вызывать из любого потока в любой момент после
создания окна; до этого вызов ничего не делает.

```rust
tvision_gui::set_title(&format!("Редактор — {}", file.display()));
```

---

## `set_screen_size`

```rust
pub fn set_screen_size(cols: u16, rows: u16);
```

Меняет размер окна так, чтобы при текущем шрифте в него помещалось ровно
`cols × rows` символов — аналог переключения 25/43/50 строк в DOS.
Программа получит обычное событие изменения размера (как при изменении
размера окна мышью).

* Значения меньше 1 заменяются на 1.
* В полноэкранном режиме вызов ничего не делает.
* Оконная система может не дать окну стать больше экрана — тогда строк будет
  меньше запрошенного.

```rust
CM_LINES50 => tvision_gui::set_screen_size(app.ctx.screen_size.x as u16, 50),
```

---

## `GuiConfig`

Подробно — в [разделе 3](03-configuration.md).

```rust
#[derive(Clone, Debug)]
pub struct GuiConfig {
    pub title: String,
    pub cols: u16,
    pub rows: u16,
    pub font_size: f32,
    pub font_path: Option<PathBuf>,
    pub palette: [u32; 16],
    pub cursor_blink: bool,
    pub resizable: bool,
    pub fullscreen: bool,
}

impl Default for GuiConfig { ... }

impl GuiConfig {
    pub fn new(title: &str) -> Self;
    pub fn size(self, cols: u16, rows: u16) -> Self;
    pub fn font_size(self, px: f32) -> Self;        // ограничивается 4..200
    pub fn font(self, path: impl Into<PathBuf>) -> Self;
    pub fn palette(self, palette: [u32; 16]) -> Self;
    pub fn cursor_blink(self, on: bool) -> Self;
    pub fn resizable(self, on: bool) -> Self;
    pub fn fullscreen(self, on: bool) -> Self;
}
```

---

## `VGA_PALETTE`

```rust
pub const VGA_PALETTE: [u32; 16];
```

Стандартные цвета текстового режима VGA в формате `0x00RRGGBB`, в порядке
цветов BIOS (0 — чёрный, 1 — синий ... 15 — белый). Таблица — в
[разделе 3](03-configuration.md#палитра).

---

## Что приходит из tvision-rs

Через `use tvision_gui::*;` доступно всё содержимое tvision-rs, в том числе:

| Что | Где описано |
|-----|-------------|
| `Program`, `AppHandler`, `Event`, `KeyEvent`, `kb`, `cm` | документация tvision-rs |
| `View`, `ViewState`, `Group`, `Window`, `Dialog`, `Desktop` | документация tvision-rs |
| `Menu`, `MenuBar`, `StatusLine`, `StatusDef`, `StatusItem` | документация tvision-rs |
| `Button`, `InputLine`, `Cluster`, `ListBox`, `ScrollBar`, `Label`, `StaticText` | документация tvision-rs |
| `Editor`, `FileEditor`, `EditWindow`, `FileDialog`, `ChDirDialog` | документация tvision-rs |
| `HelpFile`, `HelpWindow`, `ColorDialog`, `Outline` | документация tvision-rs |
| `message_box`, `input_box`, валидаторы | документация tvision-rs |
| `locale::set_language` | см. [раздел 2, шаг 5](02-porting.md#шаг-5-язык-надписей) |
| `platform::Backend`, `platform::HeadlessBackend` | трейт экрана и тестовый экран |
