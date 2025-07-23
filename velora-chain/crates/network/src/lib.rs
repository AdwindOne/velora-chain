use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub,
    identity,
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent, SwarmBuilder},
    tcp,
    yamux,
    Multiaddr,
    PeerId,
    Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::{io, select, time};
use velora_core::{Block, Transaction};
use anyhow::Result;

#[derive(NetworkBehaviour)]
struct VeloraBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct Network {
    swarm: libp2p::Swarm<VeloraBehaviour>,
}

impl Network {
    pub async fn new(port: u16) -> Result<Self> {
        let id_keys = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(id_keys.public());

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&id_keys)?)
            .multiplex(yamux::Config::default())
            .boxed();

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()?;

        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(id_keys),
            gossipsub_config,
        )?;

        let topic = gossipsub::IdentTopic::new("velora-blocks");
        gossipsub.subscribe(&topic)?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = VeloraBehaviour { gossipsub, mdns };

        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
        swarm.listen_on(listen_addr)?;

        Ok(Self { swarm })
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                event = self.swarm.select_next_some() => {
                    // Handle swarm events
                }
            }
        }
    }
}
