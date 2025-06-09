use std::time::Duration;

/// Protocol version - increment when making breaking changes
pub const PROTOCOL_VERSION: u32 = 1;

/// Default network ports
pub const DEFAULT_TCP_PORT: u16 = 25565;
pub const DEFAULT_UDP_PORT: u16 = 25566;

/// Network timing constants
pub const TICK_RATE: u32 = 20; // Server ticks per second
pub const TICK_DURATION: Duration = Duration::from_millis(1000 / TICK_RATE as u64);
pub const POSITION_UPDATE_RATE: u32 = 10; // Position updates per second
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(5);
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Network limits
pub const MAX_PACKET_SIZE: usize = 65536; // 64KB max packet size
pub const MAX_USERNAME_LENGTH: usize = 16;
pub const MAX_CHAT_MESSAGE_LENGTH: usize = 256;
pub const MAX_PLAYERS: u32 = 100;

/// Protocol handler
pub struct Protocol;

impl Protocol {
    /// Validate username
    pub fn validate_username(username: &str) -> Result<(), String> {
        if username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        
        if username.len() > MAX_USERNAME_LENGTH {
            return Err(format!("Username too long (max {} characters)", MAX_USERNAME_LENGTH));
        }
        
        // Check for valid characters (alphanumeric and underscore)
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }
        
        Ok(())
    }
    
    /// Validate chat message
    pub fn validate_chat_message(message: &str) -> Result<(), String> {
        if message.is_empty() {
            return Err("Message cannot be empty".to_string());
        }
        
        if message.len() > MAX_CHAT_MESSAGE_LENGTH {
            return Err(format!("Message too long (max {} characters)", MAX_CHAT_MESSAGE_LENGTH));
        }
        
        Ok(())
    }
    
    /// Calculate network tick from timestamp
    pub fn calculate_tick(start_time: std::time::Instant) -> u32 {
        let elapsed = start_time.elapsed();
        (elapsed.as_millis() / TICK_DURATION.as_millis()) as u32
    }
}