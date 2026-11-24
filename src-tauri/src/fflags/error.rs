#[derive(Debug, thiserror::Error)]
pub enum FlagError {
    #[error("invalid flag name")]
    InvalidName,
    #[error("invalid value for {0}")]
    InvalidValue(String),
    #[error("duplicate flag {0}")]
    Duplicate(String),
    #[error("blocked flag {0}")]
    Blocked(String),
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("configuration is too large")]
    TooLarge,
    #[error("profile not found")]
    ProfileNotFound,
    #[error("flag not found")]
    FlagNotFound,
    #[error("preset profiles are read-only")]
    ReadOnlyProfile,
    #[error("invalid profile name")]
    InvalidProfileName,
    #[error("profile name already used")]
    ProfileNameTaken,
    #[error("too many profiles")]
    TooManyProfiles,
    #[error("backup not found")]
    BackupNotFound,
    #[error("unsafe settings path")]
    UnsafeTarget,
}

impl FlagError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidName => "fflag_invalid_name",
            Self::InvalidValue(_) => "fflag_invalid_value",
            Self::Duplicate(_) => "fflag_duplicate",
            Self::Blocked(_) => "fflag_blocked",
            Self::InvalidJson(_) => "fflag_invalid_json",
            Self::TooLarge => "fflag_too_large",
            Self::ProfileNotFound => "fflag_profile_not_found",
            Self::FlagNotFound => "fflag_not_found",
            Self::ReadOnlyProfile => "fflag_read_only",
            Self::InvalidProfileName => "fflag_invalid_profile_name",
            Self::ProfileNameTaken => "fflag_profile_name_taken",
            Self::TooManyProfiles => "fflag_too_many_profiles",
            Self::BackupNotFound => "fflag_backup_not_found",
            Self::UnsafeTarget => "fflag_unsafe_target",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidName => {
                "Flag names can only use letters, numbers and underscores, and must start with a letter.".into()
            }
            Self::InvalidValue(name) => format!("The value for {name} doesn't match its type."),
            Self::Duplicate(name) => format!("{name} appears more than once. Each flag can only be set once."),
            Self::Blocked(name) => {
                format!("{name} looks related to Roblox's security systems, so BloxClient won't set it.")
            }
            Self::InvalidJson(detail) => format!("That isn't a valid flag configuration: {detail}."),
            Self::TooLarge => "That configuration is too large. The limit is 2,000 flags.".into(),
            Self::ProfileNotFound => "That profile no longer exists.".into(),
            Self::FlagNotFound => "That flag no longer exists in this profile.".into(),
            Self::ReadOnlyProfile => "Recommended presets can't be edited. Duplicate it to make your own copy.".into(),
            Self::InvalidProfileName => "Profile names must be 1 to 40 characters.".into(),
            Self::ProfileNameTaken => "A profile with that name already exists.".into(),
            Self::TooManyProfiles => "You've reached the limit of 50 profiles.".into(),
            Self::BackupNotFound => "That backup no longer exists.".into(),
            Self::UnsafeTarget => {
                "BloxClient didn't write flags because the version's settings folder looks unsafe.".into()
            }
        }
    }
}
