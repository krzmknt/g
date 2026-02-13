use std::time::{Duration, Instant};

/// Loading state for async data fetching
#[derive(Debug, Clone, Default)]
pub enum LoadingState<T> {
    /// Initial state, not yet loaded
    #[default]
    NotLoaded,
    /// Currently loading data (shows spinner)
    Loading {
        started_at: Instant,
        spinner_frame: usize,
    },
    /// Successfully loaded data
    Loaded(T),
    /// Loading failed with error
    Error(String),
    /// Loading timed out
    Timeout,
    /// Refreshing in background (keeps showing old data, no spinner)
    BackgroundRefreshing { started_at: Instant },
}

impl<T> LoadingState<T> {
    pub fn new() -> Self {
        LoadingState::NotLoaded
    }

    pub fn start_loading() -> Self {
        LoadingState::Loading {
            started_at: Instant::now(),
            spinner_frame: 0,
        }
    }

    pub fn start_background_refresh() -> Self {
        LoadingState::BackgroundRefreshing {
            started_at: Instant::now(),
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading { .. })
    }

    pub fn is_refreshing(&self) -> bool {
        matches!(
            self,
            LoadingState::Loading { .. } | LoadingState::BackgroundRefreshing { .. }
        )
    }

    pub fn is_background_refreshing(&self) -> bool {
        matches!(self, LoadingState::BackgroundRefreshing { .. })
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, LoadingState::Loaded(_))
    }

    pub fn can_retry(&self) -> bool {
        matches!(
            self,
            LoadingState::Error(_) | LoadingState::Timeout | LoadingState::NotLoaded
        )
    }

    /// Check if loading has exceeded timeout duration
    pub fn check_timeout(&self, timeout: Duration) -> bool {
        match self {
            LoadingState::Loading { started_at, .. } => started_at.elapsed() > timeout,
            LoadingState::BackgroundRefreshing { started_at } => started_at.elapsed() > timeout,
            _ => false,
        }
    }

    /// Advance spinner frame
    pub fn tick_spinner(&mut self) {
        if let LoadingState::Loading { spinner_frame, .. } = self {
            *spinner_frame = (*spinner_frame + 1) % SPINNER_FRAMES.len();
        }
    }

    /// Get current spinner character
    pub fn spinner_char(&self) -> &'static str {
        if let LoadingState::Loading { spinner_frame, .. } = self {
            SPINNER_FRAMES[*spinner_frame]
        } else {
            " "
        }
    }
}

/// Spinner animation frames
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Default timeout duration for network requests
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
