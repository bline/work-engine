use research_register::anchor::line_hunks;

fn repeated(width: usize, count: usize, framed: bool) -> Vec<u8> {
    let mut line = vec![b'L'; width - 1];
    line.push(b'\n');
    let mut data = if framed { b"h\n".to_vec() } else { Vec::new() };
    data.extend(line.repeat(count));
    if framed {
        data.extend_from_slice(b"m\n");
        data.extend(line.repeat(14));
    }
    data
}

#[test]
fn all_ten_external_byte_fixtures() {
    // These compact constructions reproduce the raw feedback inputs exactly.
    let cases = [
        (30, 29, false, (10, 1, 9, 0)),
        (30, 29, true, (5, 1, 4, 0)),
        (29, 30, false, (9, 0, 10, 1)),
        (29, 30, true, (4, 0, 5, 1)),
        (25, 21, false, (2, 4, 1, 0)),
        (25, 21, true, (7, 4, 6, 0)),
        (11, 12, false, (1, 0, 2, 1)),
        (11, 12, true, (6, 0, 7, 1)),
        (40, 38, false, (9, 2, 8, 0)),
        (40, 38, true, (3, 2, 2, 0)),
    ];
    for (old, new, framed, expected) in cases {
        assert_eq!(
            line_hunks(&repeated(100, old, framed), &repeated(100, new, framed)),
            vec![expected]
        );
    }
}

#[test]
fn cutoff_at_exact_line_start_retains_that_line() {
    // Additional independently queried cases, not supplied feedback.
    assert_eq!(
        line_hunks(&repeated(512, 20, false), &repeated(512, 49, false)),
        vec![(1, 0, 2, 29)]
    );
    assert_eq!(
        line_hunks(&repeated(512, 54, false), &repeated(512, 45, false)),
        vec![(3, 9, 2, 0)]
    );
}

#[test]
fn small_repetitions_still_slide_to_end() {
    assert_eq!(
        line_hunks(&repeated(100, 10, false), &repeated(100, 9, false)),
        vec![(10, 1, 9, 0)]
    );
    assert_eq!(
        line_hunks(&repeated(100, 9, false), &repeated(100, 10, false)),
        vec![(9, 0, 10, 1)]
    );
}
