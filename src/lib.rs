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
//!    `platform::default_backend()?` — `GuiBackend::boxed()?`;
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
//! * закрытие окна крестиком посылает программе команду **`cm::QUIT`**
//!   (как пункт меню «Выход»); если программа не отвечает больше 2 секунд,
//!   повторное закрытие завершает процесс принудительно.
//!
//! ## Диалог настроек
//!
//! [`settings_dialog`] — стандартный диалог: тема оформления ([`themes`]),
//! моноширинный шрифт системы и размер шрифта. Изменения видны сразу, выбор
//! запоминается между запусками.
//!
//! ## Файл настроек
//!
//! При выходе тема, шрифт, размер шрифта, размер и положение окна
//! записываются в `ИМЯ.conf` рядом с программой (если там писать нельзя —
//! в профиль пользователя), при запуске читаются оттуда. Нет файла —
//! программа запускается с настройками по умолчанию из [`GuiConfig`].
//! См. [`GuiSettings`], [`GuiConfig::remember_settings`].
//!
//! ## Язык надписей
//!
//! Язык встроенных надписей tvision-rs (`locale::set_language`) хранится
//! отдельно для каждого потока. Программа под [`run`] работает в своём
//! потоке, поэтому `run` сам переносит в него язык, выбранный в `main` до
//! вызова `run`. Вызывать `set_language` можно и в `main`, и в `app_main`.
//!
//! ## Примеры tvision-rs в окне
//!
//! Все примеры tvision-rs (кроме утилиты `tvhc`) есть и здесь — в каталоге
//! `examples/`. Они **сгенерированы** из исходников tvision-rs примером
//! `sync_examples`: `cargo run --example sync_examples` пересоздаёт их после
//! обновления tvision-rs. Код примеров не меняется — заменяются только
//! создание программы (`with_terminal` → `with_window`) и `main`.
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
mod settings;
mod app_window;

pub use backend::{run, set_screen_size, set_title, GuiBackend, ProgramExt};
pub use config::{GuiConfig, GRUVBOX_PALETTE, VGA_PALETTE};
pub use font::{monospace_fonts, FontInfo};
pub use settings::{
    apply_saved_settings, apply_theme, current_settings, find_theme, set_font, set_window_palette, settings_dialog, themes, GuiSettings, Theme, WindowGeometry,
    FONT_SIZE_MAX, FONT_SIZE_MIN,
};
