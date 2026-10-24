//! Классический `hello.cpp` из Turbo Vision — в графическом окне.
//!
//! Запуск: `cargo run --example hello`
//! Меню «Привет» → «Приветствие...» (или Alt+G) открывает диалог; Alt+X — выход.
//! Все надписи переведены на русский.
//!
//! По сравнению с версией для терминала (tvision-rs/examples/hello.rs)
//! изменились `use tvision_gui::*`, `Program::with_window`, обёртка
//! `tvision_gui::run` в `main` и язык надписей.

use tvision_gui::*;

/// Своя команда приложения (`GreetThemCmd`).
const GREET_THEM_CMD: u16 = 100;

/// Аналог класса `THelloApp`.
struct HelloApp;

impl HelloApp {
    /// Аналог `THelloApp::greetingBox()`.
    fn greeting_box(&self, app: &mut Program) {
        let mut d = Dialog::new(Rect::new(25, 5, 55, 16), "Привет, мир!");
        d.add(StaticText::new(Rect::new(3, 5, 15, 6), "Как дела?"));
        d.add(Button::new(Rect::new(16, 2, 28, 4), "Отлично", cm::CANCEL, bf::NORMAL));
        d.add(Button::new(Rect::new(16, 4, 28, 6), "Нормально", cm::CANCEL, bf::NORMAL));
        d.add(Button::new(Rect::new(16, 6, 28, 8), "Плохо", cm::CANCEL, bf::NORMAL));
        d.add(Button::new(Rect::new(16, 8, 28, 10), "Отмена", cm::CANCEL, bf::NORMAL));
        app.exec_dialog(d);
    }
}

impl AppHandler for HelloApp {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        if ev.command() == Some(GREET_THEM_CMD) {
            self.greeting_box(app);
            ev.clear();
        }
    }
}

/// Тело программы — то же, что `main` в версии для терминала.
fn app_main() -> std::io::Result<()> {
    let menu = Menu::new().submenu(
        "~П~ривет",
        Menu::new().item_key("П~р~иветствие...", GREET_THEM_CMD, kb::alt('G'), "Alt-G").separator().item_key("~В~ыход", cm::QUIT, kb::ALT_X, "Alt-X"),
    );
    let status = vec![StatusDef::all(vec![StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT), StatusItem::new("", kb::F10, cm::MENU)])];

    // Встроенные надписи библиотеки — тоже по-русски. Язык хранится
    // отдельно для каждого потока, поэтому включаем его здесь, в потоке программы.
    locale::set_language(locale::Language::Ru);
    let mut app = Program::with_window(Some(menu), Some(status))?;
    app.run(&mut HelloApp);
    Ok(())
}

fn main() -> std::io::Result<()> {
    // Окно — в главном потоке (обязательно для macOS), программа — в своём.
    run(GuiConfig::new("Привет, Turbo Vision"), app_main)
}
