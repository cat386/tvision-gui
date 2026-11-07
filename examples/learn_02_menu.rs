// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/learn_02_menu.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! Урок 2. Меню, строка состояния и свои команды.
//!
//! Запуск: `cargo run --example learn_02_menu`. F10 — меню, F2 — приветствие,
//! Alt+X — выход.

use tvision::*;

// Свои команды. Это просто числа; главное — не пересечься со стандартными
// (`cm::QUIT`, `cm::OK`…), которые заняли номера меньше 100.
const CM_HELLO: u16 = 100;
const CM_ABOUT: u16 = 101;

/// Наше приложение. Здесь живут данные программы — пока только счётчик.
struct App {
    greetings: u32,
}

// `AppHandler` — «мозг» программы: сюда приходят команды из меню и строки
// состояния, которые не обработал никто другой.
impl AppHandler for App {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        match ev.command() {
            Some(CM_HELLO) => {
                self.greetings += 1;
                let text = format!("\x03Привет! Это приветствие номер {}.", self.greetings);
                message_box(app, &text, mf::INFORMATION | mf::OK_BUTTON);
            }
            Some(CM_ABOUT) => {
                message_box(app, "\x03Урок 2\n\x03Меню и команды", mf::INFORMATION | mf::OK_BUTTON);
            }
            // Чужие события не трогаем — просто выходим.
            _ => return,
        }
        // Команда обработана: помечаем событие, чтобы оно не пошло дальше.
        ev.clear();
    }
}

/// Строка меню: два выпадающих меню.
fn menu() -> Menu {
    Menu::new()
        .submenu(
            "~Ф~айл", // тильды выделяют горячую букву: Alt+Ф откроет это меню
            Menu::new()
                .item_key("~П~ривет", CM_HELLO, kb::F2, "F2") // пункт, клавиша F2, подпись «F2»
                .separator() // черта-разделитель
                .item_key("~В~ыход", cm::QUIT, kb::ALT_X, "Alt-X"),
        )
        .submenu("~С~правка", Menu::new().item("~О~ программе", CM_ABOUT))
}

/// Строка состояния.
fn status() -> Vec<StatusDef> {
    vec![StatusDef::all(vec![
        StatusItem::new("~F2~ Привет", kb::F2, CM_HELLO),
        StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT),
        StatusItem::new("", kb::F10, cm::MENU), // без надписи: F10 открывает меню
    ])]
}

fn tv_main() -> std::io::Result<()> {
    // Кнопки окон сообщений («Да», «Отмена»…) — по-русски.
    locale::set_language(locale::Language::Ru);
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(Some(menu()), Some(status()))?;
    app.run(&mut App { greetings: 0 });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision::platform::HeadlessBackend;

    #[test]
    fn f2_greets() {
        let mut b = HeadlessBackend::new(80, 25);
        b.push_key(kb::F2); // окно сообщения закроет Esc тестового терминала
        let mut app = Program::new(Box::new(b), Some(menu()), Some(status()));
        let mut h = App { greetings: 0 };
        app.pump(&mut h);
        assert_eq!(h.greetings, 1);
        // Второй раз — командой, как будто выбрали пункт меню.
        app.put_event(Event::command_event(CM_HELLO));
        app.pump(&mut h);
        assert_eq!(h.greetings, 2);
        assert!(app.screen_text().contains("Файл"));
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("Урок 2. Меню, строка состояния и свои команды"), tv_main)
}
