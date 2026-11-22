//! Auto Eat, the native-route native mod: eats for you when hunger drops,
//! so the drain never wins.
//!
//! The brain is pure standard library — no game types — so it compiles and
//! tests wherever. Hunger on a server is the server's number: nothing a
//! client holds can freeze what another machine counts down. What a client
//! can do is answer it — eating is a real action the server honors wherever
//! it runs, so the mod picks food from the hotbar and holds use until the
//! bite lands, and the drain never wins. Keep-full mode is the any-server
//! nohunger on that same truth: the bar is pinned at the top by eating the
//! moment it moves. The freeze only a host can have is iron_stomach's; this
//! is the guest's answer.
//!
//! The hold is the whole trick: a bite takes about thirty ticks, and a
//! released button cancels it. The press is the game's own mapping seam for
//! mods (`set_mapping_down` for `key.use`) — the same held state a real
//! button sets, so the item-use is the player's own, packets and all, and
//! the item-use shape (`GuestUse::Item`) keeps a block under the crosshair
//! from taking the click on a hosted world: the press eats, whatever the
//! player is looking at.
//!
//! Clean-room: written for Horizon from the game's own public behavior; it
//! copies nothing from any mod.
//!
//! Copyright (c) 2026 hakergeniusz. MIT.

use core::cell::RefCell;

use mod_api::{Hooks, RegistrationContext, Registrar};

/// The mod's id in the game's settings store.
pub const ID: &str = "auto_eat";

/// The keep-full toggle's key in the settings store.
pub const KEEP_FULL: &str = "keep_full";

/// Eat once the hunger level is at or below this: the regen band (18 and
/// up) is kept fed, and the bar never walks into the starving half.
pub const EAT_AT: f32 = 15.0;

/// A full hunger bar. Keep-full mode eats the moment the bar is anywhere
/// under this — the closest a client gets to no-hunger on a server that
/// counts its own food.
pub const FULL: f32 = 20.0;

/// Ticks the use is held before the mod gives up on the bite: the plain
/// foods' thirty-two, plus margin for the tick the press lands.
pub const EAT_TICKS: u32 = 34;

/// Foods the mod will eat, best first: cooked before raw, staples before
/// treats, golden apples last — they are too good to spend on hunger alone.
pub const FOODS: [&str; 24] = [
    "minecraft:bread",
    "minecraft:cooked_beef",
    "minecraft:cooked_porkchop",
    "minecraft:cooked_chicken",
    "minecraft:cooked_mutton",
    "minecraft:cooked_rabbit",
    "minecraft:cooked_cod",
    "minecraft:cooked_salmon",
    "minecraft:baked_potato",
    "minecraft:apple",
    "minecraft:carrot",
    "minecraft:pumpkin_pie",
    "minecraft:potato",
    "minecraft:beetroot",
    "minecraft:sweet_berries",
    "minecraft:glow_berries",
    "minecraft:porkchop",
    "minecraft:beef",
    "minecraft:mutton",
    "minecraft:rabbit",
    "minecraft:cod",
    "minecraft:salmon",
    "minecraft:golden_apple",
    "minecraft:enchanted_golden_apple",
];

/// Whether this hunger level asks for food now: at or below the threshold —
/// or, in keep-full mode, any point under full. A level already being fed
/// (the caller knows a bite is in flight) never asks twice.
pub fn should_eat(food: f32, keep_full: bool) -> bool {
    if keep_full {
        food < FULL
    } else {
        food <= EAT_AT
    }
}

/// The keep-full toggle, read fresh every tick: the game's settings writes
/// never call back into a mod's cache, so there is no cache to go stale.
pub fn keep_full(get: impl FnOnce(&str) -> Option<String>) -> bool {
    get(KEEP_FULL).is_some_and(|on| on.trim() == "true")
}

/// Whether the item is food the mod will spend.
pub fn is_food(item: &str) -> bool {
    FOODS.contains(&item)
}

/// One meal, from the food slot's selection to the button's release.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Idle,
    /// The food slot is chosen; the carried-slot packet goes out this tick,
    /// so the next tick's press is read with the right slot held.
    Selecting { slot: usize, saved: usize },
    /// The button is held; the bite runs its ticks.
    Holding { slot: usize, saved: usize, ticks: u32, had: i64 },
    /// Handing the hotbar back to the player.
    Done { saved: usize },
}

thread_local! {
    static MODE: RefCell<Mode> = const { RefCell::new(Mode::Idle) };
}

/// The machine's step now.
pub fn get() -> Mode {
    MODE.with(|m| *m.borrow())
}

/// Move the machine to `mode`.
pub fn set(mode: Mode) {
    MODE.with(|m| *m.borrow_mut() = mode);
}

/// Forget the meal: the next low tick starts over.
pub fn reset() {
    set(Mode::Idle);
}

/// What a held bite does this tick.
pub enum HoldDecision {
    /// The bite landed: let go before the next one starts.
    Release,
    /// The bite never landed — the use was taken, the world refused it:
    /// let go and try again on the next low tick.
    GiveUp,
    /// Keep holding, one tick older.
    Wait,
}

/// A held bite's tick: the count the slot held when the press landed
/// (`had`) against what it holds now — an emptied slot reads as the bite's
/// own success — and the hold's age against the longest bite.
pub fn holding_step(ticks: u32, had: i64, count: Option<i64>) -> HoldDecision {
    if count.unwrap_or(had - 1) < had {
        HoldDecision::Release
    } else if ticks + 1 >= EAT_TICKS {
        HoldDecision::GiveUp
    } else {
        HoldDecision::Wait
    }
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// One toggle: keep-full mode, the any-server nohunger. The plain meal
    /// still has no switch — installing it is asking for it, and the hunger
    /// bar decides the rest.
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        use mod_api::SettingSpec;
        let _ = registrar.register_setting(SettingSpec::toggle(
            KEEP_FULL,
            "auto_eat.setting.keep_full",
            false,
        ));
    }

    /// Implemented as a no-op so the game always calls the
    /// `native_client_tick_pre` dispatch the tick mixin lives in: the game
    /// skips dispatches no mod listens to, and the mixin must run.
    fn client_tick_pre(&self, _: &mut dyn mod_api::ClientHost, _: &mut mod_api::ClientTickPreEvent) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunger_at_the_threshold_asks_for_food() {
        assert!(should_eat(15.0, false));
        assert!(should_eat(0.0, false));
        assert!(should_eat(EAT_AT, false));
    }

    #[test]
    fn fed_hunger_does_not() {
        assert!(!should_eat(15.01, false));
        assert!(!should_eat(20.0, false));
    }

    #[test]
    fn keep_full_asks_at_any_point_under_full() {
        assert!(should_eat(19.0, true));
        assert!(should_eat(0.0, true));
        assert!(!should_eat(FULL, true));
        assert!(!should_eat(20.0, true));
    }

    #[test]
    fn the_toggle_defaults_off_and_reads_true() {
        assert!(!keep_full(|_| None), "an absent toggle is off");
        assert!(keep_full(|k| (k == KEEP_FULL).then(|| "true".into())));
        assert!(!keep_full(|k| (k == KEEP_FULL).then(|| "false".into())));
    }

    #[test]
    fn the_staples_are_food() {
        for item in [
            "minecraft:bread",
            "minecraft:cooked_beef",
            "minecraft:golden_apple",
        ] {
            assert!(is_food(item), "{item}");
        }
    }

    #[test]
    fn the_traps_are_not() {
        // Raw chicken poisons, rotten flesh starves, poison is not food.
        for item in [
            "minecraft:rotten_flesh",
            "minecraft:spider_eye",
            "minecraft:poisonous_potato",
            "minecraft:pufferfish",
            "minecraft:chicken",
        ] {
            assert!(!is_food(item), "{item}");
        }
    }

    #[test]
    fn cooked_staples_rank_before_raw_and_golden() {
        let rank = |item: &str| FOODS.iter().position(|&f| f == item).unwrap();
        assert!(rank("minecraft:bread") < rank("minecraft:apple"));
        assert!(rank("minecraft:apple") < rank("minecraft:beef"));
        assert!(rank("minecraft:salmon") < rank("minecraft:golden_apple"));
        assert_eq!(rank("minecraft:enchanted_golden_apple"), FOODS.len() - 1);
    }

    #[test]
    fn the_hold_outlasts_a_bite() {
        // The plain foods' bite is thirty-two ticks; the hold must cover it
        // or the release cancels the eat mid-chew.
        assert!(EAT_TICKS >= 33);
    }

    #[test]
    fn a_held_bite_releases_when_the_count_drops() {
        // The bite landed: let go.
        assert!(matches!(holding_step(10, 3, Some(2)), HoldDecision::Release));
        // The slot went empty on the last bite: the same.
        assert!(matches!(holding_step(10, 1, None), HoldDecision::Release));
        // Still chewing inside the hold.
        assert!(matches!(holding_step(10, 3, Some(3)), HoldDecision::Wait));
        // The bite never landed: give up at the hold's end.
        assert!(matches!(holding_step(EAT_TICKS - 1, 3, Some(3)), HoldDecision::GiveUp));
    }

    #[test]
    fn the_state_round_trips() {
        assert_eq!(get(), Mode::Idle, "the machine starts idle");
        set(Mode::Selecting { slot: 5, saved: 2 });
        assert_eq!(get(), Mode::Selecting { slot: 5, saved: 2 });
        reset();
        assert_eq!(get(), Mode::Idle);
    }
}
