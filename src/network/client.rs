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
};
use crate::world::{World, BlockId, VoxelPos, ChunkPos};
use crate::ecs::EcsWorld;
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
    world: Arc<Mutex<World>>,
    ecs_world: Arc<Mutex<EcsWorld>>,
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
            world: Arc::new(Mutex::new(World::new(32))), // 32x32x32 chunks
            ecs_world: Arc::new(Mutex::new(EcsWorld::new())),
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
    
    /// Connect to a server
    pub fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), String> {
        // Resolve address
        let socket_addr = addr.to_socket_addrs()
            .map_err(|e| format!("Failed to resolve address: {}", e))?
            .next()
            .ok_or_else(|| "No valid address found".to_string())?;
        
        // Update state
        *self.state.lock().unwrap() = ClientState::Connecting;
        
        // Connect TCP
        let tcp_stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
            .map_err(|e| format!("Failed to connect: {}", e))?;
        
        // Create connection
        let mut connection = Connection::new(tcp_stream, socket_addr)
            .map_err(|e| format!("Failed to create connection: {}", e))?;
        
        // Bind UDP socket
        let local_addr = format!("0.0.0.0:{}", DEFAULT_UDP_PORT);
        let udp_socket = UdpSocket::bind(&local_addr)
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;
        udp_socket.set_nonblocking(true)
            .map_err(|e| format!("Failed to set UDP non-blocking: {}", e))?;
        
        // Connect UDP to server
        let server_udp_addr = SocketAddr::new(socket_addr.ip(), DEFAULT_UDP_PORT);
        udp_socket.connect(server_udp_addr)
            .map_err(|e| format!("Failed to connect UDP: {}", e))?;
        
        connection.set_udp_socket(Arc::new(udp_socket));
        
        // Send connect packet
        connection.send_packet(Packet::Client(ClientPacket::Connect {
            protocol_version: crate::network::PROTOCOL_VERSION,
            username: self.username.clone(),
            password: None,
        })).map_err(|e| format!("Failed to send connect packet: {}", e))?;
        
        // Store connection
        let connection = Arc::new(Mutex::new(connection));
        self.connection = Some(Arc::clone(&connection));
        
        // Start receive thread
        self.start_receive_thread(connection);
        
        // Update state
        *self.state.lock().unwrap() = ClientState::Connected;
        
        Ok(())
    }
    
    /// Disconnect from server
    pub fn disconnect(&mut self) {
        if let Some(connection) = &self.connection {
            let mut conn = connection.lock().unwrap();
            let _ = conn.send_packet(Packet::Client(ClientPacket::Disconnect {
                reason: "Client disconnecting".to_string(),
            }));
            conn.close();
        }
        
        *self.state.lock().unwrap() = ClientState::Disconnected;
        self.connection = None;
        self.player_id = None;
        self.remote_players.lock().unwrap().clear();
    }
    
    /// Get current state
    pub fn state(&self) -> ClientState {
        *self.state.lock().unwrap()
    }
    
    /// Get player ID
    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }
    
    /// Get ping in milliseconds
    pub fn ping_ms(&self) -> u32 {
        *self.ping_ms.lock().unwrap()
    }
    
    /// Update local player state and send to server
    pub fn update_player(&mut self, position: Vec3, rotation: Quat, velocity: Vec3, movement_state: MovementState) {
        *self.position.lock().unwrap() = position;
        *self.rotation.lock().unwrap() = rotation;
        *self.velocity.lock().unwrap() = velocity;
        *self.movement_state.lock().unwrap() = movement_state;
        
        // Increment sequence
        let sequence = {
            let mut seq = self.input_sequence.lock().unwrap();
            *seq += 1;
            *seq
        };
        
        // Send to server
        self.send_packet(Packet::Client(ClientPacket::PlayerInput {
            position,
            rotation,
            velocity,
            movement_state,
            sequence,
        }));
    }
    
    /// Request to break a block
    pub fn break_block(&mut self, position: VoxelPos) {
        let sequence = {
            let mut seq = self.input_sequence.lock().unwrap();
            *seq += 1;
            *seq
        };
        
        self.send_packet(Packet::Client(ClientPacket::BlockBreak {
            position,
            sequence,
        }));
    }
    
    /// Request to place a block
    pub fn place_block(&mut self, position: VoxelPos, block_id: BlockId, face: crate::network::packet::BlockFace) {
        let sequence = {
            let mut seq = self.input_sequence.lock().unwrap();
            *seq += 1;
            *seq
        };
        
        self.send_packet(Packet::Client(ClientPacket::BlockPlace {
            position,
            block_id,
            face,
            sequence,
        }));
    }
    
    /// Send chat message
    pub fn send_chat(&mut self, message: String) {
        self.send_packet(Packet::Client(ClientPacket::ChatMessage {
            message,
        }));
    }
    
    /// Request chunk data
    pub fn request_chunk(&mut self, chunk_pos: ChunkPos) {
        self.send_packet(Packet::Client(ClientPacket::ChunkRequest {
            chunk_pos,
        }));
    }
    
    /// Process network updates
    pub fn update(&mut self) {
        // Process received packets
        while let Ok(packet) = self.packet_rx.try_recv() {
            self.handle_packet(packet);
        }
        
        // Send keepalive if needed
        let now = Instant::now();
        if now.duration_since(*self.last_ping.lock().unwrap()) > Duration::from_secs(5) {
            *self.last_ping.lock().unwrap() = now;
            
            let timestamp = now.elapsed().as_millis() as u64;
            self.send_packet(Packet::Client(ClientPacket::Ping {
                timestamp,
            }));
        }
        
        // Process send queue
        if let Some(connection) = &self.connection {
            let mut conn = connection.lock().unwrap();
            let _ = conn.process_send_queue();
        }
    }
    
    /// Start receive thread
    fn start_receive_thread(&self, connection: Arc<Mutex<Connection>>) {
        let packet_tx = self.packet_tx.clone();
        let state = Arc::clone(&self.state);
        
        thread::spawn(move || {
            loop {
                // Check if still connected
                if *state.lock().unwrap() == ClientState::Disconnected {
                    break;
                }
                
                // Receive packets
                let packets = {
                    let mut conn = connection.lock().unwrap();
                    match conn.receive_tcp_packets() {
                        Ok(packets) => packets,
                        Err(e) => {
                            eprintln!("Failed to receive packets: {}", e);
                            *state.lock().unwrap() = ClientState::Disconnected;
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
    fn send_packet(&self, packet: Packet) {
        if let Some(connection) = &self.connection {
            let mut conn = connection.lock().unwrap();
            if let Err(e) = conn.send_packet(packet) {
                eprintln!("Failed to send packet: {}", e);
            }
        }
    }
    
    /// Handle received packet
    fn handle_packet(&mut self, packet: Packet) {
        match packet {
            Packet::Server(server_packet) => {
                match server_packet {
                    ServerPacket::ConnectAccept { player_id, spawn_position, world_time } => {
                        self.player_id = Some(player_id);
                        *self.position.lock().unwrap() = spawn_position;
                        *self.state.lock().unwrap() = ClientState::InGame;
                        println!("Connected to server as player {}", player_id);
                    }
                    ServerPacket::ConnectReject { reason } => {
                        eprintln!("Connection rejected: {}", reason);
                        self.disconnect();
                    }
                    ServerPacket::PlayerJoin { player_id, username, position, rotation } => {
                        if Some(player_id) != self.player_id {
                            let player = RemotePlayer {
                                id: player_id,
                                username: username.clone(),
                                position,
                                rotation,
                                velocity: Vec3::ZERO,
                                movement_state: MovementState::Normal,
                                last_update: Instant::now(),
                            };
                            self.remote_players.lock().unwrap().insert(player_id, player);
                            println!("Player {} joined", username);
                        }
                    }
                    ServerPacket::PlayerDisconnect { player_id, reason } => {
                        if let Some(player) = self.remote_players.lock().unwrap().remove(&player_id) {
                            println!("Player {} disconnected: {}", player.username, reason);
                        }
                    }
                    ServerPacket::PlayerUpdates { updates, server_tick } => {
                        for update in updates {
                            if Some(update.player_id) != self.player_id {
                                if let Some(player) = self.remote_players.lock().unwrap().get_mut(&update.player_id) {
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
                        self.world.lock().unwrap().set_block(position, block_id);
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
                        *self.ping_ms.lock().unwrap() = ping;
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Client received client packet");
            }
        }
    }
    
    /// Get remote players
    pub fn get_remote_players(&self) -> HashMap<u32, RemotePlayer> {
        self.remote_players.lock().unwrap().clone()
    }
    
    /// Get world reference
    pub fn world(&self) -> Arc<Mutex<World>> {
        Arc::clone(&self.world)
    }
}