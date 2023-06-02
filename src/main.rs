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
        if evnt.is_none() {
            continue;
        }
        //println!("received event: {:#?}", evnt);

        let evnt = evnt.unwrap();
        match evnt.kind() {
            EventKind::Connect => println!("new connection!"),
            EventKind::Disconnect { .. } => println!("disconnect!"),
            EventKind::Receive {
                channel_id,
                ref packet
            } => {
                let data = packet.data();
                println!("got packet on channel {}, size {}", channel_id, data.len());

                let packet:Result<gdmp::Packet, DecodeError> = prost::Message::decode(data);
                if packet.is_err() {
                    println!("error decoding packet: {:?}", packet);
                    println!("data: {:?}", data);
                    continue;
                }

                println!("packet: {:#?}", packet);
                // save to file
                let mut file = File::create("test2")?;
                file.write_all(data)?;
            }
        }
    }
}
