use crate::ast::{Decl, DeclKind};
use crate::lexer::Span;
use crate::semantic::SemanticAnalyzer;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::symbols::Symbol;
use crate::semantic::types::TypeId;
use std::collections::{HashMap, VecDeque};

enum State {
    Unvisited,
    Visiting,
    Visited,
}

impl SemanticAnalyzer {
    pub fn check_circular_protocols_extension(&mut self, decls: &[Decl]) -> bool {
        let mut graph: HashMap<TypeId, Vec<TypeId>> = HashMap::new();
        let mut state: HashMap<TypeId, State> = HashMap::new();
        let mut id2span: HashMap<TypeId, Span> = HashMap::new();
        for decl in decls {
            if let DeclKind::Protocol { name, parents, .. } = &decl.node {
                let protocol_id = self.ctx.types.resolve(name).unwrap();
                let mut neighbors: Vec<TypeId> = Vec::new();
                if let Some(parents) = parents {
                    for parent in parents {
                        let parent_id = self.ctx.types.resolve(parent).unwrap();
                        neighbors.push(parent_id);
                    }
                }
                state.insert(protocol_id, State::Unvisited);
                graph.insert(protocol_id, neighbors);
                id2span.insert(protocol_id, decl.span);
            }
        }
        let mut ok = true;
        for &pid in graph.keys() {
            if matches!(state.get(&pid), Some(State::Unvisited)) {
                if let Some(cyclic_id) = self.dfs(pid, &graph, &mut state) {
                    let cyclic_protocol_name = self.ctx.types.infos[cyclic_id.0].name.clone();
                    self.diagnostics.push(SemanticError::new(
                        SemanticErrorKind::CyclicProtocolExtension {
                            protocol_name: cyclic_protocol_name,
                        },
                        id2span
                            .get(&cyclic_id)
                            .expect("Protocol id not stored")
                            .clone(),
                    ));
                    ok = false;
                }
            }
        }
        return ok;
    }

    fn dfs(
        &self,
        node: TypeId,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        state: &mut HashMap<TypeId, State>,
    ) -> Option<TypeId> {
        state.insert(node, State::Visiting);
        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                match state.get(&neighbor).unwrap_or(&State::Unvisited) {
                    State::Unvisited => {
                        if let Some(id) = self.dfs(neighbor, graph, state) {
                            return Some(id);
                        }
                    }
                    State::Visiting => {
                        return Some(neighbor);
                    }
                    State::Visited => {
                        continue;
                    }
                }
            }
        }
        state.insert(node, State::Visited);
        None
    }

    pub fn collect_extended_methods(&mut self, decls: &[Decl]) {
        let mut graph: HashMap<TypeId, Vec<TypeId>> = HashMap::new();
        for decl in decls {
            if let DeclKind::Protocol { name, parents, .. } = &decl.node {
                let id = self.ctx.types.resolve(name).unwrap();
                let mut neighbors: Vec<TypeId> = Vec::new();
                if let Some(parents) = parents {
                    for parent in parents {
                        let pid = self.ctx.types.resolve(parent).unwrap();
                        neighbors.push(pid);
                    }
                }
                graph.insert(id, neighbors);
            }
        }
        let mut store_methods: HashMap<TypeId, HashMap<String, Symbol>> = HashMap::new();
        for id in graph.keys() {
            let id = *id;
            let mut queue: VecDeque<TypeId> = VecDeque::new();
            let mut used: HashMap<TypeId, bool> = HashMap::new();
            let mut all_methods: HashMap<String, Symbol> = HashMap::new();
            queue.push_back(id);
            used.insert(id, true);
            while let Some(id) = queue.pop_front() {
                let name = self.ctx.types.infos[id.0].name.clone();
                let new_methods = self.ctx.types.infos[id.0].methods.clone();
                for (method_name, method_symbol) in new_methods {
                    if all_methods.contains_key(&method_name) {
                        self.diagnostics.push(SemanticError::new(
                            SemanticErrorKind::ProtocolMethodCollision {
                                protocol_name: name.clone(),
                                method_name: method_name.clone(),
                            },
                            method_symbol.span,
                        ));
                    } else {
                        all_methods.insert(method_name.clone(), method_symbol);
                    }
                }
                if let Some(neighbors) = graph.get(&id) {
                    for neighbor in neighbors {
                        if !used.contains_key(neighbor) {
                            used.insert(*neighbor, true);
                            queue.push_back(*neighbor);
                        }
                    }
                }
            }
            store_methods.insert(id, all_methods);
        }
        for (id, all_methods) in store_methods {
            self.ctx.types.infos[id.0].methods = all_methods;
        }
    }
}
