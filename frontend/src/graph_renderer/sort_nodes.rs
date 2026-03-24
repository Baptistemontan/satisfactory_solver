use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    mem,
};

use graph::{Graph, Node as GNode};

fn cycle_removal(nodes: &[GNode], reverse_search: &mut BTreeMap<usize, Vec<usize>>) {
    let outputs = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Output { .. }))
        .map(|(n, _)| n);

    let mut queue = VecDeque::new();
    let mut next_queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    let mut visit_queue = Vec::new();
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
                    if !visited.contains(&parent) {
                        parents.push(parent);
                        next_queue.push_back(parent);
                    }
                }
            }
        }
        visited.clear();
    }
}

pub fn sort_nodes(graph: &Graph) -> (BTreeMap<usize, usize>, usize) {
    let mut reverse_search = BTreeMap::<usize, Vec<usize>>::new();
    for edge in graph.edges() {
        let parent = reverse_search.entry(edge.to).or_default();
        parent.push(edge.from);
    }

    cycle_removal(graph.nodes(), &mut reverse_search);

    let outputs = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Output { .. }))
        .map(|(n, _)| n);

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

    let excess_nodes = graph
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, GNode::Excess { .. }))
        .map(|(n, _)| n);

    for excess in excess_nodes {
        let from = reverse_search.get(&excess).unwrap();
        let mut max_level = 0;
        for from_node in from {
            let level = levels.get(from_node).unwrap();
            max_level = max_level.max(*level);
        }
        levels.insert(excess, max_level + 1);
    }

    (levels, maximum_level)
}
