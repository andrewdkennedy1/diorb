//! Main application controller
//!
//! Manages the TUI, application state, and screen rendering loop.

use crate::{
    app::{
        screens::{
            ConfigScreen, HistoryScreen, ResultAction, ResultsScreen, RunningScreen, StartScreen,
        },
        state::{AppState, NavigationAction, StateManager},
        tui::Tui,
    },
    bench::worker::WorkerManager,
    config::{persistence, BenchmarkConfig},
    error,
    Result,
};
use std::io;
use tokio::sync::mpsc;

/// TUI application controller
pub struct App {
    /// Terminal UI handler
    tui: Tui,
    /// Application state manager
    state_manager: StateManager,
    /// Application config
    config: BenchmarkConfig,
    /// Screen components
    start_screen: StartScreen,
    config_screen: ConfigScreen,
    running_screen: RunningScreen,
    results_screen: ResultsScreen,
    history_screen: HistoryScreen,
    /// Whether history results have been loaded
    history_loaded: bool,
    /// Benchmark worker manager
    worker_manager: Option<WorkerManager>,
    /// Progress receiver
    progress_rx: Option<mpsc::Receiver<crate::bench::worker::AggregatedProgress>>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let config = BenchmarkConfig::load()?;
        Ok(Self {
            tui: Tui::new()?,
            state_manager: StateManager::new(),
            config: config.clone(),
            start_screen: StartScreen::new(),
            config_screen: ConfigScreen::new(&config),
            running_screen: RunningScreen::new(),
            results_screen: ResultsScreen::new(),
            history_screen: HistoryScreen::default(),
            history_loaded: false,
            worker_manager: None,
            progress_rx: None,
        })
    }

    /// Initialize the application and TUI
    pub fn init(&mut self) -> Result<()> {
        self.tui.init()?;
        // Load initial data if needed
        Ok(())
    }

    /// Lazily load history results from disk
    fn load_history_results(&mut self) -> Result<()> {
        if !self.history_loaded {
            let results = persistence::ResultsStorage::new()?.load_results()?;
            self.history_screen.set_results(results);
            self.history_loaded = true;
        }
        Ok(())
    }

    /// Start a benchmark using the current configuration
    async fn start_benchmark(&mut self) -> Result<()> {
        let mut manager = WorkerManager::new(self.config.clone())?;
        let (tx, rx) = mpsc::channel(100);
        manager.start_benchmark(tx).await?;
        self.worker_manager = Some(manager);
        self.progress_rx = Some(rx);
        Ok(())
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Starting application in state: {:?}", self.state_manager.current_state());
        
        while !self.state_manager.should_quit() {
            if let Some(rx) = &mut self.progress_rx {
                if let Ok(progress) = rx.try_recv() {
                    self.running_screen.update_progress(progress);
                }
            }
            self.draw()?;
            self.handle_events().await?;
        }
        Ok(())
    }

    /// Draw the current screen
    fn draw(&mut self) -> io::Result<()> {
        self.tui.draw(|f| {
            match self.state_manager.current_state() {
                AppState::Start => self.start_screen.render(f),
                AppState::Config => self.config_screen.render(f.size(), f, &mut ()),
                AppState::Running => self.running_screen.render(f),
                AppState::Results => self.results_screen.render(f),
                AppState::History => self.history_screen.render(f),
                _ => {
                    // Handle other states like Settings, Exit
                }
            }
        })
    }

    /// Handle keyboard events and update state
    async fn handle_events(&mut self) -> Result<()> {
        if let Some(key) = self.tui.handle_events()? {
            let nav_action = StateManager::key_to_navigation(key);

            // Global key handling
            if nav_action == NavigationAction::Quit {
                self.state_manager.quit();
                return Ok(());
            }

            // Screen-specific key handling
            match self.state_manager.current_state().clone() {
                AppState::Start => self.handle_start_screen_events(key).await?,
                AppState::Config => self.handle_config_screen_events(key).await?,
                AppState::Running => self.handle_running_screen_events(nav_action).await,
                AppState::Results => self.handle_results_screen_events(nav_action),
                AppState::History => self.handle_history_screen_events(nav_action),
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_start_screen_events(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        println!("Start screen received key: {:?}", key);
        let action = self.start_screen.handle_key(key.code);
        println!("Start screen action: {:?}", action);
        
        match action {
            NavigationAction::Up => self.start_screen.select_previous(),
            NavigationAction::Down => self.start_screen.select_next(),
            NavigationAction::Select => {
                // Start benchmark with default config for selected disk
                self.config = BenchmarkConfig::default()
                    .with_disk_path(self.start_screen.selected_disk().clone());
                match self.start_benchmark().await {
                    Ok(()) => self.state_manager.transition_to(AppState::Running),
                    Err(e) => {
                        let mut msg = error::user_friendly_message(&e);
                        if let Some(fallback) = error::create_fallback_strategy(&e) {
                            msg = format!("{}\n{}", msg, fallback);
                        }
                        self.running_screen.set_error(msg);
                        self.state_manager.transition_to(AppState::Running);
                    }
                }
            }
            NavigationAction::Right => {
                self.load_history_results()?;
                self.state_manager.transition_to(AppState::History);
            }
            StartScreenAction::OpenConfig => {
                // Set disk path in config and open config screen
                self.config = self.config.clone().with_disk_path(self.start_screen.selected_disk().clone());
                self.config_screen = ConfigScreen::new(&self.config);
                self.state_manager.transition_to(AppState::Config);
            }
            StartScreenAction::Quit => {
                self.state_manager.quit();
            }
            StartScreenAction::None => {}
        }
        Ok(())
    }

    async fn handle_config_screen_events(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        if let Some(new_screen) = self.config_screen.handle_key_event(key) {
            if new_screen == AppState::Start {
                // Potentially save config before exiting
                self.config = self.config_screen.get_config();
                match self.start_benchmark().await {
                    Ok(()) => self.state_manager.transition_to(AppState::Running),
                    Err(e) => {
                        let mut msg = error::user_friendly_message(&e);
                        if let Some(fallback) = error::create_fallback_strategy(&e) {
                            msg = format!("{}\n{}", msg, fallback);
                        }
                        self.running_screen.set_error(msg);
                        self.state_manager.transition_to(AppState::Running);
                    }
                }
            } else {
                self.state_manager.go_back();
            }
        }
        Ok(())
    }

    async fn handle_running_screen_events(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::Back => {
                if let Some(manager) = &self.worker_manager {
                    manager.cancel_all().await.ok();
                }
                self.state_manager.go_back()
            }
            NavigationAction::Cancel => {
                if let Some(manager) = &self.worker_manager {
                    manager.cancel_all().await.ok();
                }
                self.running_screen.request_cancellation();
            }
            NavigationAction::Retry if self.running_screen.has_error() => {
                // Retry benchmark using existing config
                if let Err(e) = self.start_benchmark().await {
                    let mut msg = error::user_friendly_message(&e);
                    if let Some(fallback) = error::create_fallback_strategy(&e) {
                        msg = format!("{}\n{}", msg, fallback);
                    }
                    self.running_screen.set_error(msg);
                } else {
                    self.running_screen.clear_error();
                }
            }
            NavigationAction::Select if self.running_screen.is_completed() => {
                if let Some(manager) = self.worker_manager.take() {
                    if let Ok(results) = manager.wait_for_completion().await {
                        if let Ok(final_result) = manager.combine_results(results) {
                            self.results_screen.set_result(final_result);
                            self.state_manager.transition_to(AppState::Results);
                        }
                    }
                }
            }
            _ => {
                // Ignore other actions
            }
        }
    }

    fn handle_results_screen_events(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::Left => self.results_screen.select_previous_action(),
            NavigationAction::Right => self.results_screen.select_next_action(),
            NavigationAction::Select => match self.results_screen.selected_action() {
                ResultAction::Save => {
                    if let Some(result) = self.results_screen.result() {
                        match persistence::ResultsStorage::new() {
                            Ok(storage) => match storage.append_result(result.clone()) {
                                Ok(_) => self
                                    .results_screen
                                    .complete_save(true, "Result saved!".to_string()),
                                Err(e) => self
                                    .results_screen
                                    .complete_save(false, format!("Error: {}", e)),
                            },
                            Err(e) => self
                                .results_screen
                                .complete_save(false, format!("Error: {}", e)),
                        }
                    }
                }
                ResultAction::Back => self.state_manager.transition_to(AppState::Start),
            },
            NavigationAction::Back => self.state_manager.transition_to(AppState::Start),
            _ => {}
        }
    }

    fn handle_history_screen_events(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::Up => self.history_screen.select_previous(),
            NavigationAction::Down => self.history_screen.select_next(),
            NavigationAction::Select => {
                if let Some(result) = self.history_screen.selected_result() {
                    self.results_screen.set_result(result.clone());
                    self.state_manager.transition_to(AppState::Results);
                }
            }
            NavigationAction::Back => self.state_manager.go_back(),
            _ => {}
        }
    }
}
