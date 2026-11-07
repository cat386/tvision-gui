//! Диалог настроек на «невидимом» экране (окно не открывается).

use tvision::platform::{HeadlessBackend, RawEvent};
use tvision_gui::*;

const CM_S: u16 = 200;
struct H { ok: Option<bool> }
impl AppHandler for H {
    fn handle_event(&mut self, app: &mut Program, ev: &mut Event) {
        if ev.command() == Some(CM_S) { self.ok = Some(settings_dialog(app)); ev.clear(); }
    }
}
fn k(key: Key) -> RawEvent { RawEvent::Key(KeyEvent::new(key, Mods::NONE)) }

fn run_with(keys: Vec<RawEvent>) -> (Program, H, String) {
    locale::set_language(locale::Language::Ru);
    let mut b = HeadlessBackend::new(100, 30);
    b.esc_budget = 0;
    b.input.push_back(RawEvent::Command(CM_S));
    b.input.extend(keys);
    let mut app = Program::new(Box::new(b), None, None);
    let mut h = H { ok: None };
    let mut shot = String::new();
    for _ in 0..40 { app.step(&mut h); if shot.is_empty() && app.screen_text().contains("Настройки") { shot = app.screen_text(); } }
    (app, h, shot)
}

#[test]
fn ok_applies_theme() {
    let (app, h, shot) = run_with(vec![k(Key::Down), k(Key::Enter)]);
    assert!(shot.contains("Тема") && shot.contains("Шрифт") && shot.contains("Размер шрифта"), "{}", shot);
    assert_eq!(h.ok, Some(true));
    assert_eq!(app.palette, find_theme("Классическая").unwrap().app_palette);
}

#[test]
fn cancel_restores_theme() {
    let (app, h, _) = run_with(vec![k(Key::Down), k(Key::Down), k(Key::Esc)]);
    assert_eq!(h.ok, Some(false));
    assert_eq!(app.palette, color::palettes::GRUVBOX.to_vec());
}
