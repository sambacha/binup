/// Central constants for the gup project

/// Application name
pub const APP_NAME: &str = "gup";

/// Application display name
pub const APP_DISPLAY_NAME: &str = "Generic Updater Program";

/// Self-update cron job marker
pub const SELFUPDATE_CRON_MARKER: &str = "4c79c12db1d34bbbab1f6c6f838f423f";

/// Shell script markers for PATH management
pub const SHELL_SCRIPT_START_MARKER: &[u8] = b"# >>> gup initialize >>>";
pub const SHELL_SCRIPT_END_MARKER: &[u8] = b"# <<< gup initialize <<<";
pub const SHELL_SCRIPT_HEADER: &[u8] =
    b"\n\n# !! Contents within this block are managed by gup !!\n\n";

/// GitHub organization and repository for self-updates
pub const GITHUB_ORG: &str = "anthropics";
pub const GITHUB_REPO: &str = "gup";

/// Download base URL for self-updates
pub const SELFUPDATE_BASE_URL: &str = "https://github.com/anthropics/gup/releases/download";

/// Network retry configuration defaults
pub const RETRY_MAX_ATTEMPTS: u32 = 3;
pub const RETRY_INITIAL_DELAY_MS: u64 = 1000;
pub const RETRY_MAX_DELAY_MS: u64 = 30000;
pub const RETRY_EXPONENTIAL_BASE: f64 = 2.0;

/// Default update channels
pub const DEFAULT_UPDATE_CHANNEL: &str = "release";

/// Self-update interval (in minutes) for background updates
pub const DEFAULT_SELFUPDATE_INTERVAL: i64 = 1440; // 24 hours
