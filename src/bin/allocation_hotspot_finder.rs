/// Find allocation hotspots in source code
/// Scans for Vec::new(), HashMap::new() and other runtime allocations

use std::fs;
use std::path::Path;
use std::collections::HashMap;

fn main() {
    println!("Hearth Engine Allocation Hotspot Finder");
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
    
    let total: usize = sorted_hotspots.iter().map(|(_, count)| count).sum();
    println!("\nTotal runtime allocations found: {}", total);
    println!("Target: Replace with object pools to achieve <10 allocations per frame");
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