/// Standalone allocation hotspot scanner
/// Find allocation patterns in source code

use std::fs;
use std::path::Path;
use std::collections::HashMap;

fn main() {
    println!("Earth Engine Allocation Hotspot Finder");
    println!("=====================================");
    
    let mut hotspots = HashMap::new();
    scan_directory("src", &mut hotspots);
    
    // Sort by count
    let mut sorted_hotspots: Vec<_> = hotspots.into_iter().collect();
    sorted_hotspots.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!("\nTop Allocation Hotspots:");
    println!("========================");
    
    for (file, count) in sorted_hotspots.iter().take(20) {
        println!("{:3} allocations: {}", count, file);
    }
    
    println!("\nDetailed Analysis:");
    println!("==================");
    
    // Show top 5 with details
    for (file, _) in sorted_hotspots.iter().take(5) {
        show_file_details(file);
    }
    
    let total: usize = sorted_hotspots.iter().map(|(_, count)| count).sum();
    println!("\nSummary:");
    println!("========");
    println!("Total runtime allocations found: {}", total);
    println!("Files with allocations: {}", sorted_hotspots.len());
    println!("Target: Replace with object pools to achieve <10 allocations per frame");
    
    println!("\nNext Steps:");
    println!("===========");
    println!("1. Focus on hot-path files (renderer, physics, networking)");
    println!("2. Replace Vec::new() with pooled_vec!() macro");
    println!("3. Replace HashMap::new() with pooled_map!() macro");
    println!("4. Use pre-allocated buffers in loops");
    println!("5. Measure allocations per frame with tracking allocator");
}

fn scan_directory(dir: &str, hotspots: &mut HashMap<String, usize>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        
        let path = entry.path();
        
        if path.is_dir() {
            scan_directory(&path.to_string_lossy(), hotspots);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            scan_file(&path, hotspots);
        }
    }
}

fn scan_file(path: &Path, hotspots: &mut HashMap<String, usize>) {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return,
    };
    
    let file_path = path.to_string_lossy().to_string();
    
    // Patterns that indicate runtime allocations
    let allocation_patterns = [
        "Vec::new()",
        "HashMap::new()",
        "HashSet::new()",
        "BTreeMap::new()",
        "BTreeSet::new()",
        "VecDeque::new()",
        "String::new()",
        ".to_string()",
        ".to_vec()",
        "vec![",
        "format!(",
        ".collect::<Vec<",
        ".collect::<HashMap<",
        "Box::new(",
    ];
    
    let mut count = 0;
    
    for pattern in &allocation_patterns {
        count += content.matches(pattern).count();
    }
    
    if count > 0 {
        hotspots.insert(file_path, count);
    }
}

fn show_file_details(file_path: &str) {
    println!("\n--- {} ---", file_path);
    
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => return,
    };
    
    let allocation_patterns = [
        ("Vec::new()", "Create empty vector"),
        ("HashMap::new()", "Create empty hashmap"),
        ("HashSet::new()", "Create empty hashset"),
        (".to_string()", "String allocation"),
        ("vec![", "Vector literal"),
        ("format!(", "String formatting"),
        ("Box::new(", "Heap allocation"),
    ];
    
    for (pattern, description) in &allocation_patterns {
        let count = content.matches(pattern).count();
        if count > 0 {
            println!("  {} x {} ({})", count, pattern, description);
        }
    }
}