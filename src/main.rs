mod manager;
mod utils;

extern crate enet;

use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context;
use enet::*;
use prost::{DecodeError, Message};

// protocol stuff
pub mod gdmp {
    include!(concat!(env!("OUT_DIR"), "/gdmp.rs"));
}

use crate::gdmp::packet::Packet::{PlayerJoin, PlayerLeave, PlayerMove};
use crate::gdmp::*;
use crate::utils::{HashableRoom, Players};

fn main() -> anyhow::Result<()> {
    let enet = Enet::new().context("could not initialize ENet")?;

    let local_addr = Address::new(Ipv4Addr::LOCALHOST, 34154);

    let mut host = enet
        .create_host::<()>(
            Some(&local_addr),
            10,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .context("could not create host")?;

    loop {
        let evnt = host
            .service(Duration::from_micros(1000))
            .context("service failed")?;

        let mut evnt = match evnt {
            Some(evnt) => evnt,
            None => continue,
        };

        match evnt.kind() {
            EventKind::Connect => println!("new connection!"),
            EventKind::Disconnect { .. } => {
                let mut rooms = manager::ROOMS.lock().unwrap();
                let h = rooms
                    .iter_mut()
                    .filter(|(_, v)| v.players.iter().any(|p| p.peer_id == evnt.peer_id()))
                    .collect::<Vec<(&HashableRoom, &mut Players)>>();

                for value in h {
                    value.1.players.retain(|p| p.peer_id != evnt.peer_id());

                    for dst_player in &value.1.players {
                        match evnt
                            .host
                            .peer_mut_this_will_go_horribly_wrong_lmao(dst_player.peer_id)
                        {
                            None => continue,
                            Some(dst_peer) => {
                                if dst_peer.state() != PeerState::Connected
                                    || evnt.peer_id() == dst_player.peer_id
                                {
                                    continue;
                                }

                                let gdmp_packet = gdmp::Packet {
                                    packet: Some(PlayerLeave(PlayerLeavePacket {
                                        room: Some(<Room>::from(value.0)),
                                        p_id: Some(utils::peer_id_to_u64(evnt.peer_id())),
                                    })),
                                };

                                let packet = enet::Packet::new(
                                    gdmp_packet.encode_to_vec(),
                                    PacketMode::ReliableSequenced,
                                )
                                .unwrap();
                                dst_peer.send_packet(packet, 0).unwrap();
                            }
                        }
                    }

                    if value.1.players.len() == 0 {
                        println!("removing room {:?} because it's empty", value.0);
                    }
                }

                rooms.retain(|_, v| v.players.len() != 0);
                println!("disconnect!");
            }
            EventKind::Receive {
                channel_id: _channel_id,
                ref packet,
            } => {
                let data = packet.data();
                //println!("got packet on channel {}, size {}", channel_id, data.len());

                let packet: Result<gdmp::Packet, DecodeError> = Message::decode(data);
                let packet = match packet {
                    Ok(packet) => packet,
                    Err(err) => {
                        eprintln!("error decoding packet: {}", err);
                        eprintln!("data: {:?}", data);
                        continue;
                    }
                };

                let packet = match packet.packet {
                    None => {
                        eprintln!("invalid (empty) packet");
                        continue;
                    }
                    Some(packet) => packet,
                };

                match packet {
                    PlayerJoin(PlayerJoinPacket {
                        room,
                        visual,
                        p_id: _,
                    }) => {
                        let room = room.expect("waaeeee room is bad :(");
                        println!(
                            "player join packet - joined room {:?} with player data {:?}",
                            room, visual
                        );

                        manager::add_player(&mut evnt, room, visual.unwrap());
                    }
                    PlayerMove(PlayerMovePacket {
                        pos_p1,
                        pos_p2,
                        p_id: _,
                        gamemode_p1,
                        gamemode_p2,
                    }) => {
                        /*
                        println!(
                            "player move packet - pos1 {:?}, pos2 {:?}",
                            &pos_p1,
                            &pos_p2
                        );
                        */

                        manager::handle_player_move(
                            &mut evnt,
                            pos_p1.unwrap_or(Position::default()),
                            pos_p2.unwrap_or(Position::default()),
                            gamemode_p1,
                            gamemode_p2,
                        );
                    }
                    PlayerLeave(PlayerLeavePacket { room, p_id: _ }) => {
                        let room = room.expect("bad room :(");
                        manager::remove_player(&mut evnt, room);
                    }
                    _ => {
                        println!("UNIMPLEMENTED PACKET: {:#?}", packet);
                    }
                }
            }
        }
    }
}
