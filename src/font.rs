//! Шрифты: поиск в системе, размер ячейки, растеризация символов.
//!
//! Основной шрифт — моноширинный (он задаёт размер ячейки). Если в нём нет
//! нужного символа (иероглифы, редкие значки), символ ищется в запасных
//! шрифтах. Запасные шрифты загружаются **лениво**, при первом обращении:
//! большие файлы с иероглифами (десятки мегабайт) разбираются долго, и
//! программе, которой они не нужны, незачем ждать.

use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};

/// Запасной шрифт: ещё не загружен, загружен или не загрузился.
enum Slot {
    Pending(PathBuf, u32),
    Loaded(Font),
    Failed,
}

/// Набор шрифтов заданного размера и метрики ячейки.
pub struct FontSet {
    /// Основной моноширинный шрифт (может отсутствовать — тогда символы,
    /// кроме псевдографики, рисуются «квадратиками»).
    primary: Option<Font>,
    /// Запасные шрифты в порядке приоритета.
    fallbacks: Vec<Slot>,
    /// Размер шрифта в физических пикселях.
    pub px: f32,
    /// Ширина ячейки в пикселях.
    pub cell_w: usize,
    /// Высота ячейки в пикселях.
    pub cell_h: usize,
    /// Положение базовой линии от верха ячейки.
    pub baseline: i32,
}

/// Загружает шрифт из файла (`index` — номер шрифта в коллекции .ttc).
fn load_font(path: &Path, index: u32, px: f32) -> Option<Font> {
    let data = std::fs::read(path).ok()?;
    let settings = FontSettings { collection_index: index, scale: px, ..FontSettings::default() };
    Font::from_bytes(data, settings).ok()
}

impl FontSet {
    /// Находит шрифты и вычисляет размер ячейки для шрифта `px` пикселей.
    /// `user` — шрифт, заданный программой или пользователем (важнее системных).
    pub fn load(user: Option<&Path>, px: f32) -> FontSet {
        let px = px.max(4.0);
        let (primary_list, fallback_list) = candidates();

        let mut primary = None;
        let mut primary_path = None;
        if let Some(p) = user {
            match load_font(p, 0, px) {
                Some(f) => {
                    primary = Some(f);
                    primary_path = Some(p.to_path_buf());
                }
                None => eprintln!("tvision-gui: не удалось загрузить шрифт {}", p.display()),
            }
        }
        if primary.is_none() {
            for (p, i) in &primary_list {
                if let Some(f) = p.is_file().then(|| load_font(p, *i, px)).flatten() {
                    primary = Some(f);
                    primary_path = Some(p.clone());
                    break;
                }
            }
        }
        if primary.is_none() {
            eprintln!("tvision-gui: моноширинный шрифт не найден; задайте его через TVISION_GUI_FONT");
        }

        // В запасные идут и остальные моноширинные шрифты из списка.
        let fallbacks = primary_list
            .into_iter()
            .chain(fallback_list)
            .filter(|(p, _)| Some(p) != primary_path.as_ref() && p.is_file())
            .map(|(p, i)| Slot::Pending(p, i))
            .collect();

        // Размер ячейки: ширина — по букве «M», высота — по метрикам строки.
        let (cell_w, cell_h, baseline) = match &primary {
            Some(f) => {
                let adv = f.metrics('M', px).advance_width;
                let (ascent, descent, gap) = match f.horizontal_line_metrics(px) {
                    Some(lm) => (lm.ascent, lm.descent, lm.line_gap),
                    None => (px * 0.8, -px * 0.2, 0.0),
                };
                let cell_w = adv.round().max(1.0) as usize;
                let text_h = ascent - descent;
                let cell_h = (text_h + gap.max(0.0)).ceil().max(1.0) as usize;
                // Строка текста — по центру ячейки, если межстрочный интервал > 0.
                let baseline = ((cell_h as f32 - text_h) / 2.0 + ascent).round() as i32;
                (cell_w, cell_h, baseline)
            }
            None => ((px * 0.6).ceil() as usize, (px * 1.2).ceil() as usize, px.round() as i32),
        };

        FontSet { primary, fallbacks, px, cell_w, cell_h, baseline }
    }

    /// Рисует символ `c` в маску покрытия `out` размером `w`×`h`
    /// (`w` — одна или две ячейки для широких символов).
    /// Возвращает `false`, если символа нет ни в одном шрифте.
    pub fn rasterize(&mut self, c: char, w: usize, h: usize, out: &mut [u8]) -> bool {
        let px = self.px;
        let baseline = self.baseline;
        let font = match self.find(c) {
            Some(f) => f,
            None => return false,
        };
        // Символ из запасного шрифта может оказаться шире ячейки —
        // тогда уменьшаем его, чтобы он не налезал на соседей.
        let adv = font.metrics(c, px).advance_width;
        let size = if adv > w as f32 * 1.05 { px * w as f32 / adv } else { px };
        let (m, bitmap) = font.rasterize(c, size);
        let adv = m.advance_width;
        // По горизонтали — по центру ячейки (для моноширинного шрифта сдвиг ≈ 0).
        let x0 = ((w as f32 - adv) / 2.0).round() as i32 + m.xmin;
        // `ymin` — расстояние от базовой линии до нижнего края символа (вверх положительно).
        let y0 = baseline - m.ymin - m.height as i32;
        for gy in 0..m.height {
            let y = y0 + gy as i32;
            if y < 0 || y >= h as i32 {
                continue;
            }
            for gx in 0..m.width {
                let x = x0 + gx as i32;
                if x < 0 || x >= w as i32 {
                    continue;
                }
                let a = bitmap[gy * m.width + gx];
                let p = &mut out[y as usize * w + x as usize];
                *p = (*p).max(a);
            }
        }
        true
    }

    /// Шрифт, в котором есть символ `c`.
    fn find(&mut self, c: char) -> Option<&Font> {
        if let Some(f) = &self.primary {
            if f.lookup_glyph_index(c) != 0 {
                // Повторный заём `self.primary` — так проще для проверки заимствований.
                return self.primary.as_ref();
            }
        }
        let px = self.px;
        let mut found = None;
        for (i, slot) in self.fallbacks.iter_mut().enumerate() {
            if let Slot::Pending(p, idx) = slot {
                *slot = match load_font(p, *idx, px) {
                    Some(f) => Slot::Loaded(f),
                    None => Slot::Failed,
                };
            }
            if let Slot::Loaded(f) = slot {
                if f.lookup_glyph_index(c) != 0 {
                    found = Some(i);
                    break;
                }
            }
        }
        match found.map(|i| &self.fallbacks[i]) {
            Some(Slot::Loaded(f)) => Some(f),
            _ => None,
        }
    }
}

/// Списки файлов шрифтов: (основные моноширинные, запасные).
/// Каждый элемент — путь и номер шрифта внутри коллекции .ttc.
fn candidates() -> (Vec<(PathBuf, u32)>, Vec<(PathBuf, u32)>) {
    let mut mono: Vec<(PathBuf, u32)> = Vec::new();
    let mut extra: Vec<(PathBuf, u32)> = Vec::new();

    #[cfg(windows)]
    {
        let root = std::env::var("WINDIR").or_else(|_| std::env::var("SystemRoot")).unwrap_or_else(|_| "C:\\Windows".into());
        let sys = PathBuf::from(root).join("Fonts");
        // Шрифты, установленные «только для текущего пользователя».
        let user = std::env::var("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Microsoft\\Windows\\Fonts")).ok();
        let dirs: Vec<PathBuf> = std::iter::once(sys).chain(user).collect();
        let add = |list: &mut Vec<(PathBuf, u32)>, names: &[&str]| {
            for n in names {
                for d in &dirs {
                    list.push((d.join(n), 0));
                }
            }
        };
        add(&mut mono, &["consola.ttf", "CascadiaMono.ttf", "CascadiaCode.ttf", "lucon.ttf", "cour.ttf"]);
        add(&mut extra, &["seguisym.ttf", "segoeui.ttf", "arial.ttf", "msgothic.ttc", "YuGothR.ttc", "malgun.ttf", "msyh.ttc", "simsun.ttc", "seguiemj.ttf"]);
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        for p in [
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Monaco.ttf",
            "/System/Library/Fonts/Supplemental/Courier New.ttf",
            "/Library/Fonts/Courier New.ttf",
        ] {
            mono.push((PathBuf::from(p), 0));
        }
        mono.push((PathBuf::from(&home).join("Library/Fonts/DejaVuSansMono.ttf"), 0));
        for p in [
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Apple Symbols.ttf",
            "/System/Library/Fonts/Geneva.ttf",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        ] {
            extra.push((PathBuf::from(p), 0));
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Сначала спрашиваем fontconfig — он знает настройки пользователя.
        if let Some(p) = fc_match("monospace") {
            mono.push((p, 0));
        }
        for dir in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            for p in [
                "truetype/dejavu/DejaVuSansMono.ttf",
                "TTF/DejaVuSansMono.ttf",
                "dejavu/DejaVuSansMono.ttf",
                "dejavu-sans-mono-fonts/DejaVuSansMono.ttf",
                "truetype/liberation/LiberationMono-Regular.ttf",
                "liberation-mono/LiberationMono-Regular.ttf",
                "TTF/LiberationMono-Regular.ttf",
                "truetype/noto/NotoSansMono-Regular.ttf",
                "noto/NotoSansMono-Regular.ttf",
                "truetype/ubuntu/UbuntuMono-R.ttf",
                "truetype/freefont/FreeMono.ttf",
            ] {
                mono.push((Path::new(dir).join(p), 0));
            }
        }
        if let Some(p) = fc_match("sans") {
            extra.push((p, 0));
        }
        for dir in ["/usr/share/fonts", "/usr/local/share/fonts"] {
            for p in [
                "truetype/dejavu/DejaVuSans.ttf",
                "TTF/DejaVuSans.ttf",
                "dejavu/DejaVuSans.ttf",
                "opentype/noto/NotoSansCJK-Regular.ttc",
                "noto-cjk/NotoSansCJK-Regular.ttc",
                "google-noto-cjk/NotoSansCJK-Regular.ttc",
                "truetype/unifont/unifont.ttf",
                "truetype/freefont/FreeSerif.ttf",
            ] {
                extra.push((Path::new(dir).join(p), 0));
            }
        }
    }

    // Убираем повторы, сохраняя порядок.
    let mut seen = std::collections::HashSet::new();
    mono.retain(|(p, _)| seen.insert(p.clone()));
    extra.retain(|(p, _)| seen.insert(p.clone()));
    (mono, extra)
}

/// Путь к шрифту по шаблону fontconfig (`fc-match -f %{file} monospace`).
#[cfg(all(unix, not(target_os = "macos")))]
fn fc_match(pattern: &str) -> Option<PathBuf> {
    let out = std::process::Command::new("fc-match").args(["-f", "%{file}", pattern]).output().ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let p = PathBuf::from(s.trim());
    // fontdue не читает растровые шрифты (.pcf и т.п.).
    let ok = p.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "ttc")).unwrap_or(false);
    (ok && p.is_file()).then_some(p)
}
