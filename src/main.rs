extern crate enet;

use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context;
use enet::*;

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
        let evnt = host.service(Duration::from_micros(1000)).context("service failed")?;
        if evnt.is_none() {
            continue;
        }
        println!("received event: {:#?}", evnt);

        let evnt = evnt.unwrap();
        match evnt.kind() {
            EventKind::Connect => println!("new connection!"),
            EventKind::Disconnect{..} => println!("disconnect!"),
            EventKind::Receive {
                     channel_id,
                     ref packet,
                     ..
                 } => println!(
                "got packet on channel {}, content: '{}'",
                channel_id,
                std::str::from_utf8(packet.data()).unwrap()
            ),
            _ => (),
        }
    }
}