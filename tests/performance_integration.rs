use diorb::app::App;
use diorb::util::units::calculate_throughput_mbps;
use std::time::{Duration, Instant};

#[test]
fn test_app_startup_time() {
    let start = Instant::now();
    let mut app = App::new().expect("app create");
    app.init().expect("init");
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn test_throughput_calculation_matches() {
    let bytes = 2 * 1024 * 1024u64;
    let dur = Duration::from_secs(2);
    let expected = calculate_throughput_mbps(bytes, dur);
    let metrics =
        diorb::models::PerformanceMetrics::new(bytes, dur, diorb::models::LatencyStats::default());
    assert!((metrics.throughput_mbps - expected).abs() < 0.01);
    assert!(metrics.validate_throughput());
}
