use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use actix_ws::{Message, Session};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WebRTCMessage {
    #[serde(rename = "role")]
    Role { role: String },
    #[serde(rename = "offer")]
    Offer { data: serde_json::Value },
    #[serde(rename = "answer")]
    Answer { data: serde_json::Value },
    #[serde(rename = "candidate")]
    Candidate { data: serde_json::Value },
    #[serde(rename = "peer_disconnected")]
    PeerDisconnected,
    #[serde(rename = "error")]
    ErrorMessage { message: String },
    #[serde(rename = "sender_ready")]
    SenderReady,
    #[serde(rename = "session_expired")]
    SessionExpired,
}

pub struct WebRTCRoom {
    pub sessions: Vec<(usize, Session)>,
    pub created_at: DateTime<Utc>,
}

static NEXT_SESSION_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

pub type WebRTCRooms = Arc<RwLock<HashMap<String, WebRTCRoom>>>;

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    rooms: web::Data<WebRTCRooms>,
) -> Result<HttpResponse, Error> {
    let slug = req.match_info().get("slug").unwrap_or("default").to_string();
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let mut rooms_write = rooms.write().unwrap_or_else(|e| e.into_inner());
    let room = rooms_write.entry(slug.clone()).or_insert(WebRTCRoom {
        sessions: Vec::new(),
        created_at: Utc::now(),
    });

    if room.sessions.len() >= 2 {
        let _ = session.text(serde_json::to_string(&WebRTCMessage::ErrorMessage {
            message: "Room full (max 2 peers)".to_string(),
        }).unwrap()).await;
        let _ = session.close(None).await;
        return Ok(res);
    }

    // Assign role
    let role = if room.sessions.is_empty() {
        "initiator"
    } else {
        "receiver"
    };

    let _ = session.text(serde_json::to_string(&WebRTCMessage::Role {
        role: role.to_string(),
    }).unwrap()).await;

    let session_id = NEXT_SESSION_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    room.sessions.push((session_id, session.clone()));
    drop(rooms_write);

    let rooms_clone = rooms.clone();
    let slug_clone = slug.clone();
    let session_clone = session.clone();

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    // Defensive parsing
                    if let Ok(incoming) = serde_json::from_str::<serde_json::Value>(&text) {
                        let msg_type = incoming.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        
                        if matches!(msg_type, "offer" | "answer" | "candidate" | "sender_ready") {
                            // Relay to the other peer
                            let rooms_read = rooms_clone.read().unwrap_or_else(|e| e.into_inner());
                            if let Some(room) = rooms_read.get(&slug_clone) {
                                for (other_id, other_session) in &room.sessions {
                                    if *other_id != session_id {
                                        let mut s = other_session.clone();
                                        let _ = s.text(text.clone()).await;
                                    }
                                }
                            }
                        }
                    }
                }
                Message::Ping(bytes) => {
                    let _ = session_clone.clone().pong(&bytes).await;
                }
                Message::Close(_) => break,
                _ => (),
            }
        }

        // Cleanup on disconnect
        let mut rooms_write = rooms_clone.write().unwrap_or_else(|e| e.into_inner());
        if let Some(room) = rooms_write.get_mut(&slug_clone) {
            room.sessions.retain(|(id, _)| *id != session_id);
            
            // Notify remaining peer
            for (_, remaining_session) in &room.sessions {
                let mut s = remaining_session.clone();
                let _ = s.text(serde_json::to_string(&WebRTCMessage::PeerDisconnected).unwrap()).await;
            }

            if room.sessions.is_empty() {
                rooms_write.remove(&slug_clone);
            }
        }
    });

    Ok(res)
}

#[get("/api/webrtc/turn-credentials")]
pub async fn turn_credentials() -> impl Responder {
    // For production, this should return credentials from a TURN provider (e.g., Twilio, Metered.ca)
    // For now, we return standard Google STUN servers.
    let response = serde_json::json!({
        "iceServers": [
            { "urls": "stun:stun.l.google.com:19302" },
            { "urls": "stun:stun1.l.google.com:19302" },
            { "urls": "stun:stun2.l.google.com:19302" },
            { "urls": "stun:stun3.l.google.com:19302" },
            { "urls": "stun:stun4.l.google.com:19302" }
        ]
    });
    HttpResponse::Ok().json(response)
}
