// Copyright © 2019 Bart Massey
// [This program is licensed under the GPL version 3 or later.]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

/*!
*One Way Out* (OWO) is a multiplayer telnet game. The game
*state is in a [Game] struct that is initialized at game
*start. [Game] implements the play loop in its [play()]
*function.

At startup, a new [Game] is created. Then the
connection listener is started. As players connect, they are
placed into the game and given a dedicated client proxy.

OWO is a "turn-based" game, with the provision that the
world updates when *any* player acts. This "time only moves
when *someone* moves" model is a bit unusual. During the
turn, the game simulation steps forward a step, regardless
of the amount of elapsed time since the last turn.
*/

mod conn;
mod field;
mod game;
mod mob;
mod player;

pub use conn::*;
pub use field::*;
pub use game::*;
pub use mob::*;
pub use player::*;

use std::collections::HashMap;
pub use std::io::{self, Write};
pub use std::sync::{Arc, Mutex};

use fastrand::u64 as random_u64;

pub fn random(r: u64) -> u64 {
    random_u64(0..r)
}

/// The "health" and other player characteristics are common
/// across all players in the instance; the individual
/// players are proxy avatars that have only their own
/// position and actions. This is the maximum health that
/// the player can have.
pub const MAX_HEALTH: u64 = 100;

/// The game is won by traversing this distance (in tiles)
/// to the exit door.
pub const DOOR_POSN: usize = 500;

fn main() {
    Connection::listen(Mutex::new(Game::default()));
}
