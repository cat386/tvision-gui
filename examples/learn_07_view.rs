// СГЕНЕРИРОВАНО sync_examples из tvision-rs/examples/learn_07_view.rs — не правьте вручную.
// Пересоздать: `cargo run --example sync_examples`.
// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)
// и функция `main`, запускающая программу через `tvision_gui::run`.
//! Урок 7. Своё представление: счётчик.
//!
//! Запуск: `cargo run --example learn_07_view`. Пробел, Enter или щелчок —
//! прибавить 1; `-` — убавить; `0` — обнулить; Tab — к другому счётчику.
//! Alt+X — выход.

use tvision::*;

/// Счётчик. Любое представление — это структура с полем `vs`
/// (положение, размер, состояние) и своими данными.
pub struct Counter {
    pub vs: ViewState,
    pub value: i32,
}

impl Counter {
    pub fn new(x: i32, y: i32) -> Self {
        // Размер 20×1: одна строка шириной 20 клеток.
        let mut vs = ViewState::new(Rect::new(x, y, x + 20, y + 1));
        // SELECTABLE — может получать фокус (клавиатуру);
        // FIRST_CLICK — первый же щелчок мышью засчитывается.
        vs.options |= of::SELECTABLE | of::FIRST_CLICK;
        Counter { vs, value: 0 }
    }
}

impl View for Counter {
    // Пять служебных методов, одинаковых у всех представлений, пишет макрос.
    tvision::impl_view_basics!();

    /// Нарисовать себя. Вызывается при каждой перерисовке экрана.
    fn draw(&self, dc: &mut DrawCtx) {
        // Цвет берём по номеру из палитры (см. `palette` ниже):
        // 1 — обычный, 2 — когда счётчик в фокусе.
        let color = dc.color(if self.vs.is_focused() { 2 } else { 1 });
        // Сначала закрашиваем всю свою строку, потом пишем текст.
        dc.fill(self.vs.extent(), ' ', color);
        dc.write_str(1, 0, &format!("Счёт: {}", self.value), color);
    }

    /// Палитра: возьмём ту же, что у строки ввода, — так счётчик будет
    /// выглядеть «как поле» в любом диалоге.
    fn palette(&self) -> &'static [u8] {
        color::palettes::INPUT_LINE
    }

    /// Обработать событие: клавишу или мышь.
    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Ctx) {
        // Стандартное поведение: щелчок по счётчику переводит на него фокус.
        view::default_handle_event(&self.vs, ev, ctx);
        match ev {
            Event::MouseDown(_) => {
                self.value += 1;
                ev.clear(); // событие обработано
            }
            // Клавиши получает только тот, кто в фокусе.
            Event::KeyDown(k) if self.vs.is_focused() => {
                match k.key {
                    Key::Char(' ') | Key::Enter => self.value += 1,
                    Key::Char('-') => self.value -= 1,
                    Key::Char('0') => self.value = 0,
                    _ => return, // чужая клавиша (например, Tab) — пусть обработают другие
                }
                ev.clear();
            }
            _ => {}
        }
    }
}

fn open_window(app: &mut Program) {
    let mut w = Window::new(Rect::new(10, 3, 46, 11), "Счётчики", 1);
    w.add(StaticText::new(Rect::new(2, 1, 34, 2), "Пробел +1, «-» −1, Tab — дальше"));
    w.add(Counter::new(2, 3));
    w.add(Counter::new(2, 5));
    app.insert_window(w);
}

fn tv_main() -> std::io::Result<()> {
    let status = vec![StatusDef::all(vec![StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT)])];
    let mut app = <Program as tvision_gui::ProgramExt>::with_window(None, Some(status))?;
    open_window(&mut app);
    app.run(&mut program::NoHandler);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision::platform::HeadlessBackend;

    #[test]
    fn counts() {
        let mut b = HeadlessBackend::new(80, 25);
        for k in [Key::Char(' '), Key::Char(' '), Key::Char(' '), Key::Char('-'), Key::Tab, Key::Enter] {
            b.push_key(KeyEvent::new(k, Mods::NONE));
        }
        let mut app = Program::new(Box::new(b), None, None);
        open_window(&mut app);
        app.pump(&mut program::NoHandler);
        let s = app.screen_text();
        assert!(s.contains("Счёт: 2") && s.contains("Счёт: 1"), "{}", s);
    }
}

/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),
/// программа примера (`tv_main`) — в отдельном потоке.
fn main() -> std::io::Result<()> {
    tvision_gui::run(tvision_gui::GuiConfig::new("Урок 7. Своё представление: счётчик"), tv_main)
}
