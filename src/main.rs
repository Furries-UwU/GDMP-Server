extern crate enet;

use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context;
use enet::*;
use prost::DecodeError;

// protocol stuff
pub mod gdmp {
    include!(concat!(env!("OUT_DIR"), "/gdmp.rs"));
}

use crate::gdmp::*;
use crate::gdmp::packet::Packet::{PlayerJoin, PlayerMove};

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

        let evnt = match evnt {
            Some(evnt) => evnt,
            None => continue,
        };

        match evnt.kind() {
            EventKind::Connect => println!("new connection!"),
            EventKind::Disconnect { .. } => println!("disconnect!"),
            EventKind::Receive {
                channel_id,
                ref packet,
            } => {
                let data = packet.data();
                println!("got packet on channel {}, size {}", channel_id, data.len());

                let packet: Result<gdmp::Packet, DecodeError> = prost::Message::decode(data);
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
                        visual: _visual,
                    }) => {
                        println!("player join packet - joined room {:?}", room.unwrap());
                    }
                    PlayerMove(PlayerMovePacket {pos_p1, pos_p2}) => {
                        println!("player move packet - position {:?}, velocity {:?}", pos_p1.unwrap(), pos_p2.unwrap_or(Position::default()));
                    }
                    _ => {
                        println!("UNIMPLEMENTED PACKET: {:#?}", packet);
                    }
                }

                // save to file
                let mut file = File::create("test2")?;
                file.write_all(data)?;
            }
        }
    }
}
