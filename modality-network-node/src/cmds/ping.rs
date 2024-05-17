use crate::config_file;
use crate::identity_utils;
use crate::swarm;
use rand::Rng;
use anyhow::{Context, Result};
use clap::Parser;
use futures::future::{select, Either};
use futures::stream;
use futures::StreamExt;
use libp2p::multiaddr::Multiaddr;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
use rand::RngCore;
use std::borrow::Borrow;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long, default_value = "./config.json")]
    config: std::path::PathBuf,

    #[clap(long)]
    keypair: Option<std::path::PathBuf>,

    #[clap(long, default_value = "/ip4/0.0.0.0/tcp/0/ws")]
    listen: String,

    #[clap(long)]
    storage: Option<std::path::PathBuf>,

    #[clap(long, default_value = "15")]
    tick_interval: u64,

    #[clap(long, default_value = "1")]
    times: i32,
}

// pub async fn run(arg_matches: &clap::ArgMatches) -> Result<()> {
pub async fn run(opts: &Opts) -> Result<()> {
    log::info!("Config: {:?}", opts.config);
 
    let config =
        config_file::read_or_create_config(&opts.config).context("Failed to read config")?;

    log::info!("Config: {:?}", config);

    let config_keypair = config.keypair.unwrap();

    let node_keypair = identity_utils::identity_from_private_key(
      config_keypair.private_key.unwrap_or_default().borrow(),
    )
    .await?;
    // .context("Failed to read identity")?;
    log::info!("Node keypair: {:?}", node_keypair.public());
    let node_peer_id = PeerId::from(node_keypair.public());


    let mut swarm = swarm::create_swarm(node_keypair).await?;

    // 1. Unpack the multiaddr and use it to open a stream to that node
    let ma = config.listen.unwrap_or(opts.listen.clone()).parse::<Multiaddr>().unwrap();
    let start = Instant::now();
    swarm.dial(ma.clone()).unwrap();

    let duration = start.elapsed();
    let duration_seconds = duration.as_secs_f64();
    
    let times_to_ping = &opts.times;
    // Create a random data payload.
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..32).map(|_| rng.gen()).collect();


    let start = std::time::Instant::now();

    // // Send and receive data over the stream.
    // for _ in 0..1 { // Replace 1 with the number of times you want to send the ping.
    //     let ping = Ping(data.clone());

    //     // Send the request.
    //     let request_id = swarm.behaviour_mut().send_request(&remote_peer_id, ping);

    //     // Handle swarm events.
    //     loop {
    //         match swarm.next().await {
    //             Some(SwarmEvent::Behaviour(RequestResponseEvent::Message { peer, message })) => {
    //                 if peer == remote_peer_id {
    //                     match message {
    //                         RequestResponseMessage::Response { request_id: resp_id, response } => {
    //                             if resp_id == request_id {
    //                                 let Pong(response_data) = response;
    //                                 let byte_matches = response_data.iter().zip(data.iter()).all(|(a, b)| a == b);
    //                                 if !byte_matches {
    //                                     eprintln!("Wrong pong");
    //                                     return Err("Wrong pong".into());
    //                                 }
    //                                 println!("Pinged successfully in {:?}", start.elapsed());
    //                                 break;
    //                             }
    //                         }
    //                         _ => {}
    //                     }
    //                 }
    //             }
    //             Some(event) => {
    //                 println!("Unhandled event: {:?}", event);
    //             }
    //             None => break,
    //         }
    //     }
    // }
    // // 3. Loop the amount of times from args, and create a random 32 bytes payload
    for _ in 0..*times_to_ping {
        let mut rng = rand::thread_rng();
        let payload: [u8; 32] = rng.gen();
        log::info!("Sending ping with payload: {:?}", payload);

        // let mut stream = stream.into_stream();

        // stream.write_all(&payload).await?;
        // let mut buf = vec![0u8; 32];
        // stream.read_exact(&mut buf).await?;
        // assert_eq!(payload, buf);
    }
    Ok(())

}
