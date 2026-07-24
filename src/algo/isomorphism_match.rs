use super::SubgraphMode;

pub(super) fn edge_lists_compatible<P, T, E>(
    pattern: &[P],
    target: &[T],
    mode: SubgraphMode,
    edge_match: &E,
) -> bool
where
    E: Fn(P, T) -> bool,
    P: Copy,
    T: Copy,
{
    if pattern.len() > target.len()
        || (mode == SubgraphMode::Induced && pattern.len() != target.len())
    {
        return false;
    }
    match_edges(
        pattern,
        target,
        edge_match,
        0,
        &mut vec![false; target.len()],
    )
}

fn match_edges<P, T, E>(
    pattern: &[P],
    target: &[T],
    edge_match: &E,
    index: usize,
    used: &mut [bool],
) -> bool
where
    E: Fn(P, T) -> bool,
    P: Copy,
    T: Copy,
{
    if index == pattern.len() {
        return true;
    }
    for candidate in 0..target.len() {
        if !used[candidate] && edge_match(pattern[index], target[candidate]) {
            used[candidate] = true;
            if match_edges(pattern, target, edge_match, index + 1, used) {
                return true;
            }
            used[candidate] = false;
        }
    }
    false
}
