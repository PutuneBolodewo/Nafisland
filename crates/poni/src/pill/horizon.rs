//! Horizon theme renderer: dynamic 5-phase sky gradient synced with Prayer Times,
//! multi-size starfield, celestial icons, and dynamic shooting stars.

use crate::gfx::Canvas;
use crate::icons::Icon;
use crate::state::AppState;
use poni_core::geom::{Color, Point, Rect};
use poni_core::svgpath::Seg;
use poni_core::horizon::{
    self, CelestialIcon, DayPhase, HorizonPalette, SkySchedule, STARS,
};
use poni_core::settings::Theme;

/// Duration of one shooting star streak animation (seconds).
pub const SHOOTING_STAR_DUR: f64 = 0.70;

/// Whether `theme` renders the Horizon sky (notched or floating Pill).
pub fn is_on(theme: Theme) -> bool {
    matches!(theme, Theme::Horizon | Theme::HorizonPill)
}

/// Shooting star animation state on `AppState`.
#[derive(Clone, Debug, Default)]
pub struct ShootingStar {
    pub active_from: Option<f64>,
    pub next_at: Option<f64>,
    pub seed: u64,
}

impl ShootingStar {
    /// Advance shooting star state at `now`. Returns `true` while animating.
    pub fn tick(&mut self, st_theme: Theme, phase: DayPhase, now: f64, paused: bool) -> bool {
        if !is_on(st_theme) || paused {
            self.active_from = None;
            return false;
        }
        if let Some(t0) = self.active_from {
            if now - t0 < SHOOTING_STAR_DUR {
                return true;
            }
            self.active_from = None;
            self.schedule_next(phase, now);
            return false;
        }
        match horizon::shooting_star_interval_s(phase) {
            None => {
                self.next_at = None;
                false
            }
            Some(_) => {
                let due = match self.next_at {
                    Some(t) => t,
                    None => {
                        self.schedule_next(phase, now);
                        return false;
                    }
                };
                if now >= due {
                    self.active_from = Some(now);
                    self.seed = self.seed.wrapping_add(1);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Time until the next shooting star should start, if scheduled.
    pub fn wake_at(&self, theme: Theme, phase: DayPhase, paused: bool) -> Option<f64> {
        if !is_on(theme) || paused || self.active_from.is_some() {
            return None;
        }
        horizon::shooting_star_interval_s(phase)?;
        self.next_at
    }

    fn schedule_next(&mut self, phase: DayPhase, now: f64) {
        self.seed = self.seed.wrapping_add(1);
        self.next_at = horizon::next_shooting_star_delay_s(phase, self.seed).map(|d| now + d);
    }
}

/// Build the solar/prayer schedule from Prayer Times when available.
pub fn schedule(st: &AppState) -> SkySchedule {
    let Some(site) = st.prayer_site() else {
        return SkySchedule::default();
    };
    let now = st.prayer_now();
    let Some(t) = poni_core::prayer::day_times(&site, site.local_day(now)) else {
        return SkySchedule::default();
    };
    let local = st.local.hour as i64 * 3600 + st.local.minute as i64 * 60 + st.local.second as i64;
    let ahead = (((local - st.unix.rem_euclid(86_400)) as f64 / 60.0).round() as i64 * 60 + 43_200).rem_euclid(86_400) - 43_200;
    let min_of = |utc: i64| ((utc + ahead).div_euclid(60).rem_euclid(1440)) as f32;
    SkySchedule {
        sunrise: min_of(t.terbit),
        noon: min_of(t.dzuhur),
        ashar: min_of(t.ashar),
        sunset: min_of(t.maghrib),
        isya: min_of(t.isya),
    }
}

pub fn local_minute(st: &AppState) -> f32 {
    (st.local.hour as f32) * 60.0 + (st.local.minute as f32) + (st.local.second as f32) / 60.0
}

pub fn phase(st: &AppState) -> DayPhase {
    horizon::phase_at(local_minute(st), schedule(st))
}

pub fn palette(st: &AppState) -> HorizonPalette {
    horizon::palette_at(local_minute(st), schedule(st))
}

pub fn celestial_icon(st: &AppState) -> Icon {
    match horizon::icon_for(phase(st)) {
        CelestialIcon::SunriseMountain => Icon::Sunrise,
        CelestialIcon::Sun => Icon::Sun,
        CelestialIcon::SunsetHorizon => Icon::Sunset,
        CelestialIcon::MoonStars => Icon::Moon,
        CelestialIcon::MidnightStars => Icon::SparkleStars,
    }
}

/// The island is tall enough (an open page) for the whole landscape; the
/// resting pill gets a low silhouette of mountains only.
const BIG_H: f32 = 90.0;

/// Draw the Horizon sky gradient, stars, celestial bodies, landscape and
/// shooting star inside `body` (extending the sky gradient by `shoulder` on
/// each side so flared Classic notch shoulders are never black). Caller should
/// clip to the notch silhouette before calling.
pub fn draw_sky(c: &mut Canvas, st: &AppState, body: Rect, shoulder: f32) {
    let pal = palette(st);
    let ph = phase(st);
    let big = body.h >= BIG_H;
    // The landscape takes the lower 20-30 % of an open page, the sky the rest.
    let land_h = if big { (body.h * 0.28).clamp(40.0, 140.0) } else { body.h * 0.32 };
    let sky_h = if big { body.h - land_h } else { body.h };
    let full = Rect::new(body.x - shoulder, body.y, body.w + 2.0 * shoulder, body.h);
    c.fill_linear_gradient_stops(full, &pal.stops);

    let land = Rect::new(body.x, body.bottom() - land_h, body.w, land_h);
    if matches!(ph, DayPhase::Night | DayPhase::Midnight) {
        let boost = if ph == DayPhase::Midnight { 1.0 } else { 0.78 };
        for star in STARS {
            let pt = Point::new(body.x + star.x * body.w, body.y + star.y * sky_h * 0.92);
            c.fill_circle(pt, star.radius, Color::white(star.alpha * boost));
        }
    }
    if big {
        if pal.has_moon {
            let r = (land_h * 0.16).clamp(5.0, 14.0);
            let at = Point::new(body.x + body.w * 0.78, body.y + sky_h * 0.20);
            glow(c, at, r, Color::hex(0xF4F1DC), 0.10);
            c.fill_circle(at, r, Color::hex(0xF4F1DC));
        }
        if pal.has_sun {
            // Rises from behind the mountains as the morning goes on.
            let up = horizon::morning_progress(local_minute(st), schedule(st));
            let r = land_h * 0.22;
            let at = Point::new(land.x + land.w * 0.46, land.y + land.h * 0.62 - up * land_h * 0.55);
            glow(c, at, r, Color::hex(0xFFE9A0), 0.16);
            c.fill_circle(at, r, Color::hex(0xFFF1B8));
        }
    }
    draw_landscape(c, &pal, land, big);
    if ph != DayPhase::Morning && ph != DayPhase::Day {
        draw_shooting_star(c, st, body);
    }
}

fn poly(c: &mut Canvas, pts: &[Point], fill: Color) {
    let mut segs: Vec<Seg> = Vec::with_capacity(pts.len() + 1);
    for (i, p) in pts.iter().enumerate() {
        segs.push(if i == 0 { Seg::MoveTo(p.x, p.y) } else { Seg::LineTo(p.x, p.y) });
    }
    segs.push(Seg::Close);
    c.path(&segs, Some(fill), None);
}

/// A soft halo: three rings fading outward.
fn glow(c: &mut Canvas, at: Point, r: f32, color: Color, strength: f32) {
    for k in (1..=3).rev() {
        c.fill_circle(at, r * (1.0 + 0.7 * k as f32), color.with_alpha(strength / k as f32));
    }
}

/// A pine of three tiers standing on `(x, base)`, `h` tall.
fn pine(c: &mut Canvas, x: f32, base: f32, h: f32, color: Color) {
    for (top, drop, w) in [(1.0, 0.40, 0.30), (0.72, 0.42, 0.44), (0.42, 0.42, 0.58)] {
        let y = base - h * top;
        poly(c, &[Point::new(x, y), Point::new(x - h * w / 2.0, y + h * drop), Point::new(x + h * w / 2.0, y + h * drop)], color);
    }
}

/// Mountains, pines, a cabin and a lake in `land` (the bottom of the
/// island). `big` draws all of it; the resting pill only the mountains.
fn draw_landscape(c: &mut Canvas, pal: &HorizonPalette, land: Rect, big: bool) {
    let [far, mid, near] = pal.land;
    let at = |nx: f32, ny: f32| Point::new(land.x + land.w * nx, land.y + land.h * ny);
    let u = land.h;
    // Far range, peaking on the right; middle range, a low swell.
    poly(c, &[at(0.0, 0.78), at(0.30, 0.45), at(0.42, 0.60), at(0.62, 0.12), at(0.78, 0.40), at(0.88, 0.28), at(1.0, 0.52), at(1.0, 1.0), at(0.0, 1.0)], far);
    poly(c, &[at(0.0, 0.92), at(0.20, 0.52), at(0.38, 0.72), at(0.55, 0.56), at(0.75, 0.86), at(1.0, 0.80), at(1.0, 1.0), at(0.0, 1.0)], mid);
    if !big {
        return;
    }
    // A far line of pines on the right shore.
    for i in 0..9 {
        let x = 0.62 + 0.037 * i as f32;
        pine(c, land.x + land.w * x, land.y + land.h * 0.86, u * (0.22 + 0.04 * (i % 3) as f32), near.with_alpha(0.7));
    }
    // The lake, with the shore's line and the light's reflections.
    let water = poly_water(far, near);
    c.fill_rect(Rect::new(land.x + land.w * 0.40, land.y + land.h * 0.86, land.w * 0.60, land.h * 0.14), water);
    c.fill_rect(Rect::new(land.x + land.w * 0.40, land.y + land.h * 0.86, land.w * 0.60, 1.0), far.with_alpha(0.5));
    let mirror = if pal.has_moon { Some((Color::hex(0xF4F1DC), 0.78)) } else if pal.has_sun { Some((Color::hex(0xFFF1B8), 0.46)) } else { None };
    if let Some((color, nx)) = mirror {
        for (i, w) in [0.30, 0.20, 0.10].into_iter().enumerate() {
            let y = land.y + land.h * (0.90 + 0.04 * i as f32);
            c.fill_rect(Rect::new(land.x + land.w * nx - u * w / 2.0, y, u * w, 1.0), color.with_alpha(0.35));
        }
    }
    // The near hill, with two tall pines at its left.
    poly(c, &[at(0.0, 0.74), at(0.10, 0.68), at(0.22, 0.70), at(0.33, 0.78), at(0.40, 0.90), at(0.44, 1.0), at(0.0, 1.0)], near);
    pine(c, land.x + land.w * 0.10, land.y + land.h * 0.80, u * 0.92, near);
    pine(c, land.x + land.w * 0.06, land.y + land.h * 0.82, u * 0.62, near);
    // The cabin: walls, roof, chimney, a door and a window that lights up.
    let (cx, base) = (land.x + land.w * 0.22, land.y + land.h * 0.70);
    let (cw, ch) = (u * 0.62, u * 0.30);
    let wall = horizon::lerp_color(near, Color::hex(0xFFFFFF), 0.16);
    let dark = horizon::lerp_color(near, Color::hex(0x000000), 0.35);
    c.fill_rect(Rect::new(cx + cw * 0.18, base - ch - u * 0.20, u * 0.06, u * 0.14), near);
    c.fill_rect(Rect::new(cx - cw / 2.0, base - ch, cw, ch), wall);
    poly(c, &[Point::new(cx - cw * 0.58, base - ch), Point::new(cx, base - ch - u * 0.18), Point::new(cx + cw * 0.58, base - ch)], dark);
    c.fill_rect(Rect::new(cx + cw * 0.12, base - ch * 0.65, cw * 0.20, ch * 0.65), dark);
    let win = Rect::new(cx - cw * 0.28, base - ch * 0.80, cw * 0.22, ch * 0.42);
    if pal.cabin_light > 0.05 {
        glow(c, win.center(), u * 0.14, Color::hex(0xFFD875), 0.10 * pal.cabin_light);
    }
    c.fill_rect(win, horizon::lerp_color(far, Color::hex(0xFFD875), pal.cabin_light));
}

/// The lake: the far range's colour sunk toward the near one.
fn poly_water(far: Color, near: Color) -> Color {
    horizon::lerp_color(far, near, 0.35)
}

fn draw_shooting_star(c: &mut Canvas, st: &AppState, body: Rect) {
    let Some(t0) = st.horizon_star.active_from else { return };
    let t = ((st.now - t0) / SHOOTING_STAR_DUR).clamp(0.0, 1.0) as f32;
    if t <= 0.0 || t >= 1.0 {
        return;
    }
    // Pick starting X from seed so consecutive shooting stars vary across the island.
    let lane = ((st.horizon_star.seed % 5) as f32) * 0.16 + 0.12;
    let start_x = body.x + body.w * lane;
    let start_y = body.y + body.h * 0.16;
    let dx = (body.w * 0.28).max(40.0);
    let dy = (body.h * 0.55).max(14.0);
    let head = Point::new(start_x + dx * t, start_y + dy * t);
    let tail_t = (t - 0.28).max(0.0);
    let tail = Point::new(start_x + dx * tail_t, start_y + dy * tail_t);
    let fade = if t < 0.2 { t / 0.2 } else { (1.0 - t) / 0.8 };
    c.line_gradient(
        tail,
        head,
        Color::white(0.0),
        Color::white(0.92 * fade),
        1.5,
    );
    c.fill_circle(head, 1.6, Color::white(fade));
}
