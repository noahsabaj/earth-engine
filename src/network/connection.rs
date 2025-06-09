use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::io::{Read, Write, ErrorKind};
use std::collections::VecDeque;
use crate::network::{Packet, PacketType, Protocol, KEEPALIVE_INTERVAL, CONNECTION_TIMEOUT};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Authenticated,
    Disconnecting,
    Disconnected,
}

/// Represents a network connection (client or server side)
pub struct Connection {
    /// TCP stream for reliable packets
    tcp_stream: TcpStream,
    /// UDP socket for unreliable packets
    udp_socket: Option<Arc<UdpSocket>>,
    /// Remote address
    remote_addr: SocketAddr,
    /// Connection state
    state: ConnectionState,
    /// Player ID (assigned after authentication)
    player_id: Option<u32>,
    /// Username
    username: Option<String>,
    /// Last activity time
    last_activity: Instant,
    /// Outgoing packet queue
    send_queue: VecDeque<(Packet, PacketType)>,
    /// Incoming packet buffer
    recv_buffer: Vec<u8>,
    /// Statistics
    stats: ConnectionStats,
}

/// Connection statistics
#[derive(Debug, Default)]
pub struct ConnectionStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub ping_ms: u32,
}

impl Connection {
    /// Create a new connection from a TCP stream
    pub fn new(tcp_stream: TcpStream, remote_addr: SocketAddr) -> std::io::Result<Self> {
        // Set non-blocking mode
        tcp_stream.set_nonblocking(true)?;
        tcp_stream.set_nodelay(true)?; // Disable Nagle's algorithm
        
        Ok(Self {
            tcp_stream,
            udp_socket: None,
            remote_addr,
            state: ConnectionState::Connecting,
            player_id: None,
            username: None,
            last_activity: Instant::now(),
            send_queue: VecDeque::new(),
            recv_buffer: Vec::with_capacity(8192),
            stats: ConnectionStats::default(),
        })
    }
    
    /// Set the UDP socket for this connection
    pub fn set_udp_socket(&mut self, socket: Arc<UdpSocket>) {
        self.udp_socket = Some(socket);
    }
    
    /// Get the connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }
    
    /// Set the connection state
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
    
    /// Get the player ID
    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }
    
    /// Set the player ID
    pub fn set_player_id(&mut self, id: u32) {
        self.player_id = Some(id);
    }
    
    /// Get the username
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
    
    /// Set the username
    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }
    
    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
    
    /// Check if connection has timed out
    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() > CONNECTION_TIMEOUT
    }
    
    /// Send a packet
    pub fn send_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        let packet_type = packet.packet_type();
        self.send_queue.push_back((packet, packet_type));
        Ok(())
    }
    
    /// Process outgoing packets
    pub fn process_send_queue(&mut self) -> std::io::Result<()> {
        while let Some((packet, packet_type)) = self.send_queue.pop_front() {
            match packet_type {
                PacketType::Reliable => self.send_tcp_packet(&packet)?,
                PacketType::Unreliable => self.send_udp_packet(&packet)?,
            }
        }
        Ok(())
    }
    
    /// Send a packet via TCP
    fn send_tcp_packet(&mut self, packet: &Packet) -> std::io::Result<()> {
        let data = packet.to_bytes()
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
        
        // Send length prefix (4 bytes)
        let len = data.len() as u32;
        self.tcp_stream.write_all(&len.to_be_bytes())?;
        
        // Send packet data
        self.tcp_stream.write_all(&data)?;
        
        self.stats.packets_sent += 1;
        self.stats.bytes_sent += data.len() as u64 + 4;
        
        Ok(())
    }
    
    /// Send a packet via UDP
    fn send_udp_packet(&mut self, packet: &Packet) -> std::io::Result<()> {
        if let Some(udp_socket) = &self.udp_socket {
            let data = packet.to_bytes()
                .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
            
            udp_socket.send_to(&data, self.remote_addr)?;
            
            self.stats.packets_sent += 1;
            self.stats.bytes_sent += data.len() as u64;
        }
        
        Ok(())
    }
    
    /// Receive packets from TCP
    pub fn receive_tcp_packets(&mut self) -> std::io::Result<Vec<Packet>> {
        let mut packets = Vec::new();
        
        // Read data into buffer
        let mut temp_buffer = [0u8; 4096];
        loop {
            match self.tcp_stream.read(&mut temp_buffer) {
                Ok(0) => {
                    // Connection closed
                    self.state = ConnectionState::Disconnected;
                    break;
                }
                Ok(n) => {
                    self.recv_buffer.extend_from_slice(&temp_buffer[..n]);
                    self.stats.bytes_received += n as u64;
                    self.last_activity = Instant::now();
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No more data available
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        
        // Process complete packets
        while self.recv_buffer.len() >= 4 {
            // Read length prefix
            let len = u32::from_be_bytes([
                self.recv_buffer[0],
                self.recv_buffer[1],
                self.recv_buffer[2],
                self.recv_buffer[3],
            ]) as usize;
            
            if len > crate::network::protocol::MAX_PACKET_SIZE {
                // Invalid packet size
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Packet too large",
                ));
            }
            
            if self.recv_buffer.len() < 4 + len {
                // Not enough data yet
                break;
            }
            
            // Extract packet data
            let packet_data = &self.recv_buffer[4..4 + len];
            match Packet::from_bytes(packet_data) {
                Ok(packet) => {
                    packets.push(packet);
                    self.stats.packets_received += 1;
                }
                Err(e) => {
                    // Invalid packet
                    eprintln!("Failed to deserialize packet: {}", e);
                }
            }
            
            // Remove processed data
            self.recv_buffer.drain(..4 + len);
        }
        
        Ok(packets)
    }
    
    /// Update connection statistics
    pub fn update_ping(&mut self, ping_ms: u32) {
        self.stats.ping_ms = ping_ms;
    }
    
    /// Get connection statistics
    pub fn stats(&self) -> &ConnectionStats {
        &self.stats
    }
    
    /// Close the connection
    pub fn close(&mut self) {
        self.state = ConnectionState::Disconnected;
        let _ = self.tcp_stream.shutdown(std::net::Shutdown::Both);
    }
}

/// Connection manager for handling multiple connections
pub struct ConnectionManager {
    connections: Vec<Connection>,
    next_player_id: u32,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            next_player_id: 1,
        }
    }
    
    /// Add a new connection
    pub fn add_connection(&mut self, mut connection: Connection) -> u32 {
        let player_id = self.next_player_id;
        self.next_player_id += 1;
        
        connection.set_player_id(player_id);
        self.connections.push(connection);
        
        player_id
    }
    
    /// Remove a connection
    pub fn remove_connection(&mut self, player_id: u32) {
        self.connections.retain(|c| c.player_id() != Some(player_id));
    }
    
    /// Get a connection by player ID
    pub fn get_connection(&mut self, player_id: u32) -> Option<&mut Connection> {
        self.connections.iter_mut()
            .find(|c| c.player_id() == Some(player_id))
    }
    
    /// Get all connections
    pub fn connections(&mut self) -> &mut Vec<Connection> {
        &mut self.connections
    }
    
    /// Process all connections
    pub fn process_all(&mut self) -> Vec<u32> {
        let mut disconnected = Vec::new();
        
        // Process each connection
        for conn in &mut self.connections {
            // Check for timeout
            if conn.is_timed_out() {
                conn.close();
                if let Some(id) = conn.player_id() {
                    disconnected.push(id);
                }
                continue;
            }
            
            // Process send queue
            if let Err(e) = conn.process_send_queue() {
                eprintln!("Failed to send packets: {}", e);
                conn.close();
                if let Some(id) = conn.player_id() {
                    disconnected.push(id);
                }
            }
        }
        
        // Remove disconnected connections
        for id in &disconnected {
            self.remove_connection(*id);
        }
        
        disconnected
    }
}