use std::sync::Arc;
use std::path::Path;
use tokio::sync::mpsc;
use wgpu::{Device, Queue};
use crate::streaming::{
    PageTable, PageTableEntry, PageFlags,
    MemoryMapper, GpuVirtualMemory, PredictiveLoader, PAGE_SIZE_BYTES,
};

/// Stream request from GPU or predictive system
#[derive(Debug, Clone)]
pub struct StreamRequest {
    pub page_x: u32,
    pub page_y: u32,
    pub page_z: u32,
    pub priority: f32,
    pub source: RequestSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestSource {
    GpuFault,
    Prediction,
    Prefetch,
    Manual,
}

/// Streaming pipeline for zero-copy disk to GPU transfers
pub struct StreamPipeline {
    /// Memory mapper for disk access
    memory_mapper: Arc<tokio::sync::Mutex<MemoryMapper>>,
    
    /// GPU virtual memory manager
    gpu_vm: Arc<tokio::sync::Mutex<GpuVirtualMemory>>,
    
    /// Page table
    page_table: Arc<tokio::sync::RwLock<PageTable>>,
    
    /// Predictive loader
    predictive_loader: Arc<tokio::sync::Mutex<PredictiveLoader>>,
    
    /// Request queue
    request_tx: mpsc::UnboundedSender<StreamRequest>,
    request_rx: mpsc::UnboundedReceiver<StreamRequest>,
    
    /// Active streaming tasks
    active_streams: Arc<tokio::sync::Mutex<Vec<StreamTask>>>,
    
    /// Pipeline statistics
    stats: Arc<tokio::sync::Mutex<PipelineStats>>,
}

/// Active streaming task
#[derive(Debug)]
struct StreamTask {
    request: StreamRequest,
    start_time: std::time::Instant,
}

/// Pipeline statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct PipelineStats {
    pub total_streamed: u64,
    pub pages_loaded: u64,
    pub faults_handled: u64,
    pub predictions_correct: u64,
    pub average_latency_ms: f32,
}

impl StreamPipeline {
    /// Create new streaming pipeline
    pub fn new(
        world_path: &Path,
        device: Arc<Device>,
        queue: Arc<Queue>,
        page_table: PageTable,
        max_memory: u64,
    ) -> std::io::Result<Self> {
        let memory_mapper = Arc::new(tokio::sync::Mutex::new(
            MemoryMapper::new(world_path, device.clone(), queue.clone(), max_memory / 2)?
        ));
        
        let gpu_vm = Arc::new(tokio::sync::Mutex::new(
            GpuVirtualMemory::new(device, queue, &page_table, max_memory / 2)
        ));
        
        let predictive_loader = Arc::new(tokio::sync::Mutex::new(
            PredictiveLoader::new(1, 128.0, 512.0)
        ));
        
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            memory_mapper,
            gpu_vm,
            page_table: Arc::new(tokio::sync::RwLock::new(page_table)),
            predictive_loader,
            request_tx,
            request_rx,
            active_streams: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            stats: Arc::new(tokio::sync::Mutex::new(PipelineStats::default())),
        })
    }
    
    /// Start the streaming pipeline
    pub fn start(mut self) -> mpsc::UnboundedSender<StreamRequest> {
        let tx = self.request_tx.clone();
        
        // Spawn main pipeline task
        tokio::spawn(async move {
            self.run_pipeline().await;
        });
        
        tx
    }
    
    /// Main pipeline loop
    async fn run_pipeline(&mut self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1));
        
        loop {
            tokio::select! {
                // Process stream requests
                Some(request) = self.request_rx.recv() => {
                    self.handle_request(request).await;
                }
                
                // Check for GPU page faults
                _ = interval.tick() => {
                    self.check_gpu_faults().await;
                }
            }
            
            // Process active streams
            self.process_active_streams().await;
        }
    }
    
    /// Handle a stream request
    async fn handle_request(&self, request: StreamRequest) {
        let page_table = self.page_table.read().await;
        
        // Check if page is already resident
        if let Some(page_idx) = page_table.page_index(
            request.page_x,
            request.page_y,
            request.page_z,
        ) {
            if page_idx < page_table.entries.len() {
                let entry = &page_table.entries[page_idx];
                if entry.is_resident() {
                    return;
                }
            }
        }
        
        drop(page_table);
        
        // Add to active streams
        let mut active = self.active_streams.lock().await;
        active.push(StreamTask {
            request,
            start_time: std::time::Instant::now(),
        });
    }
    
    /// Check for GPU page faults
    async fn check_gpu_faults(&self) {
        let gpu_vm = self.gpu_vm.lock().await;
        let faults = gpu_vm.read_page_faults().await;
        drop(gpu_vm);
        
        for fault in faults {
            let request = StreamRequest {
                page_x: fault.page_x,
                page_y: fault.page_y,
                page_z: fault.page_z,
                priority: 1000.0, // High priority for faults
                source: RequestSource::GpuFault,
            };
            
            self.request_tx.send(request).ok();
            
            let mut stats = self.stats.lock().await;
            stats.faults_handled += 1;
        }
    }
    
    /// Process active streaming tasks
    async fn process_active_streams(&self) {
        let mut active = self.active_streams.lock().await;
        let mut completed = Vec::new();
        
        for (i, task) in active.iter().enumerate() {
            if let Ok(success) = self.stream_page(&task.request).await {
                if success {
                    completed.push(i);
                    
                    // Update statistics
                    let mut stats = self.stats.lock().await;
                    stats.pages_loaded += 1;
                    stats.total_streamed += PAGE_SIZE_BYTES;
                    
                    let latency = task.start_time.elapsed().as_millis() as f32;
                    stats.average_latency_ms = 
                        (stats.average_latency_ms * (stats.pages_loaded - 1) as f32 + latency) 
                        / stats.pages_loaded as f32;
                }
            }
        }
        
        // Remove completed tasks
        for &i in completed.iter().rev() {
            active.swap_remove(i);
        }
    }
    
    /// Stream a single page
    async fn stream_page(&self, request: &StreamRequest) -> std::io::Result<bool> {
        let mut page_table = self.page_table.write().await;
        
        // Get page index
        let page_idx = match page_table.page_index(
            request.page_x,
            request.page_y,
            request.page_z,
        ) {
            Some(idx) if idx < page_table.entries.len() => idx,
            _ => return Ok(false),
        };
        
        let entry = &mut page_table.entries[page_idx];
        
        // Check if already streaming
        if entry.flags & PageFlags::Streaming as u8 != 0 {
            return Ok(false);
        }
        
        // Mark as streaming
        entry.flags |= PageFlags::Streaming as u8;
        
        // Allocate GPU page if needed
        let mut gpu_vm = self.gpu_vm.lock().await;
        let (page_index, physical_offset) = match gpu_vm.allocate_page() {
            Some(alloc) => alloc,
            None => {
                // Need to evict a page first
                drop(gpu_vm);
                self.evict_pages(1).await?;
                
                gpu_vm = self.gpu_vm.lock().await;
                match gpu_vm.allocate_page() {
                    Some(alloc) => alloc,
                    None => return Ok(false),
                }
            }
        };
        
        // Map page from disk
        let mut memory_mapper = self.memory_mapper.lock().await;
        let mmap = memory_mapper.map_page(entry)?;
        
        // Calculate data offset
        let offset_in_mmap = (entry.disk_offset % mmap.len() as u64) as usize;
        let page_data = &mmap[offset_in_mmap..][..PAGE_SIZE_BYTES as usize];
        
        // Upload to GPU
        gpu_vm.upload_page(physical_offset, page_data);
        
        // Update page table entry
        entry.physical_offset = physical_offset;
        entry.flags |= PageFlags::Resident as u8;
        entry.flags &= !(PageFlags::Streaming as u8);
        entry.access_count = 1;
        
        // Update GPU page table
        gpu_vm.update_page_table_entry(page_idx, entry);
        
        // Update resident count
        page_table.resident_pages += 1;
        
        Ok(true)
    }
    
    /// Evict pages to make room
    async fn evict_pages(&self, num_pages: usize) -> std::io::Result<()> {
        let page_table = self.page_table.read().await;
        
        // Find eviction candidates
        let candidates = crate::streaming::gpu_vm::calculate_eviction_candidates(
            &page_table,
            (0.0, 0.0, 0.0), // TODO: Get actual camera position
            num_pages,
        );
        
        drop(page_table);
        
        for candidate in candidates {
            self.evict_page(candidate.page_index).await?;
        }
        
        Ok(())
    }
    
    /// Evict a single page
    async fn evict_page(&self, page_idx: usize) -> std::io::Result<()> {
        let mut page_table = self.page_table.write().await;
        let entry = &mut page_table.entries[page_idx];
        
        if !entry.is_resident() || entry.is_locked() {
            return Ok(());
        }
        
        // Write back if dirty
        if entry.is_dirty() {
            // TODO: Implement write-back
        }
        
        // Free GPU page
        let mut gpu_vm = self.gpu_vm.lock().await;
        let page_index = (entry.physical_offset / PAGE_SIZE_BYTES) as u32;
        gpu_vm.free_page(page_index);
        
        // Update entry
        entry.physical_offset = PageTableEntry::INVALID_OFFSET;
        entry.flags &= !(PageFlags::Resident as u8);
        
        // Update GPU page table
        gpu_vm.update_page_table_entry(page_idx, entry);
        
        // Update resident count
        page_table.resident_pages -= 1;
        
        Ok(())
    }
    
    /// Update player position for predictive loading
    pub async fn update_player_position(
        &self,
        player_id: usize,
        position: (f32, f32, f32),
        timestamp: f64,
    ) {
        let page_table = self.page_table.read().await;
        let mut predictive_loader = self.predictive_loader.lock().await;
        
        predictive_loader.update_player(player_id, position, timestamp, &page_table);
        
        // Get predicted load requests
        let requests = predictive_loader.get_load_requests(32);
        
        for req in requests {
            let stream_req = StreamRequest {
                page_x: req.page_x,
                page_y: req.page_y,
                page_z: req.page_z,
                priority: req.priority,
                source: RequestSource::Prediction,
            };
            
            self.request_tx.send(stream_req).ok();
        }
    }
    
    /// Get pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        self.stats.lock().await.clone()
    }
}

/// Asynchronous page uploader for background streaming
pub struct AsyncPageUploader {
    /// Upload tasks
    tasks: Vec<tokio::task::JoinHandle<()>>,
    
    /// Maximum concurrent uploads
    max_concurrent: usize,
}

impl AsyncPageUploader {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: Vec::new(),
            max_concurrent,
        }
    }
    
    /// Queue a page for upload
    pub fn queue_upload(
        &mut self,
        data: Vec<u8>,
        physical_offset: u64,
        gpu_vm: Arc<tokio::sync::Mutex<GpuVirtualMemory>>,
    ) {
        // Remove completed tasks
        self.tasks.retain(|task| !task.is_finished());
        
        // Wait if at capacity
        while self.tasks.len() >= self.max_concurrent {
            self.tasks.retain(|task| !task.is_finished());
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        
        // Spawn upload task
        let task = tokio::spawn(async move {
            let gpu_vm = gpu_vm.lock().await;
            gpu_vm.upload_page(physical_offset, &data);
        });
        
        self.tasks.push(task);
    }
    
    /// Wait for all uploads to complete
    pub async fn wait_all(&mut self) {
        for task in self.tasks.drain(..) {
            task.await.ok();
        }
    }
}

