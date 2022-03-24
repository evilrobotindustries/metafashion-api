use crate::{db, error};
use axum::extract::ws::WebSocket;
use chrono::{DateTime, Utc};
use futures::stream::SplitStream;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::{broadcast, mpsc};

static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
type Clients = tokio::sync::RwLock<HashSet<usize>>;

pub struct Hub {
    tx: broadcast::Sender<String>,
    clients: Clients,
    pool: db::ConnectionPool,
    api_key: String,
}

impl Hub {
    pub fn init(pool: db::ConnectionPool, api_key: String) -> Hub {
        let (tx, _rx) = broadcast::channel(10_000);
        Hub {
            tx,
            clients: Clients::default(),
            pool,
            api_key,
        }
    }

    pub fn broadcast(&self, message: Message) -> crate::Result<()> {
        if let Ok(v) = serde_json::to_string(&message) {
            if let Err(e) = self.tx.send(v) {
                tracing::error!("unable to broadcast message {} {:?}", e, message)
            }
            tracing::debug!("{:?}", message);
        } else {
            tracing::warn!("unable to serialise message for broadcast {:?}", message)
        }

        Ok(())
    }

    async fn auth(&self, receiver: &mut SplitStream<WebSocket>) -> crate::Result<()> {
        if let Some(Ok(message)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(value) = message {
                if self.api_key.eq(value.trim()) {
                    return Ok(());
                }
            }
        }

        Err(error::Error::UnauthorisedError)
    }

    pub async fn connect(&self, stream: WebSocket) {
        // Split stream into send/receive channels
        let (mut sender, mut receiver) = stream.split();

        // Authenticate
        if let Err(e) = self.auth(&mut receiver).await {
            tracing::error!("client could not be authenticated: {:?}", e);
            let _ = sender.close().await;
            return;
        }

        // Create mpsc channel for sending message from multiple producers
        let (tx, mut rx) = mpsc::channel(10);
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // Attempt to forward message on to websocket as text, breaking if error
                if sender
                    .send(axum::extract::ws::Message::Text(msg))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        // Create client identifier and track number of peers
        let id = NEXT_USERID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        tracing::debug!("client {} connected", id);
        self.clients.write().await.insert(id);

        // Update peer with number of registrations on join
        let connection = self.pool.get_connection().await.unwrap();
        let registrations = db::vip::total(&connection).await.unwrap();
        let sender = MessageSender(tx.clone());
        sender
            .send(Message::Registered {
                total: registrations.total,
                address: None,
                last_registered: registrations.last_registered,
            })
            .await;

        // Subscribe client to broadcasts (broadcast messages received are sent on to client)
        let mut broadcast = self.tx.subscribe();
        let sender = tx.clone();
        let broadcast_task = tokio::spawn(async move {
            while let Ok(msg) = broadcast.recv().await {
                // Anything sent to broadcast channel should be forwarded to sender, breaking if error
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Broadcast peer joined
        if let Err(e) = self.broadcast(Message::PeerJoined {
            total: self.clients.read().await.len() as i64,
            last_joined: Some(chrono::Utc::now()),
        }) {
            tracing::warn!("unable to notify clients of peer {} joining: {}", id, e);
        }

        // Wait for next text message from peer
        while let Some(Ok(message)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(value) = message {
                // Attempt to parse/process message
                if let Ok(m) = serde_json::from_str::<Request>(value.as_str()) {
                    if let Err(e) = self.process(m, MessageSender(tx.clone())).await {
                        tracing::error!("unable to process the message {} {:?}", e, value)
                    }
                }
            } else {
                tracing::debug!("unsupported message: {:?}", message);
            }
        }

        // Finally unsubscribe client
        send_task.abort();
        broadcast_task.abort();
        self.clients.write().await.remove(&id);
        tracing::debug!("client {} disconnected", id);

        // Broadcast peer left to remaining subscribers
        if let Err(e) = self.broadcast(Message::PeerLeft {
            total: self.clients.read().await.len() as i64,
            last_left: Some(chrono::Utc::now()),
        }) {
            tracing::warn!("unable to notify clients of peer {} leaving: {}", id, e);
        }
    }

    async fn process(
        &self,
        message: Request,
        sender: MessageSender,
    ) -> Result<(), crate::error::Error> {
        let connection = self.pool.get_connection().await?;

        let mut registered: bool;
        match message {
            Request::Register { address } => {
                tracing::debug!("register received");

                // Check if address already registered
                registered = db::vip::check(&connection, &address).await?;
                if !registered {
                    // Register address
                    db::vip::register(&connection, &address).await?;
                    registered = true;
                }
            }
            Request::Check { address } => {
                tracing::debug!("check received");

                registered = db::vip::check(&connection, &address).await?;
            }
        }

        // Broadcast updated total to clients
        let registrations = db::vip::total(&connection).await?;
        self.broadcast(Message::Registered {
            total: registrations.total,
            address: None,
            last_registered: registrations.last_registered,
        })?;

        // Send checked message back to sender with registration status
        sender
            .send(Message::Registered {
                total: registrations.total,
                address: Some(registered),
                last_registered: registrations.last_registered,
            })
            .await;
        Ok(())
    }
}

struct MessageSender(mpsc::Sender<String>);

impl MessageSender {
    async fn send(&self, message: Message) {
        if let Ok(v) = serde_json::to_string(&message) {
            if let Err(e) = self.0.send(v).await {
                tracing::error!("unable to send message {} {:?}", e, message)
            }
            tracing::debug!("{:?}", message);
        } else {
            tracing::warn!("unable to serialise message for sending {:?}", message)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(rename = "register")]
    Register { address: String },
    #[serde(rename = "check")]
    Check { address: String },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "registered")]
    Registered {
        total: i64,
        address: Option<bool>,
        last_registered: Option<DateTime<Utc>>,
    },
    #[serde(rename = "peer-joined")]
    PeerJoined {
        total: i64,
        last_joined: Option<DateTime<Utc>>,
    },
    #[serde(rename = "peer-left")]
    PeerLeft {
        total: i64,
        last_left: Option<DateTime<Utc>>,
    },
}
