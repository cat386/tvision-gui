//! Программная отрисовка псевдографики: рамки (U+2500–U+257F) и блоки
//! (U+2580–U+259F).
//!
//! Почему не шрифтом: в шрифтах линии рамок часто не доходят до края ячейки
//! или вылезают за неё, и тогда рамки окон Turbo Vision получаются
//! «пунктирными». Терминалы (Windows Terminal, kitty, alacritty) по той же
//! причине рисуют эти символы сами. Здесь символ превращается в маску
//! покрытия (0 — фон, 255 — цвет текста) размером ровно в одну ячейку.

/// Стиль «луча» линии от центра ячейки к краю.
const NONE: u8 = 0;
const LIGHT: u8 = 1;
const HEAVY: u8 = 2;
const DOUBLE: u8 = 3;

/// Индексы лучей в описании символа.
const UP: usize = 0;
const RIGHT: usize = 1;
const DOWN: usize = 2;
const LEFT: usize = 3;

/// Символы рамок: код → стили лучей [вверх, вправо, вниз, влево].
/// Таблица сгенерирована по именам символов Unicode
/// (`BOX DRAWINGS DOWN SINGLE AND RIGHT DOUBLE` и т.п.).
/// Пунктирные линии рисуются сплошными, скруглённые углы — прямыми.
/// Диагонали (U+2571–U+2573) берутся из шрифта.
#[rustfmt::skip]
const LINES: &[(u32, [u8; 4])] = &[
    (0x2500, [0, 1, 0, 1]), // light horizontal
    (0x2501, [0, 2, 0, 2]), // heavy horizontal
    (0x2502, [1, 0, 1, 0]), // light vertical
    (0x2503, [2, 0, 2, 0]), // heavy vertical
    (0x2504, [0, 1, 0, 1]), // light triple dash horizontal
    (0x2505, [0, 2, 0, 2]), // heavy triple dash horizontal
    (0x2506, [1, 0, 1, 0]), // light triple dash vertical
    (0x2507, [2, 0, 2, 0]), // heavy triple dash vertical
    (0x2508, [0, 1, 0, 1]), // light quadruple dash horizontal
    (0x2509, [0, 2, 0, 2]), // heavy quadruple dash horizontal
    (0x250A, [1, 0, 1, 0]), // light quadruple dash vertical
    (0x250B, [2, 0, 2, 0]), // heavy quadruple dash vertical
    (0x250C, [0, 1, 1, 0]), // light down and right
    (0x250D, [0, 2, 1, 0]), // down light and right heavy
    (0x250E, [0, 1, 2, 0]), // down heavy and right light
    (0x250F, [0, 2, 2, 0]), // heavy down and right
    (0x2510, [0, 0, 1, 1]), // light down and left
    (0x2511, [0, 0, 1, 2]), // down light and left heavy
    (0x2512, [0, 0, 2, 1]), // down heavy and left light
    (0x2513, [0, 0, 2, 2]), // heavy down and left
    (0x2514, [1, 1, 0, 0]), // light up and right
    (0x2515, [1, 2, 0, 0]), // up light and right heavy
    (0x2516, [2, 1, 0, 0]), // up heavy and right light
    (0x2517, [2, 2, 0, 0]), // heavy up and right
    (0x2518, [1, 0, 0, 1]), // light up and left
    (0x2519, [1, 0, 0, 2]), // up light and left heavy
    (0x251A, [2, 0, 0, 1]), // up heavy and left light
    (0x251B, [2, 0, 0, 2]), // heavy up and left
    (0x251C, [1, 1, 1, 0]), // light vertical and right
    (0x251D, [1, 2, 1, 0]), // vertical light and right heavy
    (0x251E, [2, 1, 1, 0]), // up heavy and right down light
    (0x251F, [1, 1, 2, 0]), // down heavy and right up light
    (0x2520, [2, 1, 2, 0]), // vertical heavy and right light
    (0x2521, [2, 2, 1, 0]), // down light and right up heavy
    (0x2522, [1, 2, 2, 0]), // up light and right down heavy
    (0x2523, [2, 2, 2, 0]), // heavy vertical and right
    (0x2524, [1, 0, 1, 1]), // light vertical and left
    (0x2525, [1, 0, 1, 2]), // vertical light and left heavy
    (0x2526, [2, 0, 1, 1]), // up heavy and left down light
    (0x2527, [1, 0, 2, 1]), // down heavy and left up light
    (0x2528, [2, 0, 2, 1]), // vertical heavy and left light
    (0x2529, [2, 0, 1, 2]), // down light and left up heavy
    (0x252A, [1, 0, 2, 2]), // up light and left down heavy
    (0x252B, [2, 0, 2, 2]), // heavy vertical and left
    (0x252C, [0, 1, 1, 1]), // light down and horizontal
    (0x252D, [0, 1, 1, 2]), // left heavy and right down light
    (0x252E, [0, 2, 1, 1]), // right heavy and left down light
    (0x252F, [0, 2, 1, 2]), // down light and horizontal heavy
    (0x2530, [0, 1, 2, 1]), // down heavy and horizontal light
    (0x2531, [0, 1, 2, 2]), // right light and left down heavy
    (0x2532, [0, 2, 2, 1]), // left light and right down heavy
    (0x2533, [0, 2, 2, 2]), // heavy down and horizontal
    (0x2534, [1, 1, 0, 1]), // light up and horizontal
    (0x2535, [1, 1, 0, 2]), // left heavy and right up light
    (0x2536, [1, 2, 0, 1]), // right heavy and left up light
    (0x2537, [1, 2, 0, 2]), // up light and horizontal heavy
    (0x2538, [2, 1, 0, 1]), // up heavy and horizontal light
    (0x2539, [2, 1, 0, 2]), // right light and left up heavy
    (0x253A, [2, 2, 0, 1]), // left light and right up heavy
    (0x253B, [2, 2, 0, 2]), // heavy up and horizontal
    (0x253C, [1, 1, 1, 1]), // light vertical and horizontal
    (0x253D, [1, 1, 1, 2]), // left heavy and right vertical light
    (0x253E, [1, 2, 1, 1]), // right heavy and left vertical light
    (0x253F, [1, 2, 1, 2]), // vertical light and horizontal heavy
    (0x2540, [2, 1, 1, 1]), // up heavy and down horizontal light
    (0x2541, [1, 1, 2, 1]), // down heavy and up horizontal light
    (0x2542, [2, 1, 2, 1]), // vertical heavy and horizontal light
    (0x2543, [2, 1, 1, 2]), // left up heavy and right down light
    (0x2544, [2, 2, 1, 1]), // right up heavy and left down light
    (0x2545, [1, 1, 2, 2]), // left down heavy and right up light
    (0x2546, [1, 2, 2, 1]), // right down heavy and left up light
    (0x2547, [2, 2, 1, 2]), // down light and up horizontal heavy
    (0x2548, [1, 2, 2, 2]), // up light and down horizontal heavy
    (0x2549, [2, 1, 2, 2]), // right light and left vertical heavy
    (0x254A, [2, 2, 2, 1]), // left light and right vertical heavy
    (0x254B, [2, 2, 2, 2]), // heavy vertical and horizontal
    (0x254C, [0, 1, 0, 1]), // light double dash horizontal
    (0x254D, [0, 2, 0, 2]), // heavy double dash horizontal
    (0x254E, [1, 0, 1, 0]), // light double dash vertical
    (0x254F, [2, 0, 2, 0]), // heavy double dash vertical
    (0x2550, [0, 3, 0, 3]), // double horizontal
    (0x2551, [3, 0, 3, 0]), // double vertical
    (0x2552, [0, 3, 1, 0]), // down single and right double
    (0x2553, [0, 1, 3, 0]), // down double and right single
    (0x2554, [0, 3, 3, 0]), // double down and right
    (0x2555, [0, 0, 1, 3]), // down single and left double
    (0x2556, [0, 0, 3, 1]), // down double and left single
    (0x2557, [0, 0, 3, 3]), // double down and left
    (0x2558, [1, 3, 0, 0]), // up single and right double
    (0x2559, [3, 1, 0, 0]), // up double and right single
    (0x255A, [3, 3, 0, 0]), // double up and right
    (0x255B, [1, 0, 0, 3]), // up single and left double
    (0x255C, [3, 0, 0, 1]), // up double and left single
    (0x255D, [3, 0, 0, 3]), // double up and left
    (0x255E, [1, 3, 1, 0]), // vertical single and right double
    (0x255F, [3, 1, 3, 0]), // vertical double and right single
    (0x2560, [3, 3, 3, 0]), // double vertical and right
    (0x2561, [1, 0, 1, 3]), // vertical single and left double
    (0x2562, [3, 0, 3, 1]), // vertical double and left single
    (0x2563, [3, 0, 3, 3]), // double vertical and left
    (0x2564, [0, 3, 1, 3]), // down single and horizontal double
    (0x2565, [0, 1, 3, 1]), // down double and horizontal single
    (0x2566, [0, 3, 3, 3]), // double down and horizontal
    (0x2567, [1, 3, 0, 3]), // up single and horizontal double
    (0x2568, [3, 1, 0, 1]), // up double and horizontal single
    (0x2569, [3, 3, 0, 3]), // double up and horizontal
    (0x256A, [1, 3, 1, 3]), // vertical single and horizontal double
    (0x256B, [3, 1, 3, 1]), // vertical double and horizontal single
    (0x256C, [3, 3, 3, 3]), // double vertical and horizontal
    (0x256D, [0, 1, 1, 0]), // light arc down and right
    (0x256E, [0, 0, 1, 1]), // light arc down and left
    (0x256F, [1, 0, 0, 1]), // light arc up and left
    (0x2570, [1, 1, 0, 0]), // light arc up and right
    (0x2574, [0, 0, 0, 1]), // light left
    (0x2575, [1, 0, 0, 0]), // light up
    (0x2576, [0, 1, 0, 0]), // light right
    (0x2577, [0, 0, 1, 0]), // light down
    (0x2578, [0, 0, 0, 2]), // heavy left
    (0x2579, [2, 0, 0, 0]), // heavy up
    (0x257A, [0, 2, 0, 0]), // heavy right
    (0x257B, [0, 0, 2, 0]), // heavy down
    (0x257C, [0, 2, 0, 1]), // light left and heavy right
    (0x257D, [1, 0, 2, 0]), // light up and heavy down
    (0x257E, [0, 1, 0, 2]), // heavy left and light right
    (0x257F, [2, 0, 1, 0]), // heavy up and light down
];

/// Стили лучей символа рамки или `None`, если это не символ рамки.
fn arms(c: char) -> Option<[u8; 4]> {
    let code = c as u32;
    LINES.binary_search_by_key(&code, |&(k, _)| k).ok().map(|i| LINES[i].1)
}

/// Маска покрытия размером `w`×`h` пикселей.
struct Mask<'a> {
    w: i32,
    h: i32,
    data: &'a mut [u8],
}

impl Mask<'_> {
    /// Закрашивает прямоугольник [x0, x1) × [y0, y1) (с обрезкой по краям).
    fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (x0, x1) = (x0.max(0), x1.min(self.w));
        let (y0, y1) = (y0.max(0), y1.min(self.h));
        for y in y0..y1 {
            let row = (y * self.w) as usize;
            for x in x0..x1 {
                self.data[row + x as usize] = 255;
            }
        }
    }
}

/// Отрезок [lo, hi) на одной оси.
#[derive(Clone, Copy)]
struct Span {
    lo: i32,
    hi: i32,
}

impl Span {
    /// Линия толщины `th` с центром в `c`.
    fn centered(c: i32, th: i32) -> Span {
        let lo = c - th / 2;
        Span { lo, hi: lo + th }
    }
}

/// Геометрия линий одной оси: где лежат центральная линия и обе линии
/// двойной рамки.
struct Axis {
    /// Центральная линия (для тонких и толстых лучей).
    center: Span,
    /// Первая (меньшая координата) линия двойной рамки.
    first: Span,
    /// Вторая линия двойной рамки.
    second: Span,
}

/// Куда доходит луч, перпендикулярный этой оси.
#[derive(Clone, Copy)]
enum Reach {
    /// До ближней линии двойной рамки (оставляет «канал» между линиями).
    Inner,
    /// До центральной линии.
    Center,
    /// До дальней линии двойной рамки (внешний угол).
    Outer,
}

/// Рисует символ `c` в маску `out` размером `w`×`h`.
/// Возвращает `false`, если символ не относится к псевдографике.
pub fn draw(c: char, w: usize, h: usize, out: &mut [u8]) -> bool {
    debug_assert!(out.len() >= w * h);
    let mut m = Mask { w: w as i32, h: h as i32, data: out };
    if let Some(a) = arms(c) {
        draw_lines(&mut m, a);
        true
    } else if (0x2580..=0x259F).contains(&(c as u32)) {
        draw_block(&mut m, c as u32);
        true
    } else {
        false
    }
}

/// Рисует линии рамки по стилям четырёх лучей.
fn draw_lines(m: &mut Mask, a: [u8; 4]) {
    let (w, h) = (m.w, m.h);
    // Толщина тонкой линии: ~1/8 ширины символа, но не меньше пикселя.
    let t = ((w as f32) / 8.0).round().max(1.0) as i32;
    // Расстояние от центра до каждой из линий двойной рамки.
    let d = t;
    let thick = |s: u8| if s == HEAVY { t * 2 } else { t };

    let (cx, cy) = (w / 2, h / 2);
    // Толщина центральной линии по оси X — самый толстый вертикальный луч
    // (чтобы горизонтальный луч закрывал угол целиком).
    let vth = [a[UP], a[DOWN]].iter().filter(|&&s| s == LIGHT || s == HEAVY).map(|&s| thick(s)).max().unwrap_or(t);
    let hth = [a[LEFT], a[RIGHT]].iter().filter(|&&s| s == LIGHT || s == HEAVY).map(|&s| thick(s)).max().unwrap_or(t);
    // Вертикальные линии лежат на оси X, горизонтальные — на оси Y.
    let ax = Axis { center: Span::centered(cx, vth), first: Span::centered(cx - d, t), second: Span::centered(cx + d, t) };
    let ay = Axis { center: Span::centered(cy, hth), first: Span::centered(cy - d, t), second: Span::centered(cy + d, t) };

    // --- Горизонтальные лучи (вправо и влево) ---
    for (dir, opp) in [(RIGHT, LEFT), (LEFT, RIGHT)] {
        let s = a[dir];
        if s == NONE {
            continue;
        }
        let to_right = dir == RIGHT;
        if s == DOUBLE {
            // Две линии: верхняя (её «своя» сторона — вверх) и нижняя.
            for (line, near, far) in [(ay.first, a[UP], a[DOWN]), (ay.second, a[DOWN], a[UP])] {
                let reach = double_reach(near, far);
                let (x0, x1) = extent(&ax, reach, to_right, w);
                m.fill(x0, line.lo, x1, line.hi);
            }
        } else {
            let reach = single_reach(a[opp], a[UP], a[DOWN]);
            let (x0, x1) = extent(&ax, reach, to_right, w);
            let line = Span::centered(cy, thick(s));
            m.fill(x0, line.lo, x1, line.hi);
        }
    }

    // --- Вертикальные лучи (вниз и вверх) ---
    for (dir, opp) in [(DOWN, UP), (UP, DOWN)] {
        let s = a[dir];
        if s == NONE {
            continue;
        }
        let to_end = dir == DOWN;
        if s == DOUBLE {
            for (line, near, far) in [(ax.first, a[LEFT], a[RIGHT]), (ax.second, a[RIGHT], a[LEFT])] {
                let reach = double_reach(near, far);
                let (y0, y1) = extent(&ay, reach, to_end, h);
                m.fill(line.lo, y0, line.hi, y1);
            }
        } else {
            let reach = single_reach(a[opp], a[LEFT], a[RIGHT]);
            let (y0, y1) = extent(&ay, reach, to_end, h);
            let line = Span::centered(cx, thick(s));
            m.fill(line.lo, y0, line.hi, y1);
        }
    }
}

/// Докуда доходит одна из линий двойного луча.
/// `near` — луч, перпендикулярный этому, со стороны этой линии,
/// `far` — перпендикулярный луч с противоположной стороны.
fn double_reach(near: u8, far: u8) -> Reach {
    match (near, far) {
        // Со своей стороны тоже двойная линия — внутренний угол «╔», «╬».
        (DOUBLE, _) => Reach::Inner,
        // Со своей стороны одинарная линия — сходимся в центре.
        (LIGHT | HEAVY, _) => Reach::Center,
        // Своей стороны нет, а с другой — двойная: внешний угол «╔».
        (_, DOUBLE) => Reach::Outer,
        // Иначе — до центра (прямая линия, «╘», полулиния).
        _ => Reach::Center,
    }
}

/// Докуда доходит одинарный (тонкий или толстый) луч.
/// `opp` — луч с противоположной стороны, `p1`, `p2` — перпендикулярные.
fn single_reach(opp: u8, p1: u8, p2: u8) -> Reach {
    if opp != NONE {
        // Линия продолжается насквозь: «╪», «╫».
        Reach::Center
    } else if p1 == DOUBLE || p2 == DOUBLE {
        if p1 != NONE && p2 != NONE {
            // Т-образное соединение с двойной линией: «╤», «╟» — до ближней линии.
            Reach::Inner
        } else {
            // Угол с двойной линией: «╕», «╙» — до дальней линии.
            Reach::Outer
        }
    } else {
        Reach::Center
    }
}

/// Отрезок луча вдоль оси: от точки соединения до края ячейки.
/// `axis` — геометрия перпендикулярных линий, `to_end` — луч идёт к
/// большей координате (вправо/вниз), `size` — размер ячейки по этой оси.
fn extent(axis: &Axis, reach: Reach, to_end: bool, size: i32) -> (i32, i32) {
    if to_end {
        // Луч вправо/вниз начинается у «левого» края линии соединения.
        let start = match reach {
            Reach::Inner => axis.second.lo,
            Reach::Center => axis.center.lo,
            Reach::Outer => axis.first.lo,
        };
        (start, size)
    } else {
        // Луч влево/вверх заканчивается у «правого» края линии соединения.
        let end = match reach {
            Reach::Inner => axis.first.hi,
            Reach::Center => axis.center.hi,
            Reach::Outer => axis.second.hi,
        };
        (0, end)
    }
}

/// Блоки, полублоки, четверти и тени (U+2580–U+259F).
fn draw_block(m: &mut Mask, code: u32) {
    let (w, h) = (m.w, m.h);
    // Доля размера: n восьмых.
    let fx = |n: i32| (w * n + 4) / 8;
    let fy = |n: i32| (h * n + 4) / 8;
    let (hx, hy) = (w / 2, h / 2);
    match code {
        0x2580 => m.fill(0, 0, w, hy),                                  // ▀ верхняя половина
        0x2581..=0x2587 => m.fill(0, h - fy((code - 0x2580) as i32), w, h), // ▁..▇ нижние n/8
        0x2588 => m.fill(0, 0, w, h),                                   // █ весь блок
        0x2589..=0x258F => m.fill(0, 0, fx((0x2590 - code) as i32), h), // ▉..▏ левые n/8
        0x2590 => m.fill(hx, 0, w, h),                                  // ▐ правая половина
        0x2591..=0x2593 => shade(m, code - 0x2590),                     // ░ ▒ ▓
        0x2594 => m.fill(0, 0, w, fy(1)),                               // ▔ верхняя 1/8
        0x2595 => m.fill(w - fx(1), 0, w, h),                           // ▕ правая 1/8
        _ => {
            // Четверти ▖▗▘▙▚▛▜▝▞▟: биты — верх-лево, верх-право, низ-лево, низ-право.
            const Q: [u8; 10] = [0b0010, 0b0001, 0b1000, 0b1011, 0b1001, 0b1110, 0b1101, 0b0100, 0b0110, 0b0111];
            let q = Q[(code - 0x2596) as usize];
            if q & 0b1000 != 0 {
                m.fill(0, 0, hx, hy);
            }
            if q & 0b0100 != 0 {
                m.fill(hx, 0, w, hy);
            }
            if q & 0b0010 != 0 {
                m.fill(0, hy, hx, h);
            }
            if q & 0b0001 != 0 {
                m.fill(hx, hy, w, h);
            }
        }
    }
}

/// Тени ░ ▒ ▓ — точечный узор, как в шрифте текстового режима VGA
/// (`level` 1 — светлая, 2 — средняя, 3 — тёмная). «Точка» узора
/// масштабируется вместе со шрифтом, чтобы на HiDPI узор не превращался
/// в серую заливку.
fn shade(m: &mut Mask, level: u32) {
    let u = (m.w / 8).max(1);
    for y in 0..m.h {
        for x in 0..m.w {
            let (gx, gy) = (x / u, y / u);
            // Светлая тень — как в шрифте VGA (байты 0x22, 0x88 через строку):
            // каждая четвёртая точка, в соседних строках со сдвигом на две (25%).
            let light = (gx + 2 * gy) % 4 == 2;
            let on = match level {
                1 => light,
                2 => (gx + gy) % 2 == 0,
                _ => !light,
            };
            if on {
                m.data[(y * m.w + x) as usize] = 255;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Маска символа в виде строк «#» и «.» (для наглядных проверок).
    fn picture(c: char, w: usize, h: usize) -> Vec<String> {
        let mut v = vec![0u8; w * h];
        assert!(draw(c, w, h, &mut v));
        v.chunks(w).map(|r| r.iter().map(|&p| if p > 0 { '#' } else { '.' }).collect()).collect()
    }

    #[test]
    fn table_sorted_and_complete() {
        assert!(LINES.windows(2).all(|p| p[0].0 < p[1].0));
        assert_eq!(LINES.len(), 125);
        assert_eq!(arms('╔'), Some([NONE, DOUBLE, DOUBLE, NONE]));
        assert_eq!(arms('┤'), Some([LIGHT, NONE, LIGHT, LIGHT]));
        assert_eq!(arms('╧'), Some([LIGHT, DOUBLE, NONE, DOUBLE]));
        assert!(arms('a').is_none());
    }

    #[test]
    fn horizontal_line_spans_whole_cell() {
        // Горизонтальная линия должна доходить до обоих краёв ячейки —
        // иначе рамки окон будут прерывистыми.
        for (w, h) in [(8, 16), (9, 19), (13, 27)] {
            let p = picture('─', w, h);
            assert!(p.iter().any(|r| r.chars().all(|c| c == '#')), "{}x{}", w, h);
            let p = picture('│', w, h);
            let col: Vec<bool> = (0..w).map(|x| p.iter().all(|r| r.as_bytes()[x] == b'#')).collect();
            assert!(col.iter().any(|&c| c));
        }
    }

    #[test]
    fn double_corner_shape() {
        // ╔ в ячейке 8×16: внешний угол и внутренний угол.
        let p = picture('╔', 8, 16);
        // Внешняя верхняя линия идёт от внешней вертикали до правого края.
        let outer_row = &p[7];
        assert!(outer_row.ends_with('#'));
        assert!(outer_row.starts_with("..."));
        // Нижний край: две вертикальные линии.
        assert_eq!(p[15].matches('#').count(), 2);
        // Правый край: две горизонтальные линии.
        assert_eq!(p.iter().filter(|r| r.ends_with('#')).count(), 2);
    }

    #[test]
    fn junctions_connect_neighbours() {
        // Символы, стоящие рядом, должны стыковаться: правый край «═» и левый край «╗»
        // закрашены в одних и тех же строках.
        let (w, h) = (9, 19);
        let a = picture('═', w, h);
        let b = picture('╗', w, h);
        for y in 0..h {
            assert_eq!(a[y].ends_with('#'), b[y].starts_with('#'), "строка {}", y);
        }
        let a = picture('║', w, h);
        let b = picture('╝', w, h);
        assert_eq!(a[h - 1], b[0]);
    }

    #[test]
    fn blocks() {
        let p = picture('▀', 8, 16);
        assert!(p[0].chars().all(|c| c == '#'));
        assert!(p[15].chars().all(|c| c == '.'));
        let p = picture('█', 8, 16);
        assert!(p.iter().all(|r| !r.contains('.')));
        // Средняя тень — ровно половина пикселей.
        let mut v = vec![0u8; 64];
        draw('▒', 8, 8, &mut v);
        assert_eq!(v.iter().filter(|&&x| x > 0).count(), 32);
    }
}
