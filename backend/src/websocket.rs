use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_ws::{Message, MessageStream, Session};
use dashmap::DashMap;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use bytes::Bytes;
use tokio::sync::broadcast;
use tokio::time::interval;
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
const BROADCAST_CHANNEL_SIZE: usize = 1000;


pub static WS_REGISTRY: Lazy<WebSocketRegistry> = Lazy::new(WebSocketRegistry::new);

#[derive(Clone)]
pub struct ClientConnection {
    pub client_id: String,
    pub sender: broadcast::Sender<WsMessage>,
    pub subscribed_vaults: Arc<DashMap<String, ()>>,
    pub connected_at: Instant,
}

pub struct WebSocketRegistry {
    
    clients: DashMap<String, ClientConnection>,
    
    vault_subscriptions: DashMap<String, DashMap<String, ()>>,
    
    global_broadcast: broadcast::Sender<WsMessage>,
}

impl WebSocketRegistry {
    pub fn new() -> Self {
        let (global_broadcast, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);
        Self {
            clients: DashMap::new(),
            vault_subscriptions: DashMap::new(),
            global_broadcast,
        }
    }


    pub fn register_client(&self) -> (String, broadcast::Receiver<WsMessage>) {
        let client_id = Uuid::new_v4().to_string();
        let (sender, receiver) = broadcast::channel(BROADCAST_CHANNEL_SIZE);
        
        let connection = ClientConnection {
            client_id: client_id.clone(),
            sender,
            subscribed_vaults: Arc::new(DashMap::new()),
            connected_at: Instant::now(),
        };
        
        self.clients.insert(client_id.clone(), connection);
        tracing::info!("Registered new WebSocket client: {}", client_id);
        
        (client_id, receiver)
    }


    pub fn unregister_client(&self, client_id: &str) {
        if let Some((_, connection)) = self.clients.remove(client_id) {
            // Remove from all vault subscriptions
            for vault_entry in connection.subscribed_vaults.iter() {
                let vault_pubkey = vault_entry.key();
                if let Some(subscribers) = self.vault_subscriptions.get(vault_pubkey) {
                    subscribers.remove(client_id);
                }
            }
            tracing::info!(
                "Unregistered WebSocket client: {} (was connected for {:?})",
                client_id,
                connection.connected_at.elapsed()
            );
        }
    }

    
    pub fn subscribe_to_vault(&self, client_id: &str, vault_pubkey: &str) -> bool {
        if let Some(connection) = self.clients.get(client_id) {

            connection.subscribed_vaults.insert(vault_pubkey.to_string(), ());
            

            self.vault_subscriptions
                .entry(vault_pubkey.to_string())
                .or_insert_with(DashMap::new)
                .insert(client_id.to_string(), ());
            
            tracing::debug!("Client {} subscribed to vault {}", client_id, vault_pubkey);
            return true;
        }
        false
    }

    
    pub fn unsubscribe_from_vault(&self, client_id: &str, vault_pubkey: &str) -> bool {
        if let Some(connection) = self.clients.get(client_id) {
            connection.subscribed_vaults.remove(vault_pubkey);
            
            if let Some(subscribers) = self.vault_subscriptions.get(vault_pubkey) {
                subscribers.remove(client_id);
            }
            
            tracing::debug!("Client {} unsubscribed from vault {}", client_id, vault_pubkey);
            return true;
        }
        false
    }

    
    pub async fn broadcast_to_vault(&self, vault_pubkey: &str, message: WsMessage) {
        if let Some(subscribers) = self.vault_subscriptions.get(vault_pubkey) {
            let mut sent_count = 0;
            let mut failed_count = 0;
            
            for subscriber in subscribers.iter() {
                let client_id = subscriber.key();
                if let Some(connection) = self.clients.get(client_id) {
                    match connection.sender.send(message.clone()) {
                        Ok(_) => sent_count += 1,
                        Err(_) => failed_count += 1,
                    }
                }
            }
            
            tracing::debug!(
                "Broadcast to vault {}: {} sent, {} failed",
                vault_pubkey,
                sent_count,
                failed_count
            );
        }
    }

    pub async fn broadcast_to_all(&self, message: WsMessage) {
        let mut sent_count = 0;
        let mut failed_count = 0;

        for client in self.clients.iter() {
            match client.sender.send(message.clone()) {
                Ok(_) => sent_count += 1,
                Err(_) => failed_count += 1,
            }
        }

        tracing::debug!(
            "Global broadcast: {} sent, {} failed",
            sent_count,
            failed_count
        );
    }


    pub fn client_count(&self) -> usize {
        self.clients.len()
    }


    pub fn vault_subscriber_count(&self, vault_pubkey: &str) -> usize {
        self.vault_subscriptions
            .get(vault_pubkey)
            .map(|s| s.len())
            .unwrap_or(0)
    }


    pub fn get_client_sender(&self, client_id: &str) -> Option<broadcast::Sender<WsMessage>> {
        self.clients.get(client_id).map(|c| c.sender.clone())
    }
}

impl Default for WebSocketRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    
    Subscribe { vault_pubkey: String },
    Unsubscribe { vault_pubkey: String },
    Ping,

    
    Connected { 
        message: String,
        client_id: String,
    },
    SubscribeAck { vault_pubkey: String, success: bool },
    UnsubscribeAck { vault_pubkey: String, success: bool },
    Pong,

    
    BalanceUpdate {
        vault_pubkey: String,
        total_balance: i64,
        available_balance: i64,
        locked_balance: i64,
        timestamp: i64,
    },

    Deposit {
        vault_pubkey: String,
        amount: i64,
        tx_signature: String,
        new_balance: i64,
        timestamp: i64,
    },

    Withdrawal {
        vault_pubkey: String,
        amount: i64,
        tx_signature: String,
        new_balance: i64,
        timestamp: i64,
    },

    Lock {
        vault_pubkey: String,
        amount: i64,
        new_locked: i64,
        new_available: i64,
        timestamp: i64,
    },

    Unlock {
        vault_pubkey: String,
        amount: i64,
        new_locked: i64,
        new_available: i64,
        timestamp: i64,
    },

    TvlUpdate {
        total_vaults: i64,
        total_value_locked: i64,
        timestamp: i64,
    },

    Alert {
        alert_type: String,
        severity: String,
        vault_pubkey: Option<String>,
        message: String,
        timestamp: i64,
    },

    Error {
        message: String,
        code: Option<String>,
    },
}


struct WsConnection {
    client_id: String,
    session: Session,
    last_heartbeat: Instant,
    receiver: broadcast::Receiver<WsMessage>,
}

impl WsConnection {
    fn new(session: Session, client_id: String, receiver: broadcast::Receiver<WsMessage>) -> Self {
        Self {
            client_id,
            session,
            last_heartbeat: Instant::now(),
            receiver,
        }
    }

    async fn send_message(&mut self, msg: &WsMessage) -> Result<(), Error> {
        let json = serde_json::to_string(msg)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        self.session
            .text(json)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }

    async fn handle_subscribe(&mut self, vault_pubkey: String) -> Result<(), Error> {
        let success = WS_REGISTRY.subscribe_to_vault(&self.client_id, &vault_pubkey);

        tracing::info!(
            "Client {} subscribed to vault {}: {}",
            self.client_id,
            vault_pubkey,
            success
        );

        let ack = WsMessage::SubscribeAck {
            vault_pubkey,
            success,
        };

        self.send_message(&ack).await
    }

    async fn handle_unsubscribe(&mut self, vault_pubkey: String) -> Result<(), Error> {
        let success = WS_REGISTRY.unsubscribe_from_vault(&self.client_id, &vault_pubkey);

        tracing::info!(
            "Client {} unsubscribed from vault {}: {}",
            self.client_id,
            vault_pubkey,
            success
        );

        let ack = WsMessage::UnsubscribeAck {
            vault_pubkey,
            success,
        };

        self.send_message(&ack).await
    }

    async fn handle_text(&mut self, text: Bytes) -> Result<(), Error> {
        let text_str =
            std::str::from_utf8(&text).map_err(|e| actix_web::error::ErrorBadRequest(e))?;

        tracing::debug!("Received WebSocket message from {}: {}", self.client_id, text_str);

        match serde_json::from_str::<WsMessage>(text_str) {
            Ok(msg) => match msg {
                WsMessage::Subscribe { vault_pubkey } => {
                    self.handle_subscribe(vault_pubkey).await?;
                }
                WsMessage::Unsubscribe { vault_pubkey } => {
                    self.handle_unsubscribe(vault_pubkey).await?;
                }
                WsMessage::Ping => {
                    self.last_heartbeat = Instant::now();
                    let pong = WsMessage::Pong;
                    self.send_message(&pong).await?;
                }
                _ => {
                    tracing::warn!("Unexpected message type from client {}", self.client_id);
                    let error = WsMessage::Error {
                        message: "Unexpected message type".to_string(),
                        code: Some("INVALID_MESSAGE_TYPE".to_string()),
                    };
                    self.send_message(&error).await?;
                }
            },
            Err(e) => {
                tracing::error!("Failed to parse WebSocket message: {}", e);
                let error = WsMessage::Error {
                    message: format!("Invalid message format: {}", e),
                    code: Some("PARSE_ERROR".to_string()),
                };
                self.send_message(&error).await?;
            }
        }

        Ok(())
    }
}

pub async fn ws_handler(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    tracing::info!(
        "WebSocket connection established from: {:?}",
        req.peer_addr()
    );

    // Register client and get ID + receiver
    let (client_id, receiver) = WS_REGISTRY.register_client();

    actix_rt::spawn(async move {
        if let Err(e) = handle_connection(&mut session, &mut msg_stream, client_id.clone(), receiver).await
        {
            tracing::error!("WebSocket connection error for client {}: {}", client_id, e);
        }
        
        WS_REGISTRY.unregister_client(&client_id);
    });

    Ok(res)
}

async fn handle_connection(
    session: &mut Session,
    msg_stream: &mut MessageStream,
    client_id: String,
    receiver: broadcast::Receiver<WsMessage>,
) -> Result<(), Error> {
    let mut conn = WsConnection::new(session.clone(), client_id.clone(), receiver);

    // Send welcome message with client ID
    let welcome = WsMessage::Connected {
        message: "Connected to Vault Management System".to_string(),
        client_id: client_id.clone(),
    };
    conn.send_message(&welcome).await?;

    let mut heartbeat_interval = interval(HEARTBEAT_INTERVAL);

    loop {
        tokio::select! {
            // Handle incoming messages from the client
            Some(Ok(msg)) = msg_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        if let Err(e) = conn.handle_text(Bytes::copy_from_slice(text.as_ref())).await {
                            tracing::error!("Error handling text message: {}", e);
                            break;
                        }
                    }
                    Message::Binary(_) => {
                        tracing::warn!("Binary messages not supported");
                        let error = WsMessage::Error {
                            message: "Binary messages not supported".to_string(),
                            code: Some("BINARY_NOT_SUPPORTED".to_string()),
                        };
                        if let Err(e) = conn.send_message(&error).await {
                            tracing::error!("Error sending error message: {}", e);
                            break;
                        }
                    }
                    Message::Ping(bytes) => {
                        conn.last_heartbeat = Instant::now();
                        if let Err(e) = conn.session.pong(&bytes).await {
                            tracing::error!("Error sending pong: {}", e);
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        conn.last_heartbeat = Instant::now();
                    }
                    Message::Close(reason) => {
                        tracing::info!("Client {} closed connection: {:?}", client_id, reason);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast messages from the registry
            Ok(broadcast_msg) = conn.receiver.recv() => {
                if let Err(e) = conn.send_message(&broadcast_msg).await {
                    tracing::error!("Error sending broadcast message: {}", e);
                    break;
                }
            }

            // Heartbeat tick
            _ = heartbeat_interval.tick() => {
                if Instant::now().duration_since(conn.last_heartbeat) > CLIENT_TIMEOUT {
                    tracing::warn!("Client {} heartbeat timeout, closing connection", client_id);
                    break;
                }

                if let Err(e) = conn.session.ping(b"").await {
                    tracing::error!("Error sending ping: {}", e);
                    break;
                }
            }

            else => break,
        }
    }

    tracing::info!("WebSocket connection closed for client {}", client_id);

    Ok(())
}

pub async fn broadcast_balance_update(
    vault_pubkey: &str,
    total_balance: i64,
    available_balance: i64,
    locked_balance: i64,
) {
    let update = WsMessage::BalanceUpdate {
        vault_pubkey: vault_pubkey.to_string(),
        total_balance,
        available_balance,
        locked_balance,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting balance update for vault {}", vault_pubkey);
    WS_REGISTRY.broadcast_to_vault(vault_pubkey, update).await;
}

pub async fn broadcast_deposit(
    vault_pubkey: &str,
    amount: i64,
    tx_signature: &str,
    new_balance: i64,
) {
    let notification = WsMessage::Deposit {
        vault_pubkey: vault_pubkey.to_string(),
        amount,
        tx_signature: tx_signature.to_string(),
        new_balance,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting deposit for vault {}: {} lamports", vault_pubkey, amount);
    WS_REGISTRY.broadcast_to_vault(vault_pubkey, notification).await;
}

pub async fn broadcast_withdrawal(
    vault_pubkey: &str,
    amount: i64,
    tx_signature: &str,
    new_balance: i64,
) {
    let notification = WsMessage::Withdrawal {
        vault_pubkey: vault_pubkey.to_string(),
        amount,
        tx_signature: tx_signature.to_string(),
        new_balance,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting withdrawal for vault {}: {} lamports", vault_pubkey, amount);
    WS_REGISTRY.broadcast_to_vault(vault_pubkey, notification).await;
}


pub async fn broadcast_lock(
    vault_pubkey: &str,
    amount: i64,
    new_locked: i64,
    new_available: i64,
) {
    let notification = WsMessage::Lock {
        vault_pubkey: vault_pubkey.to_string(),
        amount,
        new_locked,
        new_available,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting lock for vault {}: {} lamports", vault_pubkey, amount);
    WS_REGISTRY.broadcast_to_vault(vault_pubkey, notification).await;
}


pub async fn broadcast_unlock(
    vault_pubkey: &str,
    amount: i64,
    new_locked: i64,
    new_available: i64,
) {
    let notification = WsMessage::Unlock {
        vault_pubkey: vault_pubkey.to_string(),
        amount,
        new_locked,
        new_available,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting unlock for vault {}: {} lamports", vault_pubkey, amount);
    WS_REGISTRY.broadcast_to_vault(vault_pubkey, notification).await;
}

pub async fn broadcast_tvl_update(total_vaults: i64, total_value_locked: i64) {
    let update = WsMessage::TvlUpdate {
        total_vaults,
        total_value_locked,
        timestamp: chrono::Utc::now().timestamp(),
    };

    tracing::debug!("Broadcasting TVL update: {} vaults, {} TVL", total_vaults, total_value_locked);
    WS_REGISTRY.broadcast_to_all(update).await;
}

pub async fn broadcast_alert(
    alert_type: &str,
    severity: &str,
    vault_pubkey: Option<&str>,
    message: &str,
) {
    let notification = WsMessage::Alert {
        alert_type: alert_type.to_string(),
        severity: severity.to_string(),
        vault_pubkey: vault_pubkey.map(String::from),
        message: message.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    if let Some(vault) = vault_pubkey {
        // Broadcast to vault subscribers
        WS_REGISTRY.broadcast_to_vault(vault, notification).await;
    } else {
        // Broadcast to all clients
        WS_REGISTRY.broadcast_to_all(notification).await;
    }
}
#[derive(Debug, Serialize)]
pub struct WebSocketStats {
    pub total_clients: usize,
    pub total_vault_subscriptions: usize,
}

pub fn get_websocket_stats() -> WebSocketStats {
    let total_vault_subscriptions: usize = WS_REGISTRY
        .vault_subscriptions
        .iter()
        .map(|v| v.len())
        .sum();

    WebSocketStats {
        total_clients: WS_REGISTRY.client_count(),
        total_vault_subscriptions,
    }
}