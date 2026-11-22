// Every game tick: run the meal's state machine — choose the food slot,
// select it a tick ahead (the carried-slot packet must lead the use on the
// wire: a server reads the bite with the slot it knows), press use through
// the mod's own mapping seam (`set_mapping_down` for `key.use`), hold it
// until the stack's count drops, and hand the hotbar back.
//
// The item-use shape (`GuestUse::Item`) keeps a block under the crosshair
// from taking the click on a hosted world: the press eats, whatever the
// player is looking at. A release never fights the player's own hand: while
// their own button holds the mapping, the meal keeps its hold and waits.
// Copyright (c) 2026 hakergeniusz. MIT.
fn auto_eat_tick(game: &mut crate::game::Game<'_>) {
    // Let the button go and hand the hotbar back.
    fn release(game: &mut crate::game::Game<'_>, saved: usize) {
        let _ = game.set_mapping_down("key.use", false);
        game.guest_use = None;
        game.selected = saved;
        auto_eat::set(auto_eat::Mode::Done { saved });
    }

    // Any screen over the world stops the machine; the meal in flight lets
    // its button go first — the screen's own release already bled the press.
    if game.screen != crate::game::Screen::Playing
        || game.book.is_some()
        || game.sign_edit.is_some()
        || game.link_prompt.is_some()
        || game.command_prompt.is_some()
        || game.code_of_conduct.is_some()
    {
        if let auto_eat::Mode::Holding { saved, .. } = auto_eat::get() {
            let _ = game.set_mapping_down("key.use", false);
            game.guest_use = None;
            game.selected = saved;
        }
        auto_eat::reset();
        return;
    }
    let Some(p) = game.player.as_ref() else {
        auto_eat::reset();
        return;
    };
    let food = p.health.food.food as f32;

    match auto_eat::get() {
        auto_eat::Mode::Idle => {
            if !auto_eat::should_eat(food) || game.right || game.spectator() {
                return;
            }
            // The hotbar only, the mod's list, best first.
            let slot = (0..=8).find(|&i| {
                game.inventory[i]
                    .as_ref()
                    .is_some_and(|s| auto_eat::is_food(s.item.as_ref()))
            });
            let Some(slot) = slot else {
                return;
            };
            auto_eat::set(auto_eat::Mode::Selecting { slot, saved: game.selected });
        }
        auto_eat::Mode::Selecting { slot, saved } => {
            // The carried-slot packet goes out with this tick's input
            // handling; the press waits for the next.
            game.selected = slot;
            auto_eat::set(auto_eat::Mode::Holding { slot, saved, ticks: 0, had: -1 });
        }
        auto_eat::Mode::Holding { slot, saved, ticks, had } => {
            if ticks == 0 {
                let Some(stack) = game.inventory[slot].as_ref() else {
                    release(game, saved);
                    return;
                };
                let had = i64::from(stack.count);
                // The item-use shape makes the press the food's own use,
                // then the button holds through the input handling.
                game.guest_use =
                    Some(crate::game::GuestUse::Item(horizon_client::first_person::Hand::Main));
                let _ = game.set_mapping_down("key.use", true);
                auto_eat::set(auto_eat::Mode::Holding { slot, saved, ticks: 1, had });
            } else {
                let count = game.inventory[slot]
                    .as_ref()
                    .map(|s| i64::from(s.count))
                    .unwrap_or(had - 1);
                match auto_eat::holding_step(ticks, had, Some(count)) {
                    auto_eat::HoldDecision::Wait => {
                        auto_eat::set(auto_eat::Mode::Holding { slot, saved, ticks: ticks + 1, had })
                    }
                    // The bite landed or the hold ran out: let go before the
                    // next one starts — but never against the player's own
                    // hand. While their button holds the mapping, the meal
                    // keeps its hold and waits for theirs to end.
                    decision => {
                        if game.physical_buttons.contains(&winit::event::MouseButton::Right) {
                            auto_eat::set(auto_eat::Mode::Holding {
                                slot,
                                saved,
                                ticks: ticks + 1,
                                had,
                            });
                        } else {
                            release(game, saved);
                            let _ = decision;
                        }
                    }
                }
            }
        }
        auto_eat::Mode::Done { saved } => {
            game.guest_use = None;
            game.selected = saved;
            auto_eat::reset();
        }
    }
}
