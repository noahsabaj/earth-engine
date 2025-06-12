use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::network::{
    Connection, ConnectionManager, ConnectionState,
    Packet, ClientPacket, ServerPacket, Protocol,
    DEFAULT_TCP_PORT, DEFAULT_UDP_PORT, TICK_DURATION,
    PlayerUpdateData, MovementState, BlockFace,
    NetworkResult, NetworkErrorContext, connection_error, protocol_error,
};
use crate::error::EngineError;
use crate::world::{BlockId, VoxelPos, ChunkPos, ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator};
use crate::ecs::EcsWorldData;
use glam::{Vec3, Quat};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub name: String,
    pub motd: String,
    pub max_players: u32,
    pub tcp_port: u16,
    pub udp_port: u16,
    pub world_seed: u64,
    pub view_distance: u32,
    pub enable_auth: bool,
    pub tick_rate: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "Earth Engine Server".to_string(),
            motd: "Welcome to Earth Engine!".to_string(),
            max_players: 20,
            tcp_port: DEFAULT_TCP_PORT,
            udp_port: DEFAULT_UDP_PORT,
            world_seed: 0,
            view_distance: 8,
            enable_auth: false,
            tick_rate: 20,
        }
    }
}

/// Player data on server
#[derive(Clone)]
pub struct ServerPlayer {
    pub id: u32,
    pub username: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub movement_state: MovementState,
    pub last_input_sequence: u32,
    pub loaded_chunks: Vec<ChunkPos>,
}

/// Server state
pub struct Server {
    config: ServerConfig,
    tcp_listener: Option<TcpListener>,
    udp_socket: Option<Arc<UdpSocket>>,
    connection_manager: Arc<Mutex<ConnectionManager>>,
    players: Arc<Mutex<HashMap<u32, ServerPlayer>>>,
    world: Arc<ParallelWorld>,
    ecs_world: Arc<Mutex<EcsWorldData>>,
    running: Arc<Mutex<bool>>,
    start_time: Instant,
    
    // Channels for thread communication
    packet_tx: Sender<(u32, Packet)>,
    packet_rx: Receiver<(u32, Packet)>,
}

impl Server {
    /// Create a new server
    pub fn new(config: ServerConfig) -> Self {
        let (packet_tx, packet_rx) = channel();
        
        Self {
            config,
            tcp_listener: None,
            udp_socket: None,
            connection_manager: Arc::new(Mutex::new(ConnectionManager::new())),
            players: Arc::new(Mutex::new(HashMap::new())),
            // Create parallel world for server
            world: {
                let generator = Box::new(DefaultWorldGenerator::new(
                    12345, // seed
                    BlockId(1), // grass
                    BlockId(2), // dirt
                    BlockId(3), // stone
                    BlockId(4), // water
                    BlockId(5), // sand
                ));
                let config = ParallelWorldConfig {
                    generation_threads: 2, // Use fewer threads on server
                    mesh_threads: 0, // Server doesn't need mesh threads
                    chunks_per_frame: 4,
                    view_distance: 8,
                    chunk_size: 32,
                };
                Arc::new(ParallelWorld::new(generator, config))
            },
            ecs_world: Arc::new(Mutex::new(EcsWorldData::new())),
            running: Arc::new(Mutex::new(false)),
            start_time: Instant::now(),
            packet_tx,
            packet_rx,
        }
    }
    
    /// Start the server
    pub fn start(&mut self) -> NetworkResult<()> {
        // Bind TCP listener
        let tcp_addr = format!("0.0.0.0:{}", self.config.tcp_port);
        self.tcp_listener = Some(
            TcpListener::bind(&tcp_addr)
                .map_err(|e| connection_error(&tcp_addr, e))?
        );
        println!("Server listening on TCP port {}", self.config.tcp_port);
        
        // Bind UDP socket
        let udp_addr = format!("0.0.0.0:{}", self.config.udp_port);
        let udp_socket = UdpSocket::bind(&udp_addr)
            .map_err(|e| connection_error(&udp_addr, e))?;
        udp_socket.set_nonblocking(true)
            .map_err(|e| EngineError::IoError {
                path: udp_addr.clone(),
                error: format!("Failed to set non-blocking: {}", e),
            })?;
        self.udp_socket = Some(Arc::new(udp_socket));
        println!("Server listening on UDP port {}", self.config.udp_port);
        
        // Set running flag
        *self.running.lock().network_context("running_flag")? = true;
        
        // Start network threads
        self.start_tcp_accept_thread()?;
        self.start_udp_receive_thread()?;
        
        // Start game loop
        self.run_game_loop()?;
        
        Ok(())
    }
    
    /// Stop the server
    pub fn stop(&mut self) -> NetworkResult<()> {
        *self.running.lock().network_context("running_flag")? = false;
        
        // Disconnect all players
        let players: Vec<u32> = self.players.lock()
            .network_context("players")?
            .keys()
            .cloned()
            .collect();
        for player_id in players {
            self.disconnect_player(player_id, "Server shutting down")?;
        }
        Ok(())
    }
    
    /// Start TCP accept thread
    fn start_tcp_accept_thread(&self) -> NetworkResult<()> {
        let listener = self.tcp_listener
            .as_ref()
            .ok_or_else(|| protocol_error("TCP listener not initialized"))?
            .try_clone()
            .map_err(|e| EngineError::IoError {
                path: "tcp_listener".to_string(),
                error: e.to_string(),
            })?;
        let connection_manager = Arc::clone(&self.connection_manager);
        let udp_socket = Arc::clone(
            self.udp_socket
                .as_ref()
                .ok_or_else(|| protocol_error("UDP socket not initialized"))?
        );
        let running = Arc::clone(&self.running);
        
        thread::spawn(move || {
            loop {
                // Check if we should stop
                if let Ok(guard) = running.lock() {
                    if !*guard {
                        break;
                    }
                } else {
                    eprintln!("Failed to acquire running lock in TCP accept thread");
                    break;
                }
                
                // Accept new connections
                match listener.accept() {
                    Ok((stream, addr)) => {
                        println!("New connection from {}", addr);
                        
                        // Create connection
                        match Connection::new(stream, addr) {
                            Ok(mut conn) => {
                                conn.set_udp_socket(Arc::clone(&udp_socket));
                                if let Ok(mut manager) = connection_manager.lock() {
                                    manager.add_connection(conn);
                                } else {
                                    eprintln!("Failed to acquire connection manager lock");
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to create connection: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            eprintln!("Failed to accept connection: {}", e);
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });
        Ok(())
    }
    
    /// Start UDP receive thread
    fn start_udp_receive_thread(&self) -> NetworkResult<()> {
        let udp_socket = Arc::clone(
            self.udp_socket
                .as_ref()
                .ok_or_else(|| protocol_error("UDP socket not initialized"))?
        );
        let packet_tx = self.packet_tx.clone();
        let running = Arc::clone(&self.running);
        let players = Arc::clone(&self.players);
        
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            
            loop {
                // Check if we should stop
                if let Ok(guard) = running.lock() {
                    if !*guard {
                        break;
                    }
                } else {
                    eprintln!("Failed to acquire running lock in UDP receive thread");
                    break;
                }
                
                // Receive UDP packets
                match udp_socket.recv_from(&mut buffer) {
                    Ok((len, addr)) => {
                        // Find player by address
                        let player_id = {
                            match players.lock() {
                                Ok(players) => {
                                    players.iter()
                                        .find(|(_, p)| {
                                            // TODO: Match by UDP address
                                            false
                                        })
                                        .map(|(id, _)| *id)
                                }
                                Err(_) => {
                                    eprintln!("Failed to acquire players lock in UDP thread");
                                    None
                                }
                            }
                        };
                        
                        if let Some(player_id) = player_id {
                            // Deserialize packet
                            match Packet::from_bytes(&buffer[..len]) {
                                Ok(packet) => {
                                    let _ = packet_tx.send((player_id, packet));
                                }
                                Err(e) => {
                                    eprintln!("Failed to deserialize UDP packet: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(e) => {
                        eprintln!("UDP receive error: {}", e);
                    }
                }
            }
        });
        Ok(())
    }
    
    /// Main game loop
    fn run_game_loop(&mut self) -> NetworkResult<()> {
        let mut last_tick = Instant::now();
        let mut tick_count = 0;
        
        loop {
            // Check if we should stop
            {
                let should_continue = self.running.lock()
                    .network_context("running_flag")?;
                if !*should_continue {
                    break;
                }
            }
            let now = Instant::now();
            let delta = now.duration_since(last_tick);
            
            if delta >= TICK_DURATION {
                // Process game tick
                self.tick(delta.as_secs_f32())?;
                
                last_tick = now;
                tick_count += 1;
                
                // Send periodic updates
                if tick_count % 2 == 0 { // Every 100ms
                    self.send_player_updates()?;
                }
                
                if tick_count % 100 == 0 { // Every 5 seconds
                    self.send_time_updates()?;
                }
            } else {
                // Sleep until next tick
                thread::sleep(TICK_DURATION - delta);
            }
        }
        Ok(())
    }
    
    /// Process one game tick
    fn tick(&mut self, delta_time: f32) -> NetworkResult<()> {
        // Process connections
        let disconnected = self.connection_manager
            .lock()
            .network_context("connection_manager")?
            .process_all();
        for player_id in disconnected {
            self.disconnect_player(player_id, "Connection lost")?;
        }
        
        // Process incoming packets
        while let Ok((player_id, packet)) = self.packet_rx.try_recv() {
            self.handle_packet(player_id, packet)?;
        }
        
        // Receive TCP packets from all connections
        let mut packets_to_handle = Vec::new();
        {
            let mut conn_manager = self.connection_manager
                .lock()
                .network_context("connection_manager")?;
            for conn in conn_manager.connections() {
                if let Some(player_id) = conn.player_id() {
                    match conn.receive_tcp_packets() {
                        Ok(packets) => {
                            for packet in packets {
                                packets_to_handle.push((player_id, packet));
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to receive packets from player {}: {}", player_id, e);
                        }
                    }
                }
            }
        }
        
        // Handle received packets
        for (player_id, packet) in packets_to_handle {
            self.handle_packet(player_id, packet)?;
        }
        
        // Update world
        // TODO: Implement world update logic
        // self.world.lock().network_context("world")?.update(delta_time);
        
        // Update ECS
        // TODO: Update ECS systems
        
        Ok(())
    }
    
    /// Handle incoming packet
    fn handle_packet(&mut self, player_id: u32, packet: Packet) -> NetworkResult<()> {
        match packet {
            Packet::Client(client_packet) => {
                match client_packet {
                    ClientPacket::Connect { protocol_version, username, password } => {
                        self.handle_connect(player_id, protocol_version, username, password)?;
                    }
                    ClientPacket::Disconnect { reason } => {
                        self.disconnect_player(player_id, &reason)?;
                    }
                    ClientPacket::PlayerInput { position, rotation, velocity, movement_state, sequence } => {
                        self.handle_player_input(player_id, position, rotation, velocity, movement_state, sequence)?;
                    }
                    ClientPacket::BlockBreak { position, sequence } => {
                        self.handle_block_break(player_id, position, sequence)?;
                    }
                    ClientPacket::BlockPlace { position, block_id, face, sequence } => {
                        self.handle_block_place(player_id, position, block_id, sequence)?;
                    }
                    ClientPacket::ChatMessage { message } => {
                        self.handle_chat_message(player_id, message)?;
                    }
                    ClientPacket::ChunkRequest { chunk_pos } => {
                        self.handle_chunk_request(player_id, chunk_pos)?;
                    }
                    ClientPacket::Ping { timestamp } => {
                        self.handle_ping(player_id, timestamp)?;
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Server received server packet from player {}", player_id);
            }
        }
        Ok(())
    }
    
    /// Handle connection request
    fn handle_connect(&mut self, player_id: u32, protocol_version: u32, username: String, password: Option<String>) -> NetworkResult<()> {
        // Check protocol version
        if protocol_version != crate::network::PROTOCOL_VERSION {
            self.send_to_player(player_id, Packet::Server(ServerPacket::ConnectReject {
                reason: format!("Protocol version mismatch (server: {}, client: {})", 
                    crate::network::PROTOCOL_VERSION, protocol_version)
            }))?;
            return Ok(());
        }
        
        // Validate username
        if let Err(e) = Protocol::validate_username(&username) {
            self.send_to_player(player_id, Packet::Server(ServerPacket::ConnectReject {
                reason: e,
            }))?;
            return Ok(());
        }
        
        // Check if username is already taken
        if self.players.lock().network_context("players")?.values().any(|p| p.username == username) {
            self.send_to_player(player_id, Packet::Server(ServerPacket::ConnectReject {
                reason: "Username already taken".to_string(),
            }))?;
            return Ok(());
        }
        
        // Check max players
        if self.players.lock().network_context("players")?.len() >= self.config.max_players as usize {
            self.send_to_player(player_id, Packet::Server(ServerPacket::ConnectReject {
                reason: "Server is full".to_string(),
            }))?;
            return Ok(());
        }
        
        // TODO: Check authentication if enabled
        
        // Accept connection
        let spawn_position = Vec3::new(0.0, 100.0, 0.0);
        
        // Update connection state
        if let Some(conn) = self.connection_manager.lock().network_context("connection_manager")?.get_connection(player_id) {
            conn.set_state(ConnectionState::Authenticated);
            conn.set_username(username.clone());
        }
        
        // Create player
        let player = ServerPlayer {
            id: player_id,
            username: username.clone(),
            position: spawn_position,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            movement_state: MovementState::Normal,
            last_input_sequence: 0,
            loaded_chunks: Vec::new(),
        };
        
        // Send accept packet
        self.send_to_player(player_id, Packet::Server(ServerPacket::ConnectAccept {
            player_id,
            spawn_position,
            world_time: 0.0, // TODO: Get actual world time
        }))?;
        
        // Notify other players
        self.broadcast_except(player_id, Packet::Server(ServerPacket::PlayerJoin {
            player_id,
            username: username.clone(),
            position: spawn_position,
            rotation: Quat::IDENTITY,
        }))?;
        
        // Send existing players to new player
        let existing_players: Vec<_> = self.players.lock()
            .network_context("players")?
            .iter()
            .map(|(id, p)| (*id, p.username.clone(), p.position, p.rotation))
            .collect();
        
        for (id, username, position, rotation) in existing_players {
            self.send_to_player(player_id, Packet::Server(ServerPacket::PlayerJoin {
                player_id: id,
                username,
                position,
                rotation,
            }))?;
        }
        
        // Add player
        self.players.lock()
            .network_context("players")?
            .insert(player_id, player);
        
        println!("Player {} ({}) connected", username, player_id);
        Ok(())
    }
    
    /// Handle player input
    fn handle_player_input(&mut self, player_id: u32, position: Vec3, rotation: Quat, 
                          velocity: Vec3, movement_state: MovementState, sequence: u32) -> NetworkResult<()> {
        if let Some(player) = self.players.lock().network_context("players")?.get_mut(&player_id) {
            // TODO: Validate movement (anti-cheat)
            
            // Update player state
            player.position = position;
            player.rotation = rotation;
            player.velocity = velocity;
            player.movement_state = movement_state;
            player.last_input_sequence = sequence;
        }
        Ok(())
    }
    
    /// Handle block break request
    fn handle_block_break(&mut self, player_id: u32, position: VoxelPos, sequence: u32) -> NetworkResult<()> {
        // TODO: Validate player can break this block
        
        // Break block
        self.world.set_block(position, BlockId::AIR);
        
        // Notify all players
        self.broadcast(Packet::Server(ServerPacket::BlockChange {
            position,
            block_id: BlockId::AIR,
            sequence,
        }))?;
        Ok(())
    }
    
    /// Handle block place request
    fn handle_block_place(&mut self, player_id: u32, position: VoxelPos, block_id: BlockId, sequence: u32) -> NetworkResult<()> {
        // TODO: Validate player can place this block
        
        // Place block
        self.world.set_block(position, block_id);
        
        // Notify all players
        self.broadcast(Packet::Server(ServerPacket::BlockChange {
            position,
            block_id,
            sequence,
        }))?;
        Ok(())
    }
    
    /// Handle chat message
    fn handle_chat_message(&mut self, player_id: u32, message: String) -> NetworkResult<()> {
        if let Err(e) = Protocol::validate_chat_message(&message) {
            // Invalid message
            return Ok(());
        }
        
        let (username, timestamp) = {
            let players = self.players.lock().network_context("players")?;
            if let Some(player) = players.get(&player_id) {
                (player.username.clone(), self.start_time.elapsed().as_millis() as u64)
            } else {
                return Ok(());
            }
        };
        
        // Broadcast to all players
        self.broadcast(Packet::Server(ServerPacket::ChatBroadcast {
            player_id: Some(player_id),
            username,
            message,
            timestamp,
        }))?;
        Ok(())
    }
    
    /// Handle chunk request
    fn handle_chunk_request(&mut self, player_id: u32, chunk_pos: ChunkPos) -> NetworkResult<()> {
        // TODO: Validate player should receive this chunk
        
        // Get chunk data
        if let Some(chunk_lock) = self.world.chunk_manager().get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            // TODO: Compress chunk data
            let compressed_data = Vec::new(); // Placeholder
            
            self.send_to_player(player_id, Packet::Server(ServerPacket::ChunkData {
                chunk_pos,
                compressed_data,
            }))?;
        }
        Ok(())
    }
    
    /// Handle ping
    fn handle_ping(&mut self, player_id: u32, client_timestamp: u64) -> NetworkResult<()> {
        let server_timestamp = self.start_time.elapsed().as_millis() as u64;
        
        self.send_to_player(player_id, Packet::Server(ServerPacket::Pong {
            client_timestamp,
            server_timestamp,
        }))?;
        
        // Update connection ping
        let ping_ms = (server_timestamp - client_timestamp) as u32;
        if let Some(conn) = self.connection_manager.lock()
            .network_context("connection_manager")?
            .get_connection(player_id) 
        {
            conn.update_ping(ping_ms);
        }
        Ok(())
    }
    
    /// Send player position updates
    fn send_player_updates(&mut self) -> NetworkResult<()> {
        let updates = {
            let players = self.players.lock().network_context("players")?;
            players.values().map(|p| PlayerUpdateData {
                player_id: p.id,
                position: p.position,
                rotation: p.rotation,
                velocity: p.velocity,
                movement_state: p.movement_state,
            }).collect::<Vec<_>>()
        };
        
        let server_tick = Protocol::calculate_tick(self.start_time);
        
        // Send to all players
        self.broadcast(Packet::Server(ServerPacket::PlayerUpdates {
            updates,
            server_tick,
        }))?;
        Ok(())
    }
    
    /// Send time updates
    fn send_time_updates(&mut self) -> NetworkResult<()> {
        let world_time = 0.0; // TODO: Get actual world time
        let day_cycle_time = 0.0; // TODO: Get day cycle time
        
        self.broadcast(Packet::Server(ServerPacket::TimeUpdate {
            world_time,
            day_cycle_time,
        }))?;
        Ok(())
    }
    
    /// Disconnect a player
    fn disconnect_player(&mut self, player_id: u32, reason: &str) -> NetworkResult<()> {
        if let Some(player) = self.players.lock()
            .network_context("players")?
            .remove(&player_id) 
        {
            println!("Player {} ({}) disconnected: {}", player.username, player_id, reason);
            
            // Notify other players
            self.broadcast_except(player_id, Packet::Server(ServerPacket::PlayerDisconnect {
                player_id,
                reason: reason.to_string(),
            }))?;
        }
        
        // Close connection
        if let Some(conn) = self.connection_manager.lock()
            .network_context("connection_manager")?
            .get_connection(player_id) 
        {
            conn.close();
        }
        Ok(())
    }
    
    /// Send packet to specific player
    fn send_to_player(&self, player_id: u32, packet: Packet) -> NetworkResult<()> {
        if let Some(conn) = self.connection_manager.lock()
            .network_context("connection_manager")?
            .get_connection(player_id) 
        {
            if let Err(e) = conn.send_packet(packet) {
                eprintln!("Failed to send packet to player {}: {}", player_id, e);
            }
        }
        Ok(())
    }
    
    /// Broadcast packet to all players
    fn broadcast(&self, packet: Packet) -> NetworkResult<()> {
        let players: Vec<u32> = self.players.lock()
            .network_context("players")?
            .keys()
            .cloned()
            .collect();
        for player_id in players {
            self.send_to_player(player_id, packet.clone())?;
        }
        Ok(())
    }
    
    /// Broadcast packet to all players except one
    fn broadcast_except(&self, except_id: u32, packet: Packet) -> NetworkResult<()> {
        let players: Vec<u32> = self.players.lock()
            .network_context("players")?
            .keys()
            .cloned()
            .collect();
        for player_id in players {
            if player_id != except_id {
                self.send_to_player(player_id, packet.clone())?;
            }
        }
        Ok(())
    }
}