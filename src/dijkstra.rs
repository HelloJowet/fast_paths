/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

use std::collections::BinaryHeap;

use crate::constants::Weight;
use crate::constants::{NodeId, INVALID_NODE, WEIGHT_MAX};
use crate::heap_item::HeapItem;
use crate::preparation_graph::PreparationGraph;
use crate::shortest_path::ShortestPath;
use crate::valid_flags::ValidFlags;

pub struct Dijkstra {
    num_nodes: usize,
    data: Vec<Data>,
    valid_flags: ValidFlags,
    heap: BinaryHeap<HeapItem>,
}

/// Dijkstra's algorithm using pre-allocated memory for the shortest path tree. Currently only used
/// to test the correctness of the path_calculator implementation. Providing a flexible Dijkstra
/// implementation that works for arbitrary weight functions and that runs on the fast_graph
/// datastructure might be useful, but this was not the intention here.
impl Dijkstra {
    pub fn new(num_nodes: usize) -> Self {
        let heap = BinaryHeap::new();
        Dijkstra {
            num_nodes,
            data: (0..num_nodes).map(|_i| Data::new()).collect(),
            valid_flags: ValidFlags::new(num_nodes),
            heap,
        }
    }

    #[allow(dead_code)]
    pub fn calc_path(
        &mut self,
        graph: &PreparationGraph,
        start: NodeId,
        end: NodeId,
    ) -> Option<ShortestPath> {
        self.init(start);
        self.do_calc_path(graph, start, end);
        self.build_path(start, end)
    }

    fn init(&mut self, start: NodeId) {
        self.heap.clear();
        self.valid_flags.invalidate_all();
        self.update_node(start, 0, INVALID_NODE);
        self.heap.push(HeapItem::new(0, start));
    }

    fn do_calc_path(&mut self, graph: &PreparationGraph, start: NodeId, end: NodeId) {
        assert_eq!(
            graph.get_num_nodes(),
            self.num_nodes,
            "given graph has invalid node count"
        );
        if start == end {
            return;
        }
        if self.is_settled(end) {
            return;
        }
        while !self.heap.is_empty() {
            let curr = self.heap.pop().unwrap();
            if self.is_settled(curr.node_id) {
                // todo: since we are not using a special decrease key operation yet we need to
                // filter out duplicate heap items here
                continue;
            }
            for i in 0..graph.out_edges[curr.node_id].len() {
                let adj = graph.out_edges[curr.node_id][i].adj_node;
                let edge_weight = graph.out_edges[curr.node_id][i].weight;
                let weight = curr.weight + edge_weight;
                if weight < self.get_weight(adj) {
                    self.update_node(adj, weight, curr.node_id);
                    self.heap.push(HeapItem::new(weight, adj));
                }
            }
            self.data[curr.node_id].settled = true;
            if curr.node_id == end {
                break;
            }
        }
    }

    fn build_path(&mut self, start: NodeId, end: NodeId) -> Option<ShortestPath> {
        if start == end {
            return Some(ShortestPath::singular(start));
        }
        if !self.valid_flags.is_valid(end) || !self.data[end].settled {
            return None;
        }
        let mut path = Vec::new();
        let mut node = end;
        while self.data[node].parent != INVALID_NODE {
            path.push(node);
            node = self.data[node].parent;
        }
        path.push(start);
        path = path.iter().rev().cloned().collect();
        Some(ShortestPath::new(start, end, self.data[end].weight, path))
    }

    fn update_node(&mut self, node: NodeId, weight: Weight, parent: NodeId) {
        self.valid_flags.set_valid(node);
        self.data[node].settled = false;
        self.data[node].weight = weight;
        self.data[node].parent = parent;
    }

    fn is_settled(&self, node: NodeId) -> bool {
        self.valid_flags.is_valid(node) && self.data[node].settled
    }

    fn get_weight(&self, node: NodeId) -> Weight {
        if self.valid_flags.is_valid(node) {
            self.data[node].weight
        } else {
            WEIGHT_MAX
        }
    }
}

struct Data {
    settled: bool,
    weight: Weight,
    parent: NodeId,
}

impl Data {
    fn new() -> Self {
        // todo: initializing with these values is not strictly necessary
        Data {
            settled: false,
            weight: WEIGHT_MAX,
            parent: INVALID_NODE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_path() {
        //      7 -> 8 -> 9
        //      |         |
        // 0 -> 5 -> 6 -  |
        // |         |  \ |
        // 1 -> 2 -> 3 -> 4
        let mut g = PreparationGraph::new(10);
        g.add_edge(0, 1, 1, 1.0);
        g.add_edge(1, 2, 1, 1.0);
        g.add_edge(2, 3, 1, 1.0);
        g.add_edge(3, 4, 20, 20.0);
        g.add_edge(0, 5, 5, 5.0);
        g.add_edge(5, 6, 1, 1.0);
        g.add_edge(6, 4, 20, 20.0);
        g.add_edge(6, 3, 20, 20.0);
        g.add_edge(5, 7, 5, 5.0);
        g.add_edge(7, 8, 1, 1.0);
        g.add_edge(8, 9, 1, 1.0);
        g.add_edge(9, 4, 1, 1.0);
        let mut d = Dijkstra::new(g.get_num_nodes());
        assert_no_path(&mut d, &g, 4, 0);
        assert_path(&mut d, &g, 4, 4, 0, vec![4]);
        assert_path(&mut d, &g, 6, 3, 20, vec![6, 3]);
        assert_path(&mut d, &g, 1, 4, 22, vec![1, 2, 3, 4]);
        assert_path(&mut d, &g, 0, 4, 13, vec![0, 5, 7, 8, 9, 4]);
    }

    #[test]
    fn go_around() {
        // 0 -> 1
        // |    |
        // 2 -> 3
        let mut g = PreparationGraph::new(4);
        g.add_edge(0, 1, 10, 10.0);
        g.add_edge(0, 2, 1, 1.0);
        g.add_edge(2, 3, 1, 1.0);
        g.add_edge(3, 1, 1, 1.0);
        let mut d = Dijkstra::new(g.get_num_nodes());
        assert_path(&mut d, &g, 0, 1, 3, vec![0, 2, 3, 1]);
    }

    #[test]
    fn more() {
        // 0 -> 1 -> 2
        //       \
        //         3 -> 4
        //        / \
        //   7 <-6   |-> 5
        //            \
        //             8 -> 9 -> 10
        let mut g = PreparationGraph::new(11);
        g.add_edge(0, 1, 1, 1.0);
        g.add_edge(1, 2, 1, 1.0);
        g.add_edge(1, 3, 1, 1.0);
        g.add_edge(3, 4, 1, 1.0);
        g.add_edge(3, 6, 1, 1.0);
        g.add_edge(6, 7, 1, 1.0);
        g.add_edge(3, 5, 1, 1.0);
        g.add_edge(3, 8, 1, 1.0);
        g.add_edge(8, 9, 1, 1.0);
        g.add_edge(9, 10, 1, 1.0);
        let mut d = Dijkstra::new(g.get_num_nodes());
        assert_path(&mut d, &g, 0, 1, 1, vec![0, 1]);
        assert_path(&mut d, &g, 0, 2, 2, vec![0, 1, 2]);
        assert_path(&mut d, &g, 0, 4, 3, vec![0, 1, 3, 4]);
        assert_path(&mut d, &g, 0, 3, 2, vec![0, 1, 3]);
        assert_path(&mut d, &g, 0, 7, 4, vec![0, 1, 3, 6, 7]);
        assert_path(&mut d, &g, 0, 5, 3, vec![0, 1, 3, 5]);
        assert_path(&mut d, &g, 0, 10, 5, vec![0, 1, 3, 8, 9, 10]);
        assert_path(&mut d, &g, 3, 10, 3, vec![3, 8, 9, 10]);
    }

    fn assert_no_path(
        dijkstra: &mut Dijkstra,
        graph: &PreparationGraph,
        source: NodeId,
        target: NodeId,
    ) {
        assert_eq!(dijkstra.calc_path(&graph, source, target), None);
    }

    fn assert_path(
        dijkstra: &mut Dijkstra,
        graph: &PreparationGraph,
        source: NodeId,
        target: NodeId,
        weight: Weight,
        nodes: Vec<NodeId>,
    ) {
        let dijkstra_path = dijkstra.calc_path(&graph, source, target);
        assert_eq!(
            dijkstra_path,
            Some(ShortestPath::new(source, target, weight, nodes.clone()))
        );
        // ShortestPath PartialEq does not consider nodes!
        assert_eq!(nodes, dijkstra_path.unwrap().get_nodes().clone());
    }
}
