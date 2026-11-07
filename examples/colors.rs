// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/colors.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! 256 цветов в tvision-rs.
//!
//! Запуск: `cargo run --example colors`
//!
//! * F2 — окно «Палитра 256»: все цвета xterm с номерами;
//! * F3 — переключить тему: gruvbox (по умолчанию) → классическая
//!   (16 цветов BIOS) → «Сумерки» (классическая палитра, перекрашенная в
//!   256 цветов функцией `color::remap`);
//! * F4 — диалог настройки цветов: сетки 16 цветов и кнопки «Текст 256…» /
//!   «Фон 256…» для любого из 256 цветов;
//! * F5 — режим вывода в терминал: автоматически → 16 цветов → 256 цветов
//!   (в режиме 16 цветов 256-цветные атрибуты показываются ближайшими
//!   цветами BIOS);
//! * Alt+X — выход.
//!
//! Если терминал не умеет 256 цветов, задайте `TVISION_COLORS=16`.

use tvision::color::{self, attr256, contrast256, ColorMode};
use tvision::*;

const CM_PALETTE: u16 = 100;
const CM_THEME: u16 = 101;
const CM_COLORS: u16 = 102;
const CM_MODE: u16 = 103;

/// Тема «Сумерки»: какой цвет xterm поставить вместо каждого цвета BIOS.
const DUSK: [u8; 16] = [
    235, // чёрный → почти чёрный
    24,  // синий → глубокий сине-бирюзовый
    71,  // зелёный → приглушённый зелёный
    30,  // бирюзовый → тёмная морская волна
    131, // красный → кирпичный
    96,  // малиновый → сливовый
    137, // коричневый → охра
    252, // светло-серый → светлый серый
    242, // тёмно-серый → серый
    74,  // ярко-синий → небесный
    114, // ярко-зелёный → салатовый
    116, // ярко-бирюзовый → бирюзовый
    210, // ярко-красный → коралловый
    176, // ярко-малиновый → сиреневый
    222, // жёлтый → песочный
    255, // белый → белый
];

/// Таблица всех 256 цветов: 16 × 16 клеток по 4 символа с номером цвета.
struct ColorTable {
    vs: ViewState,
}

impl View for ColorTable {
    tvision::impl_view_basics!();

    fn draw(&self, dc: &mut DrawCtx) {
        let w = self.vs.size.x.max(0) as usize;
        for row in 0..self.vs.size.y {
            let mut b = DrawBuffer::new(w);
            b.move_char(0, ' ', dc.color(6), w);
            if row < 16 {
                for col in 0..16 {
                    let idx = (row * 16 + col) as u8;
                    b.move_str(col as usize * 4, &format!("{:>4}", idx), attr256(contrast256(idx), idx));
                }
            }
            dc.write_line(0, row, self.vs.size.x, 1, &b);
        }
    }
}

struct App {
    /// Тема: 0 — gruvbox, 1 — классическая, 2 — «Сумерки».
    theme: usize,
    mode: Option<ColorMode>,
}

impl App {
    fn palette_window(&self, app: &mut Program) {
        let mut w = Window::new(Rect::new(1, 1, 67, 19), "Палитра 256", 0);
        w.flags &= !wf::GROW;
        w.add(ColorTable { vs: ViewState::new(Rect::new(1, 1, 65, 17)) });
        app.insert_window(w);
    }

    fn toggle_theme(&mut self, app: &mut Program) {
        self.theme = (self.theme + 1) % 3;
        app.palette = match self.theme {
            0 => color::palettes::GRUVBOX.to_vec(),
            1 => color::palettes::APP_COLOR.to_vec(),
            _ => color::remap(color::palettes::APP_COLOR, &DUSK),
        };
    }

    fn colors(&self, app: &mut Program) {
        let d = ColorDialog::new(&app.palette, standard_color_groups());
        let (c, d) = app.exec_dialog(d);
        if c == cm::OK {
            app.palette = d.palette.clone();
        }
    }

    fn next_mode(&mut self, app: &mut Program) {
        self.mode = match self.mode {
            None => Some(ColorMode::Ansi16),
            Some(ColorMode::Ansi16) => Some(ColorMode::Xterm256),
            Some(ColorMode::Xterm256) => None,
        };
        color::set_color_mode(self.mode);
        let name = match self.mode {
            None => format!("автоматически (сейчас: {})", mode_name(color::color_mode())),
            Some(m) => mode_name(m).to_string(),
        };
        message_box(app, &format!("\x03Режим вывода:\n\x03{}", name), mf::INFORMATION | mf::OK_BUTTON);
    }
}

fn mode_name(m: ColorMode) -> &'static str {
    match m {
        ColorMode::Ansi16 => "16 цветов",
        ColorMode::Xterm256 => "256 цветов",
    }
}

impl AppHandler for App {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        match ev.command() {
            Some(CM_PALETTE) => self.palette_window(app),
            Some(CM_THEME) => self.toggle_theme(app),
            Some(CM_COLORS) => self.colors(app),
            Some(CM_MODE) => self.next_mode(app),
            _ => return,
        }
        ev.clear();
    }
}

fn menu() -> Menu {
    Menu::new().submenu(
        "~Ц~вета",
        Menu::new()
            .item_key("~П~алитра 256", CM_PALETTE, kb::F2, "F2")
            .item_key("~Т~ема: gruvbox/классика/сумерки", CM_THEME, kb::f(3), "F3")
            .item_key("~Н~астройка цветов…", CM_COLORS, kb::f(4), "F4")
            .item_key("~Р~ежим вывода", CM_MODE, kb::F5, "F5")
            .separator()
            .item_key("В~ы~ход", cm::QUIT, kb::ALT_X, "Alt+X"),
    )
}

fn status() -> Vec<StatusDef> {
    vec![StatusDef::all(vec![
        StatusItem::new("~Alt+X~ Выход", kb::ALT_X, cm::QUIT),
        StatusItem::new("~F2~ Палитра", kb::F2, CM_PALETTE),
        StatusItem::new("~F3~ Тема", kb::f(3), CM_THEME),
        StatusItem::new("~F4~ Цвета", kb::f(4), CM_COLORS),
        StatusItem::new("~F5~ Режим", kb::F5, CM_MODE),
        StatusItem::new("", kb::F10, cm::MENU),
    ])]
}

fn tv_main() -> std::io::Result<()> {
    locale::set_language(locale::Language::Ru);
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(Some(menu()), Some(status()))?;
    // В 16-цветном терминале программа начинает с классической палитры.
    let theme = if app.palette == color::palettes::GRUVBOX { 0 } else { 1 };
    app.run(&mut App { theme, mode: None });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision::platform::HeadlessBackend;

    #[test]
    fn theme_and_table() {
        let mut b = HeadlessBackend::new(80, 25);
        b.push_key(kb::F2); // таблица цветов
        b.push_key(kb::f(3)); // классическая
        b.push_key(kb::f(3)); // «Сумерки»
        let mut app = Program::new(Box::new(b), Some(menu()), Some(status()));
        assert_eq!(app.palette, color::palettes::GRUVBOX); // тема по умолчанию
        let mut h = App { theme: 0, mode: None };
        app.pump(&mut h);
        app.redraw();
        assert_eq!(h.theme, 2);
        // Рабочий стол (цвет 1 палитры, было 0x71) — теперь xterm 24 на 252.
        let desk: color::Attr = app.screen().get(79, 22).attr;
        assert_eq!(desk, attr256(24, 252));
        // В таблице клетка с номером 196 закрашена цветом 196.
        let s = app.screen_text();
        let (y, line) = s.lines().enumerate().find(|(_, l)| l.contains(" 196")).unwrap();
        let x = line.chars().take_while(|_| true).collect::<Vec<_>>().windows(4).position(|w| w.iter().collect::<String>() == " 196").unwrap();
        assert_eq!(color::bg256(app.screen().get(x + 1, y).attr), 196);
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("256 цветов в tvision-rs"), tv_main)
}
