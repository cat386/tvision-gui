//! # tvision-gui
//!
//! Turbo Vision в **графическом окне**. Это та же библиотека
//! [tvision-rs](../tvision/index.html) — те же `Program`, `View`, `Group`,
//! `Window`, `Dialog`, меню, редактор, файловые диалоги и т.д., — но вместо
//! терминала программа рисуется в собственном окне ОС:
//!
//! * Windows (Win32), macOS (Cocoa), Linux/BSD (X11 и Wayland);
//! * сетка символов рисуется шрифтом TrueType из системы (Consolas, Menlo,
//!   DejaVu Sans Mono ...), рамки и блоки псевдографики (`─│┌╔═░▒▓█▀▄`)
//!   рисуются программно — они стыкуются без зазоров при любом размере шрифта;
//! * 16 цветов BIOS (палитру можно заменить), мигающий курсор;
//! * мышь, колесо, изменение размера окна, HiDPI, полноэкранный режим;
//! * системный буфер обмена (через tvision-rs).
//!
//! ## Как перевести программу с tvision-rs на tvision-gui
//!
//! Код представлений и обработчиков не меняется. Нужно только:
//!
//! 1. в `Cargo.toml` добавить зависимость `tvision-gui`;
//! 2. заменить `use tvision::*;` на `use tvision_gui::*;`
//!    (крейт реэкспортирует всё содержимое tvision-rs);
//! 3. вместо `Program::with_terminal(menu, status)` писать
//!    `Program::with_window(menu, status)`, а вместо
//!    `platform::default_backend()?` — `Box::new(GuiBackend::new()?)`;
//! 4. обернуть `main` в [`run`] — это обязательно для macOS, где окно может
//!    жить только в главном потоке:
//!
//! ```no_run
//! use tvision_gui::*;
//!
//! fn app_main() -> std::io::Result<()> {
//!     let menu = Menu::new().submenu("~F~ile", Menu::new().item_key("E~x~it", cm::QUIT, kb::ALT_X, "Alt-X"));
//!     let mut app = Program::with_window(Some(menu), None)?;
//!     struct App;
//!     impl AppHandler for App {}
//!     app.run(&mut App);
//!     Ok(())
//! }
//!
//! fn main() -> std::io::Result<()> {
//!     tvision_gui::run(GuiConfig::new("Моя программа").size(80, 25), app_main)
//! }
//! ```
//!
//! На Windows и Linux шаг 4 можно пропустить: `with_window` сам запустит окно
//! в отдельном потоке. Но с `run` программа переносима на все три ОС.
//!
//! ## Как это устроено
//!
//! ```text
//!   главный поток (окно)                     поток программы (tvision)
//!   ─────────────────────                    ─────────────────────────
//!   winit: клавиатура, мышь  ── RawEvent ──▶  GuiBackend::read()
//!   размер окна                               Program::run() — как обычно
//!   Renderer + softbuffer    ◀── кадр ─────   GuiBackend::flush(ScreenBuffer)
//! ```
//!
//! Программа Turbo Vision по-прежнему работает «в ячейках»: она получает
//! события и отдаёт готовый экранный буфер [`ScreenBuffer`]. Окно переводит
//! пиксели в ячейки (для мыши) и ячейки в пиксели (для отрисовки).
//!
//! ## Клавиши самого окна
//!
//! * **Alt+Enter** — полноэкранный режим;
//! * **Ctrl+=** / **Ctrl+-** / **Ctrl+0** — крупнее / мельче / исходный шрифт;
//! * закрытие окна крестиком посылает программе **Alt+X** (стандартный выход
//!   Turbo Vision); повторное закрытие завершает процесс принудительно.
//!
//! ## Язык надписей
//!
//! Язык встроенных надписей tvision-rs (`locale::set_language`) хранится
//! отдельно для каждого потока. Программа под [`run`] работает в своём
//! потоке, поэтому язык нужно включать **внутри** `app_main`, а не в `main`.
//!
//! ## Переменные окружения
//!
//! * `TVISION_GUI_FONT` — путь к файлу шрифта (.ttf/.otf/.ttc);
//! * `TVISION_GUI_FONT_SIZE` — размер шрифта в логических пикселях.

// Вся библиотека tvision-rs доступна через этот крейт, чтобы программе
// хватало одной строки `use tvision_gui::*;`.
pub use tvision;
pub use tvision::*;

mod backend;
mod boxdraw;
mod config;
mod font;
mod keys;
mod render;
mod app_window;

pub use backend::{run, set_screen_size, set_title, GuiBackend, ProgramExt};
pub use config::{GuiConfig, VGA_PALETTE};
