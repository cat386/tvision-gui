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

// ----------------------------------------------------------------------
// Список моноширинных шрифтов системы (для диалога настроек)
// ----------------------------------------------------------------------

/// Моноширинный шрифт, найденный в системе.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontInfo {
    /// Название семейства, например «Consolas» или «DejaVu Sans Mono».
    pub name: String,
    /// Файл шрифта.
    pub path: PathBuf,
}

/// Все моноширинные шрифты системы с обычным начертанием, по алфавиту.
///
/// Файлы не загружаются целиком: из каждого читаются только каталог таблиц,
/// таблица `name` (название и начертание) и таблицы `post` / `OS/2`
/// (признак моноширинности). Поэтому просмотр нескольких сотен шрифтов
/// занимает доли секунды. Список кэшируется.
pub fn monospace_fonts() -> Vec<FontInfo> {
    static CACHE: std::sync::OnceLock<Vec<FontInfo>> = std::sync::OnceLock::new();
    CACHE.get_or_init(scan_fonts).clone()
}

fn scan_fonts() -> Vec<FontInfo> {
    let mut files = Vec::new();
    for d in font_dirs() {
        collect_files(&d, 0, &mut files);
    }
    // Известные моноширинные шрифты — даже если в файле не стоит признак.
    let (known, _) = candidates();
    let known: Vec<PathBuf> = known.into_iter().map(|(p, _)| p).collect();

    let mut out: Vec<FontInfo> = Vec::new();
    for f in files {
        let Some(meta) = read_meta(&f) else { continue };
        if !(meta.fixed || known.contains(&f)) || !meta.regular {
            continue;
        }
        // Цветные шрифты эмодзи и образцы — не для текста программы.
        let lower = meta.family.to_lowercase();
        if lower.contains("emoji") || lower.contains("sample") {
            continue;
        }
        if out.iter().any(|i| i.name.eq_ignore_ascii_case(&meta.family)) {
            continue;
        }
        out.push(FontInfo { name: meta.family, path: f });
    }
    out.sort_by_key(|i| i.name.to_lowercase());
    out
}

/// Каталоги со шрифтами для текущей ОС.
fn font_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        let root = std::env::var("WINDIR").or_else(|_| std::env::var("SystemRoot")).unwrap_or_else(|_| "C:\\Windows".into());
        v.push(PathBuf::from(root).join("Fonts"));
        if let Ok(p) = std::env::var("LOCALAPPDATA") {
            v.push(PathBuf::from(p).join("Microsoft\\Windows\\Fonts"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        v.push(PathBuf::from("/System/Library/Fonts"));
        v.push(PathBuf::from("/Library/Fonts"));
        if let Ok(h) = std::env::var("HOME") {
            v.push(PathBuf::from(h).join("Library/Fonts"));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        v.push(PathBuf::from("/usr/share/fonts"));
        v.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(h) = std::env::var("HOME") {
            v.push(PathBuf::from(&h).join(".local/share/fonts"));
            v.push(PathBuf::from(&h).join(".fonts"));
        }
        if let Ok(x) = std::env::var("XDG_DATA_HOME") {
            v.push(PathBuf::from(x).join("fonts"));
        }
    }
    v
}

/// Файлы .ttf / .otf / .ttc в каталоге и его подкаталогах (до 4 уровней).
fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                collect_files(&p, depth + 1, out);
            }
        } else if p.extension().map(|x| matches!(x.to_string_lossy().to_lowercase().as_str(), "ttf" | "otf" | "ttc")).unwrap_or(false) {
            out.push(p);
        }
    }
}

/// Что удалось узнать о шрифте из его таблиц.
struct FontMeta {
    family: String,
    /// Моноширинный (`post.isFixedPitch` или `OS/2.panose`).
    fixed: bool,
    /// Обычное начертание (не жирный, не курсив).
    regular: bool,
}

/// Читает название и признаки шрифта (первого в коллекции .ttc).
fn read_meta(path: &Path) -> Option<FontMeta> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut rd = |off: u64, len: usize| -> Option<Vec<u8>> {
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut b = vec![0u8; len];
        f.read_exact(&mut b).ok()?;
        Some(b)
    };
    let be16 = |b: &[u8], i: usize| -> usize { u16::from_be_bytes([b[i], b[i + 1]]) as usize };
    let be32 = |b: &[u8], i: usize| -> u64 { u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as u64 };

    // Коллекция: берём первый шрифт.
    let head = rd(0, 12)?;
    let base = if &head[0..4] == b"ttcf" { be32(&rd(12, 4)?, 0) } else { 0 };
    let dir = rd(base, 12)?;
    let n = be16(&dir, 4);
    if n == 0 || n > 200 {
        return None;
    }
    let recs = rd(base + 12, n * 16)?;
    let table = |tag: &[u8; 4]| -> Option<(u64, usize)> {
        (0..n).map(|i| &recs[i * 16..i * 16 + 16]).find(|r| &r[0..4] == tag).map(|r| (be32(r, 8), be32(r, 12) as usize))
    };
    let (name_off, name_len) = table(b"name")?;
    let post = table(b"post");
    let os2 = table(b"OS/2");

    let mut fixed = false;
    if let Some((off, len)) = post {
        if len >= 16 {
            let b = rd(off, 16)?;
            fixed |= be32(&b, 12) != 0;
        }
    }
    if let Some((off, len)) = os2 {
        if len >= 42 {
            let b = rd(off, 42)?;
            // panose[3] (bProportion) == 9 — моноширинный.
            fixed |= b[32 + 3] == 9;
        }
    }

    let name = rd(name_off, name_len.min(1 << 20))?;
    if name.len() < 6 {
        return None;
    }
    let count = be16(&name, 2);
    let strings = be16(&name, 4);
    // nameID → (приоритет, текст). Лучше всего — Windows, английский (0x409);
    // затем любой Windows; затем Macintosh Roman.
    let mut best: [(u8, String); 3] = Default::default();
    for i in 0..count {
        let r = 6 + i * 12;
        if r + 12 > name.len() {
            break;
        }
        let (platform, lang, id) = (be16(&name, r), be16(&name, r + 4), be16(&name, r + 6));
        let (len, off) = (be16(&name, r + 8), be16(&name, r + 10));
        let slot = match id {
            1 => 0,  // семейство
            2 => 1,  // начертание
            16 => 2, // «типографское» семейство
            _ => continue,
        };
        let start = strings + off;
        let Some(raw) = name.get(start..start + len) else { continue };
        let (prio, text) = match platform {
            3 => {
                let u: Vec<u16> = raw.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                (if lang == 0x409 { 3 } else { 2 }, String::from_utf16_lossy(&u))
            }
            1 => (1, raw.iter().map(|&b| b as char).collect()),
            _ => continue,
        };
        if prio > best[slot].0 {
            best[slot] = (prio, text);
        }
    }
    let family = if !best[2].1.trim().is_empty() { best[2].1.trim().to_string() } else { best[0].1.trim().to_string() };
    if family.is_empty() {
        return None;
    }
    let sub = best[1].1.trim().to_lowercase();
    let regular = sub.is_empty() || matches!(sub.as_str(), "regular" | "normal" | "book" | "roman" | "medium" | "обычный" | "standard");
    Some(FontMeta { family, fixed, regular })
}

#[cfg(test)]
mod scan_tests {
    use super::*;

    /// Простейший файл TrueType с таблицами `name` и `post`.
    fn fake_font(family: &str, sub: &str, fixed: bool) -> Vec<u8> {
        let utf16 = |s: &str| s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect::<Vec<u8>>();
        let (f, s) = (utf16(family), utf16(sub));
        // Таблица name: 2 записи (nameID 1 и 2, Windows, en-US).
        let mut name = Vec::new();
        name.extend([0, 0, 0, 2, 0, 6 + 24]);
        let rec = |id: u16, len: usize, off: usize| {
            let mut r = vec![0, 3, 0, 1, 0x04, 0x09];
            r.extend(id.to_be_bytes());
            r.extend((len as u16).to_be_bytes());
            r.extend((off as u16).to_be_bytes());
            r
        };
        name.extend(rec(1, f.len(), 0));
        name.extend(rec(2, s.len(), f.len()));
        name.extend(&f);
        name.extend(&s);
        let mut post = vec![0u8; 32];
        post[15] = fixed as u8;
        // Каталог таблиц: 2 таблицы.
        let mut out = vec![0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        let name_off = 12 + 32;
        let post_off = name_off + name.len();
        let entry = |tag: &[u8; 4], off: usize, len: usize| {
            let mut e = tag.to_vec();
            e.extend([0, 0, 0, 0]);
            e.extend((off as u32).to_be_bytes());
            e.extend((len as u32).to_be_bytes());
            e
        };
        out.extend(entry(b"name", name_off, name.len()));
        out.extend(entry(b"post", post_off, post.len()));
        out.extend(name);
        out.extend(post);
        out
    }

    #[test]
    fn reads_name_and_fixed_pitch() {
        let dir = std::env::temp_dir().join(format!("tvgui-fonts-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let a = dir.join("mono.ttf");
        std::fs::write(&a, fake_font("Тестовый Моно", "Regular", true)).unwrap();
        let m = read_meta(&a).unwrap();
        assert_eq!(m.family, "Тестовый Моно");
        assert!(m.fixed && m.regular);
        let b = dir.join("bold.ttf");
        std::fs::write(&b, fake_font("Другой", "Bold", false)).unwrap();
        let m = read_meta(&b).unwrap();
        assert!(!m.fixed && !m.regular);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
