/// GPU Compute Benchmark Binary
/// 
/// Run comprehensive GPU vs CPU benchmarks to validate whether GPU compute
/// actually provides performance benefits for the Earth Engine.

use earth_engine::benchmarks::{GpuVsCpuBenchmark, analyze_results};

fn main() {
    println!("Earth Engine GPU Compute Validation");
    println!("===================================\n");
    
    // Initialize benchmark runner
    let benchmark = match GpuVsCpuBenchmark::new() {
        Some(b) => b,
        None => {
            eprintln!("ERROR: Failed to initialize GPU. No GPU available?");
            std::process::exit(1);
        }
    };
    
    // Run all benchmarks
    let results = benchmark.run_all_benchmarks();
    
    // Analyze and print summary
    analyze_results(&results);
    
    // Save detailed results
    save_results_to_file(&results);
    
    println!("\nBenchmark complete. Results saved to gpu_benchmark_results.txt");
}

fn save_results_to_file(results: &[earth_engine::benchmarks::BenchmarkResult]) {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = match File::create("gpu_benchmark_results.txt") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create results file: {}", e);
            return;
        }
    };
    
    writeln!(file, "Earth Engine GPU vs CPU Benchmark Results").unwrap();
    writeln!(file, "=========================================\n").unwrap();
    writeln!(file, "Date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")).unwrap();
    writeln!(file, "\nDetailed Results:\n").unwrap();
    
    for result in results {
        writeln!(file, "Operation: {}", result.operation).unwrap();
        writeln!(file, "  Data size: {:.2} MB", result.data_size_mb).unwrap();
        writeln!(file, "  CPU time: {:.3} ms", result.cpu_time.as_secs_f64() * 1000.0).unwrap();
        writeln!(file, "  GPU time (compute): {:.3} ms", result.gpu_time.as_secs_f64() * 1000.0).unwrap();
        writeln!(file, "  GPU time (total): {:.3} ms", result.gpu_time_with_transfer.as_secs_f64() * 1000.0).unwrap();
        writeln!(file, "  Speedup (compute): {:.2}x", result.speedup).unwrap();
        writeln!(file, "  Speedup (total): {:.2}x", result.speedup_with_transfer).unwrap();
        if !result.notes.is_empty() {
            writeln!(file, "  Notes: {}", result.notes).unwrap();
        }
        writeln!(file).unwrap();
    }
    
    // Summary statistics
    let gpu_wins = results.iter().filter(|r| r.speedup_with_transfer > 1.2).count();
    let cpu_wins = results.iter().filter(|r| r.speedup_with_transfer < 0.8).count();
    let draws = results.len() - gpu_wins - cpu_wins;
    
    writeln!(file, "\nSummary:").unwrap();
    writeln!(file, "  Total tests: {}", results.len()).unwrap();
    writeln!(file, "  GPU faster: {} ({:.1}%)", gpu_wins, gpu_wins as f64 / results.len() as f64 * 100.0).unwrap();
    writeln!(file, "  CPU faster: {} ({:.1}%)", cpu_wins, cpu_wins as f64 / results.len() as f64 * 100.0).unwrap();
    writeln!(file, "  Too close: {} ({:.1}%)", draws, draws as f64 / results.len() as f64 * 100.0).unwrap();
    
    // Average speedups
    let avg_compute_speedup = results.iter().map(|r| r.speedup).sum::<f32>() / results.len() as f32;
    let avg_total_speedup = results.iter().map(|r| r.speedup_with_transfer).sum::<f32>() / results.len() as f32;
    
    writeln!(file, "\nAverage speedups:").unwrap();
    writeln!(file, "  Compute only: {:.2}x", avg_compute_speedup).unwrap();
    writeln!(file, "  With transfer: {:.2}x", avg_total_speedup).unwrap();
}