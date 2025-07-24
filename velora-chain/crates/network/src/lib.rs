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
use std::time::Duration;
use tokio::{io, select, time, sync::mpsc};
use velora_core::{Block, Transaction};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use log::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub enum GossipMessage {
    NewBlock(Block),
    NewTransaction(Transaction),
}

#[derive(NetworkBehaviour)]
struct VeloraBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct Network {
    swarm: libp2p::Swarm<VeloraBehaviour>,
    block_sender: mpsc::UnboundedSender<Block>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
}

impl Network {
    pub async fn new(
        port: u16,
        block_sender: mpsc::UnboundedSender<Block>,
        tx_sender: mpsc::UnboundedSender<Transaction>
    ) -> Result<Self> {
        let id_keys = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(id_keys.public());
        info!("Local peer id: {}", local_peer_id);

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

        let topic = gossipsub::IdentTopic::new("velora-chain");
        gossipsub.subscribe(&topic)?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = VeloraBehaviour { gossipsub, mdns };

        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
        swarm.listen_on(listen_addr)?;

        Ok(Self { swarm, block_sender, tx_sender })
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening on {}", address);
                    }
                    SwarmEvent::Behaviour(VeloraBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, _multiaddr) in list {
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    SwarmEvent::Behaviour(VeloraBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: _peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        if let Ok(msg) = bincode::deserialize::<GossipMessage>(&message.data) {
                            match msg {
                                GossipMessage::NewBlock(block) => {
                                    if let Err(e) = self.block_sender.send(block) {
                                        warn!("Failed to send block to consensus: {}", e);
                                    }
                                }
                                GossipMessage::NewTransaction(tx) => {
                                    if let Err(e) = self.tx_sender.send(tx) {
                                        warn!("Failed to send transaction to txpool: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn broadcast_block(&mut self, block: &Block) -> Result<()> {
        let topic = gossipsub::IdentTopic::new("velora-chain");
        let msg = bincode::serialize(&GossipMessage::NewBlock(block.clone()))?;
        self.swarm.behaviour_mut().gossipsub.publish(topic, msg)?;
        Ok(())
    }

    pub fn broadcast_transaction(&mut self, tx: &Transaction) -> Result<()> {
        let topic = gossipsub::IdentTopic::new("velora-chain");
        let msg = bincode::serialize(&GossipMessage::NewTransaction(tx.clone()))?;
        self.swarm.behaviour_mut().gossipsub.publish(topic, msg)?;
        Ok(())
    }
}
