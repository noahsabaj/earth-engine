# Performance Documentation

This directory contains performance benchmarks, optimization tracking, and performance analysis for the Earth Engine project.

## Purpose

- Track performance metrics over time
- Document optimization efforts and their results
- Store benchmark results and comparisons
- Analyze performance bottlenecks
- Monitor the path to 10,000+ concurrent players at 144+ FPS

## What Belongs Here

- Benchmark results and methodology
- Performance profiling reports
- Optimization case studies
- Before/after performance comparisons
- Memory usage analysis
- GPU utilization metrics
- Frame timing analysis

## What Doesn't Belong Here

- Implementation details (see `/docs/technical/`)
- Code examples (see `/docs/examples/`)
- Test strategies (see `/docs/testing/`)

## Key Metrics We Track

- Voxels processed per second
- Memory allocations per frame (target: 0)
- GPU kernel execution times
- Network bandwidth usage
- Physics simulation performance
- Render pipeline throughput