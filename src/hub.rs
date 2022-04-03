use crate::{db, error};
use axum::extract::ws::WebSocket;
use chrono::{DateTime, Utc};
use futures::stream::SplitStream;
use futures::{sink::SinkExt, stream::StreamExt};
use primitive_types::H160;
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

        // Update peer with number of sign-ups on join
        let connection = self.pool.get_connection().await.unwrap();
        let sign_ups = db::vip::total(&connection).await.unwrap();
        let sender = MessageSender(tx.clone());
        sender
            .send(Message::SignedUp {
                total: sign_ups.total,
                address: None,
                last_signed_up: sign_ups.last_signed_up,
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
            total: self.clients.read().await.len() as u64,
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
            total: self.clients.read().await.len() as u64,
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

        let mut signed_up: bool;
        match message {
            Request::SignUp { address } => {
                tracing::debug!("sign-up received");

                // Check if address already signed up
                signed_up = db::vip::check(&connection, address).await?;
                if !signed_up {
                    // Sign up address
                    db::vip::sign_up(&connection, address).await?;
                    signed_up = true;
                }
            }
            Request::Check { address } => {
                tracing::debug!("check received");

                signed_up = db::vip::check(&connection, address).await?;
            }
        }

        // Broadcast updated total to clients
        let signups = db::vip::total(&connection).await?;
        self.broadcast(Message::SignedUp {
            total: signups.total,
            address: None,
            last_signed_up: signups.last_signed_up,
        })?;

        // Send checked message back to sender with signup status
        sender
            .send(Message::SignedUp {
                total: signups.total,
                address: Some(signed_up),
                last_signed_up: signups.last_signed_up,
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
    #[serde(rename = "sign-up")]
    SignUp { address: H160 },
    #[serde(rename = "check")]
    Check { address: H160 },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "signed-up")]
    SignedUp {
        total: u64,
        address: Option<bool>,
        last_signed_up: Option<DateTime<Utc>>,
    },
    #[serde(rename = "peer-joined")]
    PeerJoined {
        total: u64,
        last_joined: Option<DateTime<Utc>>,
    },
    #[serde(rename = "peer-left")]
    PeerLeft {
        total: u64,
        last_left: Option<DateTime<Utc>>,
    },
}
