//! Окно ОС: создание, события клавиатуры и мыши, вывод кадра.
//!
//! [`App`] — обработчик цикла событий winit (`ApplicationHandler`).
//! Он переводит события окна в «сырые» события Turbo Vision
//! ([`RawEvent`]) и отправляет их программе, а кадры программы рисует
//! через [`Renderer`] и softbuffer.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tvision::event::{MB_LEFT, MB_MIDDLE, MB_RIGHT};
use tvision::platform::RawEvent;
use tvision::{cm, Point};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, ModifiersState};
use winit::window::{Fullscreen, Window, WindowId};

use crate::backend::Shared;
use crate::config::GuiConfig;
use crate::font::FontSet;
use crate::keys;
use crate::render::{Frame, Renderer};
use crate::settings::WindowGeometry;

/// Период мигания курсора (как в консоли Windows).
const BLINK: Duration = Duration::from_millis(530);

/// Сообщения от потока программы к окну.
#[derive(Debug)]
pub(crate) enum UserEvent {
    /// Готов новый кадр (лежит в `Shared::frame`).
    Frame,
    /// Программа завершилась — закрыть окно.
    Exit,
    /// Сменить заголовок окна.
    Title(String),
    /// Показать / спрятать окно (на время `dos_shell`).
    Visible(bool),
    /// Подогнать окно под заданное число столбцов и строк.
    Grid(u16, u16),
    /// Сменить шрифт: файл (`None` — системный) и размер (логические пиксели).
    SetFont(Option<std::path::PathBuf>, f32),
    /// Сменить 16 цветов BIOS окна.
    SetPalette([u32; 16]),
}

/// Всё, что появляется после создания окна.
struct Gfx {
    window: Rc<Window>,
    // Контекст должен жить не меньше поверхности.
    _context: softbuffer::Context<Rc<Window>>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    renderer: Renderer,
}

/// Обработчик цикла событий окна.
pub(crate) struct App {
    cfg: GuiConfig,
    shared: Arc<Shared>,
    /// Канал событий к программе.
    tx: Sender<RawEvent>,
    /// Одноразовый канал «окно готово, вот размер в ячейках».
    ready: Option<Sender<Result<Point, String>>>,
    gfx: Option<Gfx>,
    /// Последний кадр программы.
    frame: Option<Frame>,
    /// Текущий размер сетки в ячейках (как его знает программа).
    grid: Point,
    /// Текущий размер шрифта в логических пикселях (меняется Ctrl+= / Ctrl+-).
    font_size: f32,
    modifiers: ModifiersState,
    /// Мышь: позиция в ячейках и нажатые кнопки.
    mouse_cell: Point,
    mouse_px: (f64, f64),
    buttons: u8,
    /// Накопленная прокрутка тачпада в пикселях.
    wheel_acc: f64,
    /// Мигание курсора.
    blink_on: bool,
    next_blink: Instant,
    /// Окно уже было показано (до первого кадра оно скрыто, чтобы не мелькало пустым).
    shown: bool,
    /// Когда пользователь в последний раз пытался закрыть окно.
    last_close: Option<Instant>,
    /// Когда пришёл последний кадр от программы.
    last_frame_at: Option<Instant>,
    /// Размер и положение «обычного» окна (не развёрнутого и не на весь
    /// экран) — то, что сохраняется в файл настроек.
    windowed: WindowGeometry,
}

impl App {
    pub fn new(cfg: GuiConfig, shared: Arc<Shared>, tx: Sender<RawEvent>, ready: Sender<Result<Point, String>>) -> Self {
        let grid = Point::new(cfg.cols as i32, cfg.rows as i32);
        let font_size = cfg.font_size;
        let windowed = WindowGeometry { cols: cfg.cols, rows: cfg.rows, position: cfg.position, maximized: cfg.maximized, fullscreen: cfg.fullscreen };
        App {
            cfg,
            shared,
            tx,
            ready: Some(ready),
            gfx: None,
            frame: None,
            grid,
            font_size,
            modifiers: ModifiersState::empty(),
            mouse_cell: Point::new(0, 0),
            mouse_px: (0.0, 0.0),
            buttons: 0,
            wheel_acc: 0.0,
            blink_on: true,
            next_blink: Instant::now() + BLINK,
            shown: false,
            last_close: None,
            last_frame_at: None,
            windowed,
        }
    }

    /// Отправить событие программе (если она уже завершилась — не важно).
    fn send(&self, ev: RawEvent) {
        let _ = self.tx.send(ev);
    }

    fn send_mouse(&self, wheel: i8) {
        self.send(RawEvent::Mouse { pos: self.mouse_cell, buttons: self.buttons, mods: keys::mods(self.modifiers), wheel });
    }

    /// Создаёт окно, поверхность для рисования и шрифт.
    fn create(&mut self, el: &ActiveEventLoop) -> Result<Gfx, String> {
        // Масштаб монитора до создания окна известен не везде (в Wayland — нет),
        // поэтому сначала оцениваем по основному монитору, а после создания
        // при необходимости пересчитываем.
        let guess = el.primary_monitor().map(|m| m.scale_factor()).unwrap_or(1.0);
        let font = FontSet::load(self.cfg.font_path.as_deref(), self.font_size * guess as f32);
        let (cw, ch) = (font.cell_w as u32, font.cell_h as u32);
        let size = PhysicalSize::new(cw * self.cfg.cols as u32, ch * self.cfg.rows as u32);

        let mut attrs = Window::default_attributes()
            .with_title(self.cfg.title.clone())
            .with_inner_size(size)
            .with_min_inner_size(PhysicalSize::new(cw * 10, ch * 4))
            .with_resizable(self.cfg.resizable)
            // Окно меняет размер шагами в одну ячейку (macOS, X11).
            .with_resize_increments(PhysicalSize::new(cw, ch))
            .with_visible(false);
        if self.cfg.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if self.cfg.maximized {
            attrs = attrs.with_maximized(true);
        }
        // Сохранённое положение — только если оно на одном из мониторов
        // (монитор могли отключить с прошлого запуска).
        if let Some((x, y)) = self.cfg.position {
            let visible = el.available_monitors().any(|m| {
                let (p, s) = (m.position(), m.size());
                let (cx, cy) = (x + 40, y + 10);
                cx >= p.x && cx < p.x + s.width as i32 && cy >= p.y && cy < p.y + s.height as i32
            });
            if visible {
                attrs = attrs.with_position(PhysicalPosition::new(x, y));
            }
        }
        let window = Rc::new(el.create_window(attrs).map_err(|e| format!("не удалось создать окно: {}", e))?);

        let scale = window.scale_factor();
        let font = if (scale - guess).abs() > 1e-3 { FontSet::load(self.cfg.font_path.as_deref(), self.font_size * scale as f32) } else { font };
        if (scale - guess).abs() > 1e-3 {
            let s = PhysicalSize::new(font.cell_w as u32 * self.cfg.cols as u32, font.cell_h as u32 * self.cfg.rows as u32);
            let _ = window.request_inner_size(s);
            window.set_resize_increments(Some(PhysicalSize::new(font.cell_w as u32, font.cell_h as u32)));
        }

        let context = softbuffer::Context::new(window.clone()).map_err(|e| format!("softbuffer: {}", e))?;
        let surface = softbuffer::Surface::new(&context, window.clone()).map_err(|e| format!("softbuffer: {}", e))?;
        let mut renderer = Renderer::new(font, self.cfg.palette);
        // Диалог настроек и программа видят, с какими настройками открыто окно.
        crate::settings::init_current(
            crate::settings::GuiSettings {
                theme: self.cfg.theme.clone(),
                font: self.cfg.font_path.clone(),
                font_size: self.font_size,
                window: Some(self.windowed.clone()),
            },
            self.cfg.palette,
            self.cfg.remember_settings,
        );
        let inner = window.inner_size();
        renderer.resize(inner.width as usize, inner.height as usize);
        Ok(Gfx { window, _context: context, surface, renderer })
    }

    /// Запоминает размер и положение окна для файла настроек. Размер и
    /// положение берутся только у обычного окна: у развёрнутого, свёрнутого
    /// или полноэкранного запоминается лишь сам этот режим.
    fn note_geometry(&mut self) {
        let Some(g) = &self.gfx else { return };
        let w = &g.window;
        if w.is_minimized() == Some(true) {
            return;
        }
        let full = w.fullscreen().is_some();
        let max = w.is_maximized();
        self.windowed.fullscreen = full;
        self.windowed.maximized = max;
        if !full && !max {
            self.windowed.cols = self.grid.x.clamp(1, u16::MAX as i32) as u16;
            self.windowed.rows = self.grid.y.clamp(1, u16::MAX as i32) as u16;
            // В Windows у свёрнутого окна координаты −32000.
            if let Ok(p) = w.outer_position() {
                if p.x > -30000 && p.y > -30000 {
                    self.windowed.position = Some((p.x, p.y));
                }
            }
        }
        crate::settings::note_geometry(self.windowed.clone());
    }

    /// Пересчитывает сетку по размеру окна; если она изменилась — сообщает программе.
    fn update_grid(&mut self) {
        let Some(g) = &self.gfx else { return };
        let r = &g.renderer;
        if r.width == 0 || r.height == 0 {
            // Окно свёрнуто — размер сетки не трогаем.
            return;
        }
        let grid = r.grid_for(r.width, r.height);
        if grid != self.grid {
            self.grid = grid;
            self.send(RawEvent::Resize(grid));
        }
    }

    /// Меняет размер шрифта (Ctrl+= / Ctrl+- / Ctrl+0) или масштаб монитора.
    fn reload_font(&mut self) {
        let Some(g) = &mut self.gfx else { return };
        let scale = g.window.scale_factor() as f32;
        let font = FontSet::load(self.cfg.font_path.as_deref(), self.font_size * scale);
        g.window.set_resize_increments(Some(PhysicalSize::new(font.cell_w as u32, font.cell_h as u32)));
        g.renderer.set_font(font);
        g.window.request_redraw();
        self.update_grid();
        self.note_geometry();
    }

    /// Горячие клавиши самого окна. Возвращает `true`, если клавиша обработана.
    fn window_hotkey(&mut self, code: Option<KeyCode>) -> bool {
        let m = keys::mods(self.modifiers);
        let Some(code) = code else { return false };
        // Alt+Enter — полный экран.
        if m.alt && !m.ctrl && code == KeyCode::Enter || code == KeyCode::NumpadEnter && m.alt {
            if let Some(g) = &self.gfx {
                let full = g.window.fullscreen().is_some();
                g.window.set_fullscreen(if full { None } else { Some(Fullscreen::Borderless(None)) });
            }
            return true;
        }
        // Ctrl+= / Ctrl+- / Ctrl+0 — размер шрифта.
        if m.ctrl && !m.alt {
            let new = match code {
                KeyCode::Equal | KeyCode::NumpadAdd => (self.font_size + 1.0).min(96.0),
                KeyCode::Minus | KeyCode::NumpadSubtract => (self.font_size - 1.0).max(6.0),
                KeyCode::Digit0 | KeyCode::Numpad0 => self.cfg.font_size,
                _ => return false,
            };
            if new != self.font_size {
                self.font_size = new;
                crate::settings::note_font_size(new);
                self.reload_font();
            }
            return true;
        }
        false
    }

    /// Выводит кадр в окно.
    fn paint(&mut self) {
        let cursor_on = self.blink_on || !self.cfg.cursor_blink;
        let Some(g) = &mut self.gfx else { return };
        let size = g.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        g.renderer.resize(size.width as usize, size.height as usize);
        if let Some(f) = &self.frame {
            g.renderer.update(f, cursor_on);
        }
        if g.surface.resize(w, h).is_err() {
            return;
        }
        if let Ok(mut buf) = g.surface.buffer_mut() {
            let src = g.renderer.pixels();
            if buf.len() == src.len() {
                buf.copy_from_slice(src);
            }
            let _ = buf.present();
        }
    }

    /// Курсор только что сдвинулся или изменился кадр — показать его сразу,
    /// а не ждать следующей фазы мигания (так курсор не «пропадает» при наборе).
    fn reset_blink(&mut self) {
        self.blink_on = true;
        self.next_blink = Instant::now() + BLINK;
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        match self.create(el) {
            Ok(g) => {
                self.gfx = Some(g);
                // Программа получает тот размер экрана, который заказан в настройках;
                // если оконная система дала другой, придёт событие Resize.
                if let Some(r) = self.ready.take() {
                    let _ = r.send(Ok(self.grid));
                }
                self.update_grid();
            }
            Err(e) => {
                eprintln!("tvision-gui: {}", e);
                if let Some(r) = self.ready.take() {
                    let _ = r.send(Err(e));
                }
                el.exit();
            }
        }
    }

    fn new_events(&mut self, _el: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if Instant::now() >= self.next_blink {
                self.blink_on = !self.blink_on;
                self.next_blink = Instant::now() + BLINK;
                if let Some(g) = &self.gfx {
                    g.window.request_redraw();
                }
            }
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Frame => {
                let f = self.shared.frame.lock().unwrap().take();
                if let Some(f) = f {
                    self.last_frame_at = Some(Instant::now());
                    if self.frame.as_ref().map(|o| o.cursor != f.cursor).unwrap_or(true) {
                        self.reset_blink();
                    }
                    self.frame = Some(f);
                    if let Some(g) = &self.gfx {
                        if !self.shown {
                            g.window.set_visible(true);
                            g.window.focus_window();
                            self.shown = true;
                        }
                        g.window.request_redraw();
                    }
                }
            }
            UserEvent::Exit => el.exit(),
            UserEvent::Title(t) => {
                if let Some(g) = &self.gfx {
                    g.window.set_title(&t);
                }
            }
            UserEvent::Grid(cols, rows) => {
                if let Some(g) = &self.gfx {
                    if g.window.fullscreen().is_none() {
                        let (cw, ch) = (g.renderer.cell_w() as u32, g.renderer.cell_h() as u32);
                        let _ = g.window.request_inner_size(PhysicalSize::new(cw * cols as u32, ch * rows as u32));
                    }
                }
            }
            UserEvent::SetFont(path, size) => {
                self.cfg.font_path = path;
                self.font_size = size;
                self.reload_font();
            }
            UserEvent::SetPalette(p) => {
                self.cfg.palette = p;
                if let Some(g) = &mut self.gfx {
                    g.renderer.set_palette(p);
                    g.window.request_redraw();
                }
            }
            UserEvent::Visible(v) => {
                if let Some(g) = &mut self.gfx {
                    g.window.set_visible(v);
                    if v {
                        g.renderer.invalidate();
                        g.window.request_redraw();
                    }
                }
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                let now = Instant::now();
                // Программа «не отвечает», если после прошлой просьбы закрыть
                // окно прошло больше 2 секунд, а нового кадра так и не было.
                let stuck = match self.last_close {
                    Some(t) => now.duration_since(t) > Duration::from_secs(2) && self.last_frame_at.map(|f| f < t).unwrap_or(true),
                    None => false,
                };
                if stuck {
                    // Программа не отвечает: настройки сохраняем сами.
                    let _ = crate::settings::save_current();
                    el.exit();
                    std::process::exit(0);
                }
                self.last_close = Some(now);
                // Обычный выход Turbo Vision — команда `cm::QUIT`: программа
                // сама спросит про несохранённые файлы и т.п.
                self.send(RawEvent::Command(cm::QUIT));
            }
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gfx {
                    g.renderer.resize(size.width as usize, size.height as usize);
                    g.window.request_redraw();
                }
                self.update_grid();
                self.note_geometry();
            }
            WindowEvent::Moved(_) => self.note_geometry(),
            WindowEvent::ScaleFactorChanged { .. } => {
                // Монитор с другим масштабом: шрифт в физических пикселях меняется.
                // Новый размер окна придёт отдельным событием Resized.
                self.reload_font();
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::Focused(false) => {
                // Отпускание кнопок вне окна могло потеряться — «отпускаем» сами.
                self.modifiers = ModifiersState::empty();
                if self.buttons != 0 {
                    self.buttons = 0;
                    self.send_mouse(0);
                }
            }
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                if is_synthetic {
                    return;
                }
                if event.state == ElementState::Pressed && self.window_hotkey(keys::code(event.physical_key)) {
                    return;
                }
                for k in keys::translate(&event, self.modifiers) {
                    self.send(RawEvent::Key(k));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_px = (position.x, position.y);
                let Some(g) = &self.gfx else { return };
                let (cw, ch) = (g.renderer.cell_w() as f64, g.renderer.cell_h() as f64);
                let x = ((position.x / cw).floor() as i32).clamp(0, self.grid.x - 1);
                let y = ((position.y / ch).floor() as i32).clamp(0, self.grid.y - 1);
                let cell = Point::new(x, y);
                if cell != self.mouse_cell {
                    self.mouse_cell = cell;
                    self.send_mouse(0);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let bit = match button {
                    MouseButton::Left => MB_LEFT,
                    MouseButton::Right => MB_RIGHT,
                    MouseButton::Middle => MB_MIDDLE,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => self.buttons |= bit,
                    ElementState::Released => self.buttons &= !bit,
                }
                self.send_mouse(0);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // В Turbo Vision колесо — это «щелчки»: -1 вверх, +1 вниз.
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -(y.round() as i32),
                    MouseScrollDelta::PixelDelta(p) => {
                        // Тачпад: копим пиксели до высоты строки.
                        let ch = self.gfx.as_ref().map(|g| g.renderer.cell_h() as f64).unwrap_or(16.0);
                        self.wheel_acc += p.y;
                        let n = (self.wheel_acc / ch).trunc();
                        self.wheel_acc -= n * ch;
                        -(n as i32)
                    }
                };
                let dir = steps.signum() as i8;
                for _ in 0..steps.abs().min(10) {
                    self.send_mouse(dir);
                }
            }
            WindowEvent::RedrawRequested => self.paint(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // Курсор виден и мигает — проснуться к следующей фазе мигания.
        let blinking = self.cfg.cursor_blink && self.frame.as_ref().map(|f| f.cursor.is_some()).unwrap_or(false);
        if blinking {
            el.set_control_flow(ControlFlow::WaitUntil(self.next_blink));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        }
    }
}
