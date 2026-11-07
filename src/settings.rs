//! Стандартный диалог настроек окна: тема оформления, шрифт и его размер.
//!
//! ```ignore
//! const CM_SETTINGS: u16 = 200;
//! // в меню: .item("~Н~астройки...", CM_SETTINGS)
//! // в обработчике:
//! if ev.command() == Some(CM_SETTINGS) {
//!     tvision_gui::settings_dialog(app);
//!     ev.clear();
//! }
//! ```
//!
//! Изменения видны **сразу**, пока диалог открыт: выбранная в списке тема
//! перекрашивает программу, шрифт и размер перестраивают окно. «Отмена»
//! возвращает всё как было, «OK» оставляет выбранное и (если включено
//! [`GuiConfig::remember_settings`](crate::GuiConfig::remember_settings))
//! записывает его в файл настроек — при следующем запуске программа
//! откроется с тем же оформлением.
//!
//! Те же действия доступны и без диалога: [`apply_theme`], [`set_font`],
//! [`current_settings`], [`GuiSettings::load`] / [`GuiSettings::save`].

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tvision::color::{self, palettes, Attr};
use tvision::locale::{language, Language};
use tvision::*;

use crate::app_window::UserEvent;
use crate::backend::PROXY;
use crate::config::{GRUVBOX_PALETTE, VGA_PALETTE};
use crate::font::{monospace_fonts, FontInfo};

/// Наименьший и наибольший размер шрифта в диалоге (логические пиксели).
pub const FONT_SIZE_MIN: i64 = 6;
pub const FONT_SIZE_MAX: i64 = 96;

// ----------------------------------------------------------------------
// Темы
// ----------------------------------------------------------------------

/// Тема оформления: палитра программы Turbo Vision (135 атрибутов, в том
/// числе 256-цветных) и 16 цветов BIOS в окне.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Название по-русски (под ним тема хранится в файле настроек).
    pub name: &'static str,
    /// Название по-английски — показывается, если язык надписей английский.
    pub name_en: &'static str,
    /// Палитра программы (`Program::palette`).
    pub app_palette: Vec<Attr>,
    /// Как выглядят 16 цветов BIOS в окне (`GuiConfig::palette`).
    pub window_palette: [u32; 16],
}

impl Theme {
    /// Название на текущем языке надписей.
    pub fn title(&self) -> &'static str {
        if language() == Language::Ru {
            self.name
        } else {
            self.name_en
        }
    }
}

/// Тема «Сумерки»: какой цвет xterm поставить вместо каждого цвета BIOS
/// (как в примере `colors` из tvision-rs).
const DUSK: [u8; 16] = [235, 24, 71, 30, 131, 96, 137, 252, 242, 74, 114, 116, 210, 176, 222, 255];

/// Все встроенные темы. Первая — тема по умолчанию.
///
/// | Тема | Палитра программы | Цвета окна |
/// |------|-------------------|-----------|
/// | Gruvbox | `palettes::GRUVBOX` (256 цветов) | gruvbox |
/// | Классическая | `APP_COLOR` (Turbo Vision) | VGA |
/// | Классическая, мягкие цвета | `APP_COLOR` | gruvbox |
/// | Сумерки | `APP_COLOR`, перекрашенная в 256 цветов | VGA |
/// | Чёрно-белая | `APP_BLACK_WHITE` | VGA |
/// | Монохромная | `APP_MONOCHROME` | VGA |
pub fn themes() -> Vec<Theme> {
    vec![
        Theme { name: "Gruvbox", name_en: "Gruvbox", app_palette: palettes::GRUVBOX.to_vec(), window_palette: GRUVBOX_PALETTE },
        Theme { name: "Классическая", name_en: "Classic", app_palette: palettes::APP_COLOR.to_vec(), window_palette: VGA_PALETTE },
        Theme {
            name: "Классическая, мягкие цвета",
            name_en: "Classic, soft colors",
            app_palette: palettes::APP_COLOR.to_vec(),
            window_palette: GRUVBOX_PALETTE,
        },
        Theme { name: "Сумерки", name_en: "Dusk", app_palette: color::remap(palettes::APP_COLOR, &DUSK), window_palette: VGA_PALETTE },
        Theme { name: "Чёрно-белая", name_en: "Black and white", app_palette: palettes::APP_BLACK_WHITE.to_vec(), window_palette: VGA_PALETTE },
        Theme { name: "Монохромная", name_en: "Monochrome", app_palette: palettes::APP_MONOCHROME.to_vec(), window_palette: VGA_PALETTE },
    ]
}

/// Тема по названию (русскому или английскому, без учёта регистра).
pub fn find_theme(name: &str) -> Option<Theme> {
    themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(name) || t.name_en.eq_ignore_ascii_case(name) || t.name.to_lowercase() == name.to_lowercase())
}

// ----------------------------------------------------------------------
// Настройки и их хранение
// ----------------------------------------------------------------------

/// Размер и положение окна.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowGeometry {
    /// Размер экрана в символах (в обычном, не полноэкранном окне).
    pub cols: u16,
    pub rows: u16,
    /// Положение левого верхнего угла окна на экране (физические пиксели).
    /// `None` — оконная система не сообщает его (Wayland): окно поставит она.
    pub position: Option<(i32, i32)>,
    /// Окно развёрнуто на весь экран кнопкой заголовка.
    pub maximized: bool,
    /// Полноэкранный режим (Alt+Enter).
    pub fullscreen: bool,
}

/// Настройки окна, которые хранятся в файле: тема, шрифт, размер шрифта,
/// размер и положение окна.
#[derive(Clone, Debug, PartialEq)]
pub struct GuiSettings {
    /// Название темы (`None` — оформление программы не менялось).
    pub theme: Option<String>,
    /// Файл шрифта (`None` — системный моноширинный шрифт).
    pub font: Option<PathBuf>,
    /// Размер шрифта в логических пикселях.
    pub font_size: f32,
    /// Размер и положение окна (`None` — не сохранялись).
    pub window: Option<WindowGeometry>,
}

/// Имя файла настроек: имя исполняемого файла без расширения + `.conf`.
fn file_name() -> Option<(PathBuf, String)> {
    let exe = std::env::current_exe().ok()?;
    let name = format!("{}.conf", exe.file_stem()?.to_string_lossy());
    Some((exe.parent()?.to_path_buf(), name))
}

impl GuiSettings {
    /// Файл настроек **рядом с программой**: `ИМЯ.conf` в каталоге
    /// исполняемого файла (`tvdemo.exe` → `tvdemo.conf`).
    pub fn local_path() -> Option<PathBuf> {
        let (dir, name) = file_name()?;
        Some(dir.join(name))
    }

    /// Запасной файл настроек в профиле пользователя — на случай, если рядом
    /// с программой писать нельзя (например, она установлена в
    /// `C:\Program Files`):
    ///
    /// * Windows — `%APPDATA%\tvision-gui\ИМЯ.conf`;
    /// * macOS — `~/Library/Application Support/tvision-gui/ИМЯ.conf`;
    /// * Linux, BSD — `$XDG_CONFIG_HOME/tvision-gui/ИМЯ.conf` (обычно `~/.config/...`).
    pub fn user_path() -> Option<PathBuf> {
        let (_, name) = file_name()?;
        Some(config_dir()?.join("tvision-gui").join(name))
    }

    /// Файл, из которого читаются настройки: рядом с программой, если он
    /// там есть, иначе в профиле пользователя, если есть там. `None` —
    /// файла нет нигде (программа запускается с настройками по умолчанию).
    pub fn path() -> Option<PathBuf> {
        [Self::local_path(), Self::user_path()].into_iter().flatten().find(|p| p.is_file())
    }

    /// Прочитать сохранённые настройки. `None` — файла нет, он пуст или не
    /// читается: тогда программа работает с настройками по умолчанию
    /// (из `GuiConfig`).
    pub fn load() -> Option<Self> {
        Self::load_from(&Self::path()?)
    }

    /// Прочитать настройки из файла.
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        Self::parse(&text)
    }

    /// Разобрать текст файла настроек (строки `ключ = значение`;
    /// неизвестные ключи и строки с `#` пропускаются).
    pub fn parse(text: &str) -> Option<Self> {
        let mut s = GuiSettings { theme: None, font: None, font_size: 0.0, window: None };
        let mut g = WindowGeometry { cols: 0, rows: 0, position: None, maximized: false, fullscreen: false };
        let (mut x, mut y) = (None, None);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            let flag = matches!(v, "1" | "true" | "yes" | "да");
            match k.trim() {
                "theme" if !v.is_empty() => s.theme = Some(v.to_string()),
                "font" if !v.is_empty() => s.font = Some(PathBuf::from(v)),
                "font_size" => s.font_size = v.parse().unwrap_or(0.0),
                "cols" => g.cols = v.parse().unwrap_or(0),
                "rows" => g.rows = v.parse().unwrap_or(0),
                "x" => x = v.parse().ok(),
                "y" => y = v.parse().ok(),
                "maximized" => g.maximized = flag,
                "fullscreen" => g.fullscreen = flag,
                _ => {}
            }
        }
        if let (Some(x), Some(y)) = (x, y) {
            g.position = Some((x, y));
        }
        if g.cols > 0 && g.rows > 0 {
            s.window = Some(g);
        }
        (s.font_size > 0.0).then_some(s)
    }

    /// Текст файла настроек.
    pub fn to_text(&self) -> String {
        let mut t = String::from("# Настройки окна tvision-gui. Файл можно удалить —\n# тогда программа запустится с настройками по умолчанию.\n");
        t.push_str(&format!("theme = {}\n", self.theme.as_deref().unwrap_or("")));
        t.push_str(&format!("font = {}\n", self.font.as_ref().map(|p| p.display().to_string()).unwrap_or_default()));
        t.push_str(&format!("font_size = {}\n", self.font_size));
        if let Some(g) = &self.window {
            t.push_str(&format!("cols = {}\nrows = {}\n", g.cols, g.rows));
            if let Some((x, y)) = g.position {
                t.push_str(&format!("x = {}\ny = {}\n", x, y));
            }
            t.push_str(&format!("maximized = {}\nfullscreen = {}\n", g.maximized as u8, g.fullscreen as u8));
        }
        t
    }

    /// Записать настройки: рядом с программой, а если там нельзя — в профиль
    /// пользователя. Возвращает путь записанного файла.
    pub fn save(&self) -> std::io::Result<PathBuf> {
        let text = self.to_text();
        let mut last_err = std::io::Error::other("не найден каталог для настроек");
        for p in [Self::local_path(), Self::user_path()].into_iter().flatten() {
            let r = p.parent().map(std::fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| std::fs::write(&p, &text));
            match r {
                Ok(()) => {
                    // Не оставляем рядом старую копию, которая читалась бы первой.
                    if Some(&p) != Self::local_path().as_ref() {
                        if let Some(l) = Self::local_path() {
                            let _ = std::fs::remove_file(l);
                        }
                    }
                    return Ok(p);
                }
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    /// Записать настройки в заданный файл.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(path, self.to_text())
    }
}

/// Каталог настроек пользователя для текущей ОС.
fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

/// Текущее состояние окна, которое видят программа и диалог.
pub(crate) struct State {
    pub settings: GuiSettings,
    pub window_palette: [u32; 16],
    pub remember: bool,
}

pub(crate) static CURRENT: Mutex<Option<State>> = Mutex::new(None);

/// Окно создано: запомнить, с какими настройками.
pub(crate) fn init_current(settings: GuiSettings, window_palette: [u32; 16], remember: bool) {
    *CURRENT.lock().unwrap() = Some(State { settings, window_palette, remember });
}

/// Окно само поменяло размер шрифта (Ctrl+= / Ctrl+- / Ctrl+0).
pub(crate) fn note_font_size(size: f32) {
    if let Some(s) = CURRENT.lock().unwrap().as_mut() {
        s.settings.font_size = size;
    }
}

/// Окно изменило размер или положение (вызывает поток окна).
pub(crate) fn note_geometry(g: WindowGeometry) {
    if let Some(s) = CURRENT.lock().unwrap().as_mut() {
        s.settings.window = Some(g);
    }
}

/// Записать текущие настройки в файл, если включено запоминание
/// (`GuiConfig::remember_settings`). Вызывается при выходе из программы и
/// по кнопке «OK» диалога настроек.
pub(crate) fn save_current() -> Option<std::io::Result<PathBuf>> {
    let s = CURRENT.lock().unwrap().as_ref().filter(|s| s.remember).map(|s| s.settings.clone())?;
    Some(s.save())
}

/// Текущие настройки окна (`None` — окна ещё нет).
pub fn current_settings() -> Option<GuiSettings> {
    CURRENT.lock().unwrap().as_ref().map(|s| s.settings.clone())
}

fn send(ev: UserEvent) {
    if let Some(p) = PROXY.lock().unwrap().as_ref() {
        let _ = p.send_event(ev);
    }
}

/// Применить тему: палитра программы `app` и 16 цветов окна.
pub fn apply_theme(app: &mut Program, theme: &Theme) {
    app.palette = theme.app_palette.clone();
    set_window_palette(theme.window_palette);
    if let Some(s) = CURRENT.lock().unwrap().as_mut() {
        s.settings.theme = Some(theme.name.to_string());
    }
}

/// Сменить только 16 цветов окна (`0x00RRGGBB`, порядок BIOS).
pub fn set_window_palette(palette: [u32; 16]) {
    if let Some(s) = CURRENT.lock().unwrap().as_mut() {
        s.window_palette = palette;
    }
    send(UserEvent::SetPalette(palette));
}

/// Сменить шрифт окна: файл (`None` — системный моноширинный) и размер в
/// логических пикселях. Число символов на экране пересчитывается, программа
/// получит событие изменения размера.
pub fn set_font(path: Option<&Path>, size: f32) {
    let size = size.clamp(FONT_SIZE_MIN as f32, FONT_SIZE_MAX as f32);
    if let Some(s) = CURRENT.lock().unwrap().as_mut() {
        s.settings.font = path.map(Path::to_path_buf);
        s.settings.font_size = size;
    }
    send(UserEvent::SetFont(path.map(Path::to_path_buf), size));
}

/// Применить тему из текущих настроек к только что созданной программе
/// (вызывается из `Program::with_window`).
pub(crate) fn apply_current_theme(app: &mut Program) {
    let name = CURRENT.lock().unwrap().as_ref().and_then(|s| s.settings.theme.clone());
    if let Some(t) = name.as_deref().and_then(find_theme) {
        app.palette = t.app_palette;
    }
}

/// Применить к программе сохранённые настройки — для программ, созданных
/// через `Program::new(GuiBackend::boxed()?, ...)`, а не `with_window`
/// (там это делается само). Шрифт и размер окно применяет само при
/// запуске; здесь — палитра программы по теме.
pub fn apply_saved_settings(app: &mut Program) {
    apply_current_theme(app);
}

// ----------------------------------------------------------------------
// Диалог
// ----------------------------------------------------------------------

/// Надпись на текущем языке.
fn tr(ru: &'static str, en: &'static str) -> &'static str {
    if language() == Language::Ru {
        ru
    } else {
        en
    }
}

/// Стандартный диалог настроек: тема, шрифт, размер шрифта.
///
/// Возвращает `true`, если пользователь нажал «OK» (выбор применён и,
/// если включено запоминание, сохранён), и `false` при «Отмене» (всё
/// возвращено как было).
pub fn settings_dialog(app: &mut Program) -> bool {
    let fonts: Vec<FontInfo> = monospace_fonts();
    let themes = themes();
    let start = current_settings().unwrap_or(GuiSettings { theme: None, font: None, font_size: 16.0, window: None });
    let start_palette = app.palette.clone();
    let start_window = CURRENT.lock().unwrap().as_ref().map(|s| s.window_palette).unwrap_or(GRUVBOX_PALETTE);

    // Какая тема сейчас: по названию или по совпадению палитры программы.
    let theme_idx = start
        .theme
        .as_deref()
        .and_then(|n| themes.iter().position(|t| t.name == n))
        .or_else(|| themes.iter().position(|t| t.app_palette == app.palette && t.window_palette == start_window))
        .unwrap_or(0);
    let font_idx = match &start.font {
        None => 0,
        Some(p) => fonts.iter().position(|f| same_file(&f.path, p)).map(|i| i + 1).unwrap_or(0),
    };

    let mut d = Dialog::centered(67, 19, tr("Настройки", "Settings"));
    // Тема.
    let tsb = d.add(ScrollBar::new(Rect::new(31, 3, 32, 11)));
    let mut tl = ListBox::new(Rect::new(3, 3, 31, 11), 1, Some(tsb));
    tl.set_items(themes.iter().map(|t| t.title().to_string()).collect());
    tl.focus_item(theme_idx);
    let theme_list = d.add(tl);
    d.add(Label::new(Rect::new(2, 2, 20, 3), tr("~Т~ема", "~T~heme"), Some(theme_list)));
    // Шрифт.
    let fsb = d.add(ScrollBar::new(Rect::new(63, 3, 64, 11)));
    let mut fl = ListBox::new(Rect::new(35, 3, 63, 11), 1, Some(fsb));
    let mut items = vec![tr("Системный (по умолчанию)", "System (default)").to_string()];
    items.extend(fonts.iter().map(|f| f.name.clone()));
    fl.set_items(items);
    fl.focus_item(font_idx);
    let font_list = d.add(fl);
    d.add(Label::new(Rect::new(34, 2, 50, 3), tr("~Ш~рифт", "~F~ont"), Some(font_list)));
    // Размер.
    let mut sz = InputLong::new(Rect::new(20, 12, 27, 13), 3, FONT_SIZE_MIN, FONT_SIZE_MAX, 0, tr("Размер шрифта", "Font size"));
    sz.set_value(start.font_size.round() as i64);
    let size_line = d.add(sz);
    d.add(Label::new(Rect::new(2, 12, 19, 13), tr("~Р~азмер шрифта", "Font ~s~ize"), Some(size_line)));
    d.add(StaticText::new(
        Rect::new(29, 12, 64, 13),
        tr("6–96; в окне: Ctrl+= / Ctrl+-", "6–96; in window: Ctrl+= / Ctrl+-"),
    ));
    d.add(StaticText::new(
        Rect::new(3, 14, 64, 15),
        tr("Изменения видны сразу. «Отмена» вернёт прежние настройки.", "Changes apply at once. Cancel restores the previous settings."),
    ));
    d.add(Button::new(Rect::new(38, 16, 50, 18), "O~K~", cm::OK, bf::DEFAULT));
    d.add(Button::new(Rect::new(51, 16, 63, 18), tr("Отмена", "Cancel"), cm::CANCEL, bf::NORMAL));
    d.select(theme_list);

    // Пока диалог открыт — применяем выбор на лету.
    let mut shown = (theme_idx, font_idx, start.font_size.round() as i64);
    let (cmd, _) = app.exec_dialog_idle(d, |app, _| {
        let t = app.find::<ListBox>(theme_list).map(|l| l.focused).unwrap_or(shown.0);
        let f = app.find::<ListBox>(font_list).map(|l| l.focused).unwrap_or(shown.1);
        let s = app.find::<InputLong>(size_line).map(|i| i.value()).unwrap_or(shown.2);
        if t != shown.0 {
            if let Some(th) = themes.get(t) {
                apply_theme(app, th);
            }
        }
        let size_ok = (FONT_SIZE_MIN..=FONT_SIZE_MAX).contains(&s);
        if f != shown.1 || (size_ok && s != shown.2) {
            let path = if f == 0 { None } else { fonts.get(f - 1).map(|i| i.path.as_path()) };
            let size = if size_ok { s } else { shown.2 };
            set_font(path, size as f32);
            shown.2 = size;
        }
        shown.0 = t;
        shown.1 = f;
        None
    });

    if cmd == cm::OK {
        if let Some(Err(e)) = save_current() {
            message_box(app, &format!("{}\n{}", tr("Не удалось сохранить настройки:", "Could not save settings:"), e), mf::ERROR | mf::OK_BUTTON);
        }
        true
    } else {
        // Вернуть всё как было.
        app.palette = start_palette;
        set_window_palette(start_window);
        set_font(start.font.as_deref(), start.font_size);
        if let Some(s) = CURRENT.lock().unwrap().as_mut() {
            s.settings.theme = start.theme.clone();
        }
        false
    }
}

/// Один и тот же файл (с учётом регистра и вида пути в Windows).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_text_roundtrip() {
        let g = WindowGeometry { cols: 100, rows: 30, position: Some((-8, 120)), maximized: true, fullscreen: false };
        let s = GuiSettings { theme: Some("Сумерки".into()), font: Some(PathBuf::from("/fonts/Mono.ttf")), font_size: 18.0, window: Some(g) };
        assert_eq!(GuiSettings::parse(&s.to_text()), Some(s));
        let sys = GuiSettings { theme: None, font: None, font_size: 14.0, window: None };
        assert_eq!(GuiSettings::parse(&sys.to_text()), Some(sys));
        assert_eq!(GuiSettings::parse("# пусто\n"), None);
        // Положение без размера не сохраняется; Wayland — размер без положения.
        let w = GuiSettings::parse("font_size = 16\ncols = 90\nrows = 40\n").unwrap().window.unwrap();
        assert_eq!((w.cols, w.rows, w.position), (90, 40, None));
    }

    #[test]
    fn save_and_load_file() {
        let p = std::env::temp_dir().join(format!("tvgui-settings-{}.conf", std::process::id()));
        let s = GuiSettings { theme: Some("Gruvbox".into()), font: None, font_size: 20.0, window: None };
        s.save_to(&p).unwrap();
        assert_eq!(GuiSettings::load_from(&p), Some(s));
        let _ = std::fs::remove_file(&p);
        // Файла нет — настроек нет (программа возьмёт значения по умолчанию).
        assert_eq!(GuiSettings::load_from(&p), None);
    }

    #[test]
    fn themes_are_complete() {
        let t = themes();
        assert_eq!(t[0].name, "Gruvbox");
        for th in &t {
            assert_eq!(th.app_palette.len(), 135, "{}", th.name);
        }
        assert_eq!(find_theme("classic").unwrap().name, "Классическая");
        assert_eq!(find_theme("сумерки").unwrap().name_en, "Dusk");
        assert!(find_theme("нет такой").is_none());
    }
}
