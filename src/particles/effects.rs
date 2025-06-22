use glam::Vec3;
use std::time::Duration;

use crate::particles::{EmitterShape, ParticleEmitter, ParticleType};

/// Pre-configured particle effects
#[derive(Debug, Clone)]
pub struct ParticleEffect {
    pub emitters: Vec<ParticleEmitter>,
    pub name: String,
    pub duration: Duration,
    pub looping: bool,
}

/// Common effect presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPreset {
    // Environmental
    Campfire,
    Torch,
    Explosion,
    Smoke,
    Steam,

    // Combat
    Hit,
    CriticalHit,
    Heal,
    Shield,

    // Magic
    Teleport,
    Enchant,
    Portal,
    Spell,

    // Block effects
    BlockBreak,
    BlockPlace,
    Mining,

    // Weather
    RainSplash,
    SnowFall,
    Lightning,

    // Liquid
    WaterSplash,
    LavaBubble,
    WaterFall,
}

impl ParticleEffect {
    /// Create an effect from a preset
    pub fn from_preset(preset: EffectPreset, position: Vec3) -> Self {
        match preset {
            EffectPreset::Campfire => Self::campfire(position),
            EffectPreset::Torch => Self::torch(position),
            EffectPreset::Explosion => Self::explosion(position),
            EffectPreset::Hit => Self::hit(position),
            EffectPreset::Heal => Self::heal(position),
            EffectPreset::Teleport => Self::teleport(position),
            EffectPreset::BlockBreak => Self::block_break(position),
            EffectPreset::WaterSplash => Self::water_splash(position),
            _ => Self::default(position),
        }
    }

    /// Campfire effect
    fn campfire(position: Vec3) -> Self {
        let mut fire = ParticleEmitter::fire(position);
        fire.shape = EmitterShape::Disc {
            radius: 0.4,
            normal: Vec3::Y,
        };
        fire.emission_rate = 40.0;

        let mut smoke = ParticleEmitter::smoke(position + Vec3::Y * 0.5);
        smoke.emission_rate = 3.0;
        smoke.velocity_range = (Vec3::new(-0.2, 1.0, -0.2), Vec3::new(0.2, 2.0, 0.2));

        let mut sparks = ParticleEmitter::new(position, ParticleType::Spark);
        sparks.emission_rate = 2.0;
        sparks.velocity_range = (Vec3::new(-1.0, 2.0, -1.0), Vec3::new(1.0, 4.0, 1.0));
        sparks.lifetime_range = (0.5, 1.0);

        Self {
            emitters: vec![fire, smoke, sparks],
            name: "Campfire".to_string(),
            duration: Duration::from_secs(3600), // 1 hour
            looping: true,
        }
    }

    /// Torch effect
    fn torch(position: Vec3) -> Self {
        let mut fire = ParticleEmitter::fire(position);
        fire.shape = EmitterShape::Point;
        fire.emission_rate = 15.0;
        fire.velocity_range = (Vec3::new(-0.2, 0.5, -0.2), Vec3::new(0.2, 1.5, 0.2));
        fire.size_range = (0.1, 0.2);

        let mut smoke = ParticleEmitter::smoke(position + Vec3::Y * 0.3);
        smoke.emission_rate = 1.0;
        smoke.size_range = (0.2, 0.3);

        Self {
            emitters: vec![fire, smoke],
            name: "Torch".to_string(),
            duration: Duration::from_secs(3600),
            looping: true,
        }
    }

    /// Explosion effect
    fn explosion(position: Vec3) -> Self {
        // Initial flash
        let mut flash = ParticleEmitter::new(position, ParticleType::Fire);
        flash.shape = EmitterShape::Sphere { radius: 0.1 };
        flash.emission_rate = 500.0;
        flash.velocity_range = (Vec3::new(-8.0, -8.0, -8.0), Vec3::new(8.0, 8.0, 8.0));
        flash.lifetime_range = (0.2, 0.4);
        flash.size_range = (0.5, 1.0);
        flash.duration = Some(Duration::from_millis(100));

        // Debris
        let mut debris = ParticleEmitter::new(position, ParticleType::BlockBreak);
        debris.shape = EmitterShape::Sphere { radius: 0.5 };
        debris.emission_rate = 200.0;
        debris.velocity_range = (Vec3::new(-5.0, 2.0, -5.0), Vec3::new(5.0, 8.0, 5.0));
        debris.lifetime_range = (1.0, 2.0);
        debris.duration = Some(Duration::from_millis(200));

        // Smoke
        let mut smoke = ParticleEmitter::smoke(position);
        smoke.shape = EmitterShape::Sphere { radius: 1.0 };
        smoke.emission_rate = 50.0;
        smoke.velocity_range = (Vec3::new(-2.0, 1.0, -2.0), Vec3::new(2.0, 4.0, 2.0));
        smoke.size_range = (0.5, 1.5);
        smoke.duration = Some(Duration::from_secs(3));

        Self {
            emitters: vec![flash, debris, smoke],
            name: "Explosion".to_string(),
            duration: Duration::from_secs(3),
            looping: false,
        }
    }

    /// Hit effect
    fn hit(position: Vec3) -> Self {
        let mut impact = ParticleEmitter::new(position, ParticleType::Damage);
        impact.shape = EmitterShape::Sphere { radius: 0.2 };
        impact.emission_rate = 30.0;
        impact.velocity_range = (Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 3.0, 2.0));
        impact.lifetime_range = (0.3, 0.6);
        impact.size_range = (0.1, 0.2);
        impact.duration = Some(Duration::from_millis(200));

        Self {
            emitters: vec![impact],
            name: "Hit".to_string(),
            duration: Duration::from_millis(200),
            looping: false,
        }
    }

    /// Heal effect
    fn heal(position: Vec3) -> Self {
        let mut particles = ParticleEmitter::new(position, ParticleType::Heal);
        particles.shape = EmitterShape::Cylinder {
            radius: 0.5,
            height: 2.0,
        };
        particles.emission_rate = 20.0;
        particles.velocity_range = (Vec3::new(-0.5, 0.5, -0.5), Vec3::new(0.5, 2.0, 0.5));
        particles.lifetime_range = (1.0, 1.5);
        particles.size_range = (0.1, 0.3);
        particles.duration = Some(Duration::from_secs(2));

        Self {
            emitters: vec![particles],
            name: "Heal".to_string(),
            duration: Duration::from_secs(2),
            looping: false,
        }
    }

    /// Teleport effect
    fn teleport(position: Vec3) -> Self {
        let mut portal = ParticleEmitter::new(position, ParticleType::Portal);
        portal.shape = EmitterShape::Cylinder {
            radius: 0.8,
            height: 2.0,
        };
        portal.emission_rate = 100.0;
        portal.velocity_range = (Vec3::ZERO, Vec3::Y * 0.5);
        portal.lifetime_range = (1.0, 2.0);
        portal.duration = Some(Duration::from_secs(1));

        let mut magic = ParticleEmitter::magic(position);
        magic.shape = EmitterShape::Sphere { radius: 1.0 };
        magic.emission_rate = 50.0;
        magic.duration = Some(Duration::from_secs(1));

        Self {
            emitters: vec![portal, magic],
            name: "Teleport".to_string(),
            duration: Duration::from_secs(1),
            looping: false,
        }
    }

    /// Block break effect
    fn block_break(position: Vec3) -> Self {
        let mut particles = ParticleEmitter::new(position, ParticleType::BlockBreak);
        particles.shape = EmitterShape::Box {
            size: Vec3::splat(0.8),
        };
        particles.emission_rate = 50.0;
        particles.velocity_range = (Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 4.0, 2.0));
        particles.lifetime_range = (0.5, 1.0);
        particles.duration = Some(Duration::from_millis(200));

        let mut dust = ParticleEmitter::new(position, ParticleType::BlockDust);
        dust.shape = EmitterShape::Box {
            size: Vec3::splat(1.0),
        };
        dust.emission_rate = 20.0;
        dust.velocity_range = (Vec3::new(-1.0, -0.5, -1.0), Vec3::new(1.0, 1.0, 1.0));
        dust.lifetime_range = (1.0, 2.0);
        dust.duration = Some(Duration::from_millis(500));

        Self {
            emitters: vec![particles, dust],
            name: "Block Break".to_string(),
            duration: Duration::from_millis(500),
            looping: false,
        }
    }

    /// Water splash effect
    fn water_splash(position: Vec3) -> Self {
        let mut splash = ParticleEmitter::new(position, ParticleType::WaterSplash);
        splash.shape = EmitterShape::Disc {
            radius: 0.5,
            normal: Vec3::Y,
        };
        splash.emission_rate = 100.0;
        splash.velocity_range = (Vec3::new(-2.0, 1.0, -2.0), Vec3::new(2.0, 4.0, 2.0));
        splash.lifetime_range = (0.5, 1.0);
        splash.size_range = (0.05, 0.15);
        splash.duration = Some(Duration::from_millis(300));

        let mut ripples = ParticleEmitter::new(position, ParticleType::WaterSplash);
        ripples.shape = EmitterShape::Disc {
            radius: 0.1,
            normal: Vec3::Y,
        };
        ripples.emission_rate = 5.0;
        ripples.velocity_range = (Vec3::ZERO, Vec3::ZERO);
        ripples.lifetime_range = (1.0, 1.5);
        ripples.size_range = (0.5, 1.0);
        ripples.duration = Some(Duration::from_millis(100));

        Self {
            emitters: vec![splash, ripples],
            name: "Water Splash".to_string(),
            duration: Duration::from_millis(300),
            looping: false,
        }
    }

    /// Default effect
    fn default(position: Vec3) -> Self {
        let emitter = ParticleEmitter::new(position, ParticleType::Dust);

        Self {
            emitters: vec![emitter],
            name: "Default".to_string(),
            duration: Duration::from_secs(1),
            looping: false,
        }
    }

    /// Update all emitters
    pub fn update(&mut self, dt: Duration) -> Vec<crate::particles::Particle> {
        let mut particles = Vec::new();

        for emitter in &mut self.emitters {
            particles.extend(emitter.update(dt));
        }

        particles
    }

    /// Check if effect is finished
    pub fn is_finished(&self) -> bool {
        if self.looping {
            false
        } else {
            self.emitters.iter().all(|e| e.is_finished())
        }
    }

    /// Reset the effect
    pub fn reset(&mut self) {
        for emitter in &mut self.emitters {
            emitter.reset();
            emitter.start();
        }
    }

    /// Stop the effect
    pub fn stop(&mut self) {
        for emitter in &mut self.emitters {
            emitter.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_presets() {
        let explosion = ParticleEffect::from_preset(EffectPreset::Explosion, Vec3::ZERO);
        assert_eq!(explosion.emitters.len(), 3); // Flash, debris, smoke
        assert!(!explosion.looping);

        let campfire = ParticleEffect::from_preset(EffectPreset::Campfire, Vec3::ZERO);
        assert!(campfire.looping);
    }
}
