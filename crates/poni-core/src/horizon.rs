//! Horizon theme model: 5 time-of-day phases (Morning, Day, Dusk, Night, Midnight),
//! multi-stop sky gradients, landscape tints, cabin lamp light, prayer-time
//! synchronization, starfield, and shooting star cadence.

use crate::geom::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayPhase {
    Morning,
    Day,
    Dusk,
    Night,
    Midnight,
}

/// Minutes since midnight (0..1440) for the five solar/prayer anchors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkySchedule {
    pub sunrise: f32, // Terbit
    pub noon: f32,    // Dzuhur
    pub ashar: f32,   // Ashar
    pub sunset: f32,  // Maghrib
    pub isya: f32,    // Isya
}

impl Default for SkySchedule {
    fn default() -> Self {
        Self {
            sunrise: 360.0, // 06:00
            noon: 720.0,    // 12:00
            ashar: 915.0,   // 15:15
            sunset: 1080.0, // 18:00
            isya: 1155.0,   // 19:15
        }
    }
}

/// Vertical sky gradient: (position 0.0 top .. 1.0 bottom, colour), six stops.
pub type Stops = [(f32, Color); 6];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizonPalette {
    /// The sky, top to bottom.
    pub stops: Stops,
    /// Landscape tints: far mountains, middle mountains, near hill and trees.
    pub land: [Color; 3],
    pub accent: Color,
    pub ink: Color,
    pub border: Color,
    pub light: bool,
    /// How lit the cabin window is: 0.0 off .. 1.0 fully lit.
    pub cabin_light: f32,
    /// The sun rising behind the mountains (Morning only).
    pub has_sun: bool,
    /// The moon in the sky (Night only).
    pub has_moon: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CelestialIcon {
    SunriseMountain,
    Sun,
    SunsetHorizon,
    MoonStars,
    MidnightStars,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Normalized X in 0.0..1.0 across the island width.
    pub x: f32,
    /// Normalized Y in 0.0..1.0 across the sky.
    pub y: f32,
    /// Radius in DIPs (small 0.7, medium 1.15, large 1.65).
    pub radius: f32,
    /// Base opacity in 0.35..0.95.
    pub alpha: f32,
}

/// Deterministic starfield (varying sizes) for Night and Midnight.
pub const STARS: [Star; 18] = [
    Star { x: 0.07, y: 0.24, radius: 0.70, alpha: 0.50 },
    Star { x: 0.13, y: 0.68, radius: 1.15, alpha: 0.75 },
    Star { x: 0.19, y: 0.32, radius: 1.65, alpha: 0.90 },
    Star { x: 0.26, y: 0.76, radius: 0.75, alpha: 0.45 },
    Star { x: 0.33, y: 0.20, radius: 1.10, alpha: 0.65 },
    Star { x: 0.41, y: 0.58, radius: 0.70, alpha: 0.40 },
    Star { x: 0.48, y: 0.26, radius: 1.55, alpha: 0.85 },
    Star { x: 0.55, y: 0.72, radius: 0.80, alpha: 0.55 },
    Star { x: 0.62, y: 0.18, radius: 1.20, alpha: 0.70 },
    Star { x: 0.68, y: 0.64, radius: 0.70, alpha: 0.45 },
    Star { x: 0.74, y: 0.30, radius: 1.65, alpha: 0.92 },
    Star { x: 0.80, y: 0.78, radius: 1.05, alpha: 0.60 },
    Star { x: 0.85, y: 0.22, radius: 0.75, alpha: 0.50 },
    Star { x: 0.90, y: 0.56, radius: 1.30, alpha: 0.80 },
    Star { x: 0.95, y: 0.34, radius: 0.70, alpha: 0.48 },
    Star { x: 0.22, y: 0.50, radius: 0.70, alpha: 0.38 },
    Star { x: 0.58, y: 0.42, radius: 1.10, alpha: 0.62 },
    Star { x: 0.77, y: 0.48, radius: 0.75, alpha: 0.42 },
];

/// Phase edges in minutes: (morning start, day start, dusk start, night start, midnight start).
fn edges(sched: SkySchedule) -> [f32; 5] {
    [
        (sched.sunrise - 20.0).max(0.0),
        (sched.sunrise + 180.0).min(sched.noon - 60.0),
        sched.ashar.max(sched.sunset - 135.0),
        sched.isya.max(sched.sunset + 25.0),
        23.0 * 60.0 + 30.0, // 23:30
    ]
}

/// Determine the current day phase from local minutes and prayer schedule.
pub fn phase_at(minute: f32, sched: SkySchedule) -> DayPhase {
    let m = minute.rem_euclid(1440.0);
    let [morning_start, day_start, dusk_start, night_start, midnight_start] = edges(sched);
    if m >= morning_start && m < day_start {
        DayPhase::Morning
    } else if m >= day_start && m < dusk_start {
        DayPhase::Day
    } else if m >= dusk_start && m < night_start {
        DayPhase::Dusk
    } else if m >= night_start && m < midnight_start {
        DayPhase::Night
    } else {
        DayPhase::Midnight
    }
}

/// How far the morning has come, 0.0 at its start to 1.0 at its end: the
/// sun climbs from behind the mountains with it.
pub fn morning_progress(minute: f32, sched: SkySchedule) -> f32 {
    let [start, end, ..] = edges(sched);
    ((minute.rem_euclid(1440.0) - start) / (end - start).max(1.0)).clamp(0.0, 1.0)
}

pub fn icon_for(phase: DayPhase) -> CelestialIcon {
    match phase {
        DayPhase::Morning => CelestialIcon::SunriseMountain,
        DayPhase::Day => CelestialIcon::Sun,
        DayPhase::Dusk => CelestialIcon::SunsetHorizon,
        DayPhase::Night => CelestialIcon::MoonStars,
        DayPhase::Midnight => CelestialIcon::MidnightStars,
    }
}

fn stops(list: [(f32, u32); 6]) -> Stops {
    list.map(|(p, hex)| (p, Color::hex(hex)))
}

fn phase_palette(phase: DayPhase) -> HorizonPalette {
    match phase {
        // Dawn: pale blue, cream, soft yellow, peach, coral, plum.
        DayPhase::Morning => HorizonPalette {
            stops: stops([(0.0, 0x86D7EA), (0.25, 0xF4F7E6), (0.48, 0xECD97A), (0.70, 0xFAA15D), (0.86, 0xE88078), (1.0, 0x9C6582)]),
            land: [Color::hex(0xCE8FA6), Color::hex(0xB47390), Color::hex(0x8D5876)],
            accent: Color::hex(0xB84A14),
            ink: Color::hex(0x1D1A22),
            border: Color::hex(0x1D1A22).with_alpha(0.14),
            cabin_light: 0.0,
            has_sun: true,
            has_moon: false,
            light: true,
        },
        // Clear day: no sun, rich daytime sky blue for crisp white ink.
        DayPhase::Day => HorizonPalette {
            stops: stops([(0.0, 0x1D5FB8), (0.25, 0x2972CF), (0.50, 0x3989E0), (0.72, 0x4FA0EA), (0.88, 0x6AB4EF), (1.0, 0x84C6F2)]),
            land: [Color::hex(0x3D6E96), Color::hex(0x2B5478), Color::hex(0x18344E)],
            accent: Color::hex(0xFFD54F),
            ink: Color::hex(0xFFFFFF),
            border: Color::white(0.18),
            cabin_light: 0.0,
            has_sun: false,
            has_moon: false,
            light: false,
        },
        // Dusk: violet to orange; the cabin lamp comes on.
        DayPhase::Dusk => HorizonPalette {
            stops: stops([(0.0, 0x2E234E), (0.2, 0x5A3566), (0.45, 0x9B4D6A), (0.65, 0xCC6866), (0.82, 0xE87A5D), (1.0, 0xF5B06B)]),
            land: [Color::hex(0xA8566A), Color::hex(0x6B3A5E), Color::hex(0x2E2040)],
            accent: Color::hex(0xFFD07B),
            ink: Color::hex(0xFFFFFF),
            border: Color::white(0.16),
            cabin_light: 0.45,
            has_sun: false,
            has_moon: false,
            light: false,
        },
        // Night: indigo, the moon, the lamp fully lit.
        DayPhase::Night => HorizonPalette {
            stops: stops([(0.0, 0x0B1328), (0.25, 0x101C38), (0.55, 0x162647), (0.75, 0x1B2F55), (0.9, 0x1F3559), (1.0, 0x223A63)]),
            land: [Color::hex(0x26385E), Color::hex(0x19284A), Color::hex(0x0D1630)],
            accent: Color::hex(0xF6D878),
            ink: Color::hex(0xF5F7FA),
            border: Color::white(0.12),
            cabin_light: 1.0,
            has_sun: false,
            has_moon: true,
            light: false,
        },
        // Midnight: near black, stars only, the lamp low.
        DayPhase::Midnight => HorizonPalette {
            stops: stops([(0.0, 0x04060E), (0.25, 0x060A16), (0.6, 0x0A1022), (0.8, 0x0E162B), (0.9, 0x111A33), (1.0, 0x131E3A)]),
            land: [Color::hex(0x131D38), Color::hex(0x0C1428), Color::hex(0x060A18)],
            accent: Color::hex(0x9EC5FF),
            ink: Color::hex(0xE8EEF8),
            border: Color::white(0.10),
            cabin_light: 0.25,
            has_sun: false,
            has_moon: false,
            light: false,
        },
    }
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn blend_palette(a: HorizonPalette, b: HorizonPalette, t: f32) -> HorizonPalette {
    let t = t.clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t); // smoothstep
    let first = s < 0.5;
    let mut st = a.stops;
    for (i, slot) in st.iter_mut().enumerate() {
        *slot = (a.stops[i].0 + (b.stops[i].0 - a.stops[i].0) * s, lerp_color(a.stops[i].1, b.stops[i].1, s));
    }
    HorizonPalette {
        stops: st,
        land: [0, 1, 2].map(|i| lerp_color(a.land[i], b.land[i], s)),
        accent: lerp_color(a.accent, b.accent, s),
        ink: if first { a.ink } else { b.ink },
        border: lerp_color(a.border, b.border, s),
        cabin_light: a.cabin_light + (b.cabin_light - a.cabin_light) * s,
        has_sun: if first { a.has_sun } else { b.has_sun },
        has_moon: if first { a.has_moon } else { b.has_moon },
        light: if first { a.light } else { b.light },
    }
}

/// Palette for the given local minute and schedule, smoothly blended across phase edges.
pub fn palette_at(minute: f32, sched: SkySchedule) -> HorizonPalette {
    let m = minute.rem_euclid(1440.0);
    let [morning_start, day_start, dusk_start, night_start, midnight_start] = edges(sched);
    let boundaries = [
        (morning_start, DayPhase::Midnight, DayPhase::Morning),
        (day_start, DayPhase::Morning, DayPhase::Day),
        (dusk_start, DayPhase::Day, DayPhase::Dusk),
        (night_start, DayPhase::Dusk, DayPhase::Night),
        (midnight_start, DayPhase::Night, DayPhase::Midnight),
    ];

    const BLEND_HALF: f32 = 12.0;
    for (edge, from, to) in boundaries {
        let mut d = m - edge;
        if d > 720.0 {
            d -= 1440.0;
        } else if d < -720.0 {
            d += 1440.0;
        }
        if d.abs() <= BLEND_HALF {
            let t = (d + BLEND_HALF) / (2.0 * BLEND_HALF);
            return blend_palette(phase_palette(from), phase_palette(to), t);
        }
    }

    phase_palette(phase_at(m, sched))
}

/// Shooting star interval range in seconds `(min_s, max_s)` for the given phase.
/// `None` during Morning and Day.
pub fn shooting_star_interval_s(phase: DayPhase) -> Option<(f64, f64)> {
    match phase {
        DayPhase::Morning | DayPhase::Day => None,
        // Dusk: VERY RARE (15 - 25 minutes)
        DayPhase::Dusk => Some((15.0 * 60.0, 25.0 * 60.0)),
        // Night: FAIRLY FREQUENT (2 - 4 minutes)
        DayPhase::Night => Some((2.0 * 60.0, 4.0 * 60.0)),
        // Midnight: SLIGHTLY MORE FREQUENT than Night (1 - 2.5 minutes)
        DayPhase::Midnight => Some((60.0, 150.0)),
    }
}

/// Pseudo-random next interval in seconds from a deterministic seed.
pub fn next_shooting_star_delay_s(phase: DayPhase, seed: u64) -> Option<f64> {
    let (min_s, max_s) = shooting_star_interval_s(phase)?;
    let mix = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let frac = ((mix >> 32) as f64) / (u32::MAX as f64);
    Some(min_s + (max_s - min_s) * frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_follow_prayer_schedule() {
        let s = SkySchedule::default();
        assert_eq!(phase_at(6.0 * 60.0, s), DayPhase::Morning);
        assert_eq!(phase_at(11.0 * 60.0, s), DayPhase::Day);
        assert_eq!(phase_at(17.5 * 60.0, s), DayPhase::Dusk);
        assert_eq!(phase_at(20.5 * 60.0, s), DayPhase::Night);
        assert_eq!(phase_at(23.75 * 60.0, s), DayPhase::Midnight);
        assert_eq!(phase_at(2.0 * 60.0, s), DayPhase::Midnight);
    }

    #[test]
    fn morning_sky_runs_from_dawn_blue_through_cream_and_peach_to_plum() {
        let p = palette_at(7.0 * 60.0, SkySchedule::default());
        assert!(p.light);
        let [top, cream, _, peach, _, plum] = p.stops.map(|s| s.1);
        assert!(top.b > top.r + 0.2, "blue at the top");
        assert!(cream.r > 0.9 && cream.g > 0.9 && cream.b > 0.85, "cream below it");
        assert!(peach.r > 0.9 && peach.b < 0.5, "peach lower down");
        assert!(plum.r < 0.7 && plum.b > plum.g, "plum at the foot");
        assert!(p.ink.r < 0.2, "dark ink");
    }

    #[test]
    fn stops_stay_in_order_in_every_phase_and_between() {
        for m in (0..1440).step_by(7) {
            let p = palette_at(m as f32, SkySchedule::default());
            assert!(p.stops.windows(2).all(|w| w[0].0 < w[1].0), "minute {m}");
            assert_eq!((p.stops[0].0, p.stops[5].0), (0.0, 1.0), "minute {m}");
        }
    }

    #[test]
    fn the_cabin_lamp_comes_on_at_dusk_and_burns_brightest_at_night() {
        let s = SkySchedule::default();
        let at = |h: f32| palette_at(h * 60.0, s).cabin_light;
        assert_eq!((at(7.0), at(11.0)), (0.0, 0.0));
        assert!(at(17.5) > 0.0 && at(17.5) < at(20.5));
        assert_eq!(at(20.5), 1.0);
        assert!(at(0.5) > 0.0 && at(0.5) < at(20.5), "low after midnight");
    }

    #[test]
    fn the_sun_shows_only_in_the_morning_and_the_moon_only_at_night() {
        let s = SkySchedule::default();
        let p = |h: f32| palette_at(h * 60.0, s);
        assert!(p(7.0).has_sun && !p(7.0).has_moon);
        for h in [11.0, 17.5] {
            assert!(!p(h).has_sun && !p(h).has_moon, "{h}");
        }
        assert!(p(20.5).has_moon && !p(20.5).has_sun);
        assert!(!p(0.5).has_moon && !p(0.5).has_sun);
    }

    #[test]
    fn the_sun_climbs_through_the_morning() {
        let s = SkySchedule::default();
        assert_eq!(morning_progress(s.sunrise - 20.0, s), 0.0);
        assert!(morning_progress(s.sunrise + 60.0, s) > 0.3);
        assert!(morning_progress(s.sunrise + 60.0, s) < morning_progress(s.sunrise + 120.0, s));
        assert_eq!(morning_progress(s.sunrise + 300.0, s), 1.0);
    }

    #[test]
    fn shooting_star_frequency_increases_from_dusk_to_midnight() {
        assert_eq!(shooting_star_interval_s(DayPhase::Morning), None);
        assert_eq!(shooting_star_interval_s(DayPhase::Day), None);
        let dusk = shooting_star_interval_s(DayPhase::Dusk).unwrap();
        let night = shooting_star_interval_s(DayPhase::Night).unwrap();
        let mid = shooting_star_interval_s(DayPhase::Midnight).unwrap();
        assert!(dusk.0 > night.1, "dusk should be much rarer than night");
        assert!(mid.0 < night.0 && mid.1 < night.1, "midnight should be slightly more frequent than night");
    }
}
