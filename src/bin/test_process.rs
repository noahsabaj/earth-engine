/// Test binary for process system
/// Run with: cargo run --bin test_process

use earth_engine::process::*;
use earth_engine::instance::InstanceId;

fn main() {
    println!("Testing Process & Transform System...\n");
    
    // Test 1: Basic process creation and execution
    test_basic_process();
    
    // Test 2: State machine transitions
    test_state_machine();
    
    // Test 3: Multi-stage transformations
    test_transform_stages();
    
    // Test 4: Parallel processing
    test_parallel_processing();
    
    // Test 5: Process control and interruption
    test_process_control();
    
    // Test 6: Visual indicators
    test_visual_indicators();
    
    println!("\nAll tests completed!");
}

fn test_basic_process() {
    println!("1. Testing Basic Process Creation:");
    
    let mut manager = ProcessManager::new();
    let owner = InstanceId::new();
    
    // Start a simple crafting process
    let process_id = manager.start_process(
        ProcessType {
            category: ProcessCategory::Crafting,
            sub_type: 1,
        },
        owner,
        vec![],
        TimeUnit::Seconds(5.0),
    );
    
    println!("  Created process: {:?}", process_id);
    
    // Update process
    manager.update(20); // 1 second
    
    if let Some(info) = manager.get_process(process_id) {
        println!("  Progress: {:.1}%", info.progress * 100.0);
        println!("  Time remaining: {} ticks", info.time_remaining);
        println!("  Status: {:?}", info.status);
    }
    
    // Complete the process
    for _ in 0..4 {
        manager.update(20);
    }
    
    if let Some(info) = manager.get_process(process_id) {
        println!("  Final status: {:?}", info.status);
    }
}

fn test_state_machine() {
    println!("\n2. Testing State Machine:");
    
    let mut sm = state_machine::StateMachineTemplates::linear_process(100);
    
    println!("  Initial state: {:?}", sm.current_state());
    
    // Progress through states
    let actions = sm.update(50, 0.0);
    println!("  After 50 ticks: {:?}", sm.current_state());
    println!("  Actions: {} triggered", actions.len());
    
    // Progress to next state
    let actions = sm.update(50, 0.5);
    println!("  At 50% progress: {:?}", sm.current_state());
    
    // Complete
    let actions = sm.update(100, 1.0);
    println!("  At 100% progress: {:?}", sm.current_state());
    println!("  Is complete: {}", sm.is_complete());
}

fn test_transform_stages() {
    println!("\n3. Testing Transform Stages:");
    
    let smelting = transform_stage::StageTemplates::smelting_stage();
    
    println!("  Stage: {}", smelting.name);
    println!("  Duration: {:?}", smelting.duration);
    println!("  Requirements: {} types", smelting.requirements.len());
    println!("  Outputs: {} types", smelting.outputs.len());
    
    // Validate requirements
    let owner = InstanceId::new();
    let items = vec![InstanceId::new(); 5];
    let context = transform_stage::ValidationContext::default();
    
    let result = transform_stage::StageValidator::validate_requirements(
        &smelting,
        owner,
        &items,
        &context,
    );
    
    println!("  Validation passed: {}", result.valid);
    if !result.valid {
        for req in &result.missing_requirements {
            println!("    Missing: {}", req);
        }
    }
    
    // Calculate outputs
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let outputs = transform_stage::StageValidator::calculate_outputs(
        &smelting,
        QualityLevel::Good,
        &mut rng,
    );
    
    println!("  Generated {} outputs", outputs.len());
}

fn test_parallel_processing() {
    println!("\n4. Testing Parallel Processing:");
    
    let mut processor = parallel_processor::ParallelProcessor::new();
    let mut data = ProcessData::new();
    let mut state_machines = Vec::new();
    
    // Create many processes
    let owner = InstanceId::new();
    for i in 0..100 {
        let id = ProcessId(i);
        data.add(id, ProcessType::default(), owner, 1000);
        if let Some(last_status) = data.status.last_mut() {
            last_status.clone_from(&ProcessStatus::Active);
        } else {
            eprintln!("Failed to get last status");
        }
        state_machines.push(state_machine::StateMachine::new());
    }
    
    let start = std::time::Instant::now();
    
    // Process in parallel
    let batch = parallel_processor::ProcessBatch {
        indices: (0..100).collect(),
        delta_ticks: 10,
    };
    
    processor.process_batch(&mut data, &mut state_machines, batch);
    
    let elapsed = start.elapsed();
    println!("  Processed 100 processes in {:?}", elapsed);
    println!("  {}", processor.metrics());
}

fn test_process_control() {
    println!("\n5. Testing Process Control:");
    
    let mut control = ProcessControl::new();
    let mut data = ProcessData::new();
    
    // Create dependent processes
    let process1 = ProcessId(1);
    let process2 = ProcessId(2);
    let owner = InstanceId::new();
    
    data.add(process1, ProcessType::default(), owner, 100);
    data.add(process2, ProcessType::default(), owner, 100);
    
    // Set up dependency
    control.add_dependency(process2, process1);
    
    // Check if process2 can start
    match control.can_start(process2, &data) {
        Ok(_) => println!("  Process 2 can start"),
        Err(e) => println!("  Process 2 cannot start: {}", e),
    }
    
    // Test interruption
    data.status[0] = ProcessStatus::Active;
    let result = control.interrupt_process(
        process1,
        process_control::InterruptReason::ResourceUnavailable(vec![1, 2]),
        &mut data,
    );
    
    println!("  Interrupted process 1: {:?}", result);
    println!("  Process 1 status: {:?}", data.status[0]);
    
    // Test concurrent limits
    let count = control.get_player_process_count(owner, &data);
    println!("  Player has {} active processes", count);
    println!("  Can start new: {}", control.can_player_start_process(owner, &data));
}

fn test_visual_indicators() {
    println!("\n6. Testing Visual Indicators:");
    
    let mut visual = visual_indicators::VisualTemplates::crafting();
    
    println!("  Initial animation: {:?}", visual.animation);
    println!("  Particles: {} active", visual.particles.len());
    
    // Update progress
    visual.update_progress(0.5);
    println!("  At 50% progress:");
    println!("    Segments: {}/10", visual.progress_bar.segments);
    println!("    Animation: {:?}", visual.animation);
    
    // Add text overlay
    visual.add_text("Crafting sword...".to_string(), 2.0);
    println!("  Text overlays: {}", visual.text_overlays.len());
    
    // Update visuals
    visual.update(1.0);
    println!("  After 1 second:");
    println!("    Remaining overlays: {}", visual.text_overlays.len());
    
    // Test quality visuals
    let (color, particles) = visual_indicators::quality_to_visual(QualityLevel::Perfect);
    println!("  Perfect quality:");
    println!("    Color: {:?}", color);
    println!("    Particle effects: {}", particles.len());
}