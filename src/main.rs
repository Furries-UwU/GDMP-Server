mod manager;
mod utils;

use anyhow::Context;
use clap::Parser;
use prost::{DecodeError, Message};
use std::net::Ipv4Addr;
use std::time::Duration;
use zeromq::*;

// protocol stuff
pub mod gdmp {
    include!(concat!(env!("OUT_DIR"), "/gdmp.rs"));
}

use crate::gdmp::packet::Packet::*;
use crate::gdmp::*;
use crate::utils::{HashableRoom, Players};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The port to run the server on
    #[arg(short, long, default_value_t = 34154)]
    port: u16,

    /// The ip to run the server on
    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    ip: String,
}

#[tokio::main]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let addr = args
        .ip
        .as_str()
        .parse::<Ipv4Addr>()
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0))
        .to_string();

    let mut socket = zeromq::PubSocket::new();
    socket.bind(format!("tcp://{}:{}", addr, args.port).as_str());

    println!("Server listening on {}:{}!", addr, args.port);
    loop {}
    /*
       let enet = Enet::new().context("could not initialize ENet")?;

       let local_addr = Address::new(
           args.ip
               .as_str()
               .parse::<Ipv4Addr>()
               .unwrap_or(Ipv4Addr::new(0, 0, 0, 0)),
           args.port,
       );

       let mut host = enet
           .create_host::<()>(
               Some(&local_addr),
               4095,
               // Consider changing this to 1
               // Unless we are making 1 channel per 1 level
               ChannelLimit::Maximum,
               // Consider changing this to 64kb
               BandwidthLimit::Unlimited,
               // Consider changing this to 64kb
               BandwidthLimit::Unlimited,
           )
           .context("could not create host")?;

       println!(
           "Server listening on {}:{}!",
           local_addr.ip(),
           local_addr.port()
       );

       loop {
           let evt = host
               .service(Duration::from_micros(1000))
               .context("service failed")?;

           let mut evt = match evt {
               Some(evt) => evt,
               None => continue,
           };

           match evt.kind() {
               EventKind::Connect => println!("new connection!"),
               EventKind::Disconnect { .. } => {
                   let mut rooms = manager::get_rooms().lock().unwrap();
                   let h = rooms
                       .iter_mut()
                       .filter(|(_, v)| v.players.iter().any(|p| p.peer_id == evt.peer_id()))
                       .collect::<Vec<(&HashableRoom, &mut Players)>>();

                   for value in h {
                       value.1.players.retain(|p| p.peer_id != evt.peer_id());

                       for dst_player in &value.1.players {
                           match evt
                               .host
                               // Consider changing this to `non_mutable_host_peer_mut`
                               .peer_mut_this_will_go_horribly_wrong_lmao(dst_player.peer_id)
                           {
                               None => continue,
                               Some(dst_peer) => {
                                   if dst_peer.state() != PeerState::Connected
                                       || evt.peer_id() == dst_player.peer_id
                                   {
                                       continue;
                                   }

                                   let gdmp_packet = gdmp::Packet {
                                       packet: Some(PlayerLeave(PlayerLeavePacket {
                                           room: Some(<Room>::from(value.0)),
                                           p_id: Some(utils::peer_id_to_u64(evt.peer_id())),
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

                       if value.1.players.is_empty() {
                           println!("removing room {:?} because it's empty", value.0);
                       }
                   }

                   rooms.retain(|_, v| !v.players.is_empty());
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
                           username,
                           p_id: _,
                       }) => {
                           match room {
                               None => {
                                   eprintln!("invalid room");
                                   continue;
                               }
                               Some(room) => {
                                   println!(
                                       "player join: {} joined room {:?} with player data {:?}",
                                       username, room, visual
                                   );

                                   manager::add_player(&mut evt, room, visual.unwrap(), username);
                               }
                           };
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
                               &mut evt,
                               pos_p1.unwrap_or(Position::default()),
                               pos_p2.unwrap_or(Position::default()),
                               gamemode_p1,
                               gamemode_p2,
                           );
                       }
                       PlayerLeave(PlayerLeavePacket { room, p_id: _ }) => {
                           match room {
                               None => {
                                   eprintln!("invalid room");
                                   continue;
                               }
                               Some(room) => {
                                   manager::remove_player(&mut evt, room);
                               }
                           };
                       }
                       PlayerRequestJoinRoom(PlayerRequestJoinRoomPacket { id, pass }) => {
                           let rooms = manager::get_rooms().lock().unwrap();

                           let room = match rooms.keys().find(|room| room.id == id) {
                               None => {
                                   eprintln!("room not found");
                                   continue;
                               }
                               Some(room) => room,
                           };

                           if room.pass.is_some()
                               && (pass.is_some()
                                   || room.pass.as_ref().unwrap() != pass.as_ref().unwrap())
                           {
                               eprintln!("invalid password");
                               continue;
                           }

                           // TODO: Get player and add them to room
                       }
                       PlayerRequestRoomList(PlayerRequestRoomListPacket {}) => {
                           let gdmp_packet = gdmp::Packet {
                               packet: Some(RoomList(RoomListPacket {
                                   room: manager::ROOMS
                                       .get()
                                       .unwrap()
                                       .lock()
                                       .unwrap()
                                       .keys()
                                       .map(Room::from)
                                       .collect(),
                                   p_id: Some(utils::peer_id_to_u64(evt.peer_id())),
                               })),
                           };

                           let data = gdmp_packet.encode_to_vec();

                           let packet =
                               enet::Packet::new(data, PacketMode::ReliableSequenced).unwrap();
                           evt.peer_mut().send_packet(packet, 0).unwrap();
                       }
                       _ => {
                           eprintln!("UNIMPLEMENTED PACKET: {:#?}", packet);
                       }
                   }
               }
           }
       }

    */
}
