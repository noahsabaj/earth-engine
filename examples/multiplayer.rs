use earth_engine::{
    network::{Server, ServerConfig, Client, ClientState, packet::MovementState},
    BlockId,
};
use std::thread;
use std::time::Duration;
use std::env;
use glam::{Vec3, Quat};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage:");
        println!("  {} server              - Start a server", args[0]);
        println!("  {} client <username>   - Connect as client", args[0]);
        return;
    }
    
    match args[1].as_str() {
        "server" => run_server(),
        "client" => {
            if args.len() < 3 {
                println!("Please provide a username!");
                return;
            }
            run_client(args[2].clone());
        }
        _ => {
            println!("Unknown mode: {}", args[1]);
        }
    }
}

fn run_server() {
    println!("=== Earth Engine Server ===");
    println!("Starting server on port 25565...\n");
    
    // Create server with default config
    let config = ServerConfig {
        name: "Test Server".to_string(),
        motd: "Welcome to the test server!".to_string(),
        max_players: 10,
        ..Default::default()
    };
    
    let mut server = Server::new(config);
    
    // Start server
    match server.start() {
        Ok(()) => {
            println!("Server started successfully!");
            println!("Players can connect to localhost:25565");
            println!("Press Ctrl+C to stop...\n");
            
            // Keep server running
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
        }
    }
}

fn run_client(username: String) {
    println!("=== Earth Engine Client ===");
    println!("Connecting as {}...\n", username);
    
    let mut client = Client::new(username);
    
    // Connect to localhost
    match client.connect("127.0.0.1:25565") {
        Ok(()) => {
            println!("Connected to server!");
            
            // Wait for authentication
            let mut connected = false;
            for _ in 0..50 { // 5 seconds timeout
                client.update();
                
                if client.state() == ClientState::InGame {
                    connected = true;
                    break;
                }
                
                thread::sleep(Duration::from_millis(100));
            }
            
            if !connected {
                eprintln!("Failed to authenticate with server");
                return;
            }
            
            println!("Successfully joined the game!");
            println!("Player ID: {:?}", client.player_id());
            
            // Send initial chat message
            client.send_chat(format!("{} joined the game!", username));
            
            // Simulate gameplay
            println!("\nSimulating player actions:");
            
            let mut position = Vec3::new(0.0, 100.0, 0.0);
            let rotation = Quat::IDENTITY;
            let mut time = 0.0f32;
            
            loop {
                // Update network
                client.update();
                
                // Simulate movement
                time += 0.1;
                position.x = time.sin() * 5.0;
                position.z = time.cos() * 5.0;
                
                // Send position update
                client.update_player(
                    position,
                    rotation,
                    Vec3::ZERO,
                    MovementState::Normal
                );
                
                // Every 5 seconds, perform an action
                if (time as i32) % 50 == 0 && (time * 10.0) as i32 % 10 == 0 {
                    match (time as i32 / 50) % 4 {
                        0 => {
                            // Send chat
                            client.send_chat("Hello from the client!".to_string());
                            println!("Sent chat message");
                        }
                        1 => {
                            // Place a block
                            let block_pos = earth_engine::world::VoxelPos {
                                x: position.x as i32,
                                y: (position.y - 2.0) as i32,
                                z: position.z as i32,
                            };
                            client.place_block(
                                block_pos,
                                BlockId(3), // Stone
                                earth_engine::network::packet::BlockFace::Top
                            );
                            println!("Placed block at {:?}", block_pos);
                        }
                        2 => {
                            // Break a block
                            let block_pos = earth_engine::world::VoxelPos {
                                x: position.x as i32,
                                y: (position.y - 2.0) as i32,
                                z: position.z as i32,
                            };
                            client.break_block(block_pos);
                            println!("Broke block at {:?}", block_pos);
                        }
                        _ => {
                            // Request chunk
                            let chunk_pos = earth_engine::world::ChunkPos {
                                x: (position.x / 32.0) as i32,
                                y: (position.y / 32.0) as i32,
                                z: (position.z / 32.0) as i32,
                            };
                            client.request_chunk(chunk_pos);
                            println!("Requested chunk {:?}", chunk_pos);
                        }
                    }
                }
                
                // Print remote players occasionally
                if (time as i32) % 100 == 0 && (time * 10.0) as i32 % 10 == 0 {
                    let players = client.get_remote_players();
                    if !players.is_empty() {
                        println!("\nRemote players:");
                        for (id, player) in players {
                            println!("  {} (ID: {}) at {:?}", 
                                player.username, id, player.position);
                        }
                    }
                }
                
                // Show ping
                if (time as i32) % 20 == 0 && (time * 10.0) as i32 % 10 == 0 {
                    println!("Ping: {}ms", client.ping_ms());
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use earth_engine::network::{Protocol, packet::*};
    
    #[test]
    fn test_protocol_validation() {
        // Test username validation
        assert!(Protocol::validate_username("Player123").is_ok());
        assert!(Protocol::validate_username("").is_err());
        assert!(Protocol::validate_username("a".repeat(20).as_str()).is_err());
        assert!(Protocol::validate_username("invalid name").is_err());
        
        // Test chat validation
        assert!(Protocol::validate_chat_message("Hello world!").is_ok());
        assert!(Protocol::validate_chat_message("").is_err());
        assert!(Protocol::validate_chat_message(&"a".repeat(300)).is_err());
    }
    
    #[test]
    fn test_packet_serialization() {
        let packet = Packet::Client(ClientPacket::Connect {
            protocol_version: 1,
            username: "TestPlayer".to_string(),
            password: None,
        });
        
        // Serialize
        let bytes = packet.to_bytes().unwrap();
        
        // Deserialize
        let decoded = Packet::from_bytes(&bytes).unwrap();
        
        match decoded {
            Packet::Client(ClientPacket::Connect { username, .. }) => {
                assert_eq!(username, "TestPlayer");
            }
            _ => panic!("Wrong packet type"),
        }
    }
    
    #[test]
    fn test_packet_types() {
        // Test reliable packets
        let connect = Packet::Client(ClientPacket::Connect {
            protocol_version: 1,
            username: "Test".to_string(),
            password: None,
        });
        assert_eq!(connect.packet_type(), PacketType::Reliable);
        
        // Test unreliable packets
        let input = Packet::Client(ClientPacket::PlayerInput {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            movement_state: MovementState::Normal,
            sequence: 1,
        });
        assert_eq!(input.packet_type(), PacketType::Unreliable);
    }
}