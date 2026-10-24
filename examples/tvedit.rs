//! Порт примера `examples/tvedit` из Turbo Vision: многооконный текстовый
//! редактор с меню, файловыми диалогами, поиском и заменой.
//!
//! Запуск: `cargo run --example tvedit [файлы...]`
//!
//! Версия для графического окна (tvision-gui): код тот же, что в
//! tvision-rs/examples/tvedit.rs, изменены только `use`, `with_window` и `main`.
//!
//! Отличие от оригинала: «Выход» — Alt+X, а не Ctrl+Q, потому что Ctrl+Q
//! в редакторе — префикс команд WordStar (Ctrl+Q F — поиск, Ctrl+Q A — замена).

use std::path::{Path, PathBuf};

use tvision_gui::*;

// Команды приложения (как в `tvedit.h`).
const CM_CHANGE_DRCT: u16 = 102;
const CM_SHOW_CLIP: u16 = 105;

struct EditorApp {
    /// Счётчик для номеров окон.
    windows: i16,
}

impl EditorApp {
    /// Открыть окно редактора (`openEditor`).
    fn open_editor(&mut self, app: &mut Program, file: &Path) {
        self.windows += 1;
        let r = app.desktop().group.vs.extent();
        let w = EditWindow::new(r, file, if self.windows < 10 { self.windows } else { 0 });
        app.insert_window(w);
    }

    /// «Открыть файл» (`fileOpen`).
    fn file_open(&mut self, app: &mut Program) {
        let d = FileDialog::new("*", "Открыть файл", "~И~мя", fd::OPEN_BUTTON, 100);
        let (cmd, d) = app.exec_dialog(d);
        if cmd != cm::CANCEL {
            let name = d.file_name();
            self.open_editor(app, &name);
        }
    }

    /// «Сменить каталог» (`changeDir`).
    fn change_dir(&mut self, app: &mut Program) {
        let d = ChDirDialog::new(cd::NORMAL, 0);
        app.exec_dialog(d);
    }
}

impl AppHandler for EditorApp {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        let c = match ev.command() {
            Some(c) => c,
            None => return,
        };
        match c {
            cm::OPEN => self.file_open(app),
            cm::NEW => self.open_editor(app, Path::new("")),
            CM_CHANGE_DRCT => self.change_dir(app),
            CM_SHOW_CLIP => {
                let t = app.ctx.clipboard.get();
                let text = if t.is_empty() { "(пусто)".to_string() } else { t };
                message_box_rect(app, Rect::new(0, 0, 60, 12), &format!("Буфер обмена:\n{}", text), mf::INFORMATION | mf::OK_BUTTON, true);
            }
            cm::DOS_SHELL => {
                message_box(app, "\x03Выход в оболочку не поддерживается в этом примере.", mf::INFORMATION | mf::OK_BUTTON);
            }
            _ => return,
        }
        ev.clear();
    }
}

fn main() -> std::io::Result<()> {
    run(GuiConfig::new("Редактор Turbo Vision").size(100, 30), app_main)
}

/// Тело программы — то же, что `main` в версии для терминала.
fn app_main() -> std::io::Result<()> {
    let menu = Menu::new()
        .submenu(
            "~Ф~айл",
            Menu::new()
                .item_key("~О~ткрыть", cm::OPEN, kb::F3, "F3")
                .item_key("~Н~овый", cm::NEW, kb::ctrl('N'), "Ctrl-N")
                .item_key("~С~охранить", cm::SAVE, kb::F2, "F2")
                .item("Сохранить ~к~ак...", cm::SAVE_AS)
                .separator()
                .item("Сменить ~п~апку...", CM_CHANGE_DRCT)
                .item("О~б~олочка ОС", cm::DOS_SHELL)
                .item_key("В~ы~ход", cm::QUIT, kb::ALT_X, "Alt-X"),
        )
        .submenu(
            "~П~равка",
            Menu::new()
                .item_key("~О~тменить", cm::UNDO, kb::ctrl('U'), "Ctrl-U")
                .item("Пов~т~орить", cm::REDO)
                .separator()
                .item_key("~В~ырезать", cm::CUT, KeyEvent::new(Key::Del, Mods::SHIFT), "Shift-Del")
                .item_key("~К~опировать", cm::COPY, KeyEvent::new(Key::Ins, Mods::CTRL), "Ctrl-Ins")
                .item_key("Вст~а~вить", cm::PASTE, KeyEvent::new(Key::Ins, Mods::SHIFT), "Shift-Ins")
                .item("~П~оказать буфер обмена", CM_SHOW_CLIP)
                .separator()
                .item_key("~У~далить", cm::CLEAR, KeyEvent::new(Key::Del, Mods::CTRL), "Ctrl-Del"),
        )
        .submenu("Пои~с~к", Menu::new().item("~Н~айти...", cm::FIND).item("~З~аменить...", cm::REPLACE).item("Найти ~д~альше", cm::SEARCH_AGAIN))
        .submenu(
            "~О~кна",
            Menu::new()
                .item_key("~Р~азмер/место", cm::RESIZE, kb::CTRL_F5, "Ctrl-F5")
                .item_key("Р~а~звернуть", cm::ZOOM, kb::F5, "F5")
                .item("~М~озаика", cm::TILE)
                .item("~К~аскад", cm::CASCADE)
                .item_key("~С~ледующее", cm::NEXT, kb::F6, "F6")
                .item_key("~П~редыдущее", cm::PREV, kb::SHIFT_F6, "Shift-F6")
                .item_key("~З~акрыть", cm::CLOSE, kb::ctrl('W'), "Ctrl-W"),
        );

    let status = vec![StatusDef::all(vec![
        StatusItem::new("", kb::ALT_X, cm::QUIT),
        StatusItem::new("~F2~ Сохранить", kb::F2, cm::SAVE),
        StatusItem::new("~F3~ Открыть", kb::F3, cm::OPEN),
        StatusItem::new("~Ctrl-W~ Закрыть", kb::ALT_F3, cm::CLOSE),
        StatusItem::new("~F5~ Развернуть", kb::F5, cm::ZOOM),
        StatusItem::new("~F6~ Следующее", kb::F6, cm::NEXT),
        StatusItem::new("~F10~ Меню", kb::F10, cm::MENU),
        StatusItem::new("", kb::SHIFT_F6, cm::PREV),
        StatusItem::new("", kb::CTRL_F5, cm::RESIZE),
    ])];

    // Встроенные надписи (диалоги файлов, поиск, окна сообщений) — по-русски.
    // Язык хранится отдельно для каждого потока: включаем его в потоке программы.
    locale::set_language(locale::Language::Ru);
    let mut app = Program::with_window(Some(menu), Some(status))?;
    let mut ed = EditorApp { windows: 0 };
    // Файлы из командной строки (как в оригинале).
    for arg in std::env::args().skip(1) {
        ed.open_editor(&mut app, &PathBuf::from(arg));
    }
    app.run(&mut ed);
    Ok(())
}
