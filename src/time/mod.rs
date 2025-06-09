pub mod day_night_cycle;
pub mod time_of_day;
pub mod celestial;
pub mod ambient_light;

pub use day_night_cycle::{DayNightCycle, TimeUpdate};
pub use time_of_day::{TimeOfDay, DayPhase, TimeSpeed};
pub use celestial::{CelestialBodies, SunPosition, MoonPhase};
pub use ambient_light::{AmbientLightSettings, calculate_ambient_light};