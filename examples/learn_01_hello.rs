// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/learn_01_hello.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! Урок 1. Самая простая программа: пустой рабочий стол и строка состояния.
//!
//! Запуск: `cargo run --example learn_01_hello`. Выход — Alt+X или щелчок
//! по надписи «Alt-X Выход» внизу экрана.

// Подключаем всё нужное из библиотеки одной строкой.
use tvision::*;

fn tv_main() -> std::io::Result<()> {
    // Строка состояния — нижняя строка экрана. В ней один пункт:
    // надпись «Alt-X Выход», клавиша Alt+X и команда «выйти из программы».
    let status = vec![StatusDef::all(vec![StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT)])];

    // Создаём программу: она займёт весь терминал.
    // Первый аргумент — меню (его пока нет), второй — строка состояния.
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(None, Some(status))?;

    // Запускаем. `run` возвращается, когда пользователь выходит из программы;
    // терминал при этом возвращается в обычный вид.
    app.run(&mut program::NoHandler);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tvision::platform::HeadlessBackend;
    use tvision::*;

    #[test]
    fn alt_x_quits() {
        let mut b = HeadlessBackend::new(80, 25);
        b.push_key(kb::ALT_X);
        let status = vec![StatusDef::all(vec![StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT)])];
        let mut app = Program::new(Box::new(b), None, Some(status));
        app.pump(&mut program::NoHandler);
        assert!(!app.is_running());
        assert!(app.screen_text().contains("Alt-X Выход"));
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("Урок 1. Самая простая программа: пустой рабочий стол и строка состояни"), tv_main)
}
