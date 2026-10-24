//! Связь программы Turbo Vision с окном: [`GuiBackend`] и запуск [`run`].
//!
//! Окно (winit) и программа (tvision) живут в **разных потоках**:
//!
//! * цикл событий окна обязан работать в главном потоке (это требование
//!   macOS; на Windows и Linux можно и в другом, но не нужно);
//! * а `Program::run()` и вложенные модальные циклы (`exec_view`) — это
//!   обычные блокирующие циклы `while`, которые сами «тянут» события
//!   через `Backend::read()`. Чтобы не переписывать их на обратные вызовы,
//!   программа работает в своём потоке, как будто в терминале.
//!
//! Окно отправляет программе «сырые» события ([`RawEvent`]) по каналу, а
//! программа отдаёт окну готовый кадр через общую ячейку `Shared::frame` и
//! будит цикл окна через `EventLoopProxy`.

use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tvision::platform::{Backend, RawEvent};
use tvision::{Key, KeyEvent, Menu, Mods, Point, Program, ScreenBuffer, StatusDef};
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::config::GuiConfig;
use crate::render::Frame;
use crate::app_window::{App, UserEvent};

/// Данные, общие для окна и программы.
pub(crate) struct Shared {
    /// Последний кадр программы, ещё не забранный окном.
    pub frame: Mutex<Option<Frame>>,
}

/// Концы каналов, которые получает программа при подключении к окну.
pub(crate) struct Connection {
    pub events: Receiver<RawEvent>,
    pub proxy: EventLoopProxy<UserEvent>,
    pub shared: Arc<Shared>,
    /// Окно создано — пришёл начальный размер экрана в ячейках
    /// (или текст ошибки, если окно создать не удалось).
    pub ready: Receiver<Result<Point, String>>,
}

/// Подключение, приготовленное функцией [`run`] для первого `GuiBackend::new()`.
static PENDING: Mutex<Option<Connection>> = Mutex::new(None);
/// Настройки для окна, которое `GuiBackend::new()` создаёт сам (без `run`).
static CONFIG: Mutex<Option<GuiConfig>> = Mutex::new(None);
/// Связь с окном для [`set_title`].
static PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);

/// Создаёт окно-приложение и концы каналов к нему.
fn connect(event_loop: &EventLoop<UserEvent>, cfg: GuiConfig) -> (Connection, App) {
    let proxy = event_loop.create_proxy();
    let shared = Arc::new(Shared { frame: Mutex::new(None) });
    let (ev_tx, ev_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    *PROXY.lock().unwrap() = Some(proxy.clone());
    let app = App::new(cfg.with_env(), shared.clone(), ev_tx, ready_tx);
    (Connection { events: ev_rx, proxy, shared, ready: ready_rx }, app)
}

/// Запускает программу Turbo Vision в графическом окне.
///
/// Окно создаётся в текущем (главном) потоке, а `app_main` выполняется в
/// отдельном потоке; внутри неё программа создаётся как обычно —
/// `Program::with_window(...)` или `Program::new(Box::new(GuiBackend::new()?), ...)`.
/// Когда `app_main` завершается, окно закрывается, и `run` возвращает её
/// результат.
///
/// ```no_run
/// fn main() -> std::io::Result<()> {
///     tvision_gui::run(tvision_gui::GuiConfig::new("Пример"), || {
///         // ... Program::with_window(...) и app.run(...)
///         Ok(())
///     })
/// }
/// ```
///
/// Вызывать нужно из главного потока и только один раз.
pub fn run<F, R>(cfg: GuiConfig, app_main: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("tvision-gui: не удалось создать цикл событий окна");
    let (conn, mut app) = connect(&event_loop, cfg);
    let proxy = conn.proxy.clone();
    *PENDING.lock().unwrap() = Some(conn);

    let worker = std::thread::Builder::new()
        .name("tvision-app".into())
        .spawn(move || {
            // Окно закроется, даже если программа завершится паникой.
            struct CloseOnDrop(EventLoopProxy<UserEvent>);
            impl Drop for CloseOnDrop {
                fn drop(&mut self) {
                    let _ = self.0.send_event(UserEvent::Exit);
                }
            }
            let _guard = CloseOnDrop(proxy);
            app_main()
        })
        .expect("tvision-gui: не удалось запустить поток программы");

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("tvision-gui: ошибка цикла событий: {}", e);
    }
    match worker.join() {
        Ok(r) => r,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Меняет заголовок окна (можно вызывать из любого потока).
pub fn set_title(title: &str) {
    if let Some(p) = PROXY.lock().unwrap().as_ref() {
        let _ = p.send_event(UserEvent::Title(title.to_string()));
    }
}

/// Меняет размер экрана в символах: окно подстраивается так, чтобы в него
/// помещалось ровно `cols`×`rows` ячеек при текущем шрифте (аналог
/// переключения 25/43/50 строк в DOS). Программа получит обычное событие
/// изменения размера. В полноэкранном режиме ничего не делает.
pub fn set_screen_size(cols: u16, rows: u16) {
    if let Some(p) = PROXY.lock().unwrap().as_ref() {
        let _ = p.send_event(UserEvent::Grid(cols.max(1), rows.max(1)));
    }
}

/// Запуск окна в отдельном потоке — когда программа не обёрнута в [`run`].
/// Возможно на Windows и Linux/BSD; на macOS окна живут только в главном потоке.
fn spawn_window_thread(cfg: GuiConfig) -> io::Result<Connection> {
    #[cfg(target_os = "macos")]
    {
        let _ = cfg;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "на macOS окно должно работать в главном потоке: оберните main в tvision_gui::run(...)",
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("tvision-window".into())
            .spawn(move || {
                let mut builder = EventLoop::<UserEvent>::with_user_event();
                #[cfg(windows)]
                {
                    use winit::platform::windows::EventLoopBuilderExtWindows;
                    builder.with_any_thread(true);
                }
                #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
                {
                    // Флаг общий для X11 и Wayland.
                    winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
                }
                let event_loop = match builder.build() {
                    Ok(el) => el,
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let (conn, mut app) = connect(&event_loop, cfg);
                let _ = tx.send(Ok(conn));
                let _ = event_loop.run_app(&mut app);
            })?;
        rx.recv()
            .map_err(|_| io::Error::other("поток окна завершился"))?
            .map_err(|e| io::Error::other(format!("не удалось создать цикл событий окна: {}", e)))
    }
}

/// «Экран» Turbo Vision в графическом окне — реализация [`Backend`].
///
/// Обычно создаётся неявно через [`ProgramExt::with_window`], но можно и
/// вручную: `Program::new(Box::new(GuiBackend::new()?), menu, status)`.
pub struct GuiBackend {
    events: Receiver<RawEvent>,
    proxy: EventLoopProxy<UserEvent>,
    shared: Arc<Shared>,
    size: Point,
    /// Последний отправленный кадр (чтобы не гонять одинаковые).
    last: Option<Frame>,
    /// Сколько раз программа «не заметила» закрытого окна.
    orphan_reads: u32,
}

impl GuiBackend {
    /// Подключается к окну. Внутри [`run`] — к уже созданному окну,
    /// иначе (Windows, Linux) открывает окно с настройками из
    /// [`GuiBackend::set_config`] или настройками по умолчанию.
    pub fn new() -> io::Result<Self> {
        let pending = PENDING.lock().unwrap().take();
        let conn = match pending {
            Some(c) => c,
            None => {
                let cfg = CONFIG.lock().unwrap().clone().unwrap_or_default();
                spawn_window_thread(cfg)?
            }
        };
        // Ждём, пока окно будет создано и станет известен размер в ячейках.
        let size = match conn.ready.recv() {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(io::Error::other(e)),
            Err(_) => return Err(io::Error::other("окно закрылось до запуска программы")),
        };
        Ok(GuiBackend { events: conn.events, proxy: conn.proxy, shared: conn.shared, size, last: None, orphan_reads: 0 })
    }

    /// Настройки для окна, которое откроет следующий `GuiBackend::new()`
    /// без [`run`]. Внутри `run` используются настройки, переданные в `run`.
    pub fn set_config(cfg: GuiConfig) {
        *CONFIG.lock().unwrap() = Some(cfg);
    }

    /// Создаёт подключение с заданными настройками (без [`run`]).
    pub fn with_config(cfg: GuiConfig) -> io::Result<Self> {
        if PENDING.lock().unwrap().is_none() {
            Self::set_config(cfg);
        }
        Self::new()
    }
}

impl Backend for GuiBackend {
    fn size(&self) -> Point {
        self.size
    }

    fn read(&mut self, timeout: Duration) -> Option<RawEvent> {
        match self.events.recv_timeout(timeout) {
            Ok(ev) => {
                if let RawEvent::Resize(s) = ev {
                    self.size = s;
                }
                Some(ev)
            }
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                // Окна больше нет. Просим программу выйти «по-хорошему» (Alt+X),
                // а если она не реагирует — завершаем процесс.
                std::thread::sleep(timeout);
                self.orphan_reads += 1;
                if self.orphan_reads > 50 {
                    std::process::exit(0);
                }
                Some(RawEvent::Key(KeyEvent::new(Key::Char('x'), Mods::ALT)))
            }
        }
    }

    fn flush(&mut self, screen: &ScreenBuffer, cursor: Option<Point>, insert: bool) {
        // Program вызывает flush на каждой итерации цикла (примерно 10 раз
        // в секунду даже без событий) — одинаковые кадры не отправляем.
        if let Some(l) = &self.last {
            if l.cursor == cursor && l.insert == insert && l.screen == *screen {
                return;
            }
        }
        let frame = Frame { screen: screen.clone(), cursor, insert };
        *self.shared.frame.lock().unwrap() = Some(frame.clone());
        self.last = Some(frame);
        let _ = self.proxy.send_event(UserEvent::Frame);
    }

    /// Выход в оболочку (`dos_shell`): окно прячется на время работы оболочки.
    /// Оболочка запускается в консоли, из которой запущена программа
    /// (в Windows без консоли система откроет для неё новое окно консоли).
    fn suspend(&mut self) {
        let _ = self.proxy.send_event(UserEvent::Visible(false));
    }

    fn resume(&mut self) {
        let _ = self.proxy.send_event(UserEvent::Visible(true));
        // После оболочки окно рисуется заново целиком.
        self.last = None;
    }

    fn system_clipboard(&self) -> bool {
        true
    }
}

impl Drop for GuiBackend {
    fn drop(&mut self) {
        // Программа закончилась — закрываем окно.
        let _ = self.proxy.send_event(UserEvent::Exit);
    }
}

/// Конструкторы `Program` для графического окна — пара к
/// `Program::with_terminal` из tvision-rs.
pub trait ProgramExt: Sized {
    /// Программа в графическом окне (аналог `Program::with_terminal`).
    fn with_window(menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self>;

    /// То же с явными настройками окна (действуют, если программа не
    /// обёрнута в [`run`] — иначе используются настройки `run`).
    fn with_window_config(cfg: GuiConfig, menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self>;
}

impl ProgramExt for Program {
    fn with_window(menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self> {
        Ok(Program::new(Box::new(GuiBackend::new()?), menu, status))
    }

    fn with_window_config(cfg: GuiConfig, menu: Option<Menu>, status: Option<Vec<StatusDef>>) -> io::Result<Self> {
        Ok(Program::new(Box::new(GuiBackend::with_config(cfg)?), menu, status))
    }
}
