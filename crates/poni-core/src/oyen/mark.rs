//! The Nafisland bull mascot's outlines, as SVG path data in mark units.
//!
//! Width spans -52..52 (`MARK_W = 104`), chin at `y = 38.52`, matching the
//! framing used across the island, tabs, and SVG exports.

/// The tall tapered head outline.
pub const HEAD: &str = "M-22 -35 Q0 -37 22 -35 Q25 -18 29 2 Q34 24 31 33 Q24 38.52 0 38.52 Q-24 38.52 -31 33 Q-34 24 -29 2 Q-25 -18 -22 -35 Z";

/// The floppy left ear at rest (animates upward when `fold < 1.0`).
pub const FOLD: &str = "M-24 -19 Q-41 -20 -51 -11 Q-53 -8 -48 -6 Q-37 -5 -25 -12 Z";

/// The floppy right ear.
pub const RIGHT_EAR: &str = "M24 -19 Q41 -20 51 -11 Q53 -8 48 -6 Q37 -5 25 -12 Z";

/// Both curved horns sweeping outward and upward from the top of the head.
pub const HORNS: &str = "M-21 -34 Q-31 -32 -37 -40 Q-43 -48 -42 -64 Q-53 -48 -50 -33 Q-46 -20 -24 -21 Z M21 -34 Q31 -32 37 -40 Q43 -48 42 -64 Q53 -48 50 -33 Q46 -20 24 -21 Z";

/// The warm peach snout / muzzle.
pub const SNOUT: &str = "M0 6 Q23 6 24 21 Q24 34 0 35 Q-24 34 -24 21 Q-23 6 0 6 Z";

/// Two dark nostrils inside the snout.
pub const NOSTRILS: &str = "M-9.5 14.5 Q-6.5 13.5 -6 17.5 Q-7 20 -9.5 18.5 Q-11 16.5 -9.5 14.5 Z M9.5 14.5 Q11 16.5 9.5 18.5 Q7 20 6 17.5 Q6.5 13.5 9.5 14.5 Z";

/// Resting eye sclera outlines (half-lidded D-shapes).
pub const EYES: &str = "M-18.7 -18.5 L-2.3 -18.5 Q-2.3 -4.5 -10.5 -4.5 Q-18.7 -4.5 -18.7 -18.5 Z M2.3 -18.5 L18.7 -18.5 Q18.7 -4.5 10.5 -4.5 Q2.3 -4.5 2.3 -18.5 Z";

/// Small-size eye sclera outlines (used at 32 px and below).
pub const EYES_SMALL: &str = "M-19.5 -18.5 L-1.5 -18.5 Q-1.5 -3.5 -10.5 -3.5 Q-19.5 -3.5 -19.5 -18.5 Z M1.5 -18.5 L19.5 -18.5 Q19.5 -3.5 10.5 -3.5 Q1.5 -3.5 1.5 -18.5 Z";
