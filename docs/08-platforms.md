# 8. Платформы

| Платформа | Окно | Состояние |
|-----------|------|-----------|
| Windows 10 / 11 | Win32 | проверено: сборка, тесты, примеры hello, tvdemo, dlgdsn-rs |
| macOS 11+ | Cocoa | код готов, на реальной машине не проверялось |
| Linux, BSD — X11 | Xlib/XCB | код готов, на реальной машине не проверялось |
| Linux, BSD — Wayland | Wayland | код готов, на реальной машине не проверялось |

## Windows

* Сборка не требует ничего, кроме Rust (MSVC или GNU).
* Программа с обычным `main` — консольная: при запуске из Проводника рядом
  откроется окно консоли. Для выпускной сборки:

  ```rust
  #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
  ```

  В такой программе `println!`/`eprintln!` никуда не выводятся.
* `dos_shell` в программе без консоли: Windows сама откроет новое окно
  консоли для `cmd.exe` (или `%COMSPEC%`); окно программы на это время
  прячется.
* Alt+Пробел передаётся программе, системное меню окна не открывается
  (проверено в tvdemo: открывается меню ≡).
* Alt+F4 работает как крестик окна (посылает программе Alt+X).
* Поддерживается масштаб экрана Windows (125 %, 150 % ...) и разные масштабы
  на разных мониторах.
* Шрифт по умолчанию — Consolas; для иероглифов подключаются MS Gothic,
  Microsoft YaHei, SimSun, Malgun Gothic.

## macOS

* **Обязательно** оборачивать `main` в `tvision_gui::run(...)`: окно может
  работать только в главном потоке. Без `run` `with_window` вернёт ошибку
  `Unsupported`.
* Option и Cmd работают как Alt (Option+X — выход в программах Turbo Vision).
  Сочетания, которые перехватывает сама macOS (Cmd+Tab, Cmd+Пробел и т.п.),
  программа не получит.
* Шрифт по умолчанию — Menlo.
* Окно меняет размер шагами в один символ.
* Для распространения программу обычно упаковывают в `.app`
  (например, утилитой `cargo-bundle`); для запуска из терминала
  (`cargo run`) этого не нужно.

## Linux и BSD

### X11 и Wayland

winit сам выбирает оконную систему: Wayland, если задана переменная
`WAYLAND_DISPLAY`, иначе X11. Принудительно выбрать X11 в сеансе Wayland
(через XWayland):

```
WAYLAND_DISPLAY= ./программа
```

Отличия Wayland:

* окно появляется только после первого кадра;
* размер окна меняется плавно, а не шагами в символ;
* заголовок и рамку окна рисует сама программа (winit), они могут выглядеть
  не так, как у других окон.

### Системные библиотеки

Для **сборки** системные пакеты не нужны: winit и softbuffer загружают
библиотеки оконной системы при запуске. Для **работы** нужны библиотеки,
которые есть в любом графическом окружении:

| Debian / Ubuntu | Fedora | Arch |
|-----------------|--------|------|
| `libx11-6 libxcursor1 libxrandr2 libxi6 libxcb1 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0` | `libX11 libXcursor libXrandr libXi libxcb libxkbcommon libxkbcommon-x11 libwayland-client` | `libx11 libxcursor libxrandr libxi libxcb libxkbcommon libxkbcommon-x11 wayland` |

### Шрифты

Шрифт ищется через `fc-match monospace` (утилита fontconfig), поэтому
используется тот моноширинный шрифт, который выбран в настройках системы.
Если fontconfig нет, перебираются стандартные пути DejaVu, Liberation,
Noto, Ubuntu Mono, FreeMono. Для иероглифов полезно установить Noto CJK
(`fonts-noto-cjk` в Debian/Ubuntu).

### Без графического сеанса

По SSH без X11 и Wayland окно создать нельзя: `run` завершится паникой.
Если программа должна работать и там, проверяйте окружение и запускайте
терминальную версию:

```rust
fn has_display() -> bool {
    !cfg!(all(unix, not(target_os = "macos")))
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
```

## Буфер обмена

`GuiBackend` включает системный буфер обмена tvision-rs, поэтому копирование
и вставка в редакторе и строках ввода работают с буфером ОС так же, как в
терминальной версии: WinAPI в Windows, `pbcopy`/`pbpaste` в macOS,
`wl-copy`/`wl-paste` или `xclip`/`xsel` в Linux.
