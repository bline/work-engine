use manual_host_pilot_fixture::{LineSummary, summarize_lines};

#[test]
fn public_contract_examples() {
    for (s, bytes, scalars, lines) in [
        ("", 0, 0, 0),
        ("abc", 3, 3, 1),
        ("abc\n", 4, 4, 1),
        ("\n", 1, 1, 1),
        ("\n\n", 2, 2, 2),
        ("a\r\nb", 4, 4, 2),
        ("é\n猫", 6, 3, 2),
        ("a\rb", 3, 3, 1),
    ] {
        assert_eq!(
            summarize_lines(s),
            LineSummary {
                bytes,
                scalars,
                lines
            }
        );
    }
}
