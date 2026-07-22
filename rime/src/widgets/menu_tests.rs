//! Tests for the submenu-flyout anchoring math shared by `menu_bar` and
//! `context_menu`. The offset is the y-distance from the top of the parent
//! panel to the expanded submenu's row, so the flyout lines up beside it
//! without measuring at layout time.

use super::{submenu_flyout, Item, PANEL_PAD, ROW_H, SEP_H};

#[derive(Clone)]
enum Msg {
    Hover,
    Pick,
}

fn action(label: &str) -> Item<Msg> {
    Item::action(label, Msg::Pick)
}

fn submenu(expanded: bool) -> Item<Msg> {
    Item::submenu("More", vec![action("Nested")], expanded, Msg::Hover)
}

/// The offset returned for `items` (panics if there is no expanded submenu).
fn offset(items: &[Item<Msg>]) -> f32 {
    submenu_flyout(items).expect("an expanded submenu").1
}

#[test]
fn none_without_an_expanded_submenu() {
    assert!(submenu_flyout::<Msg>(&[]).is_none());
    assert!(submenu_flyout(&[action("A"), action("B")]).is_none());
    // A submenu that exists but is collapsed does not fly out.
    assert!(submenu_flyout(&[action("A"), submenu(false)]).is_none());
}

#[test]
fn offset_is_the_parent_padding_plus_preceding_row_heights() {
    // First row: just the panel padding.
    assert_eq!(offset(&[submenu(true)]), PANEL_PAD);
    // One action above it.
    assert_eq!(offset(&[action("A"), submenu(true)]), PANEL_PAD + ROW_H);
    // Action + separator above it (separators are shorter).
    assert_eq!(
        offset(&[action("A"), Item::separator(), submenu(true)]),
        PANEL_PAD + ROW_H + SEP_H,
    );
    // Several actions.
    assert_eq!(
        offset(&[action("A"), action("B"), action("C"), submenu(true)]),
        PANEL_PAD + 3.0 * ROW_H,
    );
}

#[test]
fn only_the_first_expanded_submenu_flies_out() {
    // Two expanded submenus: the first one wins (one flyout level).
    let items = vec![submenu(true), action("A"), submenu(true)];
    assert_eq!(offset(&items), PANEL_PAD);
}
