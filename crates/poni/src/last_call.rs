//! Caffeine's reminder before the day's last call, the app's side (the
//! numbers are `poni_core::caffeine`'s): while the add-on and its "Remind me
//! before last call" switch are on, the island shows one notice a day,
//! `caffeine::REMIND_BEFORE` (30) minutes before the usual drink's last call
//! (`caffeine::reminder_at`), in the words of `caffeine::reminder_text`. A
//! click on it opens the Caffeine tab. Both apps keep one [`Reminder`] and
//! look at it in every tick, after Prayer times' moments.
//!
//! Costs at rest: nothing of its own. It rides the wakes the app has anyway
//! (each frame, and the minute's tick the clock wakes for), and the app's
//! next wake includes the reminder's minute while it is ahead
//! ([`Reminder::wake`]), so it comes on time while the island is asleep: no
//! thread, timer or poll of its own. A look is a pass over the kept drinks
//! ([`basis`]); the reminder's minute is worked out again
//! (`caffeine::reminder_at`) only when the drinks, the settings or the page's
//! day change. When the day changes it also lets drinks older than two
//! weeks go. Off, it does nothing at all.
//!
//! What it keeps is in memory only. The reminder stays due until last call
//! itself: a locked screen, a full-screen app or an open island at its
//! minute only delay it, and a PC that wakes before last call still shows
//! it. After last call it is left alone, and so is a restart then.

use crate::icons::Icon;
use crate::pages::PageKind;
use crate::state::{AppState, Notice, NoticeAction};
use poni_core::caffeine::{day_start, hhmm, local_minute, reminder_at, reminder_text, LocalMinute, Log, Prefs, REMIND_BEFORE};
use poni_core::geom::palette;

/// How long the notice stays (seconds): it comes once a day, so it stays
/// long enough for someone who looked away.
pub const NOTICE_S: f64 = 30.0;
/// The wake comes this long after the reminder's minute begins (seconds), so
/// the clock it finds already reads that minute.
const WAKE_AFTER_S: f64 = 0.05;

/// What the app keeps of the reminder.
#[derive(Debug, Default)]
pub struct Reminder {
    /// Today's reminder, its minute, while it is ahead, or due and waiting
    /// for the island to be free.
    ahead: Option<LocalMinute>,
    /// What `ahead` was worked out from ([`basis`]).
    basis: Option<u64>,
    /// The page's day (its 06:00) whose reminder showed: one a day.
    shown: Option<LocalMinute>,
    /// The page's day last looked at, to let old drinks go when it changes.
    day: Option<LocalMinute>,
}

/// The add-on and its switch are on.
fn on(st: &AppState) -> bool {
    st.caffeine_on() && st.settings.caffeine.remind
}

impl Reminder {
    /// Each tick: once the reminder's minute has come, puts it on the island
    /// while the island is `free` for it (someone is at the computer, and
    /// the island is closed and on screen) and shows no other notice, or
    /// only one that stays (a new version's, which comes back after). True
    /// when it went up.
    pub fn tick(&mut self, st: &mut AppState, free: bool) -> bool {
        if !on(st) {
            self.ahead = None;
            self.basis = None;
            return false;
        }
        let now = local_minute(&st.local);
        if self.day != Some(day_start(now)) {
            self.day = Some(day_start(now));
            if st.store.caffeine.tidy(now) {
                st.save();
            }
        }
        let (log, prefs) = (&st.store.caffeine, &st.settings.caffeine);
        let basis = basis(log, prefs, day_start(now));
        if self.basis != Some(basis) {
            self.basis = Some(basis);
            self.ahead = reminder_at(log, prefs, now).filter(|at| self.shown != Some(day_start(*at)));
        }
        let Some(at) = self.ahead.filter(|at| *at <= now) else { return false };
        if now - at >= REMIND_BEFORE {
            crate::pinfo!("caffeine: the reminder at {} left alone, last call passed", hhmm(at));
            self.ahead = None;
            return false;
        }
        if !free || st.notice.as_ref().is_some_and(|n| !n.stays()) {
            return false;
        }
        self.ahead = None;
        let Some(n) = notice(st) else { return false };
        crate::pinfo!("caffeine: the reminder at {} (last call {})", hhmm(at), hhmm(at + REMIND_BEFORE));
        self.shown = Some(day_start(at));
        st.notice = Some(n);
        true
    }

    /// When the app wakes for the reminder (mono seconds), just after its
    /// minute begins, while it is ahead; None while the add-on or its switch
    /// is off.
    pub fn wake(&self, st: &AppState) -> Option<f64> {
        let at = self.ahead.filter(|_| on(st))?;
        let secs = (at - local_minute(&st.local)) as f64 * 60.0 - f64::from(st.local.second);
        (secs > 0.0).then_some(st.now + secs + WAKE_AFTER_S)
    }
}

/// The reminder's notice for the day on `st`'s clock: Caffeine's cup in
/// orange, the words of `caffeine::reminder_text`, and a click opens the
/// Caffeine tab. None when the usual drink has no last call.
pub fn notice(st: &AppState) -> Option<Notice> {
    let (title, subtitle) = reminder_text(&st.store.caffeine, &st.settings.caffeine, local_minute(&st.local))?;
    Some(Notice {
        icon: Icon::Coffee,
        color: palette::ORANGE,
        title,
        title_dim: String::new(),
        subtitle,
        until: st.now + NOTICE_S,
        action: Some(NoticeAction::OpenPage(PageKind::Caffeine)),
        inline: None,
        reveal: None,
    })
}

/// What the reminder's minute is worked out from, as one number: the page's
/// day, the settings it uses and every drink kept (the usual drink comes
/// from them). One pass over the drinks, cheap enough for every tick, where
/// `Today::of` goes through every minute of the day.
fn basis(log: &Log, prefs: &Prefs, day: LocalMinute) -> u64 {
    // FNV-1a over whole words: it only has to change when one of them does.
    fn mix(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let mut h = [day as u64, prefs.bedtime.into(), prefs.half_life.to_bits().into(), prefs.bed_limit_mg.into()].into_iter().fold(0xcbf2_9ce4_8422_2325, mix);
    for d in &log.drinks {
        let what = d.kind as u64 | u64::from(d.amount) << 8 | d.bean.map_or(0xff, |b| b as u64) << 16;
        h = [d.at as u64, what, d.mg.to_bits()].into_iter().fold(h, mix);
        h = d.name.bytes().fold(h, |h, b| mix(h, b.into()));
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use poni_core::caffeine::{clock, Bean, Drink, Kind, Today, DAY};
    use poni_core::datetime::{civil_from_days, LocalDateTime};
    use poni_core::settings::Settings;
    use poni_core::store::Store;

    /// Midnight on 10 Nov 2026.
    fn day0() -> LocalMinute {
        local_minute(&LocalDateTime { year: 2026, month: 11, day: 10, hour: 0, minute: 0, second: 0 })
    }

    fn at(h: i64, m: i64) -> LocalMinute {
        day0() + h * 60 + m
    }

    /// Both of the app's clocks at minute `m`, `sec` seconds into it: the
    /// wall clock, and mono seconds counted from `day0`.
    fn set_clock(st: &mut AppState, m: LocalMinute, sec: u8) {
        let (year, month, day) = civil_from_days(m.div_euclid(DAY));
        let (hour, minute) = clock(m);
        st.local = LocalDateTime { year, month, day, hour, minute, second: sec };
        st.now = ((m - day0()) * 60) as f64 + f64::from(sec);
    }

    /// Caffeine on, with a 2-shot Americano at 08:30: its last call for a
    /// 23:00 bed is about 12:07, so the reminder is about 11:37.
    fn state() -> AppState {
        let mut s = Settings::default();
        s.set_addon(poni_core::addons::CAFFEINE.id, true);
        let mut st = AppState::new(s, Store::default());
        st.store.caffeine.add(Drink { id: 0, at: at(8, 30), kind: Kind::Americano, amount: 2, bean: Some(Bean::Arabica), mg: 150.0, name: String::new() });
        set_clock(&mut st, at(9, 0), 0);
        st
    }

    /// The day's reminder as the page works it out.
    fn reminder(st: &AppState) -> LocalMinute {
        Today::of(&st.store.caffeine, &st.settings.caffeine, local_minute(&st.local)).last_call.expect("a last call") - REMIND_BEFORE
    }

    /// Ticks from `from` to `to` (not included), a minute apart, with the
    /// island free or not; true when the notice went up on any of them.
    fn run(r: &mut Reminder, st: &mut AppState, from: LocalMinute, to: LocalMinute, free: bool) -> bool {
        let mut up = false;
        for m in from..to {
            set_clock(st, m, 20);
            up |= r.tick(st, free);
        }
        up
    }

    #[test]
    fn it_shows_once_at_its_minute() {
        let mut st = state();
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true), "caffeine add-on is removed");
    }

    #[test]
    fn nothing_at_all_while_the_add_on_or_its_switch_is_off() {
        let mut st = state();
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true));
        assert!(st.notice.is_none() && r.wake(&st).is_none());
    }

    #[test]
    fn late_it_still_shows_until_last_call_after_it_never() {
        let mut st = state();
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true) && r.wake(&st).is_none());
    }

    #[test]
    fn a_new_day_lets_drinks_older_than_two_weeks_go() {
        let mut st = state();
        let old = at(8, 0) - 20 * DAY;
        st.store.caffeine.drinks.insert(0, Drink { id: 99, at: old, kind: Kind::Tea, amount: 1, bean: None, mg: 47.0, name: String::new() });
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true), "caffeine add-on is removed");
    }

    #[test]
    fn it_waits_for_another_notice_but_not_for_a_new_versions() {
        let mut st = state();
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true));
    }

    #[test]
    fn it_follows_the_drinks_and_the_settings() {
        let mut st = state();
        let mut r = Reminder::default();
        assert!(!r.tick(&mut st, true));
    }

    #[test]
    fn the_app_wakes_for_its_minute_while_it_is_ahead() {
        let st = state();
        let r = Reminder::default();
        assert_eq!(r.wake(&st), None);
    }

    #[test]
    fn no_last_call_no_reminder() {
        // Green tea all day never leaves 35 mg at bedtime.
        let mut st = state();
        st.store.caffeine = Log::default();
        for h in [8, 11, 14] {
            st.store.caffeine.add(Drink { id: 0, at: at(h, 0), kind: Kind::GreenTea, amount: 1, bean: None, mg: 28.0, name: String::new() });
        }
        let mut r = Reminder::default();
        assert!(!run(&mut r, &mut st, at(9, 0), at(23, 0), true));
        assert!(notice(&st).is_none() && r.wake(&st).is_none());
    }

    #[test]
    fn the_basis_changes_with_what_the_minute_comes_from() {
        let st = state();
        let (log, prefs) = (&st.store.caffeine, &st.settings.caffeine);
        let b = basis(log, prefs, at(6, 0));
        assert_eq!(b, basis(log, prefs, at(6, 0)));
        assert_ne!(b, basis(log, prefs, at(30, 0)), "another day");
        assert_ne!(b, basis(log, &Prefs { half_life: 6.0, ..prefs.clone() }, at(6, 0)));
        assert_ne!(b, basis(log, &Prefs { bed_limit_mg: 40, ..prefs.clone() }, at(6, 0)));
        let mut moved = log.clone();
        let id = moved.drinks[0].id;
        moved.move_to(id, at(8, 45));
        assert_ne!(b, basis(&moved, prefs, at(6, 0)), "a drink moved");
        let mut named = log.clone();
        named.drinks[0].name = "Kopi".into();
        assert_ne!(b, basis(&named, prefs, at(6, 0)), "a name");
        // What the page's warnings use, not the reminder: the same.
        assert_eq!(b, basis(log, &Prefs { day_limit_mg: 300, bean: Bean::Robusta, ..prefs.clone() }, at(6, 0)));
    }
}
