use super::*;

#[test]
fn band_of_finds_the_containing_range() {
    let bands = [
        BitBand::new("lo", 0, 4), // bits 0..=3
        BitBand::new("hi", 4, 4), // bits 4..=7
    ];
    assert_eq!(band_of(&bands, 0), Some(0));
    assert_eq!(band_of(&bands, 3), Some(0));
    assert_eq!(band_of(&bands, 4), Some(1));
    assert_eq!(band_of(&bands, 7), Some(1));
}

#[test]
fn band_of_is_none_outside_every_range() {
    let bands = [BitBand::new("mid", 2, 3)]; // bits 2..=4
    assert_eq!(band_of(&bands, 1), None);
    assert_eq!(band_of(&bands, 5), None);
    assert_eq!(band_of(&[], 0), None);
}

#[test]
fn band_of_takes_the_first_overlapping_range() {
    let bands = [BitBand::new("a", 0, 8), BitBand::new("b", 4, 4)];
    // Bit 5 is in both; the first wins.
    assert_eq!(band_of(&bands, 5), Some(0));
}

#[test]
fn high_low_reports_inclusive_bounds() {
    assert_eq!(BitBand::new("byte", 0, 8).high_low(), (7, 0));
    assert_eq!(BitBand::new("nibble", 4, 4).high_low(), (7, 4));
    // A one-bit field is [n:n].
    assert_eq!(BitBand::new("flag", 6, 1).high_low(), (6, 6));
    // A zero-length field degrades to [start:start] via saturating_sub.
    assert_eq!(BitBand::new("empty", 3, 0).high_low(), (3, 3));
}
