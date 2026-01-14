use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_ws::{Message, MessageStream, Session};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use bytes::Bytes;
use tokio::time::interval;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {

    Subscribe { vault_pubkey: String },
    Unsubscribe { vault_pubkey: String },
    Ping,
    

    Connected { message: String },
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
    session: Session,
    last_heartbeat: Instant,
    subscribed_vaults: std::collections::HashSet<String>,
}

impl WsConnection {
    fn new(session: Session) -> Self {
        Self {
            session,
            last_heartbeat: Instant::now(),
            subscribed_vaults: std::collections::HashSet::new(),
        }
    }
    

    async fn send_message(&mut self, msg: &WsMessage) -> Result<(), Error> {
        let json = serde_json::to_string(msg)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        
        self.session.text(json).await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }
    
    async fn handle_subscribe(&mut self, vault_pubkey: String) -> Result<(), Error> {
        let success = self.subscribed_vaults.insert(vault_pubkey.clone());
        
        if success {
            tracing::info!("Client subscribed to vault: {}", vault_pubkey);
        }
        
        let ack = WsMessage::SubscribeAck {
            vault_pubkey,
            success,
        };
        
        self.send_message(&ack).await
    }
    
    async fn handle_unsubscribe(&mut self, vault_pubkey: String) -> Result<(), Error> {
        let success = self.subscribed_vaults.remove(&vault_pubkey);
        
        if success {
            tracing::info!("Client unsubscribed from vault: {}", vault_pubkey);
        }
        
        let ack = WsMessage::UnsubscribeAck {
            vault_pubkey,
            success,
        };
        
        self.send_message(&ack).await
    }
    
    
    async fn handle_text(&mut self, text: Bytes) -> Result<(), Error> {
        let text_str = std::str::from_utf8(&text)
            .map_err(|e| actix_web::error::ErrorBadRequest(e))?;
        
        tracing::debug!("Received WebSocket message: {}", text_str);
        
        match serde_json::from_str::<WsMessage>(text_str) {
            Ok(msg) => {
                match msg {
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
                        tracing::warn!("Unexpected message type from client");
                        let error = WsMessage::Error {
                            message: "Unexpected message type".to_string(),
                            code: Some("INVALID_MESSAGE_TYPE".to_string()),
                        };
                        self.send_message(&error).await?;
                    }
                }
            }
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

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    
    tracing::info!("WebSocket connection established from: {:?}", req.peer_addr());
    

    actix_rt::spawn(async move {
        if let Err(e) = handle_connection(&mut session, &mut msg_stream).await {
            tracing::error!("WebSocket connection error: {}", e);
        }
    });
    
    Ok(res)
}


async fn handle_connection(
    session: &mut Session,
    msg_stream: &mut MessageStream,
) -> Result<(), Error> {
    let mut conn = WsConnection::new(session.clone());
    

    let welcome = WsMessage::Connected {
        message: "Connected to Vault Management System".to_string(),
    };
    conn.send_message(&welcome).await?;
    

    let mut heartbeat_interval = interval(HEARTBEAT_INTERVAL);
    
    loop {
        tokio::select! {

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
                        tracing::info!("Client closed connection: {:?}", reason);
                        break;
                    }
                    
                    _ => {}
                }
            }
            

            _ = heartbeat_interval.tick() => {

                if Instant::now().duration_since(conn.last_heartbeat) > CLIENT_TIMEOUT {
                    tracing::warn!("Client heartbeat timeout, closing connection");
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
    
    tracing::info!(
        "WebSocket connection closed (was subscribed to {} vaults)",
        conn.subscribed_vaults.len()
    );
    
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
    
    tracing::debug!("Broadcasting balance update: {:?}", update);
    
    // TODO: In production, broadcast to all subscribed clients
    // This would require a global client registry like:
    // WS_REGISTRY.broadcast_to_vault(vault_pubkey, update).await;
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
    
    tracing::debug!("Broadcasting deposit: {:?}", notification);
}

/// Broadcast withdrawal notification
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
    
    tracing::debug!("Broadcasting withdrawal: {:?}", notification);
}

/// Broadcast lock event
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
    
    tracing::debug!("Broadcasting lock: {:?}", notification);
}

/// Broadcast unlock event
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
    
    tracing::debug!("Broadcasting unlock: {:?}", notification);
}

/// Broadcast TVL update to all clients
pub async fn broadcast_tvl_update(
    total_vaults: i64,
    total_value_locked: i64,
) {
    let update = WsMessage::TvlUpdate {
        total_vaults,
        total_value_locked,
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    tracing::debug!("Broadcasting TVL update: {:?}", update);
}

/// Broadcast alert notification
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
    
    tracing::debug!("Broadcasting alert: {:?}", notification);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Connected {
            message: "Test".to_string(),
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("Test"));
    }
    
    #[test]
    fn test_ws_message_deserialization() {
        let json = r#"{"type":"subscribe","vault_pubkey":"test123"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            WsMessage::Subscribe { vault_pubkey } => {
                assert_eq!(vault_pubkey, "test123");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
