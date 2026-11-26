// Copyright © 2019 Bart Massey
// [This program is licensed under the GPL version 3 or later.]
// Please see the file LICENSE in the source
// distribution of this software for license terms.

//! The overall game state for the running game. This
//! includes the shared player stats, the monster records
//! the playfield, etc. Also methods to update the state.

use crate::*;

/// Game state.
pub struct Game {
    /// Player ID for *next* client to enter.
    pub next_player_id: u64,
    /// Player ID map.
    pub players: HashMap<u64, Player>,
    /// Playfield.
    pub field: Field,
    /// Clock: turns passed.
    pub turns: u64,
    /// MOB ID map.
    pub monsters: HashMap<u64, Mob>,
    /// Player ID for *next* MOB to enter.
    pub next_monster_id: u64,
    /// Shared player health.
    pub health: u64,
}

pub trait GameHandle {
    fn with_game<T, F>(&self, action: F) -> T where F: FnMut(&mut Game) -> T;
    fn play(&self, remote: Connection);
    fn turn(&self);
}

impl GameHandle for Mutex<Game> {
    /// Execute some game action code under the state lock.
    fn with_game<T, F>(&self, mut action: F) -> T where F: FnMut(&mut Game) -> T {
        let mut state = self.lock().unwrap();
        action(&mut state)
    }

    /// The main play loop for a single player client avatar.
    ///
    /// Note that the "player" is an avatar of the single
    /// notional entity here. A player has a unique position
    /// and can take unique actions.  A player is associated
    /// with a unique remote connection.
    fn play(&self, mut remote: Connection) {
        // Start the player as far to the left as feasible,
        // then set up their view.
        let player_id = self.with_game(|game| {
            let player_id = game.next_player_id;
            game.next_player_id = player_id + 1;
            let mut player = Player::new(player_id, remote.width);
            let mut posn = player.posn;
            while game.field.has_object(posn) {
                posn += 1;
            }
            player.posn = posn;
            game.players.insert(player_id, player);
            game.field[posn].object = Some(Object::Player(player_id));
            game.field.establish(posn + Player::MARGIN);
            player_id
        });

        // Read and execute player actions.
        loop {
            let optcmd = match remote.read() {
                Ok(cmd) => cmd,
                Err(e) => {
                    eprintln!("net read error: {}", e);
                    return;
                }
            };
            if let Some(cmd) = optcmd {
                let cmd = cmd.trim();
                match cmd {
                    "h" | "l" => self.with_game(|game| {
                        // Movement command.
                        let player = game.players.get_mut(&player_id).unwrap();
                        let off = match cmd {
                            "h" => -1,
                            "l" => 1,
                            _ => panic!("internal error: bad cmd"),
                        };

                        //  Act on command.
                        if let Some(new_posn) = offset(player.posn, off) {
                            let clear = match game.field[new_posn].top() {
                                Some(Object::Monster(id)) => {
                                    // Combat.
                                    let mob = game.monsters.get_mut(id).unwrap();
                                    if !mob.hit() {
                                        // Killed the monster.
                                        game.monsters.remove(id);
                                        game.field[new_posn].object = None;
                                        true
                                    } else {
                                        false
                                    }
                                }
                                // Movement blocked.
                                Some(_) => false,
                                // Just move.
                                _ => true,
                            };

                            // If successfully moved, set up position and view.
                            if clear {
                                player.adjust_display(off);
                                let posn = player.posn;
                                game.field.establish(new_posn + Player::MARGIN);
                                player.posn = new_posn;
                                game.field[posn].object = None;
                                game.field[new_posn].object = Some(Object::Player(player.id));
                            }
                        }
                    }),
                    // Rest.
                    "." => self.with_game(|game| game.rest()),
                    // Quit the game.
                    "q" => {
                        self.with_game(|game| {
                            let player = &game.players[&player_id];
                            game.field[player.posn].object = None;
                            game.players.remove(&player_id).unwrap();
                            if game.players.is_empty() {
                                writeln!(remote, "\rno more players, new game    \r").unwrap();
                                *game = Game::default();
                                return;
                            }
                            writeln!(remote, "\ryou quit, how sad    \r").unwrap();
                        });
                        return;
                    }
                    // Ignore random commands.
                    _ => continue,
                }

                // Run the rest of the game turn.
                self.turn();
            }

            // Check for player dead or game win.
            let done = self.with_game(|game| {
                if game.health == 0 {
                    // Only one player, and they died.
                    writeln!(remote, "\rboard wipe, game over    \r").unwrap();
                    game.players.remove(&player_id).unwrap();
                    if game.players.is_empty() {
                        *game = Game::default();
                    }
                    return true;
                }
                let player = game.players.get(&player_id).unwrap();
                if player.posn >= DOOR_POSN {
                    // This player avatar escaped the game.
                    game.players.remove(&player_id).unwrap();
                    if game.players.is_empty() {
                        // Every player avatar escaped the game.
                        writeln!(remote, "\ry'all escaped, win!    \r").unwrap();
                        *game = Game::default();
                        return true;
                    }
                    writeln!(remote, "\ryou escaped, one down    \r").unwrap();
                    return true;
                }

                // Render player scene.
                //
                // Absolute position of player in field coords.
                let posn = player.posn;
                // Absolute position of left edge in field coords.
                let left = posn - player.left;
                // Width of display in characters.
                let width = player.width as usize;
                // Absolute position of right edge in field coords.
                let right = left + width;
                // Render player board view.
                let mut board = game.field.render(left, right);

                // Render player icon.
                assert_eq!(board.len(), width);
                for (_, p) in game.players.iter() {
                    if p.posn >= left && p.posn < right {
                        board[p.posn - left] = '@';
                    }
                }

                // Set up the render and send it.
                let render: String = board.into_iter().collect();
                let posn = player.posn;
                if posn != player.posn_cache || render != player.display_cache {
                    write!(remote, "\r{}", render).unwrap();
                    write!(remote, "\r{}", &render[0..posn - left]).unwrap();
                    let player = game.players.get_mut(&player_id).unwrap();
                    player.display_cache = render;
                    player.posn_cache = posn;
                }
                false
            });

            // Player and maybe game over.
            if done {
                return;
            }
        }
    }

    /// Update non-player game state for a new tick.
    fn turn(&self) {
        self.with_game(|game| {
            // Bump the clock.
            game.turns += 1;

            // Spawn MOBs as needed.
            let len = game.field.len();
            let nmonsters = game.monsters.len();
            if nmonsters < len / 20 && nmonsters < game.turns as usize / 5 {
                let posn = random(len as u64) as usize;
                if !game.field.has_object(posn) {
                    let id = game.next_monster_id;
                    game.next_monster_id += 1;
                    game.field[posn].object = Some(Object::Monster(id));
                    game.monsters.insert(id, Mob::new(id, posn));
                }
            }

            // Resolve MOB attacks.
            for (_, p) in game.players.iter() {
                for &posn in &[p.posn - 1, p.posn + 1] {
                    if game.field.has_monster(posn) && game.health > 0 {
                        game.health -= 1;
                    }
                }
            }

            // Move MOBs.
            let mids: Vec<u64> = game.monsters.keys().copied().collect();
            for mid in mids {
                let m = game.monsters.get(&mid).unwrap();
                let posn = m.posn;
                let new_posn = m.get_move(&game);
                if new_posn == posn {
                    continue;
                }
                if game.field[new_posn].top().is_some() {
                    continue;
                }
                assert_eq!(game.field[posn].top(), Some(&Object::Monster(m.id)));
                game.field[posn].object = None;
                game.field[new_posn].object = Some(Object::Monster(m.id));

                let m = game.monsters.get_mut(&mid).unwrap();
                m.posn = new_posn;
            }
        });
    }
}

impl Game {
    /// Player rest actions heal the player.
    pub fn rest(&mut self) {
        let health = self.health;
        self.health = MAX_HEALTH.min(health + random(2));
    }
}

impl Default for Game {
    fn default() -> Self {
        Game {
            next_player_id: 1,
            players: HashMap::default(),
            field: Field::default(),
            turns: 0,
            monsters: HashMap::default(),
            next_monster_id: 1,
            health: MAX_HEALTH,
        }
    }
}
