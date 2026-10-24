//! Перевод экранного буфера Turbo Vision (ячейки) в пиксели окна.
//!
//! [`Renderer`] держит собственный кадр в памяти (`Vec<u32>`, формат
//! `0x00RRGGBB`, как у softbuffer) и при каждом новом экранном буфере
//! перерисовывает **только изменившиеся ячейки** — так же, как терминальные
//! бэкенды tvision-rs выводят только изменения. Готовый кадр копируется в
//! окно целиком (это просто `memcpy`).
//!
//! Маски символов (покрытие 0..255) кэшируются: каждый символ растеризуется
//! один раз, а потом лишь «раскрашивается» цветами текста и фона.

use std::collections::HashMap;

use tvision::{Cell, Point, ScreenBuffer};

use crate::boxdraw;
use crate::font::FontSet;

/// Кадр от программы: экран, курсор и режим вставки.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub screen: ScreenBuffer,
    pub cursor: Option<Point>,
    pub insert: bool,
}

/// Отрисовщик сетки символов.
pub struct Renderer {
    pub font: FontSet,
    palette: [u32; 16],
    /// Размер кадра в пикселях.
    pub width: usize,
    pub height: usize,
    pixels: Vec<u32>,
    /// Что сейчас нарисовано в `pixels` (для поиска изменений).
    shown: Option<ScreenBuffer>,
    /// Ячейка, в которой сейчас нарисован курсор.
    cursor_drawn: Option<(usize, usize)>,
    /// Кэш масок: (символ, ширина в ячейках) → покрытие.
    cache: HashMap<(char, u8), Vec<u8>>,
}

impl Renderer {
    pub fn new(font: FontSet, palette: [u32; 16]) -> Self {
        Renderer { font, palette, width: 0, height: 0, pixels: Vec::new(), shown: None, cursor_drawn: None, cache: HashMap::new() }
    }

    pub fn cell_w(&self) -> usize {
        self.font.cell_w
    }

    pub fn cell_h(&self) -> usize {
        self.font.cell_h
    }

    /// Сколько ячеек помещается в окно заданного размера.
    pub fn grid_for(&self, width: usize, height: usize) -> Point {
        Point::new((width / self.cell_w()).max(1) as i32, (height / self.cell_h()).max(1) as i32)
    }

    /// Новый размер окна: кадр очищается и при следующем обновлении
    /// перерисовывается полностью.
    pub fn resize(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        self.pixels = vec![self.palette[0]; width * height];
        self.invalidate();
    }

    /// Новый шрифт (смена размера или масштаба монитора).
    pub fn set_font(&mut self, font: FontSet) {
        self.font = font;
        self.cache.clear();
        self.pixels.fill(self.palette[0]);
        self.invalidate();
    }

    /// Забыть, что нарисовано: следующий кадр рисуется целиком.
    pub fn invalidate(&mut self) {
        self.shown = None;
        self.cursor_drawn = None;
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Рисует изменения кадра. `cursor_on` — фаза мигания курсора.
    pub fn update(&mut self, frame: &Frame, cursor_on: bool) {
        let scr = &frame.screen;
        let (cw, ch) = (self.cell_w(), self.cell_h());
        if cw == 0 || ch == 0 || self.width == 0 || self.height == 0 {
            return;
        }
        // Размер экрана программы поменялся — рисуем всё заново,
        // а поля окна за пределами сетки заливаем чёрным.
        if self.shown.as_ref().map(|s| (s.width, s.height)) != Some((scr.width, scr.height)) {
            self.pixels.fill(self.palette[0]);
            self.shown = None;
            self.cursor_drawn = None;
        }
        // Где был нарисован курсор — эту ячейку перерисуем без него.
        let old_cursor = self.cursor_drawn.take();

        let cols = scr.width.min(self.width / cw);
        let rows = scr.height.min(self.height / ch);
        for y in 0..rows {
            let mut x = 0;
            while x < cols {
                let cell = scr.get(x, y);
                // Широкий символ занимает эту и следующую ячейку («хвост» — '\0').
                let wide = x + 1 < scr.width && scr.get(x + 1, y).is_wide_tail();
                let span = if wide { 2 } else { 1 };
                let changed = match &self.shown {
                    None => true,
                    Some(s) => (0..span).any(|i| s.get(x + i, y) != scr.get(x + i, y)),
                };
                let had_cursor = old_cursor.map(|(cx, cy)| cy == y && cx >= x && cx < x + span).unwrap_or(false);
                if changed || had_cursor {
                    if cell.is_wide_tail() {
                        // «Хвост» без головы (не должен встречаться) — просто фон.
                        self.draw_cell(x, y, Cell::new(' ', cell.attr), 1);
                    } else {
                        self.draw_cell(x, y, cell, span.min(cols - x));
                    }
                }
                x += span;
            }
        }
        self.shown = Some(scr.clone());

        // Курсор: подчёркивание, а в режиме вставки — блок (как в tvision-rs:
        // `insert` = «режим вставки, блочный курсор»).
        if let (Some(p), true) = (frame.cursor, cursor_on) {
            if p.x >= 0 && p.y >= 0 && (p.x as usize) < cols && (p.y as usize) < rows {
                let (x, y) = (p.x as usize, p.y as usize);
                let h = if frame.insert { ch } else { (ch / 6).max(2).min(ch) };
                self.invert(x * cw, y * ch + ch - h, cw, h);
                self.cursor_drawn = Some((x, y));
            }
        }
    }

    /// Инвертирует цвета прямоугольника (курсор).
    fn invert(&mut self, x0: usize, y0: usize, w: usize, h: usize) {
        for y in y0..(y0 + h).min(self.height) {
            let row = y * self.width;
            for x in x0..(x0 + w).min(self.width) {
                self.pixels[row + x] ^= 0x00FF_FFFF;
            }
        }
    }

    /// Рисует ячейку (или две — для широкого символа).
    fn draw_cell(&mut self, x: usize, y: usize, cell: Cell, span: usize) {
        let (cw, ch) = (self.cell_w(), self.cell_h());
        // Цвета BIOS и номера xterm 0–15 берутся из палитры окна (её можно
        // задать в `GuiConfig::palette`), остальные 240 цветов xterm-256 —
        // стандартные (куб 6×6×6 и оттенки серого).
        let fg = tvision::color::fg_rgb(cell.attr, &self.palette);
        let bg = tvision::color::bg_rgb(cell.attr, &self.palette);
        let w = cw * span;
        let (px0, py0) = (x * cw, y * ch);
        if px0 >= self.width || py0 >= self.height {
            return;
        }
        let vis_w = w.min(self.width - px0);
        let vis_h = ch.min(self.height - py0);

        // Пробел и «пустые» символы — только фон, маска не нужна.
        if cell.ch == ' ' || cell.ch == '\u{a0}' || fg == bg {
            for row in 0..vis_h {
                let start = (py0 + row) * self.width + px0;
                self.pixels[start..start + vis_w].fill(bg);
            }
            return;
        }

        let key = self.ensure_mask(display_char(cell.ch), span);
        // Маска берётся из кэша, а пишем в кадр — это разные поля структуры,
        // поэтому заимствования не конфликтуют.
        let mask = &self.cache[&key];
        let stride = self.width;
        for row in 0..vis_h {
            let start = (py0 + row) * stride + px0;
            let mrow = &mask[row * w..row * w + vis_w];
            for (dst, &a) in self.pixels[start..start + vis_w].iter_mut().zip(mrow) {
                *dst = blend(bg, fg, a);
            }
        }
    }

    /// Готовит маску символа в кэше (растеризует при первом обращении)
    /// и возвращает ключ кэша.
    fn ensure_mask(&mut self, c: char, span: usize) -> (char, u8) {
        let key = (c, span as u8);
        if !self.cache.contains_key(&key) {
            let (w, h) = (self.cell_w() * span, self.cell_h());
            let mut m = vec![0u8; w * h];
            let done = (span == 1 && boxdraw::draw(c, w, h, &mut m)) || self.font.rasterize(c, w, h, &mut m);
            if !done && !c.is_whitespace() && !c.is_control() {
                tofu(w, h, &mut m);
            }
            self.cache.insert(key, m);
        }
        key
    }
}

/// Управляющие символы 0x01–0x1F и 0x7F показываются картинками кодовой
/// страницы 437 (☺ ♥ ► ◄ ↑ ↓ → ← ▲ ▼ ⌂ ...), как на экране DOS и как в
/// magiblot/tvision: старые программы и файлы справки используют их как
/// значки (например, стрелки в tvdemo).
fn display_char(c: char) -> char {
    const CP437_LOW: [char; 32] = [
        ' ', '☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼', //
        '►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
    ];
    match c as u32 {
        n @ 0x01..=0x1F => CP437_LOW[n as usize],
        0x7F => '⌂',
        _ => c,
    }
}

/// «Квадратик» для символа, которого нет ни в одном шрифте.
fn tofu(w: usize, h: usize, m: &mut [u8]) {
    if w < 4 || h < 6 {
        return;
    }
    let (x0, x1) = (1, w - 2);
    let (y0, y1) = (h / 5, h - h / 5);
    for y in y0..=y1 {
        for x in x0..=x1 {
            if y == y0 || y == y1 || x == x0 || x == x1 {
                m[y * w + x] = 160;
            }
        }
    }
}

/// Смешивает цвета фона и текста по покрытию `a` (0 — фон, 255 — текст).
#[inline]
fn blend(bg: u32, fg: u32, a: u8) -> u32 {
    match a {
        0 => bg,
        255 => fg,
        _ => {
            let a = a as u32;
            let mix = |shift: u32| {
                let b = (bg >> shift) & 0xFF;
                let f = (fg >> shift) & 0xFF;
                ((f * a + b * (255 - a) + 127) / 255) << shift
            };
            mix(16) | mix(8) | mix(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_chars_as_cp437() {
        assert_eq!(display_char('\x1b'), '←');
        assert_eq!(display_char('\x1e'), '▲');
        assert_eq!(display_char('a'), 'a');
    }

    #[test]
    fn blend_edges() {
        assert_eq!(blend(0x000000, 0xFFFFFF, 0), 0x000000);
        assert_eq!(blend(0x000000, 0xFFFFFF, 255), 0xFFFFFF);
        assert_eq!(blend(0x000000, 0xFFFFFF, 128), 0x808080);
    }

    /// Отрисовщик без системных шрифтов (псевдографика работает и так).
    fn renderer() -> Renderer {
        let font = FontSet::load(Some(std::path::Path::new("/нет/такого/шрифта.ttf")), 16.0);
        let mut r = Renderer::new(font, crate::VGA_PALETTE);
        let (cw, ch) = (r.cell_w(), r.cell_h());
        r.resize(cw * 10, ch * 3);
        r
    }

    #[test]
    fn draws_256_colors() {
        use tvision::color::{attr256, with_bg256, xterm_rgb};
        let mut r = renderer();
        let mut scr = ScreenBuffer::new(10, 3);
        scr.clear(with_bg256(0x07, 236)); // фон — серый xterm 236
        *scr.get_mut(0, 0) = Cell::new('█', attr256(208, 17)); // оранжевый блок
        *scr.get_mut(2, 0) = Cell::new(' ', attr256(15, 4)); // xterm 4 — синий из палитры окна
        let f = Frame { screen: scr, cursor: None, insert: false };
        r.update(&f, true);
        let cw = r.cell_w();
        assert_eq!(r.pixels()[cw + 1], xterm_rgb(236));
        assert_eq!(r.pixels()[1], xterm_rgb(208));
        assert_eq!(r.pixels()[2 * cw + 1], crate::VGA_PALETTE[1]);
    }

    #[test]
    fn draws_background_and_blocks() {
        let mut r = renderer();
        let mut scr = ScreenBuffer::new(10, 3);
        scr.clear(0x17); // светло-серый на синем
        *scr.get_mut(0, 0) = Cell::new('█', 0x1E); // жёлтый блок
        let f = Frame { screen: scr, cursor: None, insert: false };
        r.update(&f, true);
        let cw = r.cell_w();
        // Ячейка (1,0) — синий фон.
        assert_eq!(r.pixels()[cw + 1], crate::VGA_PALETTE[1]);
        // Ячейка (0,0) — сплошной жёлтый.
        assert_eq!(r.pixels()[1], crate::VGA_PALETTE[14]);
    }

    #[test]
    fn cursor_is_restored() {
        let mut r = renderer();
        let mut scr = ScreenBuffer::new(10, 3);
        scr.clear(0x07);
        let mut f = Frame { screen: scr, cursor: Some(Point::new(2, 1)), insert: true };
        r.update(&f, true);
        let (cw, ch) = (r.cell_w(), r.cell_h());
        let at = |r: &Renderer| r.pixels()[(ch + 1) * r.width + 2 * cw + 1];
        assert_eq!(at(&r), 0xFFFFFF); // чёрный фон инвертирован
                                      // Фаза «погашен»: ячейка рисуется заново без курсора.
        r.update(&f, false);
        assert_eq!(at(&r), 0x000000);
        // Курсор переехал — старая ячейка чистая.
        r.update(&f, true);
        f.cursor = Some(Point::new(5, 1));
        r.update(&f, true);
        assert_eq!(at(&r), 0x000000);
    }
}
