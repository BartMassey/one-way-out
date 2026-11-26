//! "Monster OBject" / "MOBile": MOB. This is the MOB state
//! and methods for manipulating it.

use std::cmp::Ordering::*;

use crate::*;

pub const MONSTER_SIGHT_DISTANCE: usize = 10;

/// MOB state.
pub struct Mob {
    /// MOB id. Globally unique across all sessions.
    pub id: u64,
    /// MOB position in field coordinates.
    pub posn: usize,
    /// MOB health.
    health: u64,
}

impl Mob {
    /// Make a new MOB with the given ID and position, and
    /// with random health.
    pub fn new(id: u64, posn: usize) -> Self {
        Mob {
            id,
            posn,
            health: random(3) + 3,
        }
    }

    /// Take a hit.
    pub fn hit(&mut self) -> bool {
        let hit = random(3);
        if hit >= self.health {
            self.health = 0;
            false
        } else {
            self.health -= hit;
            true
        }
    }

    /// MOB "AI". Heuristically decide how to move the MOB.
    /// Returns the new position.
    pub fn get_move(&self, game: &Game) -> usize {
        let posn = self.posn;

        // Look for nearby players.
        let min_pposn = (posn - MONSTER_SIGHT_DISTANCE).max(0);
        let max_pposn = (posn + MONSTER_SIGHT_DISTANCE).min(DOOR_POSN - 1);
        let mut nearest = None;
        for pposn in min_pposn..=max_pposn {
            if !game.field.has_player(pposn) {
                continue;
            }
            let dist = usize::abs_diff(posn, pposn);
            if dist > MONSTER_SIGHT_DISTANCE {
                continue;
            }
            if let Some(nearest) = nearest {
                let ndist = usize::abs_diff(nearest, pposn);
                if dist >= ndist {
                    continue;
                }
            }
            nearest = Some(pposn);
        }

        // Player in sight. Move toward nearest.
        if let Some(pposn) = nearest {
            return match posn.cmp(&pposn) {
                Less => posn + 1,
                Equal => posn,
                Greater => posn - 1,
            };
        }

        // Nothing in sight. Just move at random.
        let dirn = random(3);
        match dirn {
            0 => (posn - 1).max(0),
            1 => posn,
            2 => (posn + 1).min(DOOR_POSN - 1),
            _ => panic!("internal error: weird mob move"),
        }
    }
}
