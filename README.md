# auto_eat

A Horizon native mod: eats for you when hunger drops, so the drain never
wins.

Hunger on a server is the server's number — nothing a client holds can
freeze what another machine counts down. But eating is a real action any
server honors: the mod picks food from the hotbar, selects its slot a tick
ahead so the server reads the bite with the right slot held, presses use
through the game's own input mapping, and holds it until the stack's count
drops — then hands the hotbar back. On any server, with or without mods.

Keeps the regen band fed (it eats at hunger 15 and below), prefers cooked
food over raw and golden apples over nothing, and never fights your own
hand: while your use button is down, it waits.

Keep cooked food in the hotbar and it does the rest.

MIT. Clean-room: written for Horizon from the game's own public behavior.
