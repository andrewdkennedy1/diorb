# Requirements Document

## Introduction

DIORB (Disk IO Rust Bench) is a cross-platform desktop TUI application that measures sequential and random disk performance with zero CLI flags. The tool targets sysadmins, developers, and power users across Windows, Linux, and macOS platforms, providing fast startup, intuitive navigation, and accurate performance measurements with live feedback.

## Requirements

### Requirement 1

**User Story:** As a system administrator, I want to quickly launch a disk benchmark tool without learning command-line flags, so that I can immediately assess disk performance.

#### Acceptance Criteria

1. WHEN the application is launched THEN the system SHALL start within 1 second
2. WHEN the application starts THEN the system SHALL display a main menu with Start Test, View Results, Settings, and Exit options
3. WHEN navigating the interface THEN the system SHALL respond to arrow keys, Tab, Enter, Esc, and Q without requiring memorization of complex shortcuts

### Requirement 2

**User Story:** As a developer, I want to configure benchmark parameters through an intuitive interface, so that I can test specific scenarios relevant to my application.

#### Acceptance Criteria

1. WHEN accessing the configuration menu THEN the system SHALL provide drop-downs for Disk, Mode, File Size, Block Size, Duration, and Threads
2. WHEN selecting benchmark mode THEN the system SHALL offer Sequential Write, Sequential Read, Random R/W, and Mixed options
3. WHEN configuring file size THEN the system SHALL default to 1 GiB with 64 KiB blocks for sequential operations
4. WHEN configuring random operations THEN the system SHALL default to 4 KiB blocks with 30 second duration
5. WHEN configuring mixed operations THEN the system SHALL default to 70% read, 30% write with 4 threads

### Requirement 3

**User Story:** As a power user, I want to see live performance metrics during testing, so that I can monitor progress and identify performance patterns in real-time.

#### Acceptance Criteria

1. WHEN a benchmark starts THEN the system SHALL display live MB/s statistics within 200 milliseconds
2. WHEN running tests THEN the system SHALL show real-time IOPS and latency measurements
3. WHEN testing is in progress THEN the system SHALL display a progress bar indicating completion status
4. WHEN displaying metrics THEN the system SHALL update statistics continuously throughout the test duration

### Requirement 4

**User Story:** As a system administrator, I want comprehensive test results with historical data, so that I can track performance trends and compare different configurations.

#### Acceptance Criteria

1. WHEN a benchmark completes THEN the system SHALL record bytes processed, elapsed time, throughput, IOPS, and min/avg/max latency
2. WHEN viewing results THEN the system SHALL display a complete summary with Save and Back options
3. WHEN saving results THEN the system SHALL store data in $HOME/.diorb/results.json
4. WHEN accessing history THEN the system SHALL display a list of past benchmark runs
5. WHEN the results file exceeds 100 entries THEN the system SHALL rotate older entries automatically

### Requirement 5

**User Story:** As a cross-platform user, I want consistent performance measurements across different operating systems, so that I can compare results regardless of the platform.

#### Acceptance Criteria

1. WHEN running on Windows THEN the system SHALL use FILE_FLAG_WRITE_THROUGH and FILE_FLAG_NO_BUFFERING for accurate measurements
2. WHEN running on Linux or macOS THEN the system SHALL use O_DIRECT with fsync fallback when unavailable
3. WHEN measuring SATA SSD performance THEN the system SHALL maintain ±5% margin of error across three repeated runs
4. WHEN measuring NVMe performance THEN the system SHALL maintain ±3% margin of error across three repeated runs
5. WHEN measuring HDD performance THEN the system SHALL maintain ±8% margin of error across three repeated runs
6. WHEN measuring latency on SSD THEN the system SHALL be accurate within ±1ms
7. WHEN measuring latency on HDD THEN the system SHALL be accurate within ±3ms

### Requirement 6

**User Story:** As a user with limited screen space, I want a responsive interface that works on standard terminal sizes, so that I can use the tool in various environments.

#### Acceptance Criteria

1. WHEN the terminal is 80×24 characters THEN the system SHALL display all interface elements properly
2. WHEN drawing the interface THEN the system SHALL use single Unicode borders only
3. WHEN displaying colors THEN the system SHALL use cyan accents, green spinner, and white text
4. WHEN rendering text THEN the system SHALL avoid em dash glyphs for compatibility

### Requirement 7

**User Story:** As a performance-conscious user, I want the benchmark to complete default tests quickly, so that I can get results without long wait times.

#### Acceptance Criteria

1. WHEN running a default write test on 1 GiB file THEN the system SHALL complete within 30 seconds
2. WHEN performing file operations THEN the system SHALL use buffered writes disabled for accurate measurements
3. WHEN creating temporary files THEN the system SHALL place them under <target>/DIORB_TMP_XXXX.dat
4. WHEN tests complete THEN the system SHALL remove temporary files unless Settings → Keep Files is enabled

### Requirement 8

**User Story:** As a user, I want my settings and results to persist between sessions, so that I can maintain my preferred configuration and access historical data.

#### Acceptance Criteria

1. WHEN saving settings THEN the system SHALL store configuration at $CONFIG_HOME/diorb.toml
2. WHEN saving results THEN the system SHALL append data to $DATA_HOME/diorb/results.json
3. WHEN the application starts THEN the system SHALL load previously saved settings automatically
4. WHEN accessing results history THEN the system SHALL display previously saved benchmark data