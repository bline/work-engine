use research_register::anchor::line_hunks;
#[test]
fn byte_and_position_edges() {
    assert_eq!(line_hunks(b"", b""), vec![]);
    assert_eq!(line_hunks(b"", b"a\n"), vec![(0, 0, 1, 1)]);
    assert_eq!(line_hunks(b"a\n", b""), vec![(1, 1, 0, 0)]);
    assert_eq!(line_hunks(b"a", b"a\n"), vec![(1, 1, 1, 1)]);
    assert_eq!(line_hunks(b"\0\xff\r\n", b"\0\xfe\r\n"), vec![(1, 1, 1, 1)]);
    assert_eq!(line_hunks(b"a\nb\nc\n", b"a\nx\nc\n"), vec![(2, 1, 2, 1)]);
}
#[test]
fn merged_equivalent_runs() {
    assert_eq!(
        line_hunks(b"b\nb\na\n\na\nb\n", b"\n\n\na\n\nb"),
        vec![(1, 3, 1, 2), (6, 1, 5, 2)]
    );
    assert_eq!(
        line_hunks(b"c\nc\na\nc\n", b"a\nb\n\na\na\nc\na\nb\n\nb\n"),
        vec![(1, 2, 1, 4), (4, 0, 7, 4)]
    );
}
#[test]
fn large_equal_and_local_change() {
    let old = b"same\n".repeat(100_000);
    assert!(line_hunks(&old, &old).is_empty());
    let mut new = old.clone();
    new.extend_from_slice(b"extra\n");
    assert_eq!(line_hunks(&old, &new), vec![(100_000, 0, 100_001, 1)]);
}
