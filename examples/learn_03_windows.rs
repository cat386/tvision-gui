// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/learn_03_windows.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! Урок 3. Окна: создать, передвинуть, распахнуть, закрыть.
//!
//! Запуск: `cargo run --example learn_03_windows`. F3 — новое окно,
//! F6 — следующее окно, F5 — распахнуть, Alt+F3 — закрыть, Alt+X — выход.
//! Окна можно таскать мышью за заголовок и тянуть за правый нижний угол.

use tvision::*;

const CM_NEW_WINDOW: u16 = 100;

struct App {
    /// Сколько окон уже открыто за всё время — для заголовков и номеров.
    opened: i32,
}

impl App {
    fn new_window(&mut self, app: &mut Program) {
        self.opened += 1;
        let n = self.opened;

        // Каждое следующее окно — чуть правее и ниже предыдущего.
        let shift = (n - 1) % 8;
        let bounds = Rect::new(2 + shift * 3, 1 + shift, 2 + shift * 3 + 40, 1 + shift + 10);

        // Номер 1–9 виден в рамке; Alt+номер выбирает окно. 0 — без номера.
        let number = if n < 10 { n as i16 } else { 0 };
        let mut w = Window::new(bounds, &format!("Окно {}", n), number);

        // Внутрь окна кладём текст. `interior()` — место внутри рамки.
        let text =
            StaticText::new(w.interior(), "Тащите окно мышью за заголовок, тяните за правый нижний угол.\n\nF5 — распахнуть, Alt+F3 — закрыть.");
        w.add(text);

        // Окно — на рабочий стол.
        app.insert_window(w);
    }
}

impl AppHandler for App {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        if ev.command() == Some(CM_NEW_WINDOW) {
            self.new_window(app);
            ev.clear();
        }
    }
}

fn menu() -> Menu {
    Menu::new()
        .submenu(
            "~Ф~айл",
            Menu::new().item_key("~Н~овое окно", CM_NEW_WINDOW, kb::F3, "F3").separator().item_key("~В~ыход", cm::QUIT, kb::ALT_X, "Alt-X"),
        )
        // Команды окон — стандартные: их обрабатывают сами окна и рабочий стол.
        .submenu(
            "~О~кна",
            Menu::new()
                .item("~М~озаика", cm::TILE)
                .item("~К~аскад", cm::CASCADE)
                .separator()
                .item_key("~С~ледующее", cm::NEXT, kb::F6, "F6")
                .item_key("~Р~аспахнуть", cm::ZOOM, kb::F5, "F5")
                .item_key("~З~акрыть", cm::CLOSE, kb::ALT_F3, "Alt-F3"),
        )
}

fn status() -> Vec<StatusDef> {
    vec![StatusDef::all(vec![
        StatusItem::new("~F3~ Новое", kb::F3, CM_NEW_WINDOW),
        StatusItem::new("~F6~ Следующее", kb::F6, cm::NEXT),
        StatusItem::new("~F5~ Распахнуть", kb::F5, cm::ZOOM),
        StatusItem::new("~Alt-F3~ Закрыть", kb::ALT_F3, cm::CLOSE),
        StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT),
        StatusItem::new("", kb::F10, cm::MENU),
    ])]
}

fn tv_main() -> std::io::Result<()> {
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(Some(menu()), Some(status()))?;
    let mut h = App { opened: 0 };
    h.new_window(&mut app); // одно окно сразу при запуске
    app.run(&mut h);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision::platform::HeadlessBackend;

    /// Сколько окон на рабочем столе.
    fn windows(app: &Program) -> usize {
        app.desktop().group.children.iter().filter(|c| c.as_window().is_some()).count()
    }

    #[test]
    fn open_and_close() {
        let mut b = HeadlessBackend::new(80, 25);
        b.push_key(kb::F3);
        b.push_key(kb::F3);
        b.push_key(kb::ALT_F3); // закрыть верхнее
        let mut app = Program::new(Box::new(b), Some(menu()), Some(status()));
        let mut h = App { opened: 0 };
        app.pump(&mut h);
        assert_eq!(windows(&app), 1);
        assert!(app.screen_text().contains("Окно 1"));
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("Урок 3. Окна: создать, передвинуть, распахнуть, закрыть"), tv_main)
}
