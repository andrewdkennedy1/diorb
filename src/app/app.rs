//! Main application controller
//! 
//! Manages the TUI, application state, and screen rendering loop.

use crate::{
    app::{
        screens::{ConfigScreen, ResultAction, RunningScreen, StartScreen, ResultsScreen},
        state::{AppState, NavigationAction, StateManager},
        tui::Tui,
    },
    bench::worker::WorkerManager,
    config::{persistence, BenchmarkConfig},
    models::result::BenchmarkResult,
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

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
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
                _ => {
                    // Handle other states like History, Settings, Exit
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
                AppState::Start => self.handle_start_screen_events(nav_action).await?,
                AppState::Config => self.handle_config_screen_events(key).await?,
                AppState::Running => self.handle_running_screen_events(nav_action).await,
                AppState::Results => self.handle_results_screen_events(nav_action),
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_start_screen_events(&mut self, action: NavigationAction) -> Result<()> {
        match action {
            NavigationAction::Up => self.start_screen.select_previous(),
            NavigationAction::Down => self.start_screen.select_next(),
            NavigationAction::Select => {
                // Start benchmark with default config for selected disk
                self.config = BenchmarkConfig::default().with_disk_path(self.start_screen.selected_disk().clone());
                let mut manager = WorkerManager::new(self.config.clone())?;
                let (tx, rx) = mpsc::channel(100);
                manager.start_benchmark(tx).await?;
                self.worker_manager = Some(manager);
                self.progress_rx = Some(rx);
                self.state_manager.transition_to(AppState::Running);
            }
            NavigationAction::Back => self.state_manager.quit(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_config_screen_events(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        if let Some(new_screen) = self.config_screen.handle_key_event(key) {
            if new_screen == AppState::Start {
                 // Potentially save config before exiting
                self.config = self.config_screen.get_config();
                let mut manager = WorkerManager::new(self.config.clone())?;
                let (tx, rx) = mpsc::channel(100);
                manager.start_benchmark(tx).await?;
                self.worker_manager = Some(manager);
                self.progress_rx = Some(rx);
                self.state_manager.transition_to(AppState::Running);
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
            },
            NavigationAction::Select if self.running_screen.is_completed() => {
                if let Some(manager) = self.worker_manager.take() {
                    if let Ok(results) = manager.wait_for_completion().await {
                        if let Ok(final_result) = manager.combine_results(results) {
                            self.results_screen.set_result(final_result);
                            self.state_manager.transition_to(AppState::Results);
                        }
                    }
                }
            },
            _ => {
                // Handle cancellation logic here
            }
        }
    }

    fn handle_results_screen_events(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::Left => self.results_screen.select_previous_action(),
            NavigationAction::Right => self.results_screen.select_next_action(),
            NavigationAction::Select => {
                match self.results_screen.selected_action() {
                    ResultAction::Save => {
                        if let Some(result) = self.results_screen.result() {
                            match persistence::ResultsStorage::new() {
                                Ok(storage) => {
                                    match storage.append_result(result.clone()) {
                                        Ok(_) => self.results_screen.complete_save(true, "Result saved!".to_string()),
                                        Err(e) => self.results_screen.complete_save(false, format!("Error: {}", e)),
                                    }
                                }
                                Err(e) => self.results_screen.complete_save(false, format!("Error: {}", e)),
                            }
                        }
                    },
                    ResultAction::Back => self.state_manager.transition_to(AppState::Start),
                }
            }
            NavigationAction::Back => self.state_manager.transition_to(AppState::Start),
            _ => {}
        }
    }
}