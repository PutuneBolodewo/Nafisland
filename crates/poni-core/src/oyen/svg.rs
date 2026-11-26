//! Writes a frame of Oyen's face as SVG, the way OyenKit's `OyenSVG` does, so
//! frames can be previewed in a browser and compared with the Swift port's
//! text for text.

use super::geometry::{max, number};
use super::{
    fold_path, head_path, horns_path, nostrils_path, right_ear_path, snout_path, Affine, Frame,
    Paint, Place, Shape, BULL_HEAD, BULL_HORN, BULL_HORN_STROKE, BULL_SNOUT, BULL_STROKE, GREEN,
    INK, PINK,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style<'a> {
    /// The app's Oyen: orange head, darker folded tip, ink eyes. `ink` colours
    /// what floats around the head.
    Colour { ink: &'a str },
    /// One colour, for a menu bar or a tray: a solid head with the eyes as
    /// holes, and the folded tip set off by a cut of `gap` mark units. A system
    /// tints it, so `color` only matters for previews.
    Template { color: &'a str, gap: f64 },
}

/// The square of mark units a face is drawn in, and how large that square is shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas {
    pub x: f64,
    pub y: f64,
    pub side: f64,
    pub points: f64,
}

impl Canvas {
    /// The app's own framing: the head fills about 80% of the width and sits
    /// low, so faces can peek over an edge.
    pub const fn face(points: f64) -> Canvas {
        Canvas {
            x: -64.0,
            y: -84.0,
            side: 128.0,
            points,
        }
    }

    /// A tight square for a menu bar (18 points there): the head fills 96% of
    /// the width, with room above for the ear and a bounce.
    pub const fn menu_bar(points: f64) -> Canvas {
        Canvas {
            x: -54.0,
            y: -66.0,
            side: 108.0,
            points,
        }
    }
}

/// `0xRRGGBB` the way CSS and the Swift port write it: "#F0892E".
pub fn css(rgb: u32) -> String {
    format!("#{rgb:06X}")
}

/// A whole `<svg>` element. `id` must be unique among the SVGs on one page.
pub fn document(frame: &Frame, style: Style, canvas: Canvas, id: &str) -> String {
    let side = number(canvas.side);
    let size = number(canvas.points);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"{} {} {side} {side}\">{}</svg>",
        number(canvas.x),
        number(canvas.y),
        body(frame, style, canvas, id, None)
    )
}

/// The drawing without its `<svg>` wrapper, in mark units. With `symbols`, the
/// head and the logo's folded tip are drawn as references to
/// [`shared_defs`], so a page with many frames carries those outlines once.
pub fn body(
    frame: &Frame,
    style: Style,
    canvas: Canvas,
    id: &str,
    symbols: Option<&str>,
) -> String {
    let parts = Parts::new(frame, symbols);
    match style {
        Style::Colour { ink } => colour_body(frame, &parts, ink, id),
        Style::Template { color, gap } => template_body(frame, &parts, color, gap, canvas, id),
    }
}

/// The outlines `body` refers to when given `symbols`. Put this once in a page's `<defs>`.
pub fn shared_defs(symbols: &str) -> String {
    format!(
        "<path id=\"{symbols}-head\" d=\"{}\"/><path id=\"{symbols}-fold\" d=\"{}\"/>",
        head_path().to_svg(),
        fold_path().to_svg()
    )
}

/// The head and the ear tip of one frame, as elements that still need their paint.
struct Parts {
    head: String,
    tip: String,
}

impl Parts {
    fn new(frame: &Frame, symbols: Option<&str>) -> Parts {
        match symbols {
            Some(symbols) => Parts {
                head: format!("<use href=\"#{symbols}-head\""),
                tip: if &frame.tip.path == fold_path() {
                    format!("<use href=\"#{symbols}-fold\"")
                } else {
                    format!("<path d=\"{}\"", frame.tip.path.to_svg())
                },
            },
            None => Parts {
                head: format!("<path d=\"{}\"", head_path().to_svg()),
                tip: format!("<path d=\"{}\"", frame.tip.path.to_svg()),
            },
        }
    }
}

fn shapes(frame: &Frame) -> impl Iterator<Item = (usize, &Shape)> {
    frame.eyes.iter().chain(&frame.extras).enumerate()
}

fn colour_body(frame: &Frame, parts: &Parts, ink: &str, id: &str) -> String {
    let mut defs = String::new();
    let mut out = String::new();
    let head_fill = css(BULL_HEAD);
    let head_stroke = css(BULL_STROKE);
    let horn_fill = css(BULL_HORN);
    let horn_stroke = css(BULL_HORN_STROKE);
    let snout_fill = css(BULL_SNOUT);
    let ink_fill = css(INK);

    out += &format!(
        "<path d=\"{}\" fill=\"{head_fill}\" stroke=\"{head_stroke}\" stroke-width=\"1.2\" stroke-linejoin=\"round\"/>",
        right_ear_path().to_svg()
    );
    out += &format!(
        "{} fill=\"{head_fill}\" stroke=\"{head_stroke}\" stroke-width=\"1.2\" stroke-linejoin=\"round\"/>",
        parts.tip
    );
    out += &format!(
        "{} fill=\"{head_fill}\" stroke=\"{head_stroke}\" stroke-width=\"1.2\" stroke-linejoin=\"round\"/>",
        parts.head
    );
    out += &format!(
        "<path d=\"{}\" fill=\"{horn_fill}\" stroke=\"{horn_stroke}\" stroke-width=\"1.4\" stroke-linejoin=\"round\"/>",
        horns_path().to_svg()
    );
    out += &format!(
        "<path d=\"{}\" fill=\"{snout_fill}\"/>",
        snout_path().to_svg()
    );
    out += &format!(
        "<path d=\"{}\" fill=\"{ink_fill}\"/>",
        nostrils_path().to_svg()
    );

    for (index, shape) in shapes(frame) {
        let color = match shape.paint {
            Paint::Ink if matches!(shape.place, Place::Eye | Place::Highlight) => css(INK),
            Paint::Ink => ink.to_string(),
            Paint::Pink => css(PINK),
            Paint::Green => css(GREEN),
            Paint::White => "#FFFFFF".to_string(),
        };
        out += &element(shape, &color, &format!("{id}c{index}"), &mut defs);
    }
    let wrapped = format!("<g transform=\"{}\">{out}</g>", matrix(frame.head));
    if defs.is_empty() {
        wrapped
    } else {
        format!("<defs>{defs}</defs>{wrapped}")
    }
}

fn template_body(
    frame: &Frame,
    parts: &Parts,
    color: &str,
    _gap: f64,
    canvas: Canvas,
    id: &str,
) -> String {
    let mut defs = String::new();
    let mut mask = String::new();
    let mut floating = String::new();
    mask += &format!(
        "<path d=\"{}\" fill=\"#fff\"/>",
        right_ear_path().to_svg()
    );
    mask += &format!("{} fill=\"#fff\"/>", parts.tip);
    mask += &format!("{} fill=\"#fff\"/>", parts.head);
    mask += &format!("<path d=\"{}\" fill=\"#fff\"/>", horns_path().to_svg());
    mask += &format!("<path d=\"{}\" fill=\"#000\"/>", nostrils_path().to_svg());
    for (index, shape) in shapes(frame) {
        let clip_id = format!("{id}c{index}");
        match shape.place {
            Place::Eye | Place::OnHead => mask += &element(shape, "#000", &clip_id, &mut defs),
            Place::Highlight => mask += &element(shape, "#fff", &clip_id, &mut defs),
            Place::Above => floating += &element(shape, color, &clip_id, &mut defs),
            Place::OverEar => {} // would only merge into the ear
        }
    }
    // The face is a plain square of the colour, cut out by the mask.
    let transform = matrix(frame.head);
    let side = number(canvas.side);
    let square = format!(
        "x=\"{}\" y=\"{}\" width=\"{side}\" height=\"{side}\"",
        number(canvas.x),
        number(canvas.y)
    );
    defs += &format!("<mask id=\"{id}m\" maskUnits=\"userSpaceOnUse\" {square}><g transform=\"{transform}\">{mask}</g></mask>");
    let mut out =
        format!("<defs>{defs}</defs><rect {square} fill=\"{color}\" mask=\"url(#{id}m)\"/>");
    if !floating.is_empty() {
        out += &format!("<g transform=\"{transform}\">{floating}</g>");
    }
    out
}

fn element(shape: &Shape, color: &str, clip_id: &str, defs: &mut String) -> String {
    let mut attributes = format!("d=\"{}\"", shape.path.to_svg());
    match shape.stroke {
        Some(stroke) => {
            attributes += &format!(
                " fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
                number(stroke)
            )
        }
        None => attributes += &format!(" fill=\"{color}\""),
    }
    if shape.opacity < 1.0 {
        attributes += &format!(" opacity=\"{}\"", number(max(0.0, shape.opacity)));
    }
    let Some(clip) = &shape.clip else {
        return format!("<path {attributes}/>");
    };
    *defs += &format!(
        "<clipPath id=\"{clip_id}\"><path d=\"{}\"/></clipPath>",
        clip.to_svg()
    );
    format!("<g clip-path=\"url(#{clip_id})\"><path {attributes}/></g>")
}

fn matrix(m: Affine) -> String {
    format!(
        "matrix({:.5} {:.5} {:.5} {:.5} {:.5} {:.5})",
        m.a, m.b, m.c, m.d, m.tx, m.ty
    )
}

#[cfg(test)]
mod tests {
    use super::super::{frame, Input, Mood};
    use super::*;

    fn idle() -> Frame {
        frame(Mood::Oyen, 0.0, Input::default(), 1.0)
    }

    #[test]
    fn the_one_colour_cut_makes_holes_of_the_eyes_and_never_paints_them() {
        let svg = document(
            &idle(),
            Style::Template {
                color: "#000",
                gap: 2.6,
            },
            Canvas::menu_bar(18.0),
            "t",
        );
        assert!(svg.contains("<mask id=\"tm\""));
        assert!(svg.contains("mask=\"url(#tm)\""));
        assert!(!svg.contains(&css(BULL_HEAD)));
        assert!(!svg.contains(&css(BULL_SNOUT)));
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"18\" height=\"18\" viewBox=\"-54 -66 108 108\">"));
    }

    #[test]
    fn the_colour_cut_uses_the_brands_three_colours() {
        let svg = document(
            &idle(),
            Style::Colour { ink: "#FBFBFB" },
            Canvas::face(96.0),
            "oyen",
        );
        assert!(svg.contains(&format!("fill=\"{}\"", css(BULL_HEAD))));
        assert!(svg.contains(&format!("fill=\"{}\"", css(BULL_HORN))));
        assert!(svg.contains(&format!("fill=\"{}\"", css(BULL_SNOUT))));
        assert!(svg.contains(&format!("fill=\"{}\"", css(INK))));
        assert!(!svg.contains("<mask"));
    }

    #[test]
    fn shared_outlines_replace_the_long_paths() {
        let f = idle();
        let style = Style::Colour { ink: "#000" };
        let whole = body(&f, style, Canvas::menu_bar(18.0), "oyen", None);
        let shared = body(&f, style, Canvas::menu_bar(18.0), "oyen", Some("o"));
        assert!(shared.contains("<use href=\"#o-head\""));
        assert!(shared.contains("<use href=\"#o-fold\""));
        assert!(shared.len() < whole.len());
        assert!(shared_defs("o").contains("id=\"o-head\""));
    }

    #[test]
    fn clips_and_fading_shapes_carry_their_attributes() {
        let sleep = frame(Mood::Sleep, 0.8, Input::default(), 1.0);
        let svg = body(
            &sleep,
            Style::Colour { ink: "#ABCDEF" },
            Canvas::face(64.0),
            "z",
            Some("o"),
        );
        assert!(svg.contains(" opacity=\"0."));
        assert!(svg.contains("stroke=\"#ABCDEF\""));
        // Z's over the ear vanish in one colour.
        let svg = body(
            &sleep,
            Style::Template {
                color: "#000",
                gap: 2.6,
            },
            Canvas::face(64.0),
            "z",
            Some("o"),
        );
        assert!(!svg.contains("opacity"));
    }

    #[test]
    fn colours_are_written_as_css_hex() {
        assert_eq!(css(BULL_HEAD), "#1C1C21");
        assert_eq!(css(BULL_HORN), "#F8FAFC");
        assert_eq!(css(BULL_SNOUT), "#E89B6C");
        assert_eq!(css(INK), "#171717");
        assert_eq!(css(PINK), "#FF4F8B");
        assert_eq!(css(GREEN), "#1E9E4A");
        assert_eq!(css(0x00000A), "#00000A");
    }

    #[test]
    fn matrices_have_five_decimals_and_keep_a_minus_zero() {
        assert_eq!(
            matrix(Affine::IDENTITY),
            "matrix(1.00000 0.00000 0.00000 1.00000 0.00000 0.00000)"
        );
        assert_eq!(
            matrix(Affine::rotation(0.0)),
            "matrix(1.00000 0.00000 -0.00000 1.00000 0.00000 0.00000)"
        );
    }
}
