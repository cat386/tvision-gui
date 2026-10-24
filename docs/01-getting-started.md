# 1. Быстрый старт

## Что нужно

* Rust 1.74 или новее (`rustup update`);
* библиотека tvision-rs, лежащая рядом: tvision-gui подключает её как
  `path = "../tvision-rs"`;
* доступ к crates.io при первой сборке: tvision-gui использует три внешних
  пакета — [winit](https://crates.io/crates/winit) (окно и события),
  [softbuffer](https://crates.io/crates/softbuffer) (вывод кадра) и
  [fontdue](https://crates.io/crates/fontdue) (растеризация шрифтов).

Ожидаемая раскладка каталогов:

```
develop/rust/
├── tvision-rs/     # сама библиотека Turbo Vision
├── tvision-gui/    # графическое окно для неё
└── моя-программа/
```

## Подключение

`моя-программа/Cargo.toml`:

```toml
[package]
name = "moya-programma"
version = "0.1.0"
edition = "2021"

[dependencies]
tvision-gui = { path = "../tvision-gui" }
```

tvision-gui реэкспортирует всё содержимое tvision-rs, поэтому программе
хватает одной строки `use tvision_gui::*;`. Сам крейт tvision-rs при этом
тоже виден под именем `tvision`, так что пути вида `tvision::locale::...`
и макросы `tvision::impl_view_basics!()` работают без изменений.

Прямая зависимость `tvision-rs = { path = "../tvision-rs" }` нужна, только
если программа должна уметь работать и в терминале без tvision-gui, или
если вы хотите явно видеть её в `Cargo.toml`. Она не мешает: обе зависимости
указывают на один и тот же крейт.

## Первая программа

`src/main.rs`:

```rust
use tvision_gui::*;

/// Своя команда.
const CM_HELLO: u16 = 100;

struct App;

impl AppHandler for App {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        if ev.command() == Some(CM_HELLO) {
            message_box(app, "\x03Привет из графического окна!", mf::INFORMATION | mf::OK_BUTTON);
            ev.clear();
        }
    }
}

/// Тело программы. Работает в отдельном потоке (см. раздел 7).
fn app_main() -> std::io::Result<()> {
    // Надписи библиотеки (кнопки «Отмена», диалоги файлов...) — по-русски.
    // Включать обязательно здесь, а не в main: язык хранится для каждого потока.
    locale::set_language(locale::Language::Ru);

    let menu = Menu::new().submenu(
        "~П~ример",
        Menu::new()
            .item_key("~П~ривет", CM_HELLO, kb::F2, "F2")
            .separator()
            .item_key("В~ы~ход", cm::QUIT, kb::ALT_X, "Alt-X"),
    );
    let status = vec![StatusDef::all(vec![
        StatusItem::new("~F2~ Привет", kb::F2, CM_HELLO),
        StatusItem::new("~Alt-X~ Выход", kb::ALT_X, cm::QUIT),
        StatusItem::new("", kb::F10, cm::MENU),
    ])];

    let mut app = Program::with_window(Some(menu), Some(status))?;
    app.run(&mut App);
    Ok(())
}

fn main() -> std::io::Result<()> {
    // Окно — в главном потоке, программа — в своём.
    run(GuiConfig::new("Первая программа").size(80, 25), app_main)
}
```

Запуск:

```
cargo run
```

Откроется окно 80×25 символов с меню, строкой состояния и рабочим столом.
F2 показывает сообщение, Alt+X или крестик окна завершают программу.

## Примеры из библиотеки

```
cd tvision-gui
cargo run --example hello    # «Привет, мир» — диалог с кнопками
cargo run --example tvedit   # многооконный текстовый редактор
cargo run --example tvdemo   # калькулятор, календарь, пятнашки, таблица ASCII,
                             # просмотр файлов, настройка цветов и мыши, справка F1
cargo test                   # тесты отрисовки и псевдографики
```

Все примеры на русском. Они собраны из одноимённых примеров tvision-rs:
код окон и диалогов тот же, отличаются только `main` и надписи.

## Сборка для распространения

```
cargo build --release
```

В Windows программа с обычным `main` — консольная: рядом с окном программы
откроется окно консоли. Чтобы его не было в выпускной сборке, добавьте первой
строкой `main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

В отладочной сборке консоль останется — в неё удобно выводить `eprintln!`.

Отладочная сборка заметно медленнее выпускной (особенно растеризация шрифтов
при первом показе символов). Для проверки «на глаз» лучше `cargo run --release`.
