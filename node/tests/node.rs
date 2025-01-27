use std::{env, time::Duration};

use celestia_node::{
    node::{Node, NodeConfig},
    p2p::P2pService,
};
use celestia_rpc::prelude::*;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade::Version},
    identity::{self, Keypair},
    noise, tcp, yamux, Multiaddr, PeerId, Transport,
};
use tokio::time::sleep;

const WS_URL: &str = "ws://localhost:26658";

fn tcp_transport(local_keypair: &Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    tcp::tokio::Transport::default()
        .upgrade(Version::V1Lazy)
        .authenticate(noise::Config::new(local_keypair).unwrap())
        .multiplex(yamux::Config::default())
        .boxed()
}

async fn get_bridge_tcp_ma() -> Multiaddr {
    let _ = dotenvy::dotenv();

    let auth_token = env::var("CELESTIA_NODE_AUTH_TOKEN_ADMIN").unwrap();
    let client = celestia_rpc::client::new_websocket(WS_URL, Some(&auth_token))
        .await
        .unwrap();

    let bridge_info = client.p2p_info().await.unwrap();

    bridge_info
        .addrs
        .into_iter()
        .find(|ma| ma.protocol_stack().any(|protocol| protocol == "tcp"))
        .expect("Bridge doesn't listen on tcp")
}

#[tokio::test]
async fn connects_to_the_go_bridge_node() {
    let bridge_ma = get_bridge_tcp_ma().await;
    let p2p_local_keypair = identity::Keypair::generate_ed25519();

    let node = Node::new(NodeConfig {
        network_id: "private".to_string(),
        p2p_transport: tcp_transport(&p2p_local_keypair),
        p2p_local_keypair,
        p2p_bootstrap_peers: vec![bridge_ma],
        p2p_listen_on: vec![],
    })
    .await
    .unwrap();

    // wait for the node to connect to the bridge
    sleep(Duration::from_millis(50)).await;

    let info = node.p2p().network_info().await.unwrap();
    assert_eq!(info.num_peers(), 1);
}
