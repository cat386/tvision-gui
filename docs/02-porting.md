# 2. Перевод программы с терминала в окно

Код представлений, групп, окон, диалогов и обработчиков (`AppHandler`) **не
меняется**. tvision-gui заменяет только «экран» — реализацию трейта
`platform::Backend`. Ниже — все места, которые нужно поправить.

## Шаг 1. Зависимость

```toml
[dependencies]
tvision-rs = { path = "../tvision-rs" }    # можно оставить
tvision-gui = { path = "../tvision-gui" }  # добавить
```

## Шаг 2. Импорт

```rust
use tvision::*;        // было
use tvision_gui::*;    // стало (реэкспортирует всё из tvision)
```

Если в программе много модулей с `use tvision::*;`, их можно не трогать: они
продолжат работать. Импорт tvision-gui нужен только там, где создаётся
программа (`with_window`, `GuiBackend`, `run`, `GuiConfig`).

## Шаг 3. Создание программы

| Было (терминал) | Стало (окно) |
|-----------------|--------------|
| `Program::with_terminal(menu, status)?` | `Program::with_window(menu, status)?` |
| `Program::new(platform::default_backend()?, menu, status)` | `Program::new(Box::new(GuiBackend::new()?), menu, status)` |

`with_window` — метод трейта [`ProgramExt`](04-api-reference.md#трейт-programext);
трейт попадает в область видимости вместе с `use tvision_gui::*;`.

## Шаг 4. Функция `main`

Было:

```rust
fn main() -> std::io::Result<()> {
    let mut app = Program::with_terminal(Some(menu()), Some(status()))?;
    app.run(&mut MyApp::new());
    Ok(())
}
```

Стало:

```rust
fn main() -> std::io::Result<()> {
    run(GuiConfig::new("Моя программа"), app_main)
}

fn app_main() -> std::io::Result<()> {
    let mut app = Program::with_window(Some(menu()), Some(status()))?;
    app.run(&mut MyApp::new());
    Ok(())
}
```

Зачем нужен `run`: `Program::run()` и модальные диалоги (`exec_dialog`,
`exec_view`) — это обычные блокирующие циклы, которые сами забирают события.
А окно ОС требует, чтобы его цикл событий работал постоянно, причём в macOS —
обязательно в главном потоке. `run` разводит их: окно работает в главном
потоке, а `app_main` — в отдельном, как будто в терминале. Подробно — в
[разделе 7](07-architecture.md).

На Windows и Linux `run` можно не использовать: если `with_window` вызван без
него, библиотека сама откроет окно в фоновом потоке. Но программа без `run` не
запустится на macOS (`with_window` вернёт ошибку `Unsupported` с подсказкой).

## Шаг 5. Язык надписей

Язык встроенных надписей tvision-rs (`locale::set_language`) хранится
**отдельно для каждого потока**. Если раньше он включался в начале `main`,
перенесите вызов в `app_main`:

```rust
fn app_main() -> std::io::Result<()> {
    locale::set_language(locale::Language::Ru);   // здесь, а не в main
    let mut app = Program::with_window(...)?;
    ...
}
```

Иначе кнопки окон сообщений, диалоги файлов и поиск останутся английскими.

## Шаг 6. Что ещё проверить

* **Аргументы командной строки, файлы настроек, пакетный режим** — разбирайте
  в `main` до вызова `run`, а результат передавайте в замыкание:

  ```rust
  fn main() -> std::io::Result<()> {
      let args: Vec<String> = std::env::args().skip(1).collect();
      if args.iter().any(|a| a == "--version") {
          println!("1.0");
          return Ok(());                     // окно даже не создаётся
      }
      run(GuiConfig::new("Программа"), move || app_main(args))
  }
  ```

  Всё, что передаётся в замыкание, должно быть `Send` (строки, пути, числа,
  обычные структуры — подходят; `Rc` — нет, его создавайте уже внутри
  `app_main`).

* **`dos_shell`** — работает: окно прячется, оболочка запускается в консоли,
  из которой запущена программа (в Windows без консоли система сама откроет
  окно консоли для `cmd.exe`). После выхода из оболочки окно показывается
  снова.

* **Число строк экрана** (режимы 25/43/50 строк) — в окне его можно менять
  программно функцией [`set_screen_size`](04-api-reference.md#set_screen_size).

* **Заголовок окна** — задаётся в `GuiConfig::new(...)` и меняется во время
  работы функцией [`set_title`](04-api-reference.md#set_title).

## Программа, которая умеет и окно, и терминал

Удобно оставить терминальный режим, например, по ключу командной строки.
Так сделано в dlgdsn-rs (ключ `/T`):

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use tvision::*;

static GUI: AtomicBool = AtomicBool::new(false);

/// Работает ли программа в окне (можно спрашивать из любого модуля).
pub fn gui_mode() -> bool {
    GUI.load(Ordering::Relaxed)
}

fn main() -> std::io::Result<()> {
    let terminal = std::env::args().any(|a| a == "/T" || a == "--terminal");
    if terminal {
        return app_main();
    }
    GUI.store(true, Ordering::Relaxed);
    tvision_gui::run(tvision_gui::GuiConfig::new("Программа"), app_main)
}

fn app_main() -> std::io::Result<()> {
    locale::set_language(locale::Language::Ru);
    let mut app = if gui_mode() {
        use tvision_gui::ProgramExt;
        Program::with_window(Some(menu()), Some(status()))?
    } else {
        Program::with_terminal(Some(menu()), Some(status()))?
    };
    app.run(&mut MyApp::new());
    Ok(())
}
```

Возможности, которых нет в терминале, вызываются только в оконном режиме:

```rust
CM_LINES50 => {
    if crate::gui_mode() {
        tvision_gui::set_screen_size(app.ctx.screen_size.x as u16, 50);
    } else {
        message_box(app, "\x03Число строк задаётся окном терминала.", mf::INFORMATION | mf::OK_BUTTON);
    }
}
```

## Тесты

Тесты на «невидимом» терминале `platform::HeadlessBackend` не меняются: они
создают `Program::new(Box::new(HeadlessBackend::new(80, 25)), ...)`, не открывают
окон и одинаково работают для терминальной и оконной версии программы.
Поэтому удобно держать сборку программы (меню, строка состояния, начальные
окна) в отдельной функции, принимающей `Box<dyn platform::Backend>`:

```rust
fn build(backend: Box<dyn platform::Backend>) -> (Program, MyApp) {
    let app = Program::new(backend, Some(menu()), Some(status()));
    (app, MyApp::new())
}

fn app_main() -> std::io::Result<()> {
    let (mut app, mut my) = build(Box::new(GuiBackend::new()?));
    app.run(&mut my);
    Ok(())
}

#[test]
fn file_menu_opens() {
    let mut b = platform::HeadlessBackend::new(80, 25);
    b.push_key(kb::F10);                       // открыть строку меню
    b.push_key(KeyEvent::new(Key::Enter, Mods::NONE));
    let (mut app, mut my) = build(Box::new(b));
    app.pump(&mut my);                         // обработать весь ввод
    assert!(app.screen_text().contains("Выход"));
}
```

Так устроен пример `tvdemo`.

## Пример: hello до и после

```diff
-use tvision::*;
+use tvision_gui::*;
 ...
-fn main() -> std::io::Result<()> {
+fn app_main() -> std::io::Result<()> {
+    locale::set_language(locale::Language::Ru);
     let menu = ...;
     let status = ...;
-    let mut app = Program::with_terminal(Some(menu), Some(status))?;
+    let mut app = Program::with_window(Some(menu), Some(status))?;
     app.run(&mut HelloApp);
     Ok(())
 }
+
+fn main() -> std::io::Result<()> {
+    run(GuiConfig::new("Привет, Turbo Vision"), app_main)
+}
```
