/// Visual Process Indicators
/// 
/// Visual representation of process progress and status.
/// Generates data for rendering progress bars, status icons, etc.

use crate::process::{ProcessStatus, ProcessState, QualityLevel};
use serde::{Serialize, Deserialize};

/// Visual indicator for a process
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessVisual {
    /// Progress bar data
    pub progress_bar: ProgressBar,
    
    /// Status icon
    pub status_icon: StatusIcon,
    
    /// Text overlays
    pub text_overlays: Vec<TextOverlay>,
    
    /// Particle effects
    pub particles: Vec<ParticleEffect>,
    
    /// Animation state
    pub animation: AnimationState,
}

impl ProcessVisual {
    /// Update progress bar
    pub fn update_progress(&mut self, progress: f32) {
        self.progress_bar.progress = progress;
        self.progress_bar.segments = Self::calculate_segments(progress);
        
        // Update animation based on progress
        if progress < 0.25 {
            self.animation = AnimationState::Starting;
        } else if progress < 0.95 {
            self.animation = AnimationState::Running;
        } else {
            self.animation = AnimationState::Finishing;
        }
    }
    
    /// Update status
    pub fn update_status(&mut self, status: ProcessStatus) {
        self.status_icon = match status {
            ProcessStatus::Pending => StatusIcon::Waiting,
            ProcessStatus::Active => StatusIcon::Working,
            ProcessStatus::Paused => StatusIcon::Paused,
            ProcessStatus::Completed => StatusIcon::Complete,
            ProcessStatus::Failed => StatusIcon::Error,
            ProcessStatus::Cancelled => StatusIcon::Cancelled,
        };
    }
    
    /// Calculate progress bar segments
    fn calculate_segments(progress: f32) -> u8 {
        (progress * 10.0) as u8
    }
    
    /// Add text overlay
    pub fn add_text(&mut self, text: String, duration: f32) {
        self.text_overlays.push(TextOverlay {
            text,
            duration,
            elapsed: 0.0,
            position: TextPosition::Above,
            style: TextStyle::Normal,
        });
    }
    
    /// Add particle effect
    pub fn add_particle(&mut self, particle_type: ParticleType) {
        self.particles.push(ParticleEffect {
            particle_type,
            intensity: 1.0,
            duration: 2.0,
            elapsed: 0.0,
        });
    }
    
    /// Update visuals (called each frame)
    pub fn update(&mut self, delta_time: f32) {
        // Update text overlays
        self.text_overlays.retain_mut(|overlay| {
            overlay.elapsed += delta_time;
            overlay.elapsed < overlay.duration
        });
        
        // Update particles
        self.particles.retain_mut(|particle| {
            particle.elapsed += delta_time;
            particle.elapsed < particle.duration
        });
    }
}

/// Progress bar visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressBar {
    /// Progress value (0.0 - 1.0)
    pub progress: f32,
    
    /// Number of filled segments (0-10)
    pub segments: u8,
    
    /// Bar color
    pub color: ProgressColor,
    
    /// Show percentage text
    pub show_percentage: bool,
    
    /// Animation style
    pub animation: BarAnimation,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            progress: 0.0,
            segments: 0,
            color: ProgressColor::Green,
            show_percentage: true,
            animation: BarAnimation::Smooth,
        }
    }
}

/// Progress bar colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressColor {
    Green,
    Blue,
    Yellow,
    Red,
    Purple,
    Custom(u8, u8, u8), // RGB
}

/// Bar animation styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarAnimation {
    None,
    Smooth,
    Pulse,
    Stripes,
}

/// Status icon types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusIcon {
    None,
    Waiting,
    Working,
    Paused,
    Complete,
    Error,
    Cancelled,
    Custom(u16),
}

impl Default for StatusIcon {
    fn default() -> Self {
        Self::None
    }
}

/// Text overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOverlay {
    pub text: String,
    pub duration: f32,
    pub elapsed: f32,
    pub position: TextPosition,
    pub style: TextStyle,
}

/// Text positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextPosition {
    Above,
    Below,
    Center,
    Left,
    Right,
}

/// Text styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextStyle {
    Normal,
    Bold,
    Italic,
    Small,
    Large,
}

/// Particle effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEffect {
    pub particle_type: ParticleType,
    pub intensity: f32,
    pub duration: f32,
    pub elapsed: f32,
}

/// Particle types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleType {
    Sparkle,
    Smoke,
    Fire,
    Steam,
    Dust,
    Magic,
    Bubbles,
}

/// Animation states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationState {
    Idle,
    Starting,
    Running,
    Finishing,
    Complete,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Visual templates for common process types
pub struct VisualTemplates;

impl VisualTemplates {
    /// Crafting process visuals
    pub fn crafting() -> ProcessVisual {
        ProcessVisual {
            progress_bar: ProgressBar {
                color: ProgressColor::Blue,
                animation: BarAnimation::Stripes,
                ..Default::default()
            },
            status_icon: StatusIcon::Working,
            text_overlays: vec![],
            particles: vec![
                ParticleEffect {
                    particle_type: ParticleType::Sparkle,
                    intensity: 0.5,
                    duration: 10.0,
                    elapsed: 0.0,
                }
            ],
            animation: AnimationState::Running,
        }
    }
    
    /// Smelting process visuals
    pub fn smelting() -> ProcessVisual {
        ProcessVisual {
            progress_bar: ProgressBar {
                color: ProgressColor::Red,
                animation: BarAnimation::Pulse,
                ..Default::default()
            },
            status_icon: StatusIcon::Working,
            text_overlays: vec![],
            particles: vec![
                ParticleEffect {
                    particle_type: ParticleType::Fire,
                    intensity: 1.0,
                    duration: 10.0,
                    elapsed: 0.0,
                },
                ParticleEffect {
                    particle_type: ParticleType::Smoke,
                    intensity: 0.3,
                    duration: 10.0,
                    elapsed: 0.0,
                }
            ],
            animation: AnimationState::Running,
        }
    }
    
    /// Growth process visuals
    pub fn growth() -> ProcessVisual {
        ProcessVisual {
            progress_bar: ProgressBar {
                color: ProgressColor::Green,
                animation: BarAnimation::Smooth,
                show_percentage: false,
                ..Default::default()
            },
            status_icon: StatusIcon::None,
            text_overlays: vec![],
            particles: vec![
                ParticleEffect {
                    particle_type: ParticleType::Magic,
                    intensity: 0.2,
                    duration: 10.0,
                    elapsed: 0.0,
                }
            ],
            animation: AnimationState::Running,
        }
    }
}

/// Visual quality indicators
pub fn quality_to_visual(quality: QualityLevel) -> (ProgressColor, Vec<ParticleType>) {
    match quality {
        QualityLevel::Poor => (ProgressColor::Red, vec![]),
        QualityLevel::Normal => (ProgressColor::Green, vec![]),
        QualityLevel::Good => (ProgressColor::Blue, vec![ParticleType::Sparkle]),
        QualityLevel::Excellent => (ProgressColor::Purple, vec![ParticleType::Sparkle, ParticleType::Magic]),
        QualityLevel::Perfect => (ProgressColor::Custom(255, 215, 0), vec![ParticleType::Sparkle, ParticleType::Magic]), // Gold
    }
}

/// Generate visual data for rendering
pub struct VisualRenderer;

impl VisualRenderer {
    /// Generate vertex data for progress bar
    pub fn generate_progress_bar_vertices(
        bar: &ProgressBar,
        position: [f32; 3],
        size: [f32; 2],
    ) -> Vec<f32> {
        let mut vertices = Vec::new();
        
        // Background quad
        let x = position[0];
        let y = position[1];
        let z = position[2];
        let w = size[0];
        let h = size[1];
        
        // Background vertices (darker)
        vertices.extend_from_slice(&[
            x, y, z, 0.2, 0.2, 0.2, 1.0,
            x + w, y, z, 0.2, 0.2, 0.2, 1.0,
            x + w, y + h, z, 0.2, 0.2, 0.2, 1.0,
            x, y + h, z, 0.2, 0.2, 0.2, 1.0,
        ]);
        
        // Progress fill
        let fill_width = w * bar.progress;
        let (r, g, b) = match bar.color {
            ProgressColor::Green => (0.0, 1.0, 0.0),
            ProgressColor::Blue => (0.0, 0.5, 1.0),
            ProgressColor::Yellow => (1.0, 1.0, 0.0),
            ProgressColor::Red => (1.0, 0.0, 0.0),
            ProgressColor::Purple => (0.5, 0.0, 1.0),
            ProgressColor::Custom(r, g, b) => (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
        };
        
        vertices.extend_from_slice(&[
            x, y, z + 0.01, r, g, b, 1.0,
            x + fill_width, y, z + 0.01, r, g, b, 1.0,
            x + fill_width, y + h, z + 0.01, r, g, b, 1.0,
            x, y + h, z + 0.01, r, g, b, 1.0,
        ]);
        
        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_progress_update() {
        let mut visual = ProcessVisual::default();
        
        visual.update_progress(0.5);
        assert_eq!(visual.progress_bar.progress, 0.5);
        assert_eq!(visual.progress_bar.segments, 5);
        assert_eq!(visual.animation, AnimationState::Running);
        
        visual.update_progress(1.0);
        assert_eq!(visual.progress_bar.segments, 10);
        assert_eq!(visual.animation, AnimationState::Finishing);
    }
    
    #[test]
    fn test_text_overlay_lifetime() {
        let mut visual = ProcessVisual::default();
        
        visual.add_text("Test".to_string(), 1.0);
        assert_eq!(visual.text_overlays.len(), 1);
        
        // Update past duration
        visual.update(1.5);
        assert_eq!(visual.text_overlays.len(), 0);
    }
    
    #[test]
    fn test_quality_visuals() {
        let (color, particles) = quality_to_visual(QualityLevel::Perfect);
        assert_eq!(color, ProgressColor::Custom(255, 215, 0));
        assert_eq!(particles.len(), 2);
    }
}