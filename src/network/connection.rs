use crate::network::{Packet, PacketType, CONNECTION_TIMEOUT};
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::Arc;
use std::time::Instant;

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

    /// Get the connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Get the player ID
    pub fn player_id(&self) -> Option<u32> {
        self.player_id
    }

    /// Get the username
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Check if connection has timed out
    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() > CONNECTION_TIMEOUT
    }

    /// Get connection statistics
    pub fn stats(&self) -> &ConnectionStats {
        &self.stats
    }
}

// DOP functions for Connection
/// Set the UDP socket for this connection
pub fn connection_set_udp_socket(connection: &mut Connection, socket: Arc<UdpSocket>) {
    connection.udp_socket = Some(socket);
}

/// Set the connection state
pub fn connection_set_state(connection: &mut Connection, state: ConnectionState) {
    connection.state = state;
}

/// Set the player ID
pub fn connection_set_player_id(connection: &mut Connection, id: u32) {
    connection.player_id = Some(id);
}

/// Set the username
pub fn connection_set_username(connection: &mut Connection, username: String) {
    connection.username = Some(username);
}

/// Send a packet
pub fn connection_send_packet(connection: &mut Connection, packet: Packet) -> std::io::Result<()> {
    let packet_type = packet.packet_type();
    connection.send_queue.push_back((packet, packet_type));
    Ok(())
}

/// Process outgoing packets
pub fn connection_process_send_queue(connection: &mut Connection) -> std::io::Result<()> {
    while let Some((packet, packet_type)) = connection.send_queue.pop_front() {
        match packet_type {
            PacketType::Reliable => connection_send_tcp_packet(connection, &packet)?,
            PacketType::Unreliable => connection_send_udp_packet(connection, &packet)?,
        }
    }
    Ok(())
}

/// Send a packet via TCP
fn connection_send_tcp_packet(connection: &mut Connection, packet: &Packet) -> std::io::Result<()> {
    let data = packet
        .to_bytes()
        .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;

    // Send length prefix (4 bytes)
    let len = data.len() as u32;
    connection.tcp_stream.write_all(&len.to_be_bytes())?;

    // Send packet data
    connection.tcp_stream.write_all(&data)?;

    connection.stats.packets_sent += 1;
    connection.stats.bytes_sent += data.len() as u64 + 4;

    Ok(())
}

/// Send a packet via UDP
fn connection_send_udp_packet(connection: &mut Connection, packet: &Packet) -> std::io::Result<()> {
    if let Some(udp_socket) = &connection.udp_socket {
        let data = packet
            .to_bytes()
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;

        udp_socket.send_to(&data, connection.remote_addr)?;

        connection.stats.packets_sent += 1;
        connection.stats.bytes_sent += data.len() as u64;
    }

    Ok(())
}

/// Receive packets from TCP
pub fn connection_receive_tcp_packets(connection: &mut Connection) -> std::io::Result<Vec<Packet>> {
    let mut packets = Vec::new();

    // Read data into buffer
    let mut temp_buffer = [0u8; 4096];
    loop {
        match connection.tcp_stream.read(&mut temp_buffer) {
            Ok(0) => {
                // Connection closed
                connection.state = ConnectionState::Disconnected;
                break;
            }
            Ok(n) => {
                connection.recv_buffer.extend_from_slice(&temp_buffer[..n]);
                connection.stats.bytes_received += n as u64;
                connection.last_activity = Instant::now();
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No more data available
                break;
            }
            Err(e) => return Err(e),
        }
    }

    // Process complete packets
    while connection.recv_buffer.len() >= 4 {
        // Read length prefix
        let len = u32::from_be_bytes([
            connection.recv_buffer[0],
            connection.recv_buffer[1],
            connection.recv_buffer[2],
            connection.recv_buffer[3],
        ]) as usize;

        if len > crate::network::protocol::MAX_PACKET_SIZE {
            // Invalid packet size
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Packet too large",
            ));
        }

        if connection.recv_buffer.len() < 4 + len {
            // Not enough data yet
            break;
        }

        // Extract packet data
        let packet_data = &connection.recv_buffer[4..4 + len];
        match Packet::from_bytes(packet_data) {
            Ok(packet) => {
                packets.push(packet);
                connection.stats.packets_received += 1;
            }
            Err(e) => {
                // Invalid packet
                eprintln!("Failed to deserialize packet: {}", e);
            }
        }

        // Remove processed data
        connection.recv_buffer.drain(..4 + len);
    }

    Ok(packets)
}

/// Update connection statistics
pub fn connection_update_ping(connection: &mut Connection, ping_ms: u32) {
    connection.stats.ping_ms = ping_ms;
}

/// Close the connection
pub fn connection_close(connection: &mut Connection) {
    connection.state = ConnectionState::Disconnected;
    let _ = connection.tcp_stream.shutdown(std::net::Shutdown::Both);
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
}

// DOP functions for ConnectionManager
/// Get all connections
pub fn connection_manager_get_connections(manager: &mut ConnectionManager) -> &mut Vec<Connection> {
    &mut manager.connections
}

/// Add a new connection
pub fn connection_manager_add_connection(
    manager: &mut ConnectionManager,
    mut connection: Connection,
) -> u32 {
    let player_id = manager.next_player_id;
    manager.next_player_id += 1;

    connection_set_player_id(&mut connection, player_id);
    manager.connections.push(connection);

    player_id
}

/// Remove a connection
pub fn connection_manager_remove_connection(manager: &mut ConnectionManager, player_id: u32) {
    manager
        .connections
        .retain(|c| c.player_id() != Some(player_id));
}

/// Get a connection by player ID
pub fn connection_manager_get_connection(
    manager: &mut ConnectionManager,
    player_id: u32,
) -> Option<&mut Connection> {
    manager
        .connections
        .iter_mut()
        .find(|c| c.player_id() == Some(player_id))
}

/// Process all connections
pub fn connection_manager_process_all(manager: &mut ConnectionManager) -> Vec<u32> {
    let mut disconnected = Vec::new();

    // Process each connection
    for conn in &mut manager.connections {
        // Check for timeout
        if conn.is_timed_out() {
            connection_close(conn);
            if let Some(id) = conn.player_id() {
                disconnected.push(id);
            }
            continue;
        }

        // Process send queue
        if let Err(e) = connection_process_send_queue(conn) {
            eprintln!("Failed to send packets: {}", e);
            connection_close(conn);
            if let Some(id) = conn.player_id() {
                disconnected.push(id);
            }
        }
    }

    // Remove disconnected connections
    for id in &disconnected {
        connection_manager_remove_connection(manager, *id);
    }

    disconnected
}
