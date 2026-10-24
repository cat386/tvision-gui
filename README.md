# tvision-gui

**Turbo Vision в графическом окне** — тот же API, что у [tvision-rs](../tvision-rs),
но программа рисуется не в терминале, а в собственном окне ОС.

* Windows, macOS, Linux/BSD (X11 и Wayland) — через [winit](https://crates.io/crates/winit);
* кадр выводится без GPU через [softbuffer](https://crates.io/crates/softbuffer),
  шрифты растеризуются [fontdue](https://crates.io/crates/fontdue);
* системный моноширинный шрифт (Consolas / Menlo / DejaVu Sans Mono ...) с
  запасными шрифтами для редких символов и иероглифов;
* рамки и блоки псевдографики (`─│┌╔═╬░▒▓█▀▄`) рисуются программно и стыкуются
  без щелей при любом размере шрифта;
* 16 цветов BIOS (палитра VGA, можно заменить), мигающий курсор, мышь и колесо,
  изменение размера окна, HiDPI, полноэкранный режим, системный буфер обмена.

Вся логика интерфейса — представления, группы, окна, диалоги, меню, редактор,
файловые диалоги, справка — берётся из tvision-rs **без изменений**. tvision-gui
добавляет только новую реализацию трейта `platform::Backend`.

## Примеры

```
cargo run --example hello    # hello.cpp
cargo run --example tvedit   # многооконный редактор
cargo run --example tvdemo   # tvdemo: калькулятор, календарь, пятнашки, ASCII ...
cargo test                   # тесты псевдографики и отрисовки
```

## Перевод программы с терминала в окно

```toml
[dependencies]
tvision-gui = { path = "../tvision-gui" }
```

```rust
use tvision_gui::*;               // было: use tvision::*;

fn app_main() -> std::io::Result<()> {
    // было: Program::with_terminal(...)
    let mut app = Program::with_window(Some(menu()), Some(status()))?;
    app.run(&mut MyApp);
    Ok(())
}

fn main() -> std::io::Result<()> {
    run(GuiConfig::new("Моя программа").size(100, 30), app_main)
}
```

| tvision-rs (терминал)                 | tvision-gui (окно)                              |
|---------------------------------------|-------------------------------------------------|
| `use tvision::*;`                     | `use tvision_gui::*;` (реэкспорт всего tvision) |
| `Program::with_terminal(m, s)`        | `Program::with_window(m, s)`                    |
| `platform::default_backend()?`        | `Box::new(GuiBackend::new()?)`                  |
| `fn main() { ... }`                   | `fn main() { run(GuiConfig::new(..), app_main) }` |

Обёртка `run` нужна потому, что на macOS окно может работать только в главном
потоке, а `Program::run()` — блокирующий цикл. `run` запускает окно в главном
потоке, а программу — в отдельном. На Windows и Linux без `run` тоже работает:
`with_window` сам откроет окно в фоновом потоке.

Чтобы на Windows не открывалось окно консоли, добавьте в начало `main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```

## Настройки

```rust
GuiConfig::new("Заголовок")
    .size(80, 25)                 // начальный размер в символах
    .font_size(18.0)              // размер шрифта, логические пиксели
    .font("C:/Windows/Fonts/lucon.ttf")
    .palette(VGA_PALETTE)         // 16 цветов 0x00RRGGBB
    .cursor_blink(true)
    .resizable(true)
    .fullscreen(false)
```

Переменные окружения (важнее настроек в коде): `TVISION_GUI_FONT` — файл шрифта,
`TVISION_GUI_FONT_SIZE` — размер шрифта.

Заголовок окна во время работы: `tvision_gui::set_title("...")`.

## Клавиши окна

| Клавиша             | Действие                                        |
|---------------------|-------------------------------------------------|
| Alt+Enter           | полный экран / окно                             |
| Ctrl+= / Ctrl+-     | крупнее / мельче шрифт (размер экрана в символах меняется) |
| Ctrl+0              | исходный размер шрифта                          |
| крестик окна        | посылает программе Alt+X; второе нажатие — принудительный выход |

Остальные клавиши переводятся так же, как в консоли Windows у tvision-rs:
Alt/Ctrl+буква — по физической клавише (Alt+X работает в русской раскладке),
на macOS Option и Cmd работают как Alt.

## Устройство

```
 главный поток (окно, winit)               поток программы (tvision)
 ───────────────────────────               ─────────────────────────
 клавиатура, мышь, размер   ── RawEvent ─▶  GuiBackend::read()
                                            Program::run() — как обычно
 Renderer → softbuffer      ◀── кадр ────   GuiBackend::flush(ScreenBuffer)
```

| Файл              | Что внутри                                                  |
|-------------------|-------------------------------------------------------------|
| `src/backend.rs`  | `GuiBackend` (трейт `Backend`), `run`, `ProgramExt::with_window` |
| `src/app_window.rs` | окно winit: события, мигание курсора, горячие клавиши       |
| `src/keys.rs`     | перевод клавиш winit в `KeyEvent` Turbo Vision              |
| `src/render.rs`   | ячейки → пиксели, перерисовка только изменившихся ячеек     |
| `src/font.rs`     | поиск системных шрифтов, метрики ячейки, растеризация       |
| `src/boxdraw.rs`  | программная отрисовка рамок и блоков                        |
| `src/config.rs`   | `GuiConfig`, палитра VGA                                    |

## Ограничения

* `dos_shell` прячет окно и запускает оболочку в консоли, из которой запущена
  программа (в Windows без консоли система откроет новое окно консоли);
* метод ввода (IME) для иероглифов пока не подключён — ввод идёт по символам
  с клавиатуры;
* одна программа (`Program`) на один вызов `run`.
