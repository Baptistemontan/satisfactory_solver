use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    mem,
};

use solver::graph::{Edge, Graph, Node as GNode};

fn cycle_removal(
    nodes: &[GNode],
    reverse_search: &mut BTreeMap<usize, BTreeSet<usize>>,
) -> BTreeMap<usize, BTreeSet<usize>> {
    // check for cycle and remove them
    // DFS from an outpout
    // if a node loopback to a parent node, remove that edge
    let outputs = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Output { .. }))
        .map(|(n, _)| n);

    let mut queue = VecDeque::new();
    let mut next_queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    let mut visit_queue = Vec::new();
    let mut cycles = BTreeMap::<usize, BTreeSet<usize>>::new();
    for out_node in outputs {
        next_queue.push_back(out_node);
        while !next_queue.is_empty() {
            for node in visit_queue.drain(..) {
                visited.insert(node);
            }
            mem::swap(&mut queue, &mut next_queue);
            while let Some(node) = queue.pop_front() {
                let parents = reverse_search.entry(node).or_default();
                let parent_queue = mem::take(parents);
                visit_queue.push(node);
                for parent in parent_queue {
                    if parent == node {
                        // remove self loopback
                        continue;
                    } else if !visited.contains(&parent) {
                        parents.insert(parent);
                        next_queue.push_back(parent);
                    } else {
                        cycles.entry(node).or_default().insert(parent);
                    }
                }
            }
        }
        visited.clear();
    }

    cycles
}

/// Analyze the graph and gives a "level" to each node to determine a flow direction
/// inputs are at levels 0, recipes after that, and last outputs
/// there might be some recipes with higher level than an output if multiple outputs are present
pub fn sort_nodes(graph: &Graph) -> (BTreeMap<usize, usize>, usize) {
    // precompute the parent of a node
    let mut reverse_search = BTreeMap::<usize, BTreeSet<usize>>::new();
    for edge in graph.edges() {
        let parent = reverse_search.entry(edge.to).or_default();
        parent.insert(edge.from);
    }
    // remove edges that cause cycles
    let cycles = cycle_removal(graph.nodes(), &mut reverse_search);

    let outputs = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Output { .. }))
        .map(|(n, _)| n);

    // start with all inputs at level 0
    let mut levels = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Input { .. }))
        .map(|(n, _)| (n, 0))
        .collect::<BTreeMap<usize, usize>>();

    let mut maximum_level = 0;

    let mut queue = VecDeque::new();
    for output_node in outputs {
        queue.push_back(output_node);
        while let Some(node) = queue.pop_front() {
            if levels.contains_key(&node) {
                continue;
            }
            let from = reverse_search.get(&node).unwrap();
            let mut max_level = 0;
            for from_node in from {
                if let Some(level) = levels.get(from_node) {
                    max_level = max_level.max(*level);
                } else {
                    max_level = usize::MAX;
                    queue.push_back(*from_node);
                }
            }

            if max_level == usize::MAX {
                queue.push_back(node);
            } else {
                maximum_level = maximum_level.max(max_level + 1);
                levels.insert(node, max_level + 1);
            }
        }
    }

    // Give level to excess

    let mut excess_nodes = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Excess { .. }))
        .map(|(n, _)| n)
        .peekable();

    if excess_nodes.peek().is_some() {
        maximum_level += 1;
    }

    for excess in excess_nodes {
        levels.insert(excess, maximum_level);
    }

    // set all outputs to max_level

    let output_nodes = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Output { .. }))
        .map(|(n, _)| n);

    for out in output_nodes {
        levels.insert(out, maximum_level);
    }

    // rearrange cycles

    update_cycles_level(graph.edges(), &cycles, &mut levels);

    // Remove empty levels

    let mut level_count = vec![0; maximum_level + 1];
    for level in levels.values() {
        level_count[*level] += 1;
    }
    let empty_levels = level_count
        .into_iter()
        .enumerate()
        .filter(|(_, c)| *c == 0)
        .map(|(n, _)| n)
        .collect::<Vec<_>>();
    if !empty_levels.is_empty() {
        for level in levels.values_mut() {
            let diff = empty_levels.iter().take_while(|n| **n < *level).count();
            *level -= diff;
        }
    }
    if maximum_level >= empty_levels.len() {
        maximum_level -= empty_levels.len();
    }

    (levels, maximum_level)
}

// This is optional but produce better levels, it looks if a node that produced a cycle can be pushed up
// it looks if its output and outputs that caused the cycles are the level just above
fn update_cycles_level(
    edges: &[Edge],
    cycles: &BTreeMap<usize, BTreeSet<usize>>,
    levels: &mut BTreeMap<usize, usize>,
) {
    if cycles.is_empty() {
        return;
    }
    let mut cycle_edges = BTreeMap::<usize, BTreeSet<usize>>::new();
    for edge in edges {
        // skip if not a cycle
        if cycles.contains_key(&edge.from) {
            let childs = cycle_edges.entry(edge.from).or_default();
            childs.insert(edge.to);
        }
    }
    'outer: for (cycle_node, cycle_with) in cycles {
        let current_level = levels[cycle_node];
        let diff = cycle_edges.get(cycle_node).unwrap().difference(cycle_with);
        for parent in diff {
            let Some(parent_level) = levels.get(parent) else {
                continue;
            };
            if *parent_level <= current_level + 1 {
                continue 'outer;
            }
        }
        levels.insert(*cycle_node, current_level + 1);
    }
}
