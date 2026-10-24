//! Настройки графического окна.

use std::path::PathBuf;

/// Стандартная палитра VGA (текстовый режим 3): 16 цветов BIOS в формате
/// `0x00RRGGBB`. Порядок — как в атрибутах BIOS: 0 чёрный, 1 синий,
/// 2 зелёный, 3 бирюзовый, 4 красный, 5 малиновый, 6 коричневый,
/// 7 светло-серый, 8–15 — яркие варианты.
pub const VGA_PALETTE: [u32; 16] = [
    0x000000, 0x0000AA, 0x00AA00, 0x00AAAA, 0xAA0000, 0xAA00AA, 0xAA5500, 0xAAAAAA, //
    0x555555, 0x5555FF, 0x55FF55, 0x55FFFF, 0xFF5555, 0xFF55FF, 0xFFFF55, 0xFFFFFF,
];

/// 16 цветов терминальной темы [gruvbox](https://github.com/morhetz/gruvbox)
/// (тёмной) в порядке BIOS — палитра окна **по умолчанию**. Сама программа
/// по умолчанию рисуется палитрой `color::palettes::GRUVBOX` (256 цветов);
/// эти 16 цветов видны там, где атрибут задан цветом BIOS (например, у
/// классической палитры `APP_COLOR`).
pub const GRUVBOX_PALETTE: [u32; 16] = tvision::color::GRUVBOX_RGB;

/// Параметры окна. Создаётся через [`GuiConfig::new`] и методы-«строители»:
///
/// ```
/// use tvision_gui::GuiConfig;
/// let cfg = GuiConfig::new("Редактор").size(100, 30).font_size(18.0);
/// assert_eq!(cfg.cols, 100);
/// ```
#[derive(Clone, Debug)]
pub struct GuiConfig {
    /// Заголовок окна.
    pub title: String,
    /// Начальная ширина экрана в символах.
    pub cols: u16,
    /// Начальная высота экрана в строках.
    pub rows: u16,
    /// Размер шрифта в логических пикселях (на HiDPI-экранах умножается
    /// на масштаб монитора). Определяет высоту символа примерно как у
    /// терминала с тем же размером шрифта.
    pub font_size: f32,
    /// Файл шрифта. `None` — найти моноширинный шрифт в системе.
    pub font_path: Option<PathBuf>,
    /// Цвета BIOS 0..15 в формате `0x00RRGGBB`.
    pub palette: [u32; 16],
    /// Мигает ли курсор.
    pub cursor_blink: bool,
    /// Можно ли менять размер окна мышью.
    pub resizable: bool,
    /// Открыть сразу на весь экран.
    pub fullscreen: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        // Заголовок по умолчанию — имя исполняемого файла.
        let title = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Turbo Vision".to_string());
        GuiConfig {
            title,
            cols: 80,
            rows: 25,
            font_size: 16.0,
            font_path: None,
            palette: GRUVBOX_PALETTE,
            cursor_blink: true,
            resizable: true,
            fullscreen: false,
        }
    }
}

impl GuiConfig {
    /// Настройки по умолчанию (80×25, шрифт 16 px) с заданным заголовком.
    pub fn new(title: &str) -> Self {
        GuiConfig { title: title.to_string(), ..Default::default() }
    }

    /// Начальный размер экрана в символах.
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        self
    }

    /// Размер шрифта в логических пикселях.
    pub fn font_size(mut self, px: f32) -> Self {
        self.font_size = px.clamp(4.0, 200.0);
        self
    }

    /// Файл шрифта (.ttf / .otf / .ttc).
    pub fn font(mut self, path: impl Into<PathBuf>) -> Self {
        self.font_path = Some(path.into());
        self
    }

    /// Своя палитра из 16 цветов `0x00RRGGBB`.
    pub fn palette(mut self, palette: [u32; 16]) -> Self {
        self.palette = palette;
        self
    }

    /// Включить или выключить мигание курсора.
    pub fn cursor_blink(mut self, on: bool) -> Self {
        self.cursor_blink = on;
        self
    }

    /// Разрешить или запретить изменение размера окна.
    pub fn resizable(mut self, on: bool) -> Self {
        self.resizable = on;
        self
    }

    /// Открыть окно на весь экран.
    pub fn fullscreen(mut self, on: bool) -> Self {
        self.fullscreen = on;
        self
    }

    /// Применяет переменные окружения `TVISION_GUI_FONT` и
    /// `TVISION_GUI_FONT_SIZE` (они важнее настроек из кода, чтобы
    /// пользователь мог поменять шрифт без перекомпиляции).
    pub(crate) fn with_env(mut self) -> Self {
        if let Ok(p) = std::env::var("TVISION_GUI_FONT") {
            if !p.is_empty() {
                self.font_path = Some(PathBuf::from(p));
            }
        }
        if let Some(px) = std::env::var("TVISION_GUI_FONT_SIZE").ok().and_then(|s| s.trim().parse::<f32>().ok()) {
            self = self.font_size(px);
        }
        self
    }
}
