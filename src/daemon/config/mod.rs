pub use self::backlight::{BacklightConfig, BacklightMethod};
pub use self::fan::FanConfig;
pub use self::profiles::{DiskConfig, ProfileConfig, PStateConfig, ScsiPolicy};
pub use self::radeon::{RadeonConfig, RadeonProfile, RadeonDpmState};
use std::collections::HashMap;
use std::{fs, io};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub fans: FanConfig,
    #[serde(default)]
    pub profiles: HashMap<String, ProfileConfig>,
}

const DEFAULT: &str = "/usr/lib/system76-power/config.toml";
const USER: &str = "/etc/system76-power/config.toml";

impl Config {
    pub fn new() -> Self {
        let mut config = Config::from_path(DEFAULT)
            .expect("system default config is missing or corrupted");

        match Config::from_path(USER) {
            Ok(ref user) => config.update_with(user),
            Err(ref why) => if let ConfigError::Parse(_) = why {
                eprintln!("{}", why);
            }
        }

        config
    }

    fn from_path(path: &'static str) -> Result<Self, ConfigError> {
        fs::read_to_string(path)
            .map_err(|error| ConfigError::Read(path, error))
            .and_then(|ref data| toml::from_str::<Self>(data).map_err(ConfigError::Parse))
            .map(|mut config| {
                config.profiles.values_mut().for_each(Clamp::clamp);
                config
            })
    }

    fn update_with(&mut self, other: &Self) {
        for (profile, config) in &other.profiles {
            self.profiles.entry(profile.to_string())
                .and_modify(|current| current.update_with(config))
                .or_insert_with(|| config.clone());
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(display = "unable to read config file at {}: {}", _0, _1)]
    Read(&'static str, io::Error),
    #[error(display = "failed to parse config: {}", _0)]
    Parse(toml::de::Error),
}

mod backlight {
    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct BacklightConfig {
        #[serde(default)]
        pub method: BacklightMethod,
        #[serde(default)]
        pub keyboard: Option<u8>,
        #[serde(default)]
        pub screen: Option<u8>,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum BacklightMethod {
        None,
        Lower
    }

    impl Default for BacklightMethod {
        fn default() -> Self {
            BacklightMethod::None
        }
    }
}

mod fan {
    use crate::fan::FanPoint;
    use std::collections::HashMap;

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct FanConfig {
        #[serde(default = "default_true")]
        pub enabled: bool,
        #[serde(default)]
        pub curves: HashMap<String, FanPointConfig>
    }

    #[derive(Clone, Copy, Debug, Default, Deserialize)]
    pub struct FanPointConfig {
        pub temp: i16,
        pub duty: u16
    }

    impl From<FanPointConfig> for FanPoint {
        fn from(config: FanPointConfig) -> Self {
            FanPoint::new(config.temp * 1000, config.duty * 1000)
        }
    }

    const fn default_true() -> bool { true }
}

mod radeon {
    #[derive(Clone, Copy, Debug, Default, Deserialize)]
    pub struct RadeonConfig {
        #[serde(default)]
        pub profile: RadeonProfile,
        #[serde(default)]
        pub dpm_state: RadeonDpmState,
        #[serde(default)]
        pub dpm_perf: RadeonProfile
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum RadeonProfile {
        Auto,
        Low,
        High
    }

    impl From<RadeonProfile> for &'static str {
        fn from(profile: RadeonProfile) -> Self {
            match profile {
                RadeonProfile::Auto => "auto",
                RadeonProfile::Low => "low",
                RadeonProfile::High => "high"
            }
        }
    }

    impl Default for RadeonProfile {
        fn default() -> Self {
            RadeonProfile::Auto
        }
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum RadeonDpmState {
        Battery,
        Performance
    }

    impl From<RadeonDpmState> for &'static str {
        fn from(state: RadeonDpmState) -> Self {
            match state {
                RadeonDpmState::Battery => "battery",
                RadeonDpmState::Performance => "performance"
            }
        }
    }

    impl Default for RadeonDpmState {
        fn default() -> Self {
            RadeonDpmState::Performance
        }
    }
}

mod profiles {
    use super::backlight::BacklightConfig;
    use super::radeon::RadeonConfig;
    use super::Clamp;

    #[derive(Clone, Copy, Debug, Default, Deserialize)]
    pub struct DiskConfig {
        pub apm_level: u8,
        pub autosuspend_delay: u32,
    }

    #[derive(Clone, Copy, Debug, Deserialize)]
    pub enum ScsiPolicy {
        #[serde(rename = "max_performance")]
        Max,
        #[serde(rename = "medium_power")]
        Medium,
        #[serde(rename = "med_power_with_dipm")]
        MediumWithDipm,
        #[serde(rename = "min_power")]
        Minimum,
    }

    impl From<ScsiPolicy> for &'static str {
        fn from(policy: ScsiPolicy) -> Self {
            match policy {
                ScsiPolicy::Max => "max_power",
                ScsiPolicy::Medium => "medium_power",
                ScsiPolicy::MediumWithDipm => "med_power_with_dipm",
                ScsiPolicy::Minimum => "min_power"
            }
        }
    }

    impl ScsiPolicy {
        pub fn default_set() -> [ScsiPolicy; 2] {
            [ScsiPolicy::MediumWithDipm, ScsiPolicy::Medium]
        }
    }

    #[derive(Clone, Copy, Debug, Default, Deserialize)]
    pub struct PStateConfig {
        pub min: u8,
        pub max: u8,
        pub turbo: bool,
    }

    impl Clamp for PStateConfig {
        fn clamp(&mut self) {
            self.min = ::std::cmp::min(self.min, 100);
            self.max = ::std::cmp::min(self.max, 100);
            self.max = ::std::cmp::max(self.min, self.max);
        }
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct ProfileConfig {
        #[serde(default)]
        pub backlight: Option<BacklightConfig>,
        #[serde(default)]
        pub disk: Option<DiskConfig>,
        #[serde(default)]
        pub laptop_mode: Option<u8>,
        #[serde(default)]
        pub max_lost_work: Option<u32>,
        #[serde(default)]
        pub pci_runtime_pm: Option<bool>,
        #[serde(default)]
        pub pstate: Option<PStateConfig>,
        #[serde(default)]
        pub radeon: Option<RadeonConfig>,
        #[serde(default)]
        pub scsi_host_link_time_pm_policy: Option<[ScsiPolicy; 2]>
    }

    impl ProfileConfig {
        pub fn update_with(&mut self, other: &Self) {
            fn update_option<T: Clone>(from: &mut Option<T>, with: Option<&T>) {
                *from = from.take().or(with.map(Clone::clone));
            }

            update_option(&mut self.backlight, other.backlight.as_ref());
            update_option(&mut self.disk, other.disk.as_ref());
            update_option(&mut self.laptop_mode, other.laptop_mode.as_ref());
            update_option(&mut self.max_lost_work, other.max_lost_work.as_ref());
            update_option(&mut self.pci_runtime_pm, other.pci_runtime_pm.as_ref());
            update_option(&mut self.pstate, other.pstate.as_ref());
            update_option(&mut self.radeon, other.radeon.as_ref());
        }
    }

    impl Clamp for ProfileConfig {
        fn clamp(&mut self) {
            if let Some(ref mut pstate) = self.pstate {
                pstate.clamp();
            }

            if let Some(ref mut laptop_mode) = self.max_lost_work {
                *laptop_mode = ::std::cmp::min(3, *laptop_mode);
            }
        }
    }
}

trait Clamp {
    fn clamp(&mut self);
}
