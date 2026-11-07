// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/learn_05_dialog.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! Урок 5. Первый диалог: спросить имя и поздороваться.
//!
//! Запуск: `cargo run --example learn_05_dialog`. F2 — диалог, Alt+X — выход.

use tvision::*;

const CM_ASK: u16 = 100;

struct App {
    /// Имя, которое ввели в прошлый раз (диалог предложит его снова).
    name: String,
}

/// Строим диалог «Знакомство». Возвращаем сам диалог и номер (id) строки
/// ввода — по нему потом достанем введённый текст.
fn name_dialog(old_name: &str) -> (Dialog, ViewId) {
    // Диалог 40×9, появится по центру экрана.
    let mut d = Dialog::centered(40, 9, "Знакомство");

    // Строка ввода: столбцы 10..37, строка 2; не длиннее 30 символов.
    let mut input = InputLine::new(Rect::new(10, 2, 37, 3), 30);
    input.set_text(old_name);
    let input_id = d.add(input);

    // Подпись слева. Связываем её со строкой: Alt+И переведёт туда курсор.
    d.add(Label::new(Rect::new(2, 2, 9, 3), "~И~мя:", Some(input_id)));

    // Кнопки. Высота кнопки — 2 строки (вторая — тень).
    d.add(Button::new(Rect::new(6, 5, 18, 7), "O~K~", cm::OK, bf::DEFAULT)); // DEFAULT — нажимается по Enter
    d.add(Button::new(Rect::new(21, 5, 33, 7), "Отмена", cm::CANCEL, bf::NORMAL));

    (d, input_id)
}

impl App {
    fn ask(&mut self, app: &mut Program) {
        let (d, input_id) = name_dialog(&self.name);

        // Показать диалог и ждать, пока его закроют. Пока он открыт, остальная
        // программа «замирает» — это и называется модальным окном.
        // Возвращаются: команда, которой закрыли (OK или Cancel), и сам диалог.
        let (cmd, d) = app.exec_dialog(d);
        if cmd != cm::OK {
            return; // нажали «Отмена» или Esc
        }

        // Диалог ещё у нас в руках — достаём из него строку ввода по id.
        let name = d.win.group.find::<InputLine>(input_id).unwrap().text();
        let name = name.trim().to_string();
        if name.is_empty() {
            message_box(app, "\x03Вы не ввели имя.", mf::WARNING | mf::OK_BUTTON);
        } else {
            message_box(app, &format!("\x03Привет, {}!", name), mf::INFORMATION | mf::OK_BUTTON);
            self.name = name;
        }
    }
}

impl AppHandler for App {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        if ev.command() == Some(CM_ASK) {
            self.ask(app);
            ev.clear();
        }
    }
}

fn status() -> Vec<StatusDef> {
    vec![StatusDef::all(vec![StatusItem::new("~F2~ Знакомство", kb::F2, CM_ASK), StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT)])]
}

fn tv_main() -> std::io::Result<()> {
    locale::set_language(locale::Language::Ru);
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(None, Some(status()))?;
    let mut h = App { name: String::new() };
    app.put_event(Event::command_event(CM_ASK)); // сразу открыть диалог
    app.run(&mut h);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision::platform::HeadlessBackend;

    #[test]
    fn remembers_name() {
        let mut b = HeadlessBackend::new(80, 25);
        b.push_key(kb::F2);
        for c in "Аня".chars() {
            b.push_key(KeyEvent::new(Key::Char(c), Mods::NONE));
        }
        b.push_key(kb::ENTER); // OK; окно «Привет» закроет Esc тестового терминала
        let mut app = Program::new(Box::new(b), None, Some(status()));
        let mut h = App { name: String::new() };
        app.pump(&mut h);
        assert_eq!(h.name, "Аня");
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("Урок 5. Первый диалог: спросить имя и поздороваться"), tv_main)
}
