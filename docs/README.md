# Документация tvision-gui

**tvision-gui** — библиотека для Rust, которая запускает программы на
[tvision-rs](../../tvision-rs) (порт Turbo Vision) в собственном графическом
окне вместо терминала. API Turbo Vision остаётся прежним: те же `Program`,
`View`, `Group`, `Window`, `Dialog`, меню, редактор, файловые диалоги и справка.

## Содержание

| Раздел | О чём |
|--------|-------|
| [1. Быстрый старт](01-getting-started.md) | подключение, первая программа, запуск примеров |
| [2. Перевод программы с терминала в окно](02-porting.md) | пошаговая инструкция, работа в обоих режимах, тесты |
| [3. Настройки окна](03-configuration.md) | `GuiConfig`, шрифты, палитры, переменные окружения |
| [4. Справочник API](04-api-reference.md) | все публичные функции и типы |
| [5. Окно, клавиатура и мышь](05-window-and-input.md) | перевод клавиш, мышь, курсор, размер окна, горячие клавиши |
| [6. Отрисовка и шрифты](06-rendering.md) | поиск шрифтов, псевдографика, цвета, скорость |
| [7. Устройство библиотеки](07-architecture.md) | потоки, каналы, жизненный цикл, модули |
| [8. Платформы](08-platforms.md) | Windows, macOS, Linux (X11 и Wayland) |
| [9. Вопросы и проблемы](09-faq.md) | частые вопросы и способы решения |

## Коротко

```rust
use tvision_gui::*;

fn app_main() -> std::io::Result<()> {
    locale::set_language(locale::Language::Ru);
    let menu = Menu::new().submenu("~Ф~айл", Menu::new().item_key("В~ы~ход", cm::QUIT, kb::ALT_X, "Alt-X"));
    let status = vec![StatusDef::all(vec![StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT)])];
    let mut app = Program::with_window(Some(menu), Some(status))?;
    struct App;
    impl AppHandler for App {}
    app.run(&mut App);
    Ok(())
}

fn main() -> std::io::Result<()> {
    run(GuiConfig::new("Моя программа"), app_main)
}
```

Описание каждой функции есть и в самом коде (комментарии на русском):
`cargo doc --open` строит справочник по исходникам.
