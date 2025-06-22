/// Transform Stage System
///
/// Multi-stage transformations with requirements and outputs.
/// Each stage can have different inputs, outputs, and conditions.
use crate::instance::InstanceId;
use crate::process::{QualityLevel, TimeUnit};
use serde::{Deserialize, Serialize};

/// Stage in a transformation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStage {
    /// Stage name/identifier
    pub name: String,

    /// Stage index (order)
    pub index: u16,

    /// Requirements to start this stage
    pub requirements: Vec<StageRequirement>,

    /// Expected outputs from this stage
    pub outputs: Vec<StageOutput>,

    /// Duration of this stage
    pub duration: TimeUnit,

    /// Quality impact of this stage
    pub quality_modifier: f32,

    /// Can this stage be skipped?
    pub optional: bool,

    /// Can this stage be repeated?
    pub repeatable: bool,
}

/// Requirement for a stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageRequirement {
    /// Specific items required
    Items(Vec<ItemRequirement>),

    /// Minimum skill level
    SkillLevel(String, u32),

    /// Tool requirement
    Tool(ToolRequirement),

    /// Environmental condition
    Environment(EnvironmentRequirement),

    /// Previous stage completion
    PreviousStage(u16),

    /// Custom requirement
    Custom(String),
}

/// Item requirement details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRequirement {
    pub item_type: u32,
    pub quantity: u32,
    pub min_quality: Option<QualityLevel>,
    pub consume: bool,
}

/// Tool requirement details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequirement {
    pub tool_type: u32,
    pub min_quality: Option<QualityLevel>,
    pub durability_cost: u32,
}

/// Environmental requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentRequirement {
    /// Near specific block type
    NearBlock(u32, u32), // (block_type, max_distance)

    /// Temperature range
    Temperature(f32, f32), // (min, max)

    /// Light level
    LightLevel(u8, u8), // (min, max)

    /// In specific biome
    Biome(u32),

    /// Weather condition
    Weather(WeatherType),
}

/// Weather types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherType {
    Clear,
    Rain,
    Thunder,
    Snow,
}

/// Output from a stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// Output type
    pub output_type: OutputType,

    /// Quantity range
    pub quantity_min: u32,
    pub quantity_max: u32,

    /// Quality impact
    pub quality_bonus: f32,

    /// Probability (0.0-1.0)
    pub probability: f32,
}

/// Types of stage outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    /// Produce items
    Item(u32), // item_type

    /// Grant experience
    Experience(String, u32), // (skill, amount)

    /// Apply effect
    Effect(String, u32), // (effect_id, duration)

    /// Unlock recipe/ability
    Unlock(String),

    /// Trigger event
    Event(String),
}

/// Stage validator - checks if requirements are met
pub struct StageValidator;

impl StageValidator {
    /// Check if all requirements for a stage are met
    pub fn validate_requirements(
        stage: &TransformStage,
        owner: InstanceId,
        available_items: &[InstanceId],
        context: &ValidationContext,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            valid: true,
            missing_requirements: Vec::new(),
            consumed_items: Vec::new(),
        };

        for req in &stage.requirements {
            match req {
                StageRequirement::Items(items) => {
                    for item_req in items {
                        // Check if we have enough items
                        // In real implementation, would check against inventory
                        if item_req.quantity > available_items.len() as u32 {
                            result.valid = false;
                            result.missing_requirements.push(format!(
                                "Missing {} items of type {}",
                                item_req.quantity - available_items.len() as u32,
                                item_req.item_type
                            ));
                        } else if item_req.consume {
                            // Mark items for consumption
                            for i in 0..item_req.quantity {
                                result.consumed_items.push(available_items[i as usize]);
                            }
                        }
                    }
                }

                StageRequirement::SkillLevel(skill, level) => {
                    // Check skill level from context
                    if let Some(&player_level) = context.skill_levels.get(skill.as_str()) {
                        if player_level < *level {
                            result.valid = false;
                            result
                                .missing_requirements
                                .push(format!("Requires {} level {}", skill, level));
                        }
                    } else {
                        result.valid = false;
                        result
                            .missing_requirements
                            .push(format!("Missing skill: {}", skill));
                    }
                }

                StageRequirement::Environment(env_req) => {
                    match env_req {
                        EnvironmentRequirement::Temperature(min, max) => {
                            if context.temperature < *min || context.temperature > *max {
                                result.valid = false;
                                result.missing_requirements.push(format!(
                                    "Temperature must be between {} and {}",
                                    min, max
                                ));
                            }
                        }

                        EnvironmentRequirement::LightLevel(min, max) => {
                            if context.light_level < *min || context.light_level > *max {
                                result.valid = false;
                                result.missing_requirements.push(format!(
                                    "Light level must be between {} and {}",
                                    min, max
                                ));
                            }
                        }

                        _ => {} // Other environment checks
                    }
                }

                _ => {} // Other requirements
            }
        }

        result
    }

    /// Calculate outputs for a completed stage
    pub fn calculate_outputs(
        stage: &TransformStage,
        quality: QualityLevel,
        rng: &mut impl rand::Rng,
    ) -> Vec<ActualOutput> {
        let mut outputs = Vec::new();

        for output in &stage.outputs {
            // Check probability
            if rng.gen::<f32>() > output.probability {
                continue;
            }

            // Calculate quantity
            let quantity = if output.quantity_min == output.quantity_max {
                output.quantity_min
            } else {
                rng.gen_range(output.quantity_min..=output.quantity_max)
            };

            // Apply quality modifier
            let quality_multiplier = 1.0 + output.quality_bonus * (quality as u8 as f32 / 4.0);
            let final_quantity = (quantity as f32 * quality_multiplier) as u32;

            outputs.push(ActualOutput {
                output_type: output.output_type.clone(),
                quantity: final_quantity,
                quality,
            });
        }

        outputs
    }
}

/// Validation context
pub struct ValidationContext {
    pub skill_levels: std::collections::HashMap<String, u32>,
    pub temperature: f32,
    pub light_level: u8,
    pub biome: u32,
    pub weather: WeatherType,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            skill_levels: std::collections::HashMap::new(),
            temperature: 20.0,
            light_level: 15,
            biome: 0,
            weather: WeatherType::Clear,
        }
    }
}

/// Validation result
pub struct ValidationResult {
    pub valid: bool,
    pub missing_requirements: Vec<String>,
    pub consumed_items: Vec<InstanceId>,
}

/// Actual output produced
#[derive(Debug, Clone)]
pub struct ActualOutput {
    pub output_type: OutputType,
    pub quantity: u32,
    pub quality: QualityLevel,
}

/// Transform stage templates
pub struct StageTemplates;

impl StageTemplates {
    /// Simple crafting stage
    pub fn crafting_stage(name: &str, index: u16, duration_seconds: f32) -> TransformStage {
        TransformStage {
            name: name.to_string(),
            index,
            requirements: vec![],
            outputs: vec![],
            duration: TimeUnit::Seconds(duration_seconds),
            quality_modifier: 1.0,
            optional: false,
            repeatable: false,
        }
    }

    /// Smelting stage template
    pub fn smelting_stage() -> TransformStage {
        TransformStage {
            name: "Smelting".to_string(),
            index: 0,
            requirements: vec![
                StageRequirement::Items(vec![ItemRequirement {
                    item_type: 1, // Iron ore
                    quantity: 1,
                    min_quality: None,
                    consume: true,
                }]),
                StageRequirement::Environment(
                    EnvironmentRequirement::NearBlock(100, 3), // Furnace within 3 blocks
                ),
            ],
            outputs: vec![
                StageOutput {
                    output_type: OutputType::Item(2), // Iron ingot
                    quantity_min: 1,
                    quantity_max: 1,
                    quality_bonus: 0.1,
                    probability: 1.0,
                },
                StageOutput {
                    output_type: OutputType::Experience("smithing".to_string(), 5),
                    quantity_min: 1,
                    quantity_max: 1,
                    quality_bonus: 0.0,
                    probability: 1.0,
                },
            ],
            duration: TimeUnit::Seconds(10.0),
            quality_modifier: 1.0,
            optional: false,
            repeatable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requirement_validation() {
        let stage = TransformStage {
            name: "Test Stage".to_string(),
            index: 0,
            requirements: vec![
                StageRequirement::Items(vec![ItemRequirement {
                    item_type: 1,
                    quantity: 5,
                    min_quality: None,
                    consume: true,
                }]),
                StageRequirement::SkillLevel("crafting".to_string(), 10),
            ],
            outputs: vec![],
            duration: TimeUnit::Seconds(5.0),
            quality_modifier: 1.0,
            optional: false,
            repeatable: false,
        };

        let owner = InstanceId::new();
        let items = vec![InstanceId::new(); 3]; // Only 3 items

        let mut context = ValidationContext::default();
        context.skill_levels.insert("crafting".to_string(), 15); // High enough

        let result = StageValidator::validate_requirements(&stage, owner, &items, &context);

        assert!(!result.valid);
        assert_eq!(result.missing_requirements.len(), 1);
        assert!(result.missing_requirements[0].contains("Missing 2 items"));
    }

    #[test]
    fn test_output_calculation() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let stage = TransformStage {
            name: "Test".to_string(),
            index: 0,
            requirements: vec![],
            outputs: vec![StageOutput {
                output_type: OutputType::Item(1),
                quantity_min: 1,
                quantity_max: 3,
                quality_bonus: 0.5,
                probability: 1.0,
            }],
            duration: TimeUnit::Seconds(1.0),
            quality_modifier: 1.0,
            optional: false,
            repeatable: false,
        };

        let outputs = StageValidator::calculate_outputs(&stage, QualityLevel::Excellent, &mut rng);

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].quantity >= 1);
        assert_eq!(outputs[0].quality, QualityLevel::Excellent);
    }
}
