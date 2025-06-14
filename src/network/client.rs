#![allow(unused_variables, dead_code)]
use std::net::{TcpStream, UdpSocket, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::network::{
    Connection,
    Packet, ClientPacket, ServerPacket,
    MovementState,
    DEFAULT_UDP_PORT,
    NetworkResult, NetworkErrorContext, connection_error,
};
use crate::network::connection::{
    connection_set_udp_socket, connection_send_packet, connection_close,
    connection_process_send_queue, connection_receive_tcp_packets,
};
use crate::error::EngineError;
use crate::world::{BlockId, VoxelPos, ChunkPos, ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator};
use crate::ecs::EcsWorldData;
use glam::{Vec3, Quat};

/// Client state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
    InGame,
}

/// Remote player data
#[derive(Clone)]
pub struct RemotePlayer {
    pub id: u32,
    pub username: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub movement_state: MovementState,
    pub last_update: Instant,
}

/// Network client
pub struct Client {
    state: Arc<Mutex<ClientState>>,
    connection: Option<Arc<Mutex<Connection>>>,
    player_id: Option<u32>,
    username: String,
    
    // Game state
    world: Arc<ParallelWorld>,
    ecs_world: Arc<Mutex<EcsWorldData>>,
    remote_players: Arc<Mutex<HashMap<u32, RemotePlayer>>>,
    
    // Local player state
    position: Arc<Mutex<Vec3>>,
    rotation: Arc<Mutex<Quat>>,
    velocity: Arc<Mutex<Vec3>>,
    movement_state: Arc<Mutex<MovementState>>,
    input_sequence: Arc<Mutex<u32>>,
    
    // Network stats
    ping_ms: Arc<Mutex<u32>>,
    last_ping: Arc<Mutex<Instant>>,
    
    // Thread communication
    packet_tx: Sender<Packet>,
    packet_rx: Receiver<Packet>,
}

impl Client {
    /// Create a new client
    pub fn new(username: String) -> Self {
        let (packet_tx, packet_rx) = channel();
        
        Self {
            state: Arc::new(Mutex::new(ClientState::Disconnected)),
            connection: None,
            player_id: None,
            username,
            // Create parallel world for client
            world: {
                let generator = Box::new(DefaultWorldGenerator::new(
                    12345, // seed (should match server)
                    BlockId(1), // grass
                    BlockId(2), // dirt
                    BlockId(3), // stone
                    BlockId(4), // water
                    BlockId(5), // sand
                ));
                let config = ParallelWorldConfig {
                    generation_threads: num_cpus::get().saturating_sub(2).max(2),
                    mesh_threads: num_cpus::get().saturating_sub(2).max(2),
                    chunks_per_frame: num_cpus::get() * 2,
                    view_distance: 8,
                    chunk_size: 32,
                };
                Arc::new(ParallelWorld::new(generator, config))
            },
            ecs_world: Arc::new(Mutex::new(EcsWorldData::new())),
            remote_players: Arc::new(Mutex::new(HashMap::new())),
            position: Arc::new(Mutex::new(Vec3::ZERO)),
            rotation: Arc::new(Mutex::new(Quat::IDENTITY)),
            velocity: Arc::new(Mutex::new(Vec3::ZERO)),
            movement_state: Arc::new(Mutex::new(MovementState::Normal)),
            input_sequence: Arc::new(Mutex::new(0)),
            ping_ms: Arc::new(Mutex::new(0)),
            last_ping: Arc::new(Mutex::new(Instant::now())),
            packet_tx,
            packet_rx,
        }
    }
    
    
    
    /// Get current state
    pub fn state(&self) -> NetworkResult<ClientState> {
        Ok(*self.state.lock().network_context("state")?)
    }
    
    /// Get player ID
    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }
    
    /// Get ping in milliseconds
    pub fn ping_ms(&self) -> NetworkResult<u32> {
        Ok(*self.ping_ms.lock().network_context("ping_ms")?)
    }
    
    
    
    
    
    
    
    /// Start receive thread
    fn start_receive_thread(&self, connection: Arc<Mutex<Connection>>) {
        let packet_tx = self.packet_tx.clone();
        let state = Arc::clone(&self.state);
        
        thread::spawn(move || {
            loop {
                // Check if still connected
                match state.lock() {
                    Ok(guard) => {
                        if *guard == ClientState::Disconnected {
                            break;
                        }
                    }
                    Err(_) => {
                        eprintln!("Failed to acquire state lock in receive thread");
                        break;
                    }
                }
                
                // Receive packets
                let packets = {
                    match connection.lock() {
                        Ok(mut conn) => {
                            match connection_receive_tcp_packets(&mut conn) {
                                Ok(packets) => packets,
                                Err(e) => {
                                    eprintln!("Failed to receive packets: {}", e);
                                    if let Ok(mut guard) = state.lock() {
                                        *guard = ClientState::Disconnected;
                                    }
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("Failed to acquire connection lock in receive thread");
                            break;
                        }
                    }
                };
                
                // Forward packets to main thread
                for packet in packets {
                    if packet_tx.send(packet).is_err() {
                        // Channel closed
                        break;
                    }
                }
                
                thread::sleep(Duration::from_millis(1));
            }
        });
    }
    
    /// Send packet to server
    fn send_packet(&self, packet: Packet) -> NetworkResult<()> {
        if let Some(connection) = &self.connection {
            let mut conn = connection.lock().network_context("connection")?;
            if let Err(e) = connection_send_packet(&mut conn, packet) {
                eprintln!("Failed to send packet: {}", e);
            }
        }
        Ok(())
    }
    
    
    /// Get remote players
    pub fn get_remote_players(&self) -> NetworkResult<HashMap<u32, RemotePlayer>> {
        Ok(self.remote_players.lock()
            .network_context("remote_players")?
            .clone())
    }
    
    /// Get world reference
    pub fn world(&self) -> Arc<ParallelWorld> {
        Arc::clone(&self.world)
    }
}

/// Connect to a server (DOP function)
pub fn client_connect<A: ToSocketAddrs>(client: &mut Client, addr: A) -> NetworkResult<()> {
    // Resolve address
    let socket_addr = addr.to_socket_addrs()
        .map_err(|e| connection_error("<address>", e))?
        .next()
        .ok_or_else(|| EngineError::ConnectionFailed {
            addr: "<address>".to_string(),
            error: "No valid address found".to_string(),
        })?;
    
    // Update state
    *client.state.lock().network_context("state")? = ClientState::Connecting;
    
    // Connect TCP
    let tcp_stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
        .map_err(|e| connection_error(&socket_addr.to_string(), e))?;
    
    // Create connection
    let mut connection = Connection::new(tcp_stream, socket_addr)
        .map_err(|e| connection_error(&socket_addr.to_string(), e))?;
    
    // Bind UDP socket
    let local_addr = format!("0.0.0.0:{}", DEFAULT_UDP_PORT);
    let udp_socket = UdpSocket::bind(&local_addr)
        .map_err(|e| connection_error(&local_addr, e))?;
    udp_socket.set_nonblocking(true)
        .map_err(|e| EngineError::IoError {
            path: local_addr.clone(),
            error: format!("Failed to set non-blocking: {}", e),
        })?;
    
    // Connect UDP to server
    let server_udp_addr = SocketAddr::new(socket_addr.ip(), DEFAULT_UDP_PORT);
    udp_socket.connect(server_udp_addr)
        .map_err(|e| connection_error(&server_udp_addr.to_string(), e))?;
    
    connection_set_udp_socket(&mut connection, Arc::new(udp_socket));
    
    // Send connect packet
    connection_send_packet(&mut connection, Packet::Client(ClientPacket::Connect {
        protocol_version: crate::network::PROTOCOL_VERSION,
        username: client.username.clone(),
        password: None,
    })).map_err(|e| EngineError::ProtocolError {
        message: format!("Failed to send connect packet: {}", e),
    })?;
    
    // Store connection
    let connection = Arc::new(Mutex::new(connection));
    client.connection = Some(Arc::clone(&connection));
    
    // Start receive thread
    client_start_receive_thread(client, connection);
    
    // Update state
    *client.state.lock().network_context("state")? = ClientState::Connected;
    
    Ok(())
}

/// Disconnect from server (DOP function)
pub fn client_disconnect(client: &mut Client) -> NetworkResult<()> {
    if let Some(connection) = &client.connection {
        let mut conn = connection.lock().network_context("connection")?;
        let _ = connection_send_packet(&mut conn, Packet::Client(ClientPacket::Disconnect {
            reason: "Client disconnecting".to_string(),
        }));
        connection_close(&mut conn);
    }
    
    *client.state.lock().network_context("state")? = ClientState::Disconnected;
    client.connection = None;
    client.player_id = None;
    client.remote_players.lock().network_context("remote_players")?.clear();
    Ok(())
}

/// Update local player state and send to server (DOP function)
pub fn client_update_player(client: &mut Client, position: Vec3, rotation: Quat, velocity: Vec3, movement_state: MovementState) -> NetworkResult<()> {
    *client.position.lock().network_context("position")? = position;
    *client.rotation.lock().network_context("rotation")? = rotation;
    *client.velocity.lock().network_context("velocity")? = velocity;
    *client.movement_state.lock().network_context("movement_state")? = movement_state;
    
    // Increment sequence
    let sequence = {
        let mut seq = client.input_sequence.lock().network_context("input_sequence")?;
        *seq += 1;
        *seq
    };
    
    // Send to server
    client_send_packet(client, Packet::Client(ClientPacket::PlayerInput {
        position,
        rotation,
        velocity,
        movement_state,
        sequence,
    }))?;
    Ok(())
}

/// Request to break a block (DOP function)
pub fn client_break_block(client: &mut Client, position: VoxelPos) -> NetworkResult<()> {
    let sequence = {
        let mut seq = client.input_sequence.lock().network_context("input_sequence")?;
        *seq += 1;
        *seq
    };
    
    client_send_packet(client, Packet::Client(ClientPacket::BlockBreak {
        position,
        sequence,
    }))?;
    Ok(())
}

/// Request to place a block (DOP function)
pub fn client_place_block(client: &mut Client, position: VoxelPos, block_id: BlockId, face: crate::network::packet::BlockFace) -> NetworkResult<()> {
    let sequence = {
        let mut seq = client.input_sequence.lock().network_context("input_sequence")?;
        *seq += 1;
        *seq
    };
    
    client_send_packet(client, Packet::Client(ClientPacket::BlockPlace {
        position,
        block_id,
        face,
        sequence,
    }))?;
    Ok(())
}

/// Send chat message (DOP function)
pub fn client_send_chat(client: &mut Client, message: String) -> NetworkResult<()> {
    client_send_packet(client, Packet::Client(ClientPacket::ChatMessage {
        message,
    }))?;
    Ok(())
}

/// Request chunk data (DOP function)
pub fn client_request_chunk(client: &mut Client, chunk_pos: ChunkPos) -> NetworkResult<()> {
    client_send_packet(client, Packet::Client(ClientPacket::ChunkRequest {
        chunk_pos,
    }))?;
    Ok(())
}

/// Process network updates (DOP function)
pub fn client_update(client: &mut Client) -> NetworkResult<()> {
    // Process received packets
    while let Ok(packet) = client.packet_rx.try_recv() {
        client_handle_packet(client, packet)?;
    }
    
    // Send keepalive if needed
    let now = Instant::now();
    let should_ping = {
        let last_ping = client.last_ping.lock().network_context("last_ping")?;
        now.duration_since(*last_ping) > Duration::from_secs(5)
    };
    
    if should_ping {
        *client.last_ping.lock().network_context("last_ping")? = now;
        
        let timestamp = now.elapsed().as_millis() as u64;
        client_send_packet(client, Packet::Client(ClientPacket::Ping {
            timestamp,
        }))?;
    }
    
    // Process send queue
    if let Some(connection) = &client.connection {
        let mut conn = connection.lock().network_context("connection")?;
        let _ = connection_process_send_queue(&mut conn);
    }
    Ok(())
}

/// Start receive thread (DOP function)
pub fn client_start_receive_thread(client: &Client, connection: Arc<Mutex<Connection>>) {
    let packet_tx = client.packet_tx.clone();
    let state = Arc::clone(&client.state);
    
    thread::spawn(move || {
        loop {
            // Check if still connected
            match state.lock() {
                Ok(guard) => {
                    if *guard == ClientState::Disconnected {
                        break;
                    }
                }
                Err(_) => {
                    eprintln!("Failed to acquire state lock in receive thread");
                    break;
                }
            }
            
            // Receive packets
            let packets = {
                match connection.lock() {
                    Ok(mut conn) => {
                        match connection_receive_tcp_packets(&mut conn) {
                            Ok(packets) => packets,
                            Err(e) => {
                                eprintln!("Failed to receive packets: {}", e);
                                if let Ok(mut guard) = state.lock() {
                                    *guard = ClientState::Disconnected;
                                }
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("Failed to acquire connection lock in receive thread");
                        break;
                    }
                }
            };
            
            // Forward packets to main thread
            for packet in packets {
                if packet_tx.send(packet).is_err() {
                    // Channel closed
                    break;
                }
            }
            
            thread::sleep(Duration::from_millis(1));
        }
    });
}

/// Send packet to server (DOP function)
pub fn client_send_packet(client: &Client, packet: Packet) -> NetworkResult<()> {
    if let Some(connection) = &client.connection {
        let mut conn = connection.lock().network_context("connection")?;
        if let Err(e) = connection_send_packet(&mut conn, packet) {
            eprintln!("Failed to send packet: {}", e);
        }
    }
    Ok(())
}

/// Handle received packet (DOP function)
pub fn client_handle_packet(client: &mut Client, packet: Packet) -> NetworkResult<()> {
    match packet {
        Packet::Server(server_packet) => {
            match server_packet {
                ServerPacket::ConnectAccept { player_id, spawn_position, world_time } => {
                    client.player_id = Some(player_id);
                    *client.position.lock().network_context("position")? = spawn_position;
                    *client.state.lock().network_context("state")? = ClientState::InGame;
                    println!("Connected to server as player {}", player_id);
                }
                ServerPacket::ConnectReject { reason } => {
                    eprintln!("Connection rejected: {}", reason);
                    client_disconnect(client)?;
                }
                ServerPacket::PlayerJoin { player_id, username, position, rotation } => {
                    if Some(player_id) != client.player_id {
                        let player = RemotePlayer {
                            id: player_id,
                            username: username.clone(),
                            position,
                            rotation,
                            velocity: Vec3::ZERO,
                            movement_state: MovementState::Normal,
                            last_update: Instant::now(),
                        };
                        client.remote_players.lock()
                            .network_context("remote_players")?
                            .insert(player_id, player);
                        println!("Player {} joined", username);
                    }
                }
                ServerPacket::PlayerDisconnect { player_id, reason } => {
                    if let Some(player) = client.remote_players.lock()
                        .network_context("remote_players")?
                        .remove(&player_id) 
                    {
                        println!("Player {} disconnected: {}", player.username, reason);
                    }
                }
                ServerPacket::PlayerUpdates { updates, server_tick } => {
                    for update in updates {
                        if Some(update.player_id) != client.player_id {
                            if let Some(player) = client.remote_players.lock()
                                .network_context("remote_players")?
                                .get_mut(&update.player_id) 
                            {
                                player.position = update.position;
                                player.rotation = update.rotation;
                                player.velocity = update.velocity;
                                player.movement_state = update.movement_state;
                                player.last_update = Instant::now();
                            }
                        }
                    }
                }
                ServerPacket::BlockChange { position, block_id, sequence } => {
                    client.world.set_block(position, block_id);
                }
                ServerPacket::ChatBroadcast { player_id, username, message, timestamp } => {
                    println!("[{}] {}: {}", timestamp, username, message);
                }
                ServerPacket::ChunkData { chunk_pos, compressed_data } => {
                    // TODO: Decompress and load chunk
                    println!("Received chunk data for {:?}", chunk_pos);
                }
                ServerPacket::Pong { client_timestamp, server_timestamp } => {
                    let now = Instant::now().elapsed().as_millis() as u64;
                    let ping = (now - client_timestamp) as u32;
                    *client.ping_ms.lock().network_context("ping_ms")? = ping;
                }
                _ => {}
            }
        }
        _ => {
            eprintln!("Client received client packet");
        }
    }
    Ok(())
}