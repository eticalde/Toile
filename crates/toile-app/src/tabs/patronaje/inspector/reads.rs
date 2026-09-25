use toile_engine::draft::{Axis, Defect, Draft, EvalError, PieceKey, PointKey};

/// What one coordinate comes to, and whether that line is a fault.
///
/// Said of the axis alone. A point stops resolving at the first of its two
/// bindings that fails, so the other one is left without a number even when it
/// evaluates perfectly; taking the point's verdict for both would paint an
/// alert over a formula that is fine and send someone to fix the wrong field.
/// The axis that has no recorded fault is therefore evaluated here.
pub(super) fn coordinate(draft: &Draft, at: (PieceKey, PointKey), axis: Axis) -> (String, bool) {
    let (piece, point) = at;
    let k = match axis {
        Axis::X => 0,
        Axis::Y => 1,
    };
    if let Some(cm) = draft.resolved(point) {
        return (format!("= {:.1} cm", cm[k]), false);
    }
    if let Some(error) = broke(draft, piece, point, axis) {
        return (why(error), true);
    }
    let held = draft.doc().points.get(point);
    match held.map(|held| held.binding(axis).eval(draft.env())) {
        Some(Ok(cm)) => (format!("= {cm:.1} cm"), false),
        Some(Err(error)) => (why(&error), true),
        None => ("no resuelve".to_owned(), true),
    }
}

/// The fault the draft recorded against one coordinate, when it recorded one.
fn broke(draft: &Draft, piece: PieceKey, point: PointKey, axis: Axis) -> Option<&EvalError> {
    draft.defects(piece).iter().find_map(|defect| match defect {
        Defect::Binding {
            point: at,
            axis: which,
            error,
        } if *at == point && *which == axis => Some(error),
        _ => None,
    })
}

/// Why a coordinate does not resolve, said in the language of the panel.
fn why(error: &EvalError) -> String {
    match error {
        EvalError::UnknownName(name) => format!("nombre desconocido: {name}"),
        EvalError::DivideByZero => "división por cero".to_owned(),
        EvalError::FractionalPower => "el exponente no es entero".to_owned(),
        EvalError::NotFinite => "no da un número".to_owned(),
        EvalError::Cycle(names) => format!("variables circulares: {names}"),
    }
}
