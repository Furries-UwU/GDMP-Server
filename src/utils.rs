use crate::gdmp::{PlayerVisuals, Room};
use enet::PeerID;

// this is pretty much exactly the same as crate::gdmp::Room
// but impls Hash and Eq
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct HashableRoom {
    pub level_id: i32,
}

impl From<Room> for HashableRoom {
    fn from(room: Room) -> Self {
        Self {
            level_id: room.level_id,
        }
    }
}

impl From<&HashableRoom> for Room {
    fn from(room: &HashableRoom) -> Self {
        Self {
            level_id: room.level_id,
        }
    }
}

#[derive(Clone)]
pub struct Player {
    pub peer_id: PeerID,
    pub visual: PlayerVisuals,
}

// this is only a struct to make code more readable
pub struct Players {
    pub players: Vec<Player>,
}

// i hope this is actually unique and i didn't fuck it up
// don't forget about big endian / little endian
pub(crate) fn peer_id_to_u64(peer_id: PeerID) -> u64 {
    let a = peer_id.index as u32;
    let b = peer_id.generation as u32;

    (a as u64) << 32 | b as u64
}
