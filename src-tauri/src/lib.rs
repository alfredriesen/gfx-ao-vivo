// src-tauri/lib.rs

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Spawn HTTP + WebSocket servers
            tauri::async_runtime::spawn(async {
                start_http_and_ws().await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

async fn start_http_and_ws() {
    // --- HTTP server for gfx-target ---
    let dist = warp::fs::dir("../gfx-target/dist");
    tauri::async_runtime::spawn(warp::serve(dist).run(([127, 0, 0, 1], 8080)));
    println!("HTTP server running at http://127.0.0.1:8080");

    // --- WebSocket server ---
    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("WebSocket server listening on ws://127.0.0.1:9000");

    // Shared list of clients (write sinks)
    let clients: Arc<Mutex<Vec<Arc<Mutex<futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        Message,
    >>>>>> = Arc::new(Mutex::new(Vec::new()));

    while let Ok((stream, _)) = listener.accept().await {
        // Accept only proper WebSocket connections
        match accept_async(stream).await {
            Ok(ws_stream) => {
                let (write, mut read) = ws_stream.split();
                let write = Arc::new(Mutex::new(write));

                // Add new client sink
                clients.lock().await.push(Arc::clone(&write));

                let clients_for_task = Arc::clone(&clients);
                let write_clone = Arc::clone(&write);

                tokio::spawn(async move {
                    while let Some(msg_result) = read.next().await {
                        match msg_result {
                            Ok(Message::Close(frame)) => {
                                println!("Client disconnected: {:?}", frame);
                                break;
                            }
                            Ok(msg) => {
                                // Only Text or Binary messages
                                let msg_to_send = match msg {
                                    Message::Text(t) => Message::Text(t.clone()),
                                    Message::Binary(b) => Message::Binary(b.clone()),
                                    _ => continue,
                                };

                                // Broadcast to all clients
                                let clients_lock = clients_for_task.lock().await;
                                for client in clients_lock.iter() {
                                    let mut client_lock = client.lock().await;
                                    let _ = client_lock.send(msg_to_send.clone()).await;
                                }
                            }
                            Err(e) => {
                                println!("WebSocket error: {:?}", e);
                                break;
                            }
                        }
                    }

                    // Remove disconnected client
                    let mut clients_lock = clients_for_task.lock().await;
                    clients_lock.retain(|c| !Arc::ptr_eq(c, &write_clone));
                });
            }
            Err(e) => {
                println!("Rejected non-WebSocket connection: {:?}", e);
            }
        }
    }
}
