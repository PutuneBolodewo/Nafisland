//! EVE Online Intel Tool page (`PageKind::EveIntel`).
//!
//! Two-column layout inside the expanded island:
//! - Left column: compact EVE SSO button (public data mode in Phase 1), manual
//!   intel channel name input, matched `.txt` status, and Alarm Settings button
//!   (opens Advanced Settings -> EVE Intel).
//! - Right column: scrollable live intel feed from the matched chatlog.

use crate::commands::{Command, EveIntelCmd};
use crate::gfx::{TextStyle, Weight};
use crate::icons::Icon;
use crate::settings_window::Section;
use crate::state::AppState;
use crate::ui::{wid, PillStyle, Ui, PILL_GAP, PILL_SMALL};
use poni_core::eve_intel::should_alert;
use poni_core::geom::{palette, Color, Point, Rect};

const HEIGHT: f32 = 256.0;
const COL_GAP: f32 = 18.0;
const LEFT_FRAC: f32 = 0.42;
const LEFT_MIN: f32 = 210.0;
const LEFT_MAX: f32 = 260.0;
const MSG_ROW_H: f32 = 34.0;

pub fn height(_st: &AppState) -> f32 {
    HEIGHT
}

pub fn draw(ui: &mut Ui, st: &mut AppState, area: Rect) {
    let accent = st.accent;
    let left_w = (area.w * LEFT_FRAC).clamp(LEFT_MIN, LEFT_MAX).min((area.w - COL_GAP - 180.0).max(160.0));
    let left = Rect::new(area.x, area.y, left_w, area.h);
    let right = Rect::new(left.right() + COL_GAP, area.y, (area.w - left_w - COL_GAP).max(0.0), area.h);

    draw_left(ui, st, left, accent);
    draw_right(ui, st, right, accent);
}

fn draw_left(ui: &mut Ui, st: &mut AppState, r: Rect, accent: Color) {
    // 1. Small EVE SSO login button (top-left).
    let sso_label = "EVE SSO · Public";
    let sso_w = ui.pill_width(sso_label, true, true).min(r.w);
    if ui.pill(wid("eve.sso"), Rect::new(r.x, r.y, sso_w, PILL_SMALL), sso_label, Some(Icon::Lock), PillStyle::Utility).clicked {
        st.cmd(Command::EveIntel(EveIntelCmd::SsoConnect));
    }
    let sso_note = st
        .eve_intel
        .sso_note
        .as_deref()
        .unwrap_or("Public data mode · SSO v2 PKCE planned");
    ui.c.text(sso_note, Rect::new(r.x, r.y + PILL_SMALL + 4.0, r.w, 15.0), &TextStyle::caption().color(palette::TEXT_DIM));

    // 2. Manual intel channel name input below SSO.
    let label_y = r.y + PILL_SMALL + 28.0;
    ui.c.text(
        "INTEL CHANNEL",
        Rect::new(r.x, label_y, r.w, 15.0),
        &TextStyle::caption().weight(Weight::Semibold).color(palette::TEXT_DIM),
    );
    let input_y = label_y + 18.0;
    let apply_label = "Watch";
    let apply_w = ui.pill_width(apply_label, false, true);
    let input_w = (r.w - apply_w - PILL_GAP).max(100.0);
    let input_rect = Rect::new(r.x, input_y, input_w, 30.0);
    let inp = ui.text_input(wid("eve.channel"), input_rect, &mut st.ui.eve_channel_input, "e.g. I. Ftn Intel", 80);
    let dirty = st.ui.eve_channel_input.trim() != st.settings.eve_intel.channel.trim();
    let btn_style = if dirty { PillStyle::Primary(accent) } else { PillStyle::Neutral };
    let apply_clicked = ui.pill(wid("eve.channel.apply"), Rect::new(input_rect.right() + PILL_GAP, input_y + 1.0, apply_w, PILL_SMALL), apply_label, None, btn_style).clicked;
    if inp.submitted || apply_clicked {
        let next = st.ui.eve_channel_input.trim().to_string();
        st.ui.eve_channel_input = next.clone();
        st.cmd(Command::EveIntel(EveIntelCmd::SetChannel(next)));
    }

    // Active matched .txt status line.
    let status_y = input_y + 36.0;
    let (status_text, status_color) = if st.settings.eve_intel.channel.trim().is_empty() {
        ("Type the exact channel name from Chatlogs.".to_string(), palette::TEXT_DIM)
    } else if let Some(err) = &st.eve_intel.error {
        (err.clone(), palette::ORANGE)
    } else if let Some(file) = &st.eve_intel.filename {
        (format!("Watching: {file}"), palette::GREEN)
    } else {
        ("Searching Chatlogs…".to_string(), palette::TEXT_DIM)
    };
    ui.c.text(&status_text, Rect::new(r.x, status_y, r.w, 34.0), &TextStyle::caption().wrap().top().color(status_color));

    // 3. Bottom controls: Alarm settings (opens Advanced Settings -> EVE Intel) + quick Test.
    let btn_y = r.bottom() - PILL_SMALL;
    let test_label = "Test";
    let test_w = ui.pill_width(test_label, true, true);
    let cfg_label = "Alarm Settings";
    let cfg_w = (r.w - test_w - PILL_GAP).min(ui.pill_width(cfg_label, true, true).max(120.0));
    if ui.pill(wid("eve.alarm.cfg"), Rect::new(r.x, btn_y, cfg_w, PILL_SMALL), cfg_label, Some(Icon::Sliders), PillStyle::Utility).clicked {
        st.cmd(Command::OpenSettingsSection(Section::EveIntel));
    }
    if ui.pill(wid("eve.alarm.test"), Rect::new(r.x + cfg_w + PILL_GAP, btn_y, test_w, PILL_SMALL), test_label, Some(Icon::Bell), PillStyle::Neutral).clicked {
        st.cmd(Command::EveIntel(EveIntelCmd::TestAlarm));
    }
}

fn draw_right(ui: &mut Ui, st: &mut AppState, r: Rect, accent: Color) {
    let card = super::card::pill_card(st, r);
    super::card::fill_box(ui, card, r, 12.0, Color::white(0.04));

    let pad = 10.0;
    let inner = Rect::new(r.x + pad, r.y + 8.0, (r.w - 2.0 * pad).max(0.0), (r.h - 16.0).max(0.0));
    ui.c.text(
        "LIVE INTEL FEED",
        Rect::new(inner.x, inner.y, inner.w * 0.55, 16.0),
        &TextStyle::caption().weight(Weight::Semibold).color(palette::TEXT_DIM),
    );
    if let Some(listener) = &st.eve_intel.listener {
        ui.c.text(
            listener,
            Rect::new(inner.x + inner.w * 0.45, inner.y, inner.w * 0.55, 16.0),
            &TextStyle::caption().right().color(palette::TEXT_DIM),
        );
    }

    let list_r = Rect::new(inner.x, inner.y + 22.0, inner.w, (inner.h - 22.0).max(0.0));
    let msgs = &st.eve_intel.messages;
    if msgs.is_empty() {
        let hint = if st.settings.eve_intel.channel.trim().is_empty() {
            "No channel selected"
        } else {
            "Waiting for intel lines…"
        };
        ui.c.text(hint, list_r, &TextStyle::footnote().center().color(palette::TEXT_DIM));
        return;
    }

    let total_h = msgs.len() as f32 * MSG_ROW_H;
    let max_scroll = (total_h - list_r.h).max(0.0);
    let wheel = ui.wheel_in(list_r);
    if wheel != 0.0 {
        st.ui.eve_feed_scroll = (st.ui.eve_feed_scroll - wheel * 28.0).clamp(0.0, max_scroll);
        ui.request_repaint();
    } else {
        st.ui.eve_feed_scroll = st.ui.eve_feed_scroll.clamp(0.0, max_scroll);
    }

    ui.c.clip(list_r);
    let ignore = &st.settings.eve_intel.ignore_words;
    // Show newest intel messages first.
    for (idx, msg) in msgs.iter().rev().enumerate() {
        let y = list_r.y + idx as f32 * MSG_ROW_H - st.ui.eve_feed_scroll;
        if y + MSG_ROW_H < list_r.y || y > list_r.bottom() {
            continue;
        }
        let alert = should_alert(msg, ignore);
        let dot_color = if alert { accent } else { palette::TEXT_DIM };
        ui.c.fill_circle(Point::new(list_r.x + 5.0, y + 8.0), 3.0, dot_color);

        let time_w = 54.0;
        ui.c.text(&msg.time_short, Rect::new(list_r.x + 13.0, y, time_w, 15.0), &TextStyle::caption().tabular().color(palette::TEXT_DIM));
        let sender_x = list_r.x + 13.0 + time_w + 4.0;
        let sender_w = (list_r.right() - sender_x).max(0.0);
        ui.c.text(
            &msg.sender,
            Rect::new(sender_x, y, sender_w, 15.0),
            &TextStyle::caption().weight(Weight::Semibold).color(if alert { palette::TEXT } else { palette::TEXT_DIM }),
        );
        ui.c.text(
            &msg.text,
            Rect::new(list_r.x + 13.0, y + 15.0, (list_r.w - 13.0).max(0.0), 16.0),
            &TextStyle::footnote().color(if alert { palette::TEXT } else { palette::TEXT_DIM }),
        );
    }
    ui.c.unclip();
}
