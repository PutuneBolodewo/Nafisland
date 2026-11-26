//! Nafisland's bull mascot: the mark's dark head, curved white horns, floppy
//! ears, peach snout, and expressive eyes that follow what the app is doing
//! (resting, music, reading, writing, the night).

pub mod geometry;
pub mod mark;
pub mod svg;

pub use geometry::{Affine, Element, Path, PathError, Point};

use geometry::{max, min, trim};
use std::f64::consts::PI;
use std::sync::OnceLock;

// The colours, as 0xRRGGBB (what `geom::Color::hex` takes).

/// Theme accent kept for the warm mascot theme.
pub const ORANGE: u32 = 0xF0892E;
/// Deep accent kept for text/fold contrast in the mascot theme.
pub const FOLD_ORANGE: u32 = 0xC9581A;
/// Bull head and ear fill.
pub const BULL_HEAD: u32 = 0x1C1C21;
/// Subtle rim stroke so dark head/ears stay separated on black islands.
pub const BULL_STROKE: u32 = 0x32323A;
/// Curved white horns.
pub const BULL_HORN: u32 = 0xF8FAFC;
/// Horn outline.
pub const BULL_HORN_STROKE: u32 = 0x111115;
/// Warm peach snout.
pub const BULL_SNOUT: u32 = 0xE89B6C;
/// Pupils and nostrils.
pub const INK: u32 = 0x171717;
/// Love's hearts.
pub const PINK: u32 = 0xFF4F8B;
/// The logged tick.
pub const GREEN: u32 = 0x1E9E4A;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mood {
    /// Idle: one blink, a tiny ear flick.
    Oyen,
    /// The ear springs up to wave, the head tilts.
    Hello,
    /// Eyes shut in a smile, bouncing.
    Happy,
    /// Heart eyes that beat.
    Love,
    /// One eye winks, with a sparkle.
    Wink,
    /// Big eyes with glints, the ear jumps.
    Wow,
    /// Half-lidded, the eyes slide to one side and back.
    Sly,
    /// Eyes up, three thinking dots.
    Think,
    /// A glance left, then right.
    Look,
    /// Drooping eyes, the head sinks (and stays down).
    Sad,
    /// Resting eyes, z's over the ear, slow breaths.
    Sleep,
    /// Logged: the ear flips up like a page corner, and a tick draws itself.
    Tercatat,
    /// Nodding off, then waking with a start.
    Drowsy,
    /// Petted: a smile, the ear lifts, the head hums.
    Purr,
    /// Writing: eyes down, following `Input::pencil`.
    Nulis,
    /// Reading: the eyes run along a line and hop back.
    Baca,
    /// Breathing along with `Input::breath`.
    Napas,
}

impl Mood {
    /// Every mood, in the Swift port's order.
    pub const ALL: [Mood; 17] = [
        Mood::Oyen,
        Mood::Hello,
        Mood::Happy,
        Mood::Love,
        Mood::Wink,
        Mood::Wow,
        Mood::Sly,
        Mood::Think,
        Mood::Look,
        Mood::Sad,
        Mood::Sleep,
        Mood::Tercatat,
        Mood::Drowsy,
        Mood::Purr,
        Mood::Nulis,
        Mood::Baca,
        Mood::Napas,
    ];

    /// The mood's name in the app and in the Swift port.
    pub const fn name(self) -> &'static str {
        match self {
            Mood::Oyen => "oyen",
            Mood::Hello => "hello",
            Mood::Happy => "happy",
            Mood::Love => "love",
            Mood::Wink => "wink",
            Mood::Wow => "wow",
            Mood::Sly => "sly",
            Mood::Think => "think",
            Mood::Look => "look",
            Mood::Sad => "sad",
            Mood::Sleep => "sleep",
            Mood::Tercatat => "tercatat",
            Mood::Drowsy => "drowsy",
            Mood::Purr => "purr",
            Mood::Nulis => "nulis",
            Mood::Baca => "baca",
            Mood::Napas => "napas",
        }
    }

    pub fn from_name(name: &str) -> Option<Mood> {
        Mood::ALL.into_iter().find(|m| m.name() == name)
    }

    /// One loop of the mood, in seconds.
    pub const fn loop_secs(self) -> f64 {
        match self {
            Mood::Oyen => 2.4,
            Mood::Hello => 1.6,
            Mood::Happy => 1.2,
            Mood::Love => 1.2,
            Mood::Wink => 1.6,
            Mood::Wow => 1.6,
            Mood::Sly => 2.0,
            Mood::Think => 2.0,
            Mood::Look => 2.4,
            Mood::Sad => 2.0,
            Mood::Sleep => 2.4,
            Mood::Tercatat => 1.6,
            Mood::Drowsy => 3.2,
            Mood::Purr => 1.4,
            Mood::Nulis => 3.2,
            Mood::Baca => 3.0,
            Mood::Napas => 9.0,
        }
    }
}

/// What the Fokus moods follow: where the pencil is on the line (-1..=1) and
/// how full the breath is (0..=1). Other moods ignore it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub pencil: f64,
    pub breath: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Ink,
    Pink,
    Green,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// An eye: ink on the head.
    Eye,
    /// A glint on an eye.
    Highlight,
    /// Drawn on the head, next to the eyes (a wink's sparkle).
    OnHead,
    /// Drawn above the head, clear of it (thinking dots, the tick).
    Above,
    /// Drawn over the upright ear (sleep's z's): fine in colour, lost in a one-colour cut.
    OverEar,
}

/// One mark on the face, in head space (draw it under `Frame::head`).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub path: Path,
    /// A stroke width in mark units (round caps and joins), or `None` for a fill.
    pub stroke: Option<f64>,
    pub paint: Paint,
    pub place: Place,
    pub opacity: f64,
    pub clip: Option<Path>,
}

impl Shape {
    /// An opaque ink fill on the head, unclipped: an eye. Other shapes start here.
    pub fn new(path: Path) -> Shape {
        Shape {
            path,
            stroke: None,
            paint: Paint::Ink,
            place: Place::Eye,
            opacity: 1.0,
            clip: None,
        }
    }
}

/// The folded ear's tip at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct EarTip {
    pub path: Path,
    /// True while the tip stands up and shows its orange front; false while it
    /// lies folded, showing the darker back.
    pub faces_up: bool,
    /// An upright tip sits behind the head; a folded one lies on the ear.
    pub behind_head: bool,
}

/// Oyen at one moment. Draw the tip (if `behind_head`), the head, the tip
/// (otherwise), then the eyes and extras, all under `head`.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Bounce, tilt, sink and breath for the whole head, about the chin.
    pub head: Affine,
    /// 1 is the logo (folded flat), 0 is upright.
    pub fold: f64,
    pub tip: EarTip,
    pub eyes: Vec<Shape>,
    pub extras: Vec<Shape>,
}

/// The head's outline, the mark's own.
pub fn head_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::HEAD))
}

/// The left ear at rest (`fold = 1.0`).
pub fn fold_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::FOLD))
}

/// The right ear's outline.
pub fn right_ear_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::RIGHT_EAR))
}

/// Both curved white horns.
pub fn horns_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::HORNS))
}

/// The warm peach snout.
pub fn snout_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::SNOUT))
}

/// The two dark nostrils inside the snout.
pub fn nostrils_path() -> &'static Path {
    static PATH: OnceLock<Path> = OnceLock::new();
    PATH.get_or_init(|| mark_path(mark::NOSTRILS))
}

fn mark_path(data: &str) -> Path {
    // Generated data, and the tests check it parses.
    Path::from_svg(data).unwrap_or_default()
}

/// The bull mascot `t` seconds into `mood`'s loop. `eye_scale` grows the eyes
/// slightly at 32 px and below.
///
/// `t` is not wrapped: to play a mood over and over, pass `t % mood.loop_secs()`
/// (except Sad, which sinks once and stays down).
pub fn frame(mood: Mood, t: f64, input: Input, eye_scale: f64) -> Frame {
    let pose = pose(mood, t, input, EYE_R * eye_scale);
    let fold = pose.fold;
    let sy = 1.0 - 0.035 * pose.bounce + 0.012 * pose.breathe;
    let sx = 1.0 + 0.03 * pose.bounce;
    let lift = -5.0 * pose.bounce + pose.sink;
    let head = Affine::translation(0.0, -CHIN)
        .then(Affine::scale(sx, sy))
        .then(Affine::rotation(pose.tilt))
        .then(Affine::translation(0.0, CHIN + lift));
    Frame {
        head,
        fold,
        tip: tip(fold),
        eyes: pose.eyes,
        extras: pose.extras,
    }
}

// The mark's numbers.

/// The chin, below which nothing of the head shows in a peek: the point the
/// head bounces and tilts about.
pub const CHIN: f64 = 38.52;
const EYE_X: f64 = 10.5;
const EYE_Y: f64 = -11.5;
const EYE_R: f64 = 6.2;
const EAR_PIVOT: Point = Point::new(-24.0, -15.0);

/// The floppy left ear: lifts upward from its attachment point as `fold` drops from 1 to 0.
fn tip(fold: f64) -> EarTip {
    if fold >= 0.999 {
        return EarTip {
            path: fold_path().clone(),
            faces_up: false,
            behind_head: true,
        };
    }
    let lift_deg = (1.0 - fold) * 28.0;
    EarTip {
        path: fold_path().applying(Affine::rotation_about(lift_deg, EAR_PIVOT)),
        faces_up: fold < 0.5,
        behind_head: true,
    }
}

// Easing.

fn clamp(x: f64, lower: f64, upper: f64) -> f64 {
    min(upper, max(lower, x))
}

fn clamp01(x: f64) -> f64 {
    clamp(x, 0.0, 1.0)
}

fn ease(x: f64) -> f64 {
    let v = clamp01(x);
    v * v * (3.0 - 2.0 * v)
}

fn ease_out(x: f64) -> f64 {
    1.0 - (1.0 - clamp01(x)).powf(3.0)
}

fn back(x: f64, s: f64) -> f64 {
    let v = clamp01(x) - 1.0;
    v * v * ((s + 1.0) * v + s) + 1.0
}

fn pulse(t: f64, at: f64, length: f64) -> f64 {
    if t >= at && t < at + length {
        (PI * (t - at) / length).sin()
    } else {
        0.0
    }
}

/// How open the eyes are (1 open, about 0 shut) for a blink starting at `at`.
// Not `contains`: it would read a NaN time as open eyes, where the Swift port gives NaN.
#[allow(clippy::manual_range_contains)]
fn blink(t: f64, at: f64, length: f64) -> f64 {
    let x = (t - at) / length;
    if x < 0.0 || x > 1.0 {
        1.0
    } else {
        1.0 - (PI * x).sin() * 0.94
    }
}

// Eye shapes.

/// Signature half-lidded white sclera D-shape + dark pupil.
fn bull_eye(cx: f64, _cy: f64, r: f64, open: f64, look_x: f64, look_y: f64) -> Vec<Shape> {
    let scale = (r / EYE_R).min(1.12);
    let top = -18.5;
    let hw = 8.2 * scale;
    let h = max(0.7, 14.0 * open * scale);
    let sclera = Path::new(vec![
        Element::Move(Point::new(cx - hw, top)),
        Element::Line(Point::new(cx + hw, top)),
        Element::Quad(Point::new(cx + hw, top + h), Point::new(cx, top + h)),
        Element::Quad(Point::new(cx - hw, top + h), Point::new(cx - hw, top)),
        Element::Close,
    ]);
    let mut out = vec![Shape {
        paint: Paint::White,
        place: Place::Eye,
        ..Shape::new(sclera)
    }];
    if open > 0.25 {
        let px = cx - cx.signum() * 2.3 + look_x;
        let py = top + min(h * 0.42, 4.3 * scale) + look_y;
        let pr = 3.3 * scale;
        out.push(Shape {
            paint: Paint::Ink,
            place: Place::Highlight,
            ..Shape::new(Path::ellipse(px, py, pr, min(pr, h * 0.36)))
        });
    }
    out
}

fn dot(cx: f64, cy: f64, r: f64, open: f64) -> Shape {
    Shape {
        paint: Paint::White,
        ..Shape::new(Path::ellipse(
            cx,
            cy + r * (1.0 - open) * 0.15,
            r,
            max(0.35, r * open),
        ))
    }
}

/// A shut eye as a curve: smiling (`up`) or resting (white over dark head).
fn arc(cx: f64, cy: f64, r: f64, up: bool, width: f64) -> Shape {
    let y0 = cy + if up { r * 0.3 } else { -r * 0.2 };
    let yc = cy + if up { -r * 0.95 } else { r * 0.8 };
    let path = Path::new(vec![
        Element::Move(Point::new(cx - r, y0)),
        Element::Quad(Point::new(cx, yc), Point::new(cx + r, y0)),
    ]);
    Shape {
        stroke: Some(r * width),
        paint: Paint::White,
        ..Shape::new(path)
    }
}

fn heart(cx: f64, cy: f64, s: f64) -> Shape {
    let points: Vec<Point> = (0..48)
        .map(|k| {
            let t = 2.0 * PI * k as f64 / 48.0;
            let x = 16.0 * t.sin().powf(3.0);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            Point::new(cx + x / 17.0 * s, cy - (y + 2.0) / 17.0 * s)
        })
        .collect();
    Shape {
        paint: Paint::Pink,
        ..Shape::new(Path::polygon(&points))
    }
}

fn sparkle(cx: f64, cy: f64, s: f64) -> Shape {
    let q = s * 0.22;
    let p = Point::new;
    let path = Path::new(vec![
        Element::Move(p(cx, cy - s)),
        Element::Quad(p(cx + q, cy - q), p(cx + s, cy)),
        Element::Quad(p(cx + q, cy + q), p(cx, cy + s)),
        Element::Quad(p(cx - q, cy + q), p(cx - s, cy)),
        Element::Quad(p(cx - q, cy - q), p(cx, cy - s)),
        Element::Close,
    ]);
    Shape {
        place: Place::OnHead,
        ..Shape::new(path)
    }
}

fn zee(x: f64, y: f64, s: f64, opacity: f64) -> Shape {
    let points = [
        Point::new(x, y),
        Point::new(x + s, y),
        Point::new(x, y + s),
        Point::new(x + s, y + s),
    ];
    Shape {
        stroke: Some(s * 0.28),
        place: Place::OverEar,
        opacity,
        ..Shape::new(Path::polyline(&points))
    }
}

/// A tick that draws itself as `k` runs 0..=1.
fn check(x: f64, y: f64, s: f64, k: f64) -> Option<Shape> {
    let points = [
        Point::new(x - s, y),
        Point::new(x - s * 0.3, y + s * 0.7),
        Point::new(x + s * 1.2, y - s * 0.8),
    ];
    let drawn = trim(&points, 2.2 * s * 1.6 * k);
    if drawn.len() < 2 {
        return None;
    }
    Some(Shape {
        stroke: Some(s * 0.42),
        paint: Paint::Green,
        place: Place::Above,
        ..Shape::new(Path::polyline(&drawn))
    })
}

// Moods.

struct Pose {
    eyes: Vec<Shape>,
    extras: Vec<Shape>,
    fold: f64,
    tilt: f64,
    bounce: f64,
    sink: f64,
    breathe: f64,
}

impl Pose {
    fn new(eyes: Vec<Shape>) -> Pose {
        Pose {
            eyes,
            extras: Vec::new(),
            fold: 1.0,
            tilt: 0.0,
            bounce: 0.0,
            sink: 0.0,
            breathe: 0.0,
        }
    }
}

fn pose(mood: Mood, t: f64, input: Input, r: f64) -> Pose {
    let left = -EYE_X;
    let right = EYE_X;
    let y = EYE_Y;
    let pair = |eye: &dyn Fn(f64) -> Shape| vec![eye(left), eye(right)];
    let smiling = || pair(&|x| arc(x, y, r, true, 0.5));
    let resting = || pair(&|x| arc(x, y, r * 0.95, false, 0.42));
    match mood {
        Mood::Hello => {
            let up = if t < 0.75 {
                back(t / 0.45, 1.2)
            } else {
                1.0 - ease((t - 0.75) / 0.45)
            };
            Pose {
                fold: 1.0 - 0.92 * clamp01(up),
                tilt: -4.0 * (PI * clamp01(t / 1.2)).sin(),
                ..Pose::new(smiling())
            }
        }
        Mood::Happy => Pose {
            bounce: (PI * t / 0.6).sin().abs(),
            ..Pose::new(smiling())
        },
        Mood::Love => {
            let k = 1.0 + 0.12 * max(0.0, (2.0 * PI * t / 0.6).sin());
            Pose {
                bounce: 0.35 * (PI * t / 0.6).sin().abs(),
                ..Pose::new(pair(&|x| heart(x, y + 1.0, r * 1.3 * k)))
            }
        }
        Mood::Wink => {
            let winking = t > 0.25 && t < 1.15;
            let k = ease((t - 0.3) / 0.25) * (1.0 - ease((t - 1.0) / 0.3));
            let mut eyes = bull_eye(left, y, r, 1.0, 0.0, 0.0);
            if winking {
                eyes.push(arc(right, y, r, true, 0.5));
            } else {
                eyes.extend(bull_eye(right, y, r, blink(t, 0.12, 0.14), 0.0, 0.0));
            }
            let extras = if k > 0.0 {
                vec![sparkle(right + r * 2.0, y - r * 2.2, 5.5 * k)]
            } else {
                Vec::new()
            };
            Pose {
                extras,
                tilt: 3.0 * k,
                ..Pose::new(eyes)
            }
        }
        Mood::Wow => {
            let s = 1.0 + 0.28 * back(t / 0.35, 2.2);
            let mut eyes = pair(&|x| dot(x, y, r * s, 1.0));
            for x in [left, right] {
                eyes.push(Shape {
                    paint: Paint::Ink,
                    place: Place::Highlight,
                    ..Shape::new(Path::ellipse(x, y, r * 0.42 * s, r * 0.42 * s))
                });
            }
            Pose {
                fold: 1.0 - 0.5 * pulse(t, 0.05, 0.5),
                bounce: 0.6 * pulse(t, 0.0, 0.35),
                ..Pose::new(eyes)
            }
        }
        Mood::Sly => {
            // Half-lidded, the pupils slide to one side and back.
            let slide = ease(t / 0.4) * (1.0 - ease((t - 1.4) / 0.4));
            let mut eyes = bull_eye(left, y, r, 0.85, 2.8 * slide, 0.0);
            eyes.extend(bull_eye(right, y, r, 0.85, 2.8 * slide, 0.0));
            Pose {
                tilt: -2.0 * slide,
                ..Pose::new(eyes)
            }
        }
        Mood::Think => {
            let dots = (0..3)
                .filter_map(|i| {
                    let i = i as f64;
                    let k = ease((t - 0.25 - i * 0.28) / 0.2) * (1.0 - ease((t - 1.6) / 0.3));
                    let radius = 3.4 * k;
                    (radius > 0.0).then(|| Shape {
                        place: Place::Above,
                        ..Shape::new(Path::ellipse(-9.0 + i * 9.0, -56.0, radius, radius))
                    })
                })
                .collect();
            let open = blink(t, 1.75, 0.14);
            let mut eyes = bull_eye(left, y, r, open, 1.5, -1.8);
            eyes.extend(bull_eye(right, y, r, open, 1.5, -1.8));
            Pose {
                extras: dots,
                ..Pose::new(eyes)
            }
        }
        Mood::Look => {
            let x = if t < 0.8 {
                -3.0 * ease(t / 0.25)
            } else if t < 1.6 {
                -3.0 + 6.0 * ease((t - 0.8) / 0.3)
            } else {
                3.0 - 3.0 * ease((t - 1.6) / 0.3)
            };
            let open = blink(t, 2.1, 0.14);
            let mut eyes = bull_eye(left, y, r, open, x, 0.0);
            eyes.extend(bull_eye(right, y, r, open, x, 0.0));
            Pose {
                tilt: x * 0.5,
                ..Pose::new(eyes)
            }
        }
        Mood::Sad => {
            let k = ease(t / 0.5);
            let drop = |cx: f64, degrees: f64| {
                let half = Path::lower_half_ellipse(cx, y - 1.0, r, r * 0.85);
                Shape {
                    paint: Paint::White,
                    ..Shape::new(half.applying(Affine::rotation_about(degrees * k, Point::new(cx, y))))
                }
            };
            Pose {
                sink: 2.0 * k,
                ..Pose::new(vec![drop(left, -14.0), drop(right, 14.0)])
            }
        }
        Mood::Sleep => {
            let zees = (0..2)
                .map(|i| {
                    let n = i as f64;
                    let k = (t / 2.4 + n * 0.5) % 1.0;
                    zee(
                        24.0 + n * 10.0 + k * 5.0,
                        -52.0 - n * 10.0 - k * 8.0,
                        9.0 - n * 2.5,
                        (PI * k).sin(),
                    )
                })
                .collect();
            Pose {
                extras: zees,
                breathe: (2.0 * PI * t / 2.4).sin(),
                ..Pose::new(resting())
            }
        }
        Mood::Drowsy => {
            // Nodding off: the lids sink and the head dips, then he wakes with a start.
            let k = if t < 2.2 {
                ease(t / 2.2)
            } else {
                1.0 - ease((t - 2.2) / 0.25)
            };
            let lid = 0.72 - 0.58 * k;
            let mut eyes = bull_eye(left, y, r, lid, 0.0, 0.8 * k);
            eyes.extend(bull_eye(right, y, r, lid, 0.0, 0.8 * k));
            Pose {
                tilt: -4.0 * k,
                sink: 3.0 * k,
                ..Pose::new(eyes)
            }
        }
        Mood::Purr => Pose {
            // Petted: eyes shut in a smile, the left ear lifts a little, and the head hums.
            fold: 1.0 - 0.3 * ease(t / 0.2) * (1.0 - ease((t - 1.1) / 0.3)),
            tilt: 1.1 * (2.0 * PI * 18.0 * t).sin() * (1.0 - ease((t - 0.9) / 0.5)),
            ..Pose::new(smiling())
        },
        Mood::Nulis => {
            // Writing: pupils follow the cursor/pencil along the line; a slow blink now and then.
            let gx = clamp(input.pencil, -1.0, 1.0);
            let open = blink(t, 2.6, 0.16);
            let mut eyes = bull_eye(left, y, r, open, 2.5 * gx, 2.0);
            eyes.extend(bull_eye(right, y, r, open, 2.5 * gx, 2.0));
            Pose {
                tilt: -1.6 * gx,
                ..Pose::new(eyes)
            }
        }
        Mood::Baca => {
            // Reading: the pupils run along a line, then hop back to the start of the next.
            let k = (t % 3.0) / 3.0;
            let x = if k < 0.82 {
                -2.8 + 5.6 * ease(k / 0.82)
            } else {
                2.8 - 5.6 * ease((k - 0.82) / 0.18)
            };
            let open = blink(t, 1.3, 0.14);
            let mut eyes = bull_eye(left, y, r, open, x, 1.4);
            eyes.extend(bull_eye(right, y, r, open, x, 1.4));
            Pose::new(eyes)
        }
        Mood::Napas => {
            // Breathing with the person: eyes shut, the left ear lifting a little
            // at the top of the breath.
            let breath = clamp01(input.breath);
            Pose {
                fold: 1.0 - 0.2 * breath,
                breathe: 2.0 * breath - 1.0,
                ..Pose::new(resting())
            }
        }
        Mood::Tercatat => {
            // Logged: the ear flips up and snaps down, and a tick draws itself between the horns.
            let up = if t < 0.35 {
                ease_out(t / 0.35)
            } else {
                1.0 - back((t - 0.35) / 0.3, 2.4)
            };
            let k = ease((t - 0.62) / 0.3) * (1.0 - ease((t - 1.35) / 0.25));
            Pose {
                extras: check(0.0, -54.0, 6.5, k).into_iter().collect(),
                fold: 1.0 - 0.85 * clamp(up, 0.0, 1.2),
                bounce: 0.7 * pulse(t, 0.62, 0.3),
                ..Pose::new(smiling())
            }
        }
        Mood::Oyen => {
            // Idle: deadpan half-lidded bull eyes, one blink, a tiny ear flick.
            let open = blink(t, 1.7, 0.16);
            let mut eyes = bull_eye(left, y, r, open, 0.0, 0.0);
            eyes.extend(bull_eye(right, y, r, open, 0.0, 0.0));
            Pose {
                fold: 1.0 - 0.12 * pulse(t, 0.6, 0.35),
                ..Pose::new(eyes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye_height(frame: &Frame) -> f64 {
        let ys: Vec<f64> = frame.eyes[0].path.points().iter().map(|p| p.y).collect();
        ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min)
    }

    fn bounds(points: &[Point]) -> (f64, f64, f64, f64) {
        points.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
        )
    }

    /// The first eye's centre, to a thousandth.
    fn eye_centre(frame: &Frame) -> (f64, f64) {
        let (x0, y0, x1, y1) = bounds(&frame.eyes[0].path.points());
        (
            ((x0 + x1) / 2.0 * 1000.0).round() / 1000.0,
            ((y0 + y1) / 2.0 * 1000.0).round() / 1000.0,
        )
    }

    fn at(mood: Mood, t: f64) -> Frame {
        frame(mood, t, Input::default(), 1.0)
    }

    #[test]
    fn mark_outlines_parse_and_keep_the_logos_size() {
        assert!(!head_path().is_empty());
        assert!(!fold_path().is_empty());
        assert!(!right_ear_path().is_empty());
        assert!(!horns_path().is_empty());
        assert!(!snout_path().is_empty());
        assert!(!nostrils_path().is_empty());
        for data in [
            mark::HEAD,
            mark::FOLD,
            mark::RIGHT_EAR,
            mark::HORNS,
            mark::SNOUT,
            mark::NOSTRILS,
            mark::EYES,
            mark::EYES_SMALL,
        ] {
            assert!(Path::from_svg(data).is_ok());
        }
        // Horns + ears span ~104 wide (`MARK_W`); chin sits at 38.52.
        let (hx0, _, hx1, _) = bounds(&horns_path().points());
        assert!(hx0 < -45.0 && hx1 > 45.0);
        let (_, _, _, y1) = bounds(&head_path().points());
        assert!((y1 - CHIN).abs() < 0.01);
    }

    #[test]
    fn the_logos_eyes_are_the_faces_resting_eyes() {
        let logo = Path::from_svg(mark::EYES).unwrap();
        let left: Vec<Point> = logo.points().into_iter().filter(|p| p.x < 0.0).collect();
        let (x0, y0, x1, y1) = bounds(&left);
        let (fx0, fy0, fx1, fy1) = bounds(
            &frame(Mood::Oyen, 0.0, Input::default(), 1.0).eyes[0]
                .path
                .points(),
        );
        for (a, b) in [(x0, fx0), (y0, fy0), (x1, fx1), (y1, fy1)] {
            assert!((a - b).abs() < 0.02, "logo {a} vs face {b}");
        }
    }

    #[test]
    fn idle_shows_the_logo_blinks_once_and_flicks_the_ear() {
        let rest = at(Mood::Oyen, 0.0);
        assert_eq!(rest.fold, 1.0);
        assert_eq!(&rest.tip.path, fold_path());
        assert_eq!(rest.head, Affine::IDENTITY);
        assert!(eye_height(&rest) > 13.0);

        let shut = at(Mood::Oyen, 1.7 + 0.08);
        assert!(eye_height(&shut) < 1.5);

        let flick = at(Mood::Oyen, 0.6 + 0.175);
        assert!((flick.fold - 0.88).abs() < 0.001);
        assert_ne!(&flick.tip.path, fold_path());
    }

    #[test]
    fn every_mood_loops_without_a_jump() {
        // Sad is the one mood that sinks and stays there.
        for mood in Mood::ALL.into_iter().filter(|&m| m != Mood::Sad) {
            let first = at(mood, 0.0);
            let last = at(mood, mood.loop_secs() - 1.0 / 240.0);
            assert!(!first.eyes.is_empty(), "{mood:?}");
            assert!(
                (first.head.ty - last.head.ty).abs() < 1.0,
                "{mood:?} jumps at the loop"
            );
            assert!(
                (first.fold - last.fold).abs() < 0.08,
                "{mood:?} ear jumps at the loop"
            );
        }
    }

    #[test]
    fn frames_are_a_pure_function_of_time() {
        for mood in Mood::ALL {
            assert_eq!(at(mood, 0.37), at(mood, 0.37));
        }
    }

    #[test]
    fn moods_have_names_and_loops() {
        assert_eq!(Mood::ALL.len(), 17);
        for mood in Mood::ALL {
            assert_eq!(Mood::from_name(mood.name()), Some(mood));
            assert!(mood.loop_secs() > 1.0);
        }
        assert_eq!(Mood::from_name("angry"), None);
        assert_eq!(Mood::Napas.loop_secs(), 9.0);
    }

    #[test]
    fn a_logged_moment_lifts_the_ear_and_draws_a_tick() {
        let up = at(Mood::Tercatat, 0.35);
        assert!(up.fold < 0.2);
        assert!(up.tip.faces_up);
        assert!(up.tip.behind_head);
        let ticked = at(Mood::Tercatat, 1.0);
        assert_eq!(ticked.extras.len(), 1);
        assert_eq!(ticked.extras[0].paint, Paint::Green);
        assert!(at(Mood::Tercatat, 0.1).extras.is_empty());
    }

    #[test]
    fn larger_eyes_for_small_sizes_keep_their_place() {
        let rest = frame(Mood::Oyen, 0.0, Input::default(), 1.0);
        let small = frame(Mood::Oyen, 0.0, Input::default(), 1.08);
        assert!(eye_height(&small) > eye_height(&rest));
        assert_eq!(eye_centre(&small).0, eye_centre(&rest).0);
    }

    #[test]
    fn the_fokus_moods_follow_their_input() {
        let ahead = frame(
            Mood::Nulis,
            0.5,
            Input {
                pencil: 1.0,
                breath: 0.0,
            },
            1.0,
        );
        let behind = frame(
            Mood::Nulis,
            0.5,
            Input {
                pencil: -1.0,
                breath: 0.0,
            },
            1.0,
        );
        let pupil_x = |f: &Frame| {
            let (x0, _, x1, _) = bounds(&f.eyes[1].path.points());
            (x0 + x1) / 2.0
        };
        assert!(pupil_x(&ahead) > pupil_x(&behind));
        // The pencil is clamped to the line.
        assert_eq!(
            frame(
                Mood::Nulis,
                0.5,
                Input {
                    pencil: 3.0,
                    breath: 0.0
                },
                1.0
            ),
            ahead
        );
        let full = frame(
            Mood::Napas,
            0.5,
            Input {
                pencil: 0.0,
                breath: 1.0,
            },
            1.0,
        );
        let empty = frame(Mood::Napas, 0.5, Input::default(), 1.0);
        assert!((full.fold - 0.8).abs() < 1e-12);
        assert_eq!(empty.fold, 1.0);
        // A full breath stretches the head up from the chin.
        assert!(full.head.d > empty.head.d);
    }

    #[test]
    fn the_head_squashes_and_turns_about_the_chin() {
        // Only the bounce's lift (up to 5) and the sink (up to 3) move the chin.
        for mood in Mood::ALL {
            for i in 0..=48 {
                let f = at(mood, mood.loop_secs() * i as f64 / 48.0);
                let chin = f.head.apply(Point::new(0.0, CHIN));
                assert!(
                    chin.x.abs() < 1e-9 && (chin.y - CHIN).abs() <= 5.0 + 1e-9,
                    "{mood:?} at {i}: {chin:?}"
                );
            }
        }
    }
}
