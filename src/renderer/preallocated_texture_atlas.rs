use cgmath::Vector2;
use image::{DynamicImage, RgbaImage};
/// Pre-allocated texture atlas using fixed-size array for material mappings
/// Replaces HashMap<MaterialId, AtlasUV> with zero-allocation lookups
use wgpu::{Device, Queue, Sampler, Texture, TextureView};

/// Maximum number of materials we can support
pub const MAX_MATERIALS: usize = 1024;

/// UV coordinates within the atlas
#[derive(Debug, Clone, Copy)]
pub struct AtlasUV {
    pub min: Vector2<f32>,
    pub max: Vector2<f32>,
}

impl AtlasUV {
    /// Transform local UV (0-1) to atlas UV
    pub fn transform(&self, local_uv: Vector2<f32>) -> Vector2<f32> {
        Vector2::new(
            self.min.x + (self.max.x - self.min.x) * local_uv.x,
            self.min.y + (self.max.y - self.min.y) * local_uv.y,
        )
    }
}

/// Rectangle packing for atlas
#[derive(Debug, Clone, Copy)]
struct PackedRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    material_id: u32,
}

/// Material ID to atlas UV mapping
pub type MaterialId = u32;

/// Pre-allocated texture atlas for efficient GPU rendering
pub struct PreallocatedTextureAtlas {
    texture: Texture,
    view: TextureView,
    sampler: Sampler,

    atlas_size: u32,
    tile_size: u32,
    padding: u32,

    // Pre-allocated arrays instead of HashMap
    material_uvs: [Option<AtlasUV>; MAX_MATERIALS],
    material_names: [Option<String>; MAX_MATERIALS],
    active_materials: Vec<MaterialId>, // Track which materials are active

    // Packing state
    packed_rects: Vec<PackedRect>,
    atlas_image: RgbaImage,
    dirty: bool,
}

impl PreallocatedTextureAtlas {
    /// Create new texture atlas
    pub fn new(device: &Device, atlas_size: u32, tile_size: u32) -> Self {
        let padding = 2; // 2 pixel padding to prevent bleeding

        // Get device limits to ensure we don't exceed GPU capabilities
        let device_limits = device.limits();
        let max_dimension = device_limits.max_texture_dimension_2d;

        // Validate and clamp atlas size
        let clamped_atlas_size = atlas_size.min(max_dimension);

        // Log if dimensions were clamped
        if clamped_atlas_size != atlas_size {
            log::warn!(
                "[TextureAtlas::new] Atlas size clamped from {} to {} due to GPU limits (max: {})",
                atlas_size,
                clamped_atlas_size,
                max_dimension
            );
        }

        // Create atlas texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Atlas"),
            size: wgpu::Extent3d {
                width: clamped_atlas_size,
                height: clamped_atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler with filtering
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create blank atlas image
        let atlas_image = RgbaImage::new(clamped_atlas_size, clamped_atlas_size);

        // Initialize pre-allocated arrays
        const NONE_UV: Option<AtlasUV> = None;
        const NONE_NAME: Option<String> = None;

        Self {
            texture,
            view,
            sampler,
            atlas_size: clamped_atlas_size,
            tile_size,
            padding,
            material_uvs: [NONE_UV; MAX_MATERIALS],
            material_names: [NONE_NAME; MAX_MATERIALS],
            active_materials: Vec::with_capacity(256),
            packed_rects: Vec::new(),
            atlas_image,
            dirty: false,
        }
    }

    /// Add a material texture to the atlas
    pub fn add_material(&mut self, name: &str, image: &DynamicImage) -> Option<MaterialId> {
        // Find next available material ID
        let material_id = self.find_free_material_id()?;

        // Convert to RGBA
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();

        // Find space in atlas
        let rect = self.pack_rect(width, height)?;

        // Copy image data to atlas
        for y in 0..height {
            for x in 0..width {
                let src_pixel = rgba_image.get_pixel(x, y);
                let dst_x = rect.x + x;
                let dst_y = rect.y + y;
                if dst_x < self.atlas_size && dst_y < self.atlas_size {
                    self.atlas_image.put_pixel(dst_x, dst_y, *src_pixel);
                }
            }
        }

        // Calculate UV coordinates
        let atlas_size_f = self.atlas_size as f32;
        let uv = AtlasUV {
            min: Vector2::new(
                (rect.x as f32) / atlas_size_f,
                (rect.y as f32) / atlas_size_f,
            ),
            max: Vector2::new(
                ((rect.x + rect.width) as f32) / atlas_size_f,
                ((rect.y + rect.height) as f32) / atlas_size_f,
            ),
        };

        // Store in pre-allocated arrays
        self.material_uvs[material_id as usize] = Some(uv);
        self.material_names[material_id as usize] = Some(name.to_string());
        self.active_materials.push(material_id);

        self.packed_rects.push(PackedRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            material_id,
        });

        self.dirty = true;

        Some(material_id)
    }

    /// Get UV coordinates for a material
    pub fn get_uv(&self, material_id: MaterialId) -> Option<AtlasUV> {
        if (material_id as usize) < MAX_MATERIALS {
            self.material_uvs[material_id as usize]
        } else {
            None
        }
    }

    /// Get material ID by name
    pub fn get_material_by_name(&self, name: &str) -> Option<MaterialId> {
        for &id in &self.active_materials {
            if let Some(ref mat_name) = self.material_names[id as usize] {
                if mat_name == name {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Upload atlas to GPU if dirty
    pub fn upload(&mut self, queue: &Queue) {
        if !self.dirty {
            return;
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.atlas_size),
                rows_per_image: Some(self.atlas_size),
            },
            wgpu::Extent3d {
                width: self.atlas_size,
                height: self.atlas_size,
                depth_or_array_layers: 1,
            },
        );

        self.dirty = false;
    }

    /// Get texture view for binding
    pub fn view(&self) -> &TextureView {
        &self.view
    }

    /// Get sampler for binding
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    /// Clear all materials
    pub fn clear(&mut self) {
        // Clear active materials
        for &id in &self.active_materials {
            self.material_uvs[id as usize] = None;
            self.material_names[id as usize] = None;
        }
        self.active_materials.clear();

        // Clear packing state
        self.packed_rects.clear();

        // Clear atlas image
        for pixel in self.atlas_image.pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }

        self.dirty = true;
    }

    /// Get atlas statistics
    pub fn stats(&self) -> AtlasStats {
        let total_area = self.atlas_size * self.atlas_size;
        let used_area: u32 = self.packed_rects.iter().map(|r| r.width * r.height).sum();

        AtlasStats {
            atlas_size: self.atlas_size,
            materials_count: self.active_materials.len(),
            utilization: (used_area as f32 / total_area as f32) * 100.0,
        }
    }

    /// Find next available material ID
    fn find_free_material_id(&self) -> Option<MaterialId> {
        for id in 0..MAX_MATERIALS as u32 {
            if self.material_uvs[id as usize].is_none() {
                return Some(id);
            }
        }
        None
    }

    /// Simple rectangle packing (first-fit)
    fn pack_rect(&self, width: u32, height: u32) -> Option<PackedRect> {
        let padded_width = width + self.padding * 2;
        let padded_height = height + self.padding * 2;

        // Try to find space
        let mut best_y = self.atlas_size;
        let mut best_x = 0;

        for y in 0..self.atlas_size - padded_height {
            let mut x = 0;
            while x < self.atlas_size - padded_width {
                if self.can_place_at(x, y, padded_width, padded_height) {
                    if y < best_y {
                        best_y = y;
                        best_x = x;
                    }
                    break;
                }
                x += 1;
            }
        }

        if best_y < self.atlas_size {
            Some(PackedRect {
                x: best_x + self.padding,
                y: best_y + self.padding,
                width,
                height,
                material_id: 0, // Will be set by caller
            })
        } else {
            None
        }
    }

    /// Check if rectangle can be placed at position
    fn can_place_at(&self, x: u32, y: u32, width: u32, height: u32) -> bool {
        for rect in &self.packed_rects {
            if x < rect.x + rect.width + self.padding * 2
                && x + width > rect.x
                && y < rect.y + rect.height + self.padding * 2
                && y + height > rect.y
            {
                return false;
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct AtlasStats {
    pub atlas_size: u32,
    pub materials_count: usize,
    pub utilization: f32,
}
