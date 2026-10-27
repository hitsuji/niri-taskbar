//! Translates a taskbar drag and drop into Niri actions.

use crate::niri::Window;

/// Which half of the target button the drop landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Before,
    After,
}

/// Where a window lives in the Niri layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub workspace_id: u64,
    /// The 1-based column index, or `None` for floating windows.
    pub column: Option<usize>,
}

impl Position {
    pub fn of(window: &Window) -> Option<Self> {
        Some(Self {
            workspace_id: window.workspace_id?,
            column: if window.is_floating {
                None
            } else {
                Some(window.layout.pos_in_scrolling_layout?.0)
            },
        })
    }
}

/// The action required to carry out a drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Move the source window's column to `index`, on `workspace_id` if it's changing workspace.
    Column {
        workspace_id: Option<u64>,
        index: usize,
    },
    /// Move a floating window to another workspace.
    Workspace(u64),
}

/// Plans the move required to drop `source` onto `target`.
///
/// `last_column` is the highest column index on the target workspace, which is used to append
/// the source column when it's dropped onto a floating window. Returns `None` if nothing needs
/// to happen.
pub fn plan(source: Position, target: Position, side: Side, last_column: usize) -> Option<Move> {
    let same_workspace = source.workspace_id == target.workspace_id;

    // Floating windows have no column order, so all they can do is change workspace.
    let Some(source_column) = source.column else {
        return (!same_workspace).then_some(Move::Workspace(target.workspace_id));
    };

    // Dropping onto a floating window appends to the end of that workspace's columns.
    let (target_column, side) = match target.column {
        Some(column) => (column, side),
        None => (last_column, Side::After),
    };

    if same_workspace && target_column == source_column {
        return None;
    }

    // MoveColumnToIndex specifies the final index, so account for the source column being
    // removed from in front of the target.
    let base = if same_workspace && source_column < target_column {
        target_column - 1
    } else {
        target_column
    };
    let index = match side {
        Side::Before => base,
        Side::After => base + 1,
    }
    .max(1);

    if same_workspace && index == source_column {
        return None;
    }

    Some(Move::Column {
        workspace_id: (!same_workspace).then_some(target.workspace_id),
        index,
    })
}

/// Plans the move required to drop the `source_id` window onto the `target_id` window, given the
/// current window snapshot.
pub fn plan_drop(windows: &[Window], source_id: u64, target_id: u64, side: Side) -> Option<Move> {
    if source_id == target_id {
        return None;
    }

    let find = |id| windows.iter().find(|window| window.id == id);
    let source = Position::of(find(source_id)?)?;
    let target = Position::of(find(target_id)?)?;

    let last_column = windows
        .iter()
        .filter(|window| window.workspace_id == Some(target.workspace_id) && !window.is_floating)
        .filter_map(|window| window.layout.pos_in_scrolling_layout)
        .map(|(column, _)| column)
        .max()
        .unwrap_or(0);

    plan(source, target, side, last_column)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiled(workspace_id: u64, column: usize) -> Position {
        Position {
            workspace_id,
            column: Some(column),
        }
    }

    fn floating(workspace_id: u64) -> Position {
        Position {
            workspace_id,
            column: None,
        }
    }

    fn column(workspace_id: Option<u64>, index: usize) -> Option<Move> {
        Some(Move::Column {
            workspace_id,
            index,
        })
    }

    #[test]
    fn same_workspace_moving_right() {
        // [1] 2 3 4: drop 1 before 3 → 2 1 3 4.
        assert_eq!(
            plan(tiled(1, 1), tiled(1, 3), Side::Before, 4),
            column(None, 2)
        );
        // Drop 1 after 3 → 2 3 1 4.
        assert_eq!(
            plan(tiled(1, 1), tiled(1, 3), Side::After, 4),
            column(None, 3)
        );
    }

    #[test]
    fn same_workspace_moving_left() {
        // 1 2 3 [4]: drop 4 before 2 → 1 4 2 3.
        assert_eq!(
            plan(tiled(1, 4), tiled(1, 2), Side::Before, 4),
            column(None, 2)
        );
        // Drop 4 after 1 → 1 4 2 3.
        assert_eq!(
            plan(tiled(1, 4), tiled(1, 1), Side::After, 4),
            column(None, 2)
        );
        // Drop 4 before 1 → 4 1 2 3.
        assert_eq!(
            plan(tiled(1, 4), tiled(1, 1), Side::Before, 4),
            column(None, 1)
        );
    }

    #[test]
    fn same_workspace_no_op() {
        // Same column (eg another tile in the same column).
        assert_eq!(plan(tiled(1, 2), tiled(1, 2), Side::Before, 4), None);
        // Drop 2 before 3, or after 1: already there.
        assert_eq!(plan(tiled(1, 2), tiled(1, 3), Side::Before, 4), None);
        assert_eq!(plan(tiled(1, 2), tiled(1, 1), Side::After, 4), None);
    }

    #[test]
    fn other_workspace() {
        assert_eq!(
            plan(tiled(1, 3), tiled(2, 1), Side::Before, 2),
            column(Some(2), 1)
        );
        assert_eq!(
            plan(tiled(1, 1), tiled(2, 2), Side::After, 2),
            column(Some(2), 3)
        );
    }

    #[test]
    fn onto_floating_appends() {
        assert_eq!(
            plan(tiled(1, 1), floating(2), Side::Before, 2),
            column(Some(2), 3)
        );
        assert_eq!(
            plan(tiled(1, 1), floating(1), Side::Before, 3),
            column(None, 3)
        );
        // Already the last column.
        assert_eq!(plan(tiled(1, 3), floating(1), Side::Before, 3), None);
        // A workspace with only floating windows.
        assert_eq!(
            plan(tiled(1, 1), floating(2), Side::Before, 0),
            column(Some(2), 1)
        );
    }

    #[test]
    fn floating_source() {
        assert_eq!(
            plan(floating(1), tiled(2, 1), Side::Before, 2),
            Some(Move::Workspace(2))
        );
        assert_eq!(plan(floating(1), tiled(1, 1), Side::Before, 2), None);
    }
}
