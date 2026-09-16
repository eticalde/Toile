use toile_engine::draft::block;

use super::super::super::state::Field;
use super::super::cite::variable_id;
use super::super::write::id_of;
use super::desk::Desk;

/// A row keeps room under its box for the line that says what it comes to, and
/// the next row starts below that line.
///
/// Placing the box rewinds the layout to the box's own bottom, so a row that
/// does not take its room back again hands that line's room to the row after
/// it, and the value ends up written across the next name.
#[test]
fn what_a_variable_comes_to_is_not_written_over_the_next_name() {
    let doc = block::trousers();
    let mut desk = Desk::tall(doc.clone());
    desk.frame(Vec::new());
    desk.frame(Vec::new());

    let rows: Vec<(f32, f32)> = doc
        .variables
        .iter()
        .map(|(key, variable)| {
            let name = variable.name.as_str();
            let chip = desk
                .ctx
                .read_response(variable_id(key))
                .unwrap_or_else(|| panic!("{name} is drawn"))
                .rect;
            let boxed = desk
                .ctx
                .read_response(id_of(&Field::Variable(key)))
                .unwrap_or_else(|| panic!("{name} has a box"))
                .rect;
            (chip.top(), boxed.bottom())
        })
        .collect();
    assert!(rows.len() > 2, "the block carries variables to stack up");

    for pair in rows.windows(2) {
        let (_, box_above) = pair[0];
        let (name_below, _) = pair[1];
        assert!(
            name_below >= box_above + 14.0,
            "a value under a box ending at {box_above} is written over the name at {name_below}"
        );
    }
}
