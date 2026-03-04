use crate::mcp::tools::McpContext;
use crate::KnowledgeEvent;
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

pub struct GraphObserver {
    context: Arc<McpContext>,
    graph: UnGraph<Uuid, String>,
    nodes: HashMap<Uuid, NodeIndex>,
}

impl GraphObserver {
    pub fn new(context: Arc<McpContext>) -> Self {
        Self {
            context,
            graph: UnGraph::default(),
            nodes: HashMap::new(),
        }
    }

    pub async fn run(mut self, mut rx: broadcast::Receiver<KnowledgeEvent>) {
        if let Ok(rels) = self.context.db.list_all_relationships() {
            for (s, t, p) in rels {
                self.add_edge(s, t, p);
            }
        }

        loop {
            tokio::select! {
                result = rx.recv() => {
                    if let Ok(event) = result {
                        match event {
                            KnowledgeEvent::EntityInserted { id, .. } => {
                                self.nodes.entry(id).or_insert_with(|| self.graph.add_node(id));
                                self.recluster_and_update_db().await;
                            }
                            KnowledgeEvent::RelationshipInserted { source_id, target_id, predicate } => {
                                self.add_edge(source_id, target_id, predicate);
                                self.recluster_and_update_db().await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn add_edge(&mut self, s: Uuid, t: Uuid, p: String) {
        let si = *self.nodes.entry(s).or_insert_with(|| self.graph.add_node(s));
        let ti = *self.nodes.entry(t).or_insert_with(|| self.graph.add_node(t));
        self.graph.add_edge(si, ti, p);
    }

    async fn recluster_and_update_db(&mut self) {
        // Incremental Label Propagation (Simplified)
        let mut communities: HashMap<NodeIndex, String> = HashMap::new();
        for node in self.graph.node_indices() {
            communities.insert(node, format!("comm_{}", node.index()));
        }

        for _ in 0..2 {
            let mut next_communities = communities.clone();
            for node in self.graph.node_indices() {
                let mut counts = HashMap::new();
                for edge in self.graph.edges(node) {
                    let neighbor = if edge.source() == node { edge.target() } else { edge.source() };
                    let comm = communities.get(&neighbor).unwrap();
                    *counts.entry(comm).or_insert(0) += 1;
                }
                if let Some((&majority_comm, _)) = counts.iter().max_by_key(|&(_, count)| count) {
                    next_communities.insert(node, majority_comm.clone());
                }
            }
            communities = next_communities;
        }

        // Write community IDs to DB and collect the distinct set that changed.
        let mut updated_comm_ids = std::collections::HashSet::new();
        for (node, comm_id) in &communities {
            let entity_id = self.graph[*node];
            if self.context.db.update_entity_community(entity_id, comm_id).is_ok() {
                updated_comm_ids.insert(comm_id.clone());
            }
        }

        // Fire CommunitiesUpdated only after all DB writes are done.
        if !updated_comm_ids.is_empty() {
            let comm_ids: Vec<String> = updated_comm_ids.into_iter().collect();
            let _ = self.context.event_tx.send(crate::KnowledgeEvent::CommunitiesUpdated { comm_ids });
        }
    }
}

pub async fn spawn_graph_observer(context: Arc<McpContext>, rx: broadcast::Receiver<KnowledgeEvent>) {
    let observer = GraphObserver::new(context);
    tokio::spawn(async move {
        observer.run(rx).await;
    });
}
