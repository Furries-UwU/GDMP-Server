use crate::gdmp::{PlayerVisuals, Position, Room};
use crate::utils::{HashableRoom, Players};
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

pub fn add_player<T>(evnt: &mut Event<'_, T>, room: Room, visual: PlayerVisuals) {
    let player = evnt.peer_id();

    let mut rooms = ROOMS.lock().unwrap();
    let players = rooms.entry(room.clone().into()).or_insert(Players {
        players: Vec::new(),
    });

    let mut players_for_room = PLAYERS_FOR_ROOM.lock().unwrap();
    players_for_room.insert(player, room.clone().into());

    players.players.push(player);

    for x in players.players.clone() {
        // for each of these players, send a packet to the new player
        // telling them that they joined
        let gdmp_packet = crate::gdmp::Packet {
            packet_type: 1,
            packet: Some(crate::gdmp::packet::Packet::PlayerJoin(
                crate::gdmp::PlayerJoinPacket {
                    room: Some(room.clone()),
                    visual: None,
                    p_id: Some(peer_id_to_u64(x)),
                },
            )),
        };

        let data = gdmp_packet.encode_to_vec();

        let packet = Packet::new(data, enet::PacketMode::ReliableSequenced).unwrap();
        evnt.peer_mut().send_packet(packet, 0).unwrap();

        let peer = evnt.host.peer_mut_this_will_go_horribly_wrong_lmao(x);

        match peer {
            None => continue,
            Some(peer) => {
                if peer.state() != enet::PeerState::Connected /*|| x == player*/ {
                    continue;
                }

                let gdmp_packet = crate::gdmp::Packet {
                    packet_type: 1,
                    packet: Some(crate::gdmp::packet::Packet::PlayerJoin(
                        crate::gdmp::PlayerJoinPacket {
                            room: Some(room.clone()),
                            visual: Some(visual.clone()),
                            p_id: Some(peer_id_to_u64(player)),
                        },
                    )),
                };

                let data = gdmp_packet.encode_to_vec();

                let packet = Packet::new(data, enet::PacketMode::ReliableSequenced).unwrap();
                peer.send_packet(packet, 0).unwrap();
            }
        }
    }
}

pub fn remove_player(room: Room, player: PeerID) {
    let mut rooms = ROOMS.lock().unwrap();
    let players = rooms.entry(room.into()).or_insert(Players {
        players: Vec::new(),
    });
    players.players.retain(|&x| x != player);
}

// i hope this is actually unique and i didn't fuck it up
fn peer_id_to_u64(peer_id: PeerID) -> u64 {
    let a = peer_id.index as u32;
    let b = peer_id.generation as u32;

    let res = (a as u64) << 32 | b as u64;
    res
}

pub fn handle_player_move<T>(evnt: &mut Event<'_, T>, pos_p1: Position, pos_p2: Position, gamemode_p1: i32, gamemode_p2: i32) {
    let player = evnt.peer_id();

    let players_for_room = PLAYERS_FOR_ROOM.lock().unwrap();
    let room = players_for_room.get(&player);
    match room {
        Some(room) => {
            let rooms = ROOMS.lock().unwrap();
            let players = rooms.get(room);
            match players {
                Some(players) => {
                    for peer_id in &players.players {
                        let peer = evnt.host.peer_mut_this_will_go_horribly_wrong_lmao(*peer_id);
                        match peer {
                            Some(peer) => {
                                if peer.state() != enet::PeerState::Connected /*|| *peer_id == player*/ {
                                    continue;
                                }

                                let gdmp_packet = crate::gdmp::Packet {
                                    packet_type: 2,
                                    /* todo: we need a diff packet struct for this to indicate player ids */
                                    packet: Some(crate::gdmp::packet::Packet::PlayerMove(
                                        crate::gdmp::PlayerMovePacket {
                                            pos_p1: Some(pos_p1.clone()),
                                            pos_p2: Some(pos_p2.clone()),
                                            gamemode_p1,
                                            gamemode_p2,
                                            p_id: Some(peer_id_to_u64(player)),
                                        },
                                    )),
                                };

                                let data = gdmp_packet.encode_to_vec();

                                let packet =
                                    Packet::new(data, enet::PacketMode::UnreliableSequenced)
                                        .unwrap();

                                peer.send_packet(packet, 0).unwrap();
                            }
                            None => {}
                        }
                    }
                }
                None => {}
            }
        }
        None => {}
    }
}
