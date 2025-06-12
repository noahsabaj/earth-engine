//! Minimal test to isolate the freeze issue

fn main() {
    // Initialize logging first
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .init();
    
    println!("[MINIMAL] Starting minimal test...");
    log::info!("[MINIMAL] Logger initialized");
    
    // Test 1: Can we create an event loop?
    log::info!("[MINIMAL] Test 1: Creating event loop...");
    
    use winit::event_loop::EventLoopBuilder;
    
    #[cfg(target_os = "linux")]
    {
        log::info!("[MINIMAL] Linux detected, trying X11...");
        use winit::platform::x11::EventLoopBuilderExtX11;
        
        match EventLoopBuilder::new().with_x11().build() {
            Ok(_) => {
                log::info!("[MINIMAL] ✓ X11 event loop created successfully");
                println!("[MINIMAL] ✓ X11 event loop created successfully");
            }
            Err(e) => {
                log::error!("[MINIMAL] ✗ X11 event loop failed: {}", e);
                println!("[MINIMAL] ✗ X11 event loop failed: {}", e);
                
                // Try without X11
                log::info!("[MINIMAL] Trying default event loop...");
                match winit::event_loop::EventLoop::new() {
                    Ok(_) => {
                        log::info!("[MINIMAL] ✓ Default event loop created successfully");
                        println!("[MINIMAL] ✓ Default event loop created successfully");
                    }
                    Err(e) => {
                        log::error!("[MINIMAL] ✗ Default event loop also failed: {}", e);
                        println!("[MINIMAL] ✗ Default event loop also failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
    
    // Test 2: Can we check display connection?
    log::info!("[MINIMAL] Test 2: Checking display...");
    
    if let Ok(display) = std::env::var("DISPLAY") {
        log::info!("[MINIMAL] DISPLAY={}", display);
        println!("[MINIMAL] DISPLAY={}", display);
    } else {
        log::warn!("[MINIMAL] DISPLAY not set!");
        println!("[MINIMAL] WARNING: DISPLAY not set!");
    }
    
    if let Ok(wayland) = std::env::var("WAYLAND_DISPLAY") {
        log::info!("[MINIMAL] WAYLAND_DISPLAY={}", wayland);
        println!("[MINIMAL] WAYLAND_DISPLAY={}", wayland);
    }
    
    // Test 3: Check for common WSL issues
    log::info!("[MINIMAL] Test 3: Checking for WSL...");
    
    if std::path::Path::new("/proc/version").exists() {
        if let Ok(contents) = std::fs::read_to_string("/proc/version") {
            if contents.contains("microsoft") || contents.contains("WSL") {
                log::info!("[MINIMAL] Running in WSL");
                println!("[MINIMAL] Running in WSL");
                
                // Check for WSLg
                if std::path::Path::new("/mnt/wslg").exists() {
                    log::info!("[MINIMAL] WSLg detected");
                    println!("[MINIMAL] WSLg detected (GUI support available)");
                } else {
                    log::warn!("[MINIMAL] WSLg not detected");
                    println!("[MINIMAL] WARNING: WSLg not detected (GUI support may be limited)");
                }
            }
        }
    }
    
    println!("\n[MINIMAL] All basic tests completed!");
    log::info!("[MINIMAL] All tests completed");
}