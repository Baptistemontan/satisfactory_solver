use std::sync::Arc;

use crate::{
    BASE_URL,
    buildings::{Building, Buildings},
    item::{Item, Items},
    recipes::{Recipe, Recipes},
};
use graph::{Edge, Graph as SolvedGraph};
use leptos::{
    ev::{MouseEvent, WheelEvent},
    prelude::*,
};
use solver::recipe::{BuildingId, ItemId, RecipeId};
use web_sys::wasm_bindgen::JsCast;

mod sort_nodes;

const NODE_WIDTH: i32 = 100;
const NODE_IMAGE_SIZE: i32 = (NODE_WIDTH * 3) / 6;
const MIN_NODE_HEIGHT: i32 = 100;
const IO_CIRCLE_RADIUS: i32 = 16;
const IO_IMAGE_SIZE: i32 = (IO_CIRCLE_RADIUS * 3) / 2;
const POSITION_INCREMENT: f64 = 25.0;
const TEXT_HEIGHT: i32 = 10;

fn get_recipe(rid: RecipeId) -> Arc<Recipe> {
    let recipes = expect_context::<Recipes>();
    let Some(recipe) = recipes.get(rid) else {
        todo!("recipe {:?} not found", rid);
    };
    recipe
}

fn get_item(iid: ItemId) -> Arc<Item> {
    let items = expect_context::<Items>();
    let Some(item) = items.items.get(&iid) else {
        todo!("item {:?} not found", iid);
    };
    item.clone()
}

fn get_building(bid: BuildingId) -> Arc<Building> {
    let buildings = expect_context::<Buildings>();
    let Some(building) = buildings.buildings.get(&bid) else {
        todo!("building {:?} not found", bid);
    };
    building.clone()
}

#[derive(Clone, Copy, Debug)]
struct NodeData(graph::Node);

fn format_icon_href(icon: &str) -> String {
    format!("{}assets/items/{}_256.png", BASE_URL, icon)
}

impl NodeData {
    pub fn to_key(self) -> (usize, usize) {
        match self.0 {
            graph::Node::Recipe { rid, amount: _ } => (rid.0, 0),
            graph::Node::Input { iid, amount: _ } => (iid.0, 1),
            graph::Node::Output { iid, amount: _ } => (iid.0, 2),
            graph::Node::Excess { iid, amount: _ } => (iid.0, 3),
        }
    }

    pub fn inputs(self) -> Arc<[(ItemId, f64)]> {
        match self.0 {
            graph::Node::Recipe { rid, amount } => {
                let recipe = get_recipe(rid);
                let coef = (60.0 / recipe.time()) * amount;
                recipe
                    .inputs()
                    .iter()
                    .map(|(iid, qty)| (*iid, coef * qty.0))
                    .collect()
            }
            graph::Node::Output { iid, amount } | graph::Node::Excess { iid, amount } => {
                Arc::from([(iid, amount)])
            }
            graph::Node::Input { .. } => Arc::default(),
        }
    }

    pub fn outputs(self) -> Arc<[(ItemId, f64)]> {
        match self.0 {
            graph::Node::Recipe { rid, amount } => {
                let recipe = get_recipe(rid);
                let coef = (60.0 / recipe.time()) * amount;
                recipe
                    .outputs()
                    .iter()
                    .map(|(iid, qty)| (*iid, coef * qty.0))
                    .collect()
            }
            graph::Node::Input { iid, amount } => Arc::from([(iid, amount)]),
            graph::Node::Output { .. } | graph::Node::Excess { .. } => Arc::default(),
        }
    }

    pub fn amount(self) -> f64 {
        match self.0 {
            graph::Node::Recipe { amount, .. } => amount,
            graph::Node::Input { amount, .. } => amount,
            graph::Node::Output { amount, .. } => amount,
            graph::Node::Excess { amount, .. } => amount,
        }
    }

    pub fn icon_href(self) -> String {
        let icon = match self.0 {
            graph::Node::Recipe { rid, .. } => {
                let recipe = get_recipe(rid);
                let building = get_building(recipe.inner.building);
                building.icon.clone()
            }
            graph::Node::Excess { iid, .. }
            | graph::Node::Input { iid, .. }
            | graph::Node::Output { iid, .. } => {
                let item = get_item(iid);
                item.icon.clone()
            }
        };

        format_icon_href(&icon)
    }
}

#[derive(Clone, Copy, Default)]
struct Position {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    data: NodeData,
    pos: RwSignal<Position>,
    size: i32,
}

struct NodeIter {
    current: usize,
    nodes: Arc<[Node]>,
}

impl Iterator for NodeIter {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.nodes.get(self.current)?;
        self.current += 1;
        Some(*next)
    }
}

impl NodeIter {
    pub fn new(nodes: Arc<[Node]>) -> Self {
        Self { current: 0, nodes }
    }
}

fn sort_nodes(graph: &SolvedGraph) -> Arc<[Node]> {
    let (levels, max_level) = sort_nodes::sort_nodes(graph);

    let mut level_y = vec![100.0; max_level + 1];

    let mut nodes = Vec::new();

    for (node_id, node) in graph.nodes().iter().enumerate() {
        let level = levels.get(&node_id).unwrap();
        let y = &mut level_y[*level];
        let node_y = *y;
        let node_x = 100.0 + ((NODE_WIDTH as f64) + POSITION_INCREMENT * 5.0) * (*level as f64);
        let size = match node {
            graph::Node::Recipe { rid, .. } => {
                let recipe = get_recipe(*rid);
                recipe.inputs().len().max(recipe.outputs().len()) as i32
            }
            _ => 1,
        };
        let height = compute_node_height(size) as f64;
        *y += height + 50.0;
        nodes.push(Node {
            data: NodeData(*node),
            pos: RwSignal::new(Position {
                x: node_x,
                y: node_y,
            }),
            size,
        });
    }

    Arc::from(nodes)
}

fn snap_to_increment(x: f64) -> f64 {
    (x / POSITION_INCREMENT).round() * POSITION_INCREMENT
}

pub struct VisualGraph {
    pub nodes: Arc<[Node]>,
    pub edges: Arc<[Edge]>,
}

impl VisualGraph {
    pub fn from_solved_graph(graph: &SolvedGraph) -> Self {
        let nodes = sort_nodes(graph);
        let edges = Arc::from(graph.edges());
        Self { nodes, edges }
    }
}

#[component]
pub fn Graph(graph: VisualGraph) -> impl IntoView {
    let nodes = graph.nodes.clone();
    let edges = graph
        .edges
        .iter()
        .map(|edge| render_edge(*edge, nodes.clone()))
        .collect::<Vec<_>>();

    let scale = RwSignal::new(1.0f64);
    let tx = RwSignal::new(0.0f64);
    let ty = RwSignal::new(0.0f64);

    // For drag tracking — not reactive, just mutable state
    let dragging = RwSignal::new(false);
    let drag_start_x = RwSignal::new(0.0f64);
    let drag_start_y = RwSignal::new(0.0f64);

    let node_drag = NodeDrag {
        dragging: RwSignal::new(false),
        drag_start: RwSignal::new(Position::default()),
        node_position: RwSignal::new(RwSignal::new(Position::default())),
    };

    let transform = move || {
        format!(
            "translate({}, {}) scale({})",
            tx.get(),
            ty.get(),
            scale.get()
        )
    };

    let svg_space_offset = move |e: MouseEvent| -> (f64, f64) {
        let svg_x = (e.client_x() as f64 - tx.get()) / scale.get();
        let svg_y = (e.client_y() as f64 - ty.get()) / scale.get();
        (svg_x, svg_y)
    };

    let on_wheel = move |e: WheelEvent| {
        e.prevent_default();

        let svg = e
            .current_target()
            .unwrap()
            .dyn_into::<web_sys::Element>()
            .unwrap();
        let rect = svg.get_bounding_client_rect();

        let mx = e.client_x() as f64 - rect.left();
        let my = e.client_y() as f64 - rect.top();

        let delta = if e.delta_y() > 0.0 { 0.9 } else { 1.1 };

        tx.set(mx - (mx - tx.get()) * delta);
        ty.set(my - (my - ty.get()) * delta);
        scale.update(|s| *s *= delta);
    };

    let on_mousedown = move |e: MouseEvent| {
        if e.button() != 1 {
            return;
        }
        e.prevent_default(); // prevents autoscroll mode that browsers trigger on middle click
        dragging.set(true);
        drag_start_x.set(e.client_x() as f64 - tx.get());
        drag_start_y.set(e.client_y() as f64 - ty.get());
    };

    let on_mousemove = move |e: MouseEvent| {
        if dragging.get() {
            tx.set(e.client_x() as f64 - drag_start_x.get());
            ty.set(e.client_y() as f64 - drag_start_y.get());
        }
        if node_drag.dragging.get() {
            let (sx, sy) = svg_space_offset(e);
            let node_drag_start = node_drag.drag_start.get();
            let x = snap_to_increment(sx - node_drag_start.x);
            let y = snap_to_increment(sy - node_drag_start.y);
            let coord = Position { x, y };
            node_drag.node_position.get().set(coord);
        }
    };

    let on_mouseup = move |e: MouseEvent| {
        if e.button() == 1 {
            dragging.set(false)
        } else if e.button() == 0 {
            node_drag.dragging.set(false);
        }
    };

    view! {
        <svg
            class="graph-root"
            on:wheel=on_wheel
            on:mousedown=on_mousedown
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:mouseleave=on_mouseup
        >
            <defs>
                <marker
                    id="arrow"
                    viewBox="0 0 11 10"
                    refX="0"
                    refY="5"
                    markerWidth="10.5"
                    markerHeight="10"
                    orient="auto-start-reverse"
                    markerUnits="userSpaceOnUse"
                    class="svelte-2zis01"
                >
                    <path
                        d="M 0 0 l 11 5 l -11 5 z"
                        fill="context-stroke"
                        class="svelte-2zis01"
                    />
                </marker>
            </defs>
            <g transform=transform>
                <For
                    each = move || NodeIter::new(nodes.clone())
                    key = |node| node.data.to_key()
                    children = move |node| render_node(node, node_drag, svg_space_offset)
                />
                {edges}
            </g>
        </svg>
    }
}

#[derive(Clone, Copy)]
struct NodeDrag {
    dragging: RwSignal<bool>,
    drag_start: RwSignal<Position>,
    node_position: RwSignal<RwSignal<Position>>,
}

fn render_node<F>(node: Node, node_drag: NodeDrag, svg_space_offset: F) -> impl IntoView
where
    F: Fn(MouseEvent) -> (f64, f64) + 'static,
{
    let Node { data, pos, size } = node;
    // let dragging = RwSignal::new(false);
    // let drag_start_x = RwSignal::new(0.0f64);
    // let drag_start_y = RwSignal::new(0.0f64);

    let on_mousedown = move |e: MouseEvent| {
        if e.button() != 0 {
            return;
        } // left click only for nodes
        e.stop_propagation(); // prevent canvas from starting its own drag
        node_drag.node_position.set(pos);
        let pos = pos.get();

        let (sx, sy) = svg_space_offset(e);

        let start_x = sx - pos.x;
        let start_y = sy - pos.y;

        node_drag.dragging.set(true);
        node_drag.drag_start.set(Position {
            x: start_x,
            y: start_y,
        });
    };
    let transform = move || pos_to_transform(pos.get());

    let inner = render_node_inner(data, size);

    view! {
        <g
            class="graph-node"
            transform=transform
            on:mousedown=on_mousedown
        >
            {inner}
        </g>
    }
}

fn pos_to_transform(position: Position) -> String {
    format!("translate({}, {})", position.x, position.y)
}

fn compute_node_height(size: i32) -> i32 {
    ((MIN_NODE_HEIGHT / 2) * (size + 1)).max(MIN_NODE_HEIGHT)
}

fn render_node_inner(data: NodeData, size: i32) -> impl IntoView {
    let inputs = data.inputs();
    let outputs = data.outputs();
    let input_count = inputs.len();
    let output_count = outputs.len();
    let height = compute_node_height(size);

    let inputs = inputs
        .iter()
        .enumerate()
        .map(|(idx, (iid, qty))| render_io(*iid, *qty, idx, input_count, size, true))
        .collect::<Vec<_>>();

    let outputs = outputs
        .iter()
        .enumerate()
        .map(|(idx, (iid, qty))| render_io(*iid, *qty, idx, output_count, size, false))
        .collect::<Vec<_>>();

    let icon_href = data.icon_href();
    let image_y = (height - NODE_IMAGE_SIZE) / 2;
    let image_x = (NODE_WIDTH - NODE_IMAGE_SIZE) / 2;
    let amount = format_amount(data.amount());
    let text_y = image_y + NODE_IMAGE_SIZE;
    let text_x = image_x;

    view! {
        <g class="graph-node-inner">
            <rect
                y=0
                x=0
                height=height
                width=NODE_WIDTH
            />
            <image href=icon_href width=NODE_IMAGE_SIZE height=NODE_IMAGE_SIZE x=image_x y=image_y />
            <foreignObject x=text_x y=text_y height=TEXT_HEIGHT width=NODE_IMAGE_SIZE>
                <div class="amount recipe-amount">
                    <div>{amount}</div>
                </div>
            </foreignObject>
            {inputs}
            {outputs}
        </g>
    }
}

fn compute_io_y_offset(idx: usize, io_count: usize, size: i32) -> i32 {
    let parent_height = compute_node_height(size);
    let increment = parent_height / (io_count + 1) as i32;
    increment * (1 + idx as i32)
}

fn format_amount(amount: f64) -> String {
    let amount = format!("{:.3}", amount);
    let trimmed = amount.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        String::from("0")
    } else {
        trimmed.to_string()
    }
}

fn render_io(
    iid: ItemId,
    amount: f64,
    idx: usize,
    io_count: usize,
    size: i32,
    input: bool,
) -> impl IntoView {
    let item = get_item(iid);
    let icon_href = format_icon_href(&item.icon);
    let y = compute_io_y_offset(idx, io_count, size);
    let x = if input { 0 } else { NODE_WIDTH };
    let transform = format!("translate({}, {})", x, y);
    let amount = format_amount(amount);
    let text_x = -IO_CIRCLE_RADIUS;
    let text_y = (IO_CIRCLE_RADIUS * 3) / 4;

    view! {
        <g transform=transform class="node-io">
            <circle r=IO_CIRCLE_RADIUS/>
            <image href=icon_href width=IO_IMAGE_SIZE height=IO_IMAGE_SIZE y={IO_IMAGE_SIZE / -2} x={IO_IMAGE_SIZE / -2} />
            <foreignObject x=text_x y=text_y width={IO_CIRCLE_RADIUS * 2} height=TEXT_HEIGHT>
                <div class="amount io-amount">
                    <div>{amount}</div>
                </div>
            </foreignObject>
        </g>
    }
}

fn edge_path(x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    // Control points for the S-curve — pull horizontally toward the midpoint
    let dx = (x2 - x1).abs().max(100.0); // minimum curve tension
    let cx1 = x1 + dx * 0.5;
    let cy1 = y1;
    let cx2 = x2 - dx * 0.5;
    let cy2 = y2;

    format!("M {x1} {y1} C {cx1} {cy1}, {cx2} {cy2}, {x2} {y2}")
}

fn compute_offset(node: Node, iid: ItemId, input: bool) -> (i32, i32) {
    let inputs = node.data.inputs();
    let outputs = node.data.outputs();

    let io = if input { &outputs } else { &inputs };

    let Some(pos) = io.iter().position(|(a, _)| *a == iid) else {
        todo!("item {:?} not found in node {:?} outputs", iid, node);
    };
    let y = compute_io_y_offset(pos, io.len(), node.size);
    let x = if input {
        NODE_WIDTH + IO_CIRCLE_RADIUS
    } else {
        -IO_CIRCLE_RADIUS - 12
    };

    (x, y)
}

fn render_edge(edge: Edge, nodes: Arc<[Node]>) -> impl IntoView {
    let from = nodes[edge.from];
    let to = nodes[edge.to];

    let input_offset = compute_offset(from, edge.iid, true);
    let out_offset = compute_offset(to, edge.iid, false);

    let path = move || {
        let from_pos = from.pos.get();
        let to_pos = to.pos.get();
        edge_path(
            from_pos.x + input_offset.0 as f64,
            from_pos.y + input_offset.1 as f64,
            to_pos.x + out_offset.0 as f64,
            to_pos.y + out_offset.1 as f64,
        )
    };

    view! {
        <path
            class="graph-edge"
            d=path
            marker-end="url(#arrow)"
        />
    }
}
