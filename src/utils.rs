use crate::gdmp::{PlayerVisuals, Room};
use enet::PeerID;

// this is pretty much exactly the same as crate::gdmp::Room
// but impls Hash and Eq
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct HashableRoom {
    pub id: String,
    pub name: String,
    pub player_amount: i32,
    pub max_player: i32,
    pub owner: String,
    pub require_pass: bool,
    pub level_id: Option<i32>,
    pub pass: Option<String>,
}

impl From<Room> for HashableRoom {
    fn from(room: Room) -> Self {
        Self {
            id: room.id,
            name: room.name,
            player_amount: room.player_amount,
            max_player: room.max_player,
            owner: room.owner,
            require_pass: room.require_pass,
            level_id: room.level_id,
            pass: None,
        }
    }
}

impl From<&HashableRoom> for Room {
    fn from(room: &HashableRoom) -> Self {
        Self {
            id: room.id.clone(),
            name: room.name.clone(),
            player_amount: room.player_amount,
            max_player: room.max_player,
            owner: room.owner.clone(),
            require_pass: room.require_pass,
            level_id: room.level_id,
        }
    }
}

#[derive(Clone)]
pub struct Player {
    pub peer_id: PeerID,
    pub username: String,
    pub visual: PlayerVisuals,
}

// this is only a struct to make code more readable
pub struct Players {
    pub players: Vec<Player>,
}

// i hope this is actually unique and i didn't fuck it up
pub(crate) fn peer_id_to_u64(peer_id: PeerID) -> u64 {
    let index = peer_id.index as u64;
    let generation = peer_id.generation as u64;

    (index << 32 | generation).to_be()
}