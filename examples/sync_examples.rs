//! Генератор примеров: переносит примеры tvision-rs в графическое окно.
//!
//! Запуск (из каталога tvision-gui):
//!
//! ```text
//! cargo run --example sync_examples            # пересоздать примеры
//! cargo run --example sync_examples -- --check # только проверить, что они не устарели
//! ```
//!
//! Для каждого файла `../tvision-rs/examples/ИМЯ.rs` создаётся
//! `examples/ИМЯ.rs` — тот же код с минимальными заменами:
//!
//! * `Program::with_terminal(...)` → `Program::with_window(...)` (окно вместо терминала);
//! * `platform::default_backend()` → `tvision_gui::GuiBackend::boxed()`;
//! * `fn main()` → `fn tv_main()`, а новый `main` запускает его через
//!   `tvision_gui::run` (окно в главном потоке, программа — в своём);
//! * пути к файлам рядом с примером (`mod json;`, `include_str!("...")`,
//!   `env!("CARGO_MANIFEST_DIR")`) переводятся на каталог tvision-rs.
//!
//! Код окон, диалогов и обработчиков не меняется, поэтому примеры всегда
//! совпадают с оригиналами. Не переносятся:
//!
//! * `hello`, `tvedit`, `tvdemo` — у tvision-gui свои версии этих примеров
//!   (полностью на русском);
//! * `tvhc` — утилита командной строки, окно ей не нужно;
//! * примеры, в которых не создаётся программа Turbo Vision.
//!
//! Сгенерированные файлы помечены первой строкой
//! `// СГЕНЕРИРОВАНО sync_examples`; файлы без этой метки генератор не трогает.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Метка сгенерированного файла (первая строка).
const MARK: &str = "// СГЕНЕРИРОВАНО sync_examples";

/// Примеры, которые не переносятся (см. описание модуля).
const SKIP: &[&str] = &["hello", "tvedit", "tvdemo", "tvhc", "sync_examples"];

/// Путь от `tvision-gui/examples` к `tvision-rs/examples` (для `#[path]`,
/// `include_str!`) — так же, как зависимость `path = "../tvision-rs"`.
const REL: &str = "../../tvision-rs/examples";

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let gui = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = gui.join("../tvision-rs/examples");
    let dst_dir = gui.join("examples");

    let mut names: Vec<String> = match fs::read_dir(&src_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "rs").unwrap_or(false))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect(),
        Err(e) => {
            eprintln!("Не найден каталог {}: {}", src_dir.display(), e);
            return ExitCode::FAILURE;
        }
    };
    names.sort();

    let (mut written, mut same, mut stale) = (0, 0, Vec::new());
    for name in &names {
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        let src_path = src_dir.join(format!("{}.rs", name));
        let src = match fs::read_to_string(&src_path) {
            Ok(s) => s.replace("\r\n", "\n"),
            Err(e) => {
                eprintln!("{}: {}", src_path.display(), e);
                continue;
            }
        };
        let out = match convert(name, &src, &src_dir) {
            Some(o) => o,
            None => {
                println!("  пропущен  {} (не создаёт программу Turbo Vision)", name);
                continue;
            }
        };
        let dst_path = dst_dir.join(format!("{}.rs", name));
        let old = fs::read_to_string(&dst_path).ok().map(|s| s.replace("\r\n", "\n"));
        if let Some(o) = &old {
            if !o.starts_with(MARK) {
                println!("  пропущен  {} (файл написан вручную)", name);
                continue;
            }
            if *o == out {
                same += 1;
                continue;
            }
        }
        if check {
            stale.push(name.clone());
            continue;
        }
        if let Err(e) = fs::write(&dst_path, &out) {
            eprintln!("{}: {}", dst_path.display(), e);
            return ExitCode::FAILURE;
        }
        println!("  {}  {}", if old.is_some() { "обновлён" } else { "создан  " }, name);
        written += 1;
    }

    if check {
        if stale.is_empty() {
            println!("Все примеры совпадают с tvision-rs ({}).", same);
            ExitCode::SUCCESS
        } else {
            println!("Устарели: {}. Запустите `cargo run --example sync_examples`.", stale.join(", "));
            ExitCode::FAILURE
        }
    } else {
        println!("Готово: записано {}, без изменений {}.", written, same);
        ExitCode::SUCCESS
    }
}

/// Переводит исходник примера на окно. `None` — пример не создаёт
/// программу Turbo Vision (переносить нечего).
fn convert(name: &str, src: &str, src_dir: &Path) -> Option<String> {
    if !src.contains("Program::with_terminal(") && !src.contains("default_backend()") {
        return None;
    }

    let mut out = String::new();
    out.push_str(MARK);
    out.push_str(&format!(" из tvision-rs/examples/{}.rs — не правьте вручную.\n", name));
    out.push_str("// Пересоздать: `cargo run --example sync_examples`.\n");
    out.push_str("// Отличия от оригинала: окно вместо терминала (`with_window`, `GuiBackend`)\n");
    out.push_str("// и функция `main`, запускающая программу через `tvision_gui::run`.\n");

    let mut main_ret: Option<String> = None;
    let mut found_main = false;
    // Предыдущая строка — уже `#[path = ...]` (тогда свой атрибут не нужен).
    let mut had_path = false;
    for line in src.lines() {
        let mut l = line.to_string();

        // `#[path = "json/mod.rs"]` у модуля верхнего уровня — путь
        // относительно tvision-rs/examples.
        if let Some(p) = l.strip_prefix("#[path = \"").and_then(|r| r.strip_suffix("\"]")) {
            out.push_str(&format!("#[path = \"{}/{}\"]\n", REL, p));
            had_path = true;
            continue;
        }
        let path_given = std::mem::take(&mut had_path);

        // `mod json;` в начале строки — модуль из каталога tvision-rs/examples.
        if let Some(m) = l.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            let m = m.trim();
            if !m.contains(' ') && !path_given {
                let file = if src_dir.join(m).join("mod.rs").is_file() { format!("{}/mod.rs", m) } else { format!("{}.rs", m) };
                out.push_str(&format!("#[path = \"{}/{}\"]\n", REL, file));
            }
        }

        // Создание программы: терминал → окно.
        l = l.replace("Program::with_terminal(", "<Program as tvision_gui::ProgramExt>::with_window(");
        l = l.replace("tvision::platform::default_backend()", "tvision_gui::GuiBackend::boxed()");
        l = l.replace("platform::default_backend()", "tvision_gui::GuiBackend::boxed()");

        // Файлы рядом с примером.
        for mac in ["include_str!(\"", "include_bytes!(\""] {
            l = l.replace(mac, &format!("{}{}/", mac, REL));
        }
        l = l.replace("env!(\"CARGO_MANIFEST_DIR\")", "concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../tvision-rs\")");

        // Точка входа.
        if !found_main && l.starts_with("fn main()") {
            found_main = true;
            let ret = l["fn main()".len()..].trim().trim_end_matches('{').trim();
            main_ret = Some(ret.strip_prefix("->").map(|r| r.trim().to_string()).unwrap_or_default());
            l = l.replacen("fn main()", "fn tv_main()", 1);
        }

        out.push_str(&l);
        out.push('\n');
    }

    let ret = main_ret?;
    let title = title_of(name, src);
    out.push_str("\n/// Точка входа в окне: окно — в главном потоке (обязательно для macOS),\n");
    out.push_str("/// программа примера (`tv_main`) — в отдельном потоке.\n");
    if ret.is_empty() {
        out.push_str("fn main() {\n");
    } else {
        out.push_str(&format!("fn main() -> {} {{\n", ret));
    }
    out.push_str(&format!("    tvision_gui::run(tvision_gui::GuiConfig::new({:?}), tv_main)\n", title));
    out.push_str("}\n");
    Some(out)
}

/// Заголовок окна: первая строка документации примера (без точки в конце)
/// или имя примера.
fn title_of(name: &str, src: &str) -> String {
    let first = src.lines().find_map(|l| l.strip_prefix("//!")).map(|l| l.trim()).filter(|l| !l.is_empty());
    match first {
        Some(t) => {
            let t: String = t.trim_end_matches('.').chars().take(70).collect();
            // Обратные кавычки из Markdown в заголовке окна не нужны.
            t.replace('`', "")
        }
        None => format!("tvision-rs: {}", name),
    }
}
