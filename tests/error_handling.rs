use diorb::error::{self, RetryConfig};
use diorb::DIOrbError;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn test_retry_async_eventually_succeeds() {
    static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
    let result = error::retry_async(
        || async {
            let a = ATTEMPTS.fetch_add(1, Ordering::SeqCst);
            if a < 2 {
                Err(DIOrbError::IoError(std::io::Error::new(std::io::ErrorKind::Interrupted, "fail")))
            } else {
                Ok(42u32)
            }
        },
        RetryConfig::default(),
    )
    .await
    .expect("retry should succeed");
    assert_eq!(result, 42);
    assert!(ATTEMPTS.load(Ordering::SeqCst) >= 3);
}

#[test]
fn test_user_friendly_message_and_fallback() {
    let msg = error::user_friendly_message(&DIOrbError::PermissionDenied("x".into()));
    assert!(msg.contains("Permission denied"));
    let fallback = error::create_fallback_strategy(&DIOrbError::DirectIoUnsupported("x".into())).unwrap();
    assert!(fallback.to_lowercase().contains("buffered"));
}
