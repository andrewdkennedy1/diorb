# Implementation Plan

- [x] 1. Set up project structure and dependencies
  - Initialize Cargo project with required dependencies (tokio, ratatui, crossterm, serde, indicatif, byte-unit, humantime)
  - Create module directory structure (config, models, util, io, bench, app)
  - Set up lib.rs with public re-exports and common error types
  - _Requirements: 1.1, 1.2_

- [x] 2. Implement core data models and configuration
  - [x] 2.1 Create benchmark configuration structures






    - Define BenchmarkConfig struct with all required fields
    - Implement BenchmarkMode enum with variants for different test types
    - Add validation methods for configuration parameters
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 2.2 Implement benchmark result data models





    - Create BenchmarkResult struct with timestamp, config, and metrics
    - Define PerformanceMetrics struct for throughput, IOPS, and latency data
    - Implement LatencyStats struct with min/avg/max and percentiles
    - Add serialization support with serde for JSON persistence
    - _Requirements: 4.1, 4.2_

- [x] 3. Create configuration management system






  - [x] 3.1 Implement configuration loading and saving



    - Write functions to load settings from $CONFIG_HOME/diorb.toml
    - Implement configuration validation and default value handling
    - Create save functionality for user preferences
    - Add error handling for configuration file operations
    - _Requirements: 8.1, 8.3_

  - [x] 3.2 Implement results persistence system


    - Create functions to append results to $DATA_HOME/diorb/results.json
    - Implement automatic rotation when results exceed 100 entries
    - Add result loading functionality for history display
    - Write unit tests for persistence operations
    - _Requirements: 4.3, 4.4, 4.5, 8.2, 8.4_


- [-] 4. Build utility modules



  - [x] 4.1 Create units helper module





    - Implement human-readable size formatting functions
    - Add duration parsing and display utilities
    - Create rate calculation functions for MB/s and IOPS
    - Write unit tests for all utility functions
    - _Requirements: 3.1, 3.2, 3.3, 3.4_


- [x] 5. Implement platform-specific I/O operations






  - [x] 5.1 Create disk I/O abstraction layer


    - Define common traits for cross-platform file operations
    - Implement Windows-specific direct I/O with FILE_FLAG_WRITE_THROUGH and FILE_FLAG_NO_BUFFERING
    - Implement Unix-specific direct I/O with O_DIRECT and fsync fallback
    - Add temporary file management with proper cleanup
    - _Requirements: 5.1, 5.2, 7.3, 7.4_


  - [x] 5.2 Add I/O performance optimizations

    - Implement buffer pre-allocation and reuse strategies
    - Add optimal block size detection based on storage type
    - Create async wrapper functions using tokio::task::spawn_blocking
    - Write integration tests for I/O operations across platforms
    - _Requirements: 5.3, 5.4, 5.5, 5.6, 5.7, 7.2_


- [-] 6. Build benchmark engine core




  - [x] 6.1 Implement sequential benchmark operations


    - Create sequential write benchmark with configurable file size and block size
    - Implement sequential read benchmark with same file reuse
    - Add real-time progress tracking and metrics collection
    - Write unit tests for sequential operations
    - _Requirements: 2.1, 2.2, 4.1, 7.1_

  - [x] 6.2 Create benchmark worker management system



    - Implement async task spawning for I/O operations
    - Create result streaming via tokio channels with real-time updates
    - Add benchmark cancellation and cleanup handling
    - Implement thread pool coordination for multiple workers
    - _Requirements: 3.1, 3.2, 3.3, 2.5_

  - [x] 6.3 Add random and mixed I/O benchmark modes
    - Implement random read/write operations with 4 KiB blocks
    - Create mixed mode with configurable read/write ratios (70%/30% default)
    - Add duration-based testing with 30-second default

    - Write comprehensive tests for all benchmark modes
    - _Requirements: 2.3, 2.4, 2.5_
- [x] 7. Implement TUI foundation





  - [x] 7.1 Create terminal management system


    - Initialize crossterm backend with proper terminal setup
    - Implement screen clearing and restoration on exit
    - Add terminal size detection and responsive layout handling
    - Create event loop for keyboard input processing
    - _Requirements: 1.2, 6.1, 6.3_

  - [x] 7.2 Build TUI application state management


    - Create application state enum for different screens
    - Implement state transitions and navigation logic
    - Add keyboard event handling for arrows, Tab, Enter, Esc, Q
    - Write unit tests for state management
    - _Requirements: 1.3, 6.4_

- [-] 8. Create TUI screen components



  - [x] 8.1 Implement start screen



    - Create main menu with Start Test, View Results, Settings, Exit options
    - Add navigation highlighting and selection handling
    - Implement responsive layout for 80×24 minimum terminal size
    - Apply color scheme with cyan accents and white text
    - _Requirements: 1.2, 6.1, 6.2, 6.3_

  - [x] 8.2 Build configuration screen



    - Create drop-down menus for Disk, Mode, File Size, Block Size, Duration, Threads
    - Implement input validation and real-time parameter updates
    - Add default value handling for different benchmark modes
    - Write tests for configuration screen interactions
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 8.3 Implement running screen with live metrics
    - Create real-time display for MB/s, IOPS, and latency statistics
    - Add progress bar with completion percentage and time estimates
    - Implement live updates within 200ms of benchmark start
    - Add cancellation handling with proper cleanup
    - _Requirements: 3.1, 3.2, 3.3, 3.4_


  - [x] 8.4 Create results screen and history view
    - Build comprehensive results display with all performance metrics
    - Implement Save and Back button functionality

    - Create history list showing past benchmark runs
    - Add result comparison and export capabilities
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 9. Integrate components and add error handling




  - [x] 9.1 Connect benchmark engine to TUI



    - Wire benchmark workers to running screen for live updates
    - Implement result flow from engine to results screen
    - Add proper error propagation and user-friendly error messages
    - Create graceful degradation for I/O permission issues
    - _Requirements: 3.1, 3.2, 3.3, 4.1, 4.2_

  - [x] 9.2 Add comprehensive error handling
    - Implement DIOrbError enum with specific error types
    - Add retry mechanisms for transient I/O failures

    - Create fallback strategies for direct I/O unavailability
    - Write error handling tests for various failure scenarios
    - _Requirements: 5.1, 5.2, 7.3, 7.4_


- [x] 10. Performance optimization and validation



  - [x] 10.1 Implement performance measurement accuracy
    - Add latency measurement with microsecond precision
    - Implement throughput calculation validation
    - Create accuracy testing against known benchmarks
    - Optimize measurement overhead to minimize interference
    - _Requirements: 5.3, 5.4, 5.5, 5.6, 5.7_

  - [x] 10.2 Add startup time and responsiveness optimizations
    - Optimize application initialization to meet 1-second startup requirement
    - Implement lazy loading for non-critical components
    - Add performance profiling and memory usage monitoring
    - Create integration tests for performance requirements
    - _Requirements: 1.1, 7.1_

- [x] 11. Final integration and testing


  - [x] 11.1 Create comprehensive integration tests
    - Write end-to-end tests for complete benchmark workflows
    - Add cross-platform compatibility tests
    - Implement automated testing for all TUI interactions
    - Create performance regression test suite
    - _Requirements: 1.1, 1.2, 1.3, 7.1_

  - [x] 11.2 Add final polish and documentation
    - Implement proper application cleanup and resource management
    - Add inline code documentation and examples
    - Create user-facing help text and error messages
    - Write final integration tests for all requirements
    - _Requirements: 7.4, 8.1, 8.2, 8.3, 8.4_

