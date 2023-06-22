use crate::gdmp::Room;
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

// this is only a struct to make code more readable
pub struct Players {
    pub players: Vec<PeerID>,
}