use crate::gdmp::{PlayerVisuals, Position, Room};
use crate::utils;
use crate::utils::{HashableRoom, Player, Players};
use enet::{Event, Packet, PeerID};
use lazy_static::lazy_static;
use prost::Message;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    pub static ref PLAYERS_FOR_ROOM: Mutex<HashMap<PeerID, HashableRoom>> =
        Mutex::new(HashMap::new());
    pub static ref ROOMS: Mutex<HashMap<HashableRoom, Players>> = Mutex::new(HashMap::new());
}

pub fn add_player<T>(
    evt: &mut Event<'_, T>,
    room: Room,
    src_visual: PlayerVisuals,
    src_username: String,
) {
    let src_peer_id = evt.peer_id();

    let mut rooms = ROOMS.lock().unwrap();
    let players = rooms.entry(room.clone().into()).or_insert(Players {
        players: Vec::new(),
    });

    let mut players_for_room = PLAYERS_FOR_ROOM.lock().unwrap();
    players_for_room.insert(src_peer_id, room.clone().into());

    players.players.push(Player {
        peer_id: src_peer_id,
        username: src_username.clone(),
        visual: src_visual.clone(),
    });

    for dst_player in players.players.clone() {
        // for each of these players, send a packet to the new player
        // telling them that they joined (since they were already in the room)
        let gdmp_packet = crate::gdmp::Packet {
            packet: Some(crate::gdmp::packet::Packet::PlayerJoin(
                crate::gdmp::PlayerJoinPacket {
                    room: Some(room.clone()),
                    visual: Some(dst_player.visual),
                    username: dst_player.username.clone(),
                    p_id: Some(utils::peer_id_to_u64(dst_player.peer_id)),
                },
            )),
        };

        let data = gdmp_packet.encode_to_vec();

        let packet = Packet::new(data, enet::PacketMode::ReliableSequenced).unwrap();
        evt.peer_mut().send_packet(packet, 0).unwrap();

        // send data to dst_player telling their client that src_player joined
        let dst_peer = evt
            .host
            .peer_mut_this_will_go_horribly_wrong_lmao(dst_player.peer_id);

        match dst_peer {
            None => continue,
            Some(dst_peer) => {
                if dst_peer.state() != enet::PeerState::Connected
                    || src_peer_id == dst_player.peer_id
                {
                    continue;
                }

                let gdmp_packet = crate::gdmp::Packet {
                    packet: Some(crate::gdmp::packet::Packet::PlayerJoin(
                        crate::gdmp::PlayerJoinPacket {
                            room: Some(room.clone()),
                            visual: Some(src_visual.clone()),
                            username: src_username.clone(),
                            p_id: Some(utils::peer_id_to_u64(src_peer_id)),
                        },
                    )),
                };

                let data = gdmp_packet.encode_to_vec();

                let packet = Packet::new(data, enet::PacketMode::ReliableSequenced).unwrap();
                dst_peer.send_packet(packet, 0).unwrap();
            }
        }
    }
}

pub fn remove_player<T>(evt: &mut Event<'_, T>, room: Room) {
    let src_peer_id = evt.peer_id();

    let mut rooms = ROOMS.lock().unwrap();
    let players = rooms.entry(room.clone().into()).or_insert(Players {
        players: Vec::new(),
    });

    let mut players_for_room = PLAYERS_FOR_ROOM.lock().unwrap();
    players_for_room.remove(&src_peer_id);

    for dst_player in players.players.clone() {
        match evt
            .host
            .peer_mut_this_will_go_horribly_wrong_lmao(dst_player.peer_id)
        {
            None => continue,
            Some(dst_peer) => {
                if dst_peer.state() != enet::PeerState::Connected
                    || src_peer_id == dst_player.peer_id
                {
                    continue;
                }

                let gdmp_packet = crate::gdmp::Packet {
                    packet: Some(crate::gdmp::packet::Packet::PlayerLeave(
                        crate::gdmp::PlayerLeavePacket {
                            room: Some(room.clone()),
                            p_id: Some(utils::peer_id_to_u64(src_peer_id)),
                        },
                    )),
                };

                let packet = Packet::new(
                    gdmp_packet.encode_to_vec(),
                    enet::PacketMode::ReliableSequenced,
                )
                .unwrap();
                dst_peer.send_packet(packet, 0).unwrap();
            }
        }
    }

    players.players.retain(|x| x.peer_id != src_peer_id);

    if players.players.is_empty() {
        println!("removing room {:?} because it's empty", room);
        rooms.remove(&room.into());
    }
}

pub fn handle_player_move<T>(
    evt: &mut Event<'_, T>,
    pos_p1: Position,
    pos_p2: Position,
    gamemode_p1: i32,
    gamemode_p2: i32,
) {
    let src_peer_id = evt.peer_id();

    let players_for_room = PLAYERS_FOR_ROOM.lock().unwrap();
    let room = players_for_room.get(&src_peer_id);
    if let Some(room) = room {
        let rooms = ROOMS.lock().unwrap();

        if let Some(players) = rooms.get(room) {
            for dst_player in &players.players {
                if let Some(dst_peer) = evt
                    .host
                    .peer_mut_this_will_go_horribly_wrong_lmao(dst_player.peer_id)
                {
                    if dst_peer.state() != enet::PeerState::Connected
                        || src_peer_id == dst_player.peer_id
                    {
                        continue;
                    }

                    let gdmp_packet = crate::gdmp::Packet {
                        packet: Some(crate::gdmp::packet::Packet::PlayerMove(
                            crate::gdmp::PlayerMovePacket {
                                pos_p1: Some(pos_p1.clone()),
                                pos_p2: Some(pos_p2.clone()),
                                gamemode_p1,
                                gamemode_p2,
                                p_id: Some(utils::peer_id_to_u64(src_peer_id)),
                            },
                        )),
                    };

                    let data = gdmp_packet.encode_to_vec();

                    let packet = Packet::new(data, enet::PacketMode::UnreliableSequenced).unwrap();

                    dst_peer.send_packet(packet, 0).unwrap();
                }
            }
        }
    }
}
