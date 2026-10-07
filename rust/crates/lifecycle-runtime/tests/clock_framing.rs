use std::time::{Duration, Instant};

use lifecycle_core::{AuthorityExpiresAt, WaitBudgetMs, WallTimeMs};
use lifecycle_runtime::{
    BoundedFrameReader, BoundedFrameWriter, DeadlineStatus, FrameDiagnosticCode, FrameDiagnostics,
    FrameError, MonotonicDeadline,
};
use tokio::io::{AsyncWriteExt, duplex};

#[test]
fn wall_authority_and_monotonic_wait_have_distinct_exact_boundaries() {
    let start = Instant::now();
    let deadline = MonotonicDeadline::new(start, WaitBudgetMs::new(50)).unwrap();
    let expiry = AuthorityExpiresAt::new(WallTimeMs::new(1_000));
    assert_eq!(
        deadline.status(start, WallTimeMs::new(999), expiry),
        DeadlineStatus::OnTime
    );
    assert_eq!(
        deadline.status(start, WallTimeMs::new(1_000), expiry),
        DeadlineStatus::WallExpired
    );
    assert_eq!(
        deadline.status(
            start + Duration::from_millis(50),
            WallTimeMs::new(999),
            expiry
        ),
        DeadlineStatus::WaitExpired
    );
    assert_eq!(
        deadline.status(
            start + Duration::from_millis(50),
            WallTimeMs::new(1_000),
            expiry
        ),
        DeadlineStatus::BothExpired
    );
    assert_eq!(
        deadline.remaining(start + Duration::from_millis(60)),
        Duration::ZERO
    );
    // A late completed close is accounted, but cannot become an on-time proof.
    assert_ne!(
        deadline.status(
            start + Duration::from_millis(60),
            WallTimeMs::new(999),
            expiry
        ),
        DeadlineStatus::OnTime
    );
}

#[tokio::test]
async fn reader_retains_partial_frame_and_rejects_partial_eof_and_overflow() {
    let (mut writer, reader) = duplex(16);
    let mut frames = BoundedFrameReader::new(reader, 5).unwrap();
    writer.write_all(b"abc").await.unwrap();
    writer.write_all(b"de\n").await.unwrap();
    assert_eq!(frames.read_frame().await.unwrap(), Some(b"abcde".to_vec()));
    writer.write_all(b"xy").await.unwrap();
    drop(writer);
    assert!(matches!(
        frames.read_frame().await,
        Err(FrameError::PartialEof)
    ));

    let (mut writer, reader) = duplex(16);
    let mut frames = BoundedFrameReader::new(reader, 3).unwrap();
    writer.write_all(b"abcd\n").await.unwrap();
    assert!(matches!(
        frames.read_frame().await,
        Err(FrameError::TooLarge)
    ));
}

#[tokio::test]
async fn reader_keeps_partial_bytes_when_a_waiter_is_cancelled() {
    let (mut writer, reader) = duplex(16);
    let mut frames = BoundedFrameReader::new(reader, 4).unwrap();
    writer.write_all(b"ab").await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), frames.read_frame())
            .await
            .is_err()
    );
    writer.write_all(b"cd\n").await.unwrap();
    assert_eq!(frames.read_frame().await.unwrap(), Some(b"abcd".to_vec()));
}

#[tokio::test]
async fn cancelled_partial_outbound_write_is_latched_uncertain() {
    let (_peer, writer) = duplex(1);
    let mut frames = BoundedFrameWriter::new(writer, 16).unwrap();
    let timed = tokio::time::timeout(
        Duration::from_millis(10),
        frames.write_frame(b"large-frame"),
    )
    .await;
    assert!(timed.is_err());
    assert!(frames.outbound_uncertain());
    assert!(matches!(
        frames.write_frame(b"retry").await,
        Err(FrameError::UncertainWrite)
    ));
}

#[tokio::test]
async fn diagnostics_do_not_change_supported_frame_eligibility_and_are_bounded() {
    for enabled in [false, true] {
        let diagnostics = FrameDiagnostics::new(enabled, 1);
        let (mut writer, reader) = duplex(32);
        let mut frames = BoundedFrameReader::new(reader, 4)
            .unwrap()
            .with_diagnostics(diagnostics.clone());
        writer.write_all(b"abcd\nabcde\n").await.unwrap();
        assert_eq!(frames.read_frame().await.unwrap(), Some(b"abcd".to_vec()));
        assert!(matches!(
            frames.read_frame().await,
            Err(FrameError::TooLarge)
        ));
        assert_eq!(
            diagnostics.snapshot(),
            if enabled {
                vec![FrameDiagnosticCode::TooLarge]
            } else {
                vec![]
            }
        );

        let (peer, writer) = duplex(32);
        let mut framed_writer = BoundedFrameWriter::new(writer, 4)
            .unwrap()
            .with_diagnostics(diagnostics.clone());
        framed_writer.write_frame(b"abcd").await.unwrap();
        assert!(matches!(
            framed_writer.write_frame(b"a\nb").await,
            Err(FrameError::Delimiter)
        ));
        assert!(matches!(
            framed_writer.write_frame(b"abcde").await,
            Err(FrameError::TooLarge)
        ));
        assert_eq!(
            diagnostics.snapshot(),
            if enabled {
                vec![FrameDiagnosticCode::TooLarge]
            } else {
                vec![]
            }
        );
        drop(peer);
    }
}
