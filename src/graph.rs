use std::collections::BTreeMap;

use crate::configuration_schema::*;
use eframe::epaint::Vec2;
use egui_graphs::{default_edge_transform, to_graph_custom, Graph, Node};
use log::warn;
use petgraph::{prelude::*, EdgeType};
use rand::Rng;

use crate::{ProjectNode, DEFAULT_SPAWN_SIZE};

impl super::ConfigAnalyzer {
    pub(crate) fn update_project_dependencies(&mut self) {
        self.project_dependencies.clear();

        // TODO: next session we will refactor this
        for (p1_key, config_1) in self.project_configs.iter() {
            for (p2_key, config_2) in self.project_configs.iter() {
                if p1_key != p2_key {
                    let mut found = false;
                    for task_1 in &config_1.tasks {
                        for task_2 in &config_2.tasks {
                            log::debug!(
                                "Checking {p1_key}.{:#?} -> {p2_key}.{:#?}",
                                task_1.source,
                                task_2.target
                            );
                            match &task_1.source {
                                SourceConfig::GitlabProjectTerraformState(
                                    GitlabProjectConfig { project_id, .. },
                                ) => {
                                    if project_id.to_string().into_boxed_str() == *p1_key {
                                        log::info!(
                                            "GitlabProjectTerraformState usage in same repo {}",
                                            project_id
                                        );
                                    } else {
                                        log::error!("GitlabProjectTerraformState usage in differnt repo -> {}", project_id);
                                    }
                                }
                                SourceConfig::GitlabProjectVariables(_) => {
                                    log::error!("GitlabProjectVariables must not be used")
                                }
                                SourceConfig::AzureKeyvault(c1) => match &task_2.target {
                                    TargetConfig::AzureKeyvault(c2) => {
                                        if c1.keyvault_url == c2.keyvault_url {
                                            log::info!(
                                                "AzureKeyvault match {} == {}",
                                                c1.keyvault_url,
                                                c2.keyvault_url
                                            );
                                            found = true;
                                        } else {
                                            log::info!(
                                                "AzureKeyvault no match {} -> {}",
                                                c1.keyvault_url,
                                                c2.keyvault_url
                                            );
                                        }
                                    }
                                    _ => {}
                                },
                                SourceConfig::Redis { hostname, .. } => match &task_2.target {
                                    TargetConfig::Redis { hostname: h2, .. } => {
                                        if *hostname == *h2 {
                                            log::info!(
                                                "Redis host are same {}... checking variables",
                                                h2
                                            );
                                            let target_variable_names = &task_2
                                                .mapping
                                                .values()
                                                .map(|v| {
                                                    v.iter()
                                                        .map(|v| match v {
                                                            MappingTarget::KeyOnly(k) => k,
                                                            MappingTarget::ConvertMapping(c) => {
                                                                &c.key
                                                            }
                                                            MappingTarget::CopyMapping(c) => &c.key,
                                                        })
                                                        .collect::<Vec<&String>>()
                                                })
                                                .flatten()
                                                .collect::<Vec<&String>>();
                                            let source_variable_names =
                                                &task_1.mapping.keys().collect::<Vec<&String>>();
                                            use array_tool::vec::Intersect;
                                            let intersects = source_variable_names
                                                .intersect(target_variable_names.clone());
                                            if !intersects.is_empty() {
                                                found = true;
                                            } else {
                                                warn!("No intersecting variables for redis: {:?} != {:?}", source_variable_names, target_variable_names);
                                            }
                                        } else {
                                            warn!(
                                                "Different hostnames for redis: {} != {}",
                                                hostname, h2
                                            );
                                        }
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                    }
                    if found {
                        self.project_dependencies
                            .push((p2_key.clone(), p1_key.clone()));
                    }
                }
            }
        }
    }

    pub(crate) fn generate_graph(&mut self) {
        // TODO implement dependency graph
        // let mut g: DiGraphMap<ProjectNode, ()> = GraphMap::new();
        let mut g: StableGraph<ProjectNode, ()> = StableGraph::new();
        let mut nodes: BTreeMap<Box<str>, NodeIndex> = BTreeMap::new();

        for project in self.data.iter() {
            // let node = g.add_node(());
            // let node = Node::default();
            // let node = node.with_data(Some(project.1));
            let index = g.add_node(project.1.clone().into());
            nodes.insert(project.0.clone(), index);
        }
        // let a = g.add_node(ProjectNode {
        //     id: "1".into(),
        //     name: "1".into(),
        //     group: "1".into(),
        // });
        // let b = g.add_node(ProjectNode {
        //     id: "2".into(),
        //     name: "2".into(),
        //     group: "2".into(),
        // });

        for edge in &self.project_dependencies {
            let (a, b) = (nodes.get(&edge.0).unwrap(), nodes.get(&edge.1).unwrap());

            g.add_edge(*a, *b, ());
        }

        self.graph = Some(to_input_graph(&g));
        // println!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]));
        // graph {
        //     0 [label="\"0\""]
        //     1 [label="\"0\""]
        //     2 [label="\"0\""]
        //     3 [label="\"0\""]
        //     1 -- 2
        //     3 -- 4
        //     2 -- 3
        // }
    }
}

pub fn random_location(size: f32) -> Vec2 {
    let mut rng = rand::thread_rng();
    Vec2::new(rng.gen_range(0. ..size), rng.gen_range(0. ..size))
}

pub fn to_input_graph<E: Clone, Ty: EdgeType>(
    g: &StableGraph<ProjectNode, E, Ty>,
) -> Graph<ProjectNode, E, Ty> {
    to_graph_custom(g, projects_node_transform, default_edge_transform)
}

pub fn projects_node_transform(_idx: NodeIndex, data: &ProjectNode) -> Node<ProjectNode> {
    let loc = random_location(DEFAULT_SPAWN_SIZE);
    Node::new(loc, data.clone()).with_label(data.name.to_string())
}
