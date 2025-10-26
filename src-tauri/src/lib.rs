// src-tauri/lib.rs

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::accept_async;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Spawn async servers
            tauri::async_runtime::spawn(async {
                start_http_and_ws().await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn start_http_and_ws() {
    // --- HTTP server for gfx-target ---
    let dist = warp::fs::dir("../gfx-target/dist");
    tauri::async_runtime::spawn(warp::serve(dist).run(([127, 0, 0, 1], 8080)));

    // --- WebSocket server ---
    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("WebSocket server listening on ws://127.0.0.1:9000");

    // Shared list of connected clients
    let clients: Arc<Mutex<Vec<Arc<Mutex<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>>>>> =
        Arc::new(Mutex::new(Vec::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let ws_stream = Arc::new(Mutex::new(ws_stream));

        // Add new client
        clients.lock().await.push(Arc::clone(&ws_stream));

        let clients_for_task = Arc::clone(&clients);
        let ws_clone = Arc::clone(&ws_stream);

        tokio::spawn(async move {
            let mut ws_guard = ws_clone.lock().await;
            while let Some(msg) = ws_guard.next().await {
                match msg {
                    Ok(tokio_tungstenite::tungstenite::Message::Close(frame)) => {
                        println!("Client disconnected: {:?}", frame);
                        break;
                    }
                    Ok(m) => {
                        println!("Received: {:?}", m);

                        // Broadcast to all clients
                        let clients_lock = clients_for_task.lock().await;
                        for client in clients_lock.iter() {
                            let mut client_guard = client.lock().await;
                            let _ = client_guard.send(m.clone()).await;
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
            clients_lock.retain(|c| !Arc::ptr_eq(c, &ws_clone));
        });
    }
}
