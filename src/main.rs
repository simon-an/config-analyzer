#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// hide console window on Windows in release
use eframe::egui::{self, Vec2};
use egui_graphs::{default_edge_transform, Graph, GraphView, Node, SettingsInteraction};
use gitlab::{
    api::{groups::GroupBuilder, projects::repository::TreeBuilder, ApiError, Query},
    Project,
};
use petgraph::{prelude::*, EdgeType};
mod gitlab_group;
use gitlab_group::Group;
pub const DEFAULT_SPAWN_SIZE: f32 = 250.;

fn main() -> Result<(), eframe::Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1980.0, 1024.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Config Analyzer",
        options,
        Box::new(|_cc| Box::<ConfigAnalyzer>::default()),
    )
}

struct ConfigAnalyzer {
    url: String,
    group: String,
    password: String,
    gitlab_client: Option<gitlab::Gitlab>,
    data: BTreeMap<Box<str>, Project>,
    graph: Option<Graph<ProjectNode, (), Directed>>,
}

#[derive(Clone, Hash, std::fmt::Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct ProjectNode {
    id: Box<str>,
    name: Box<str>,
    group: Box<str>,
}
impl From<Project> for ProjectNode {
    fn from(value: Project) -> Self {
        Self {
            id: value.id.to_string().into_box_str(),
            name: value.name.into_box_str(),
            group: value.namespace.name.into_box_str(),
        }
    }
}

impl ConfigAnalyzer {
    fn generate_graph(&mut self) {
        // TODO implement dependency graph
        // let mut g: DiGraphMap<ProjectNode, ()> = GraphMap::new();
        let mut g: StableGraph<ProjectNode, ()> = StableGraph::new();
        let mut d: BTreeMap<Box<str>, usize> = BTreeMap::new();

        for project in self.data.iter() {
            // let node = g.add_node(());
            // let node = Node::default();
            // let node = node.with_data(Some(project.1));
            let index = g.add_node(project.1.clone());
            d.insert(project.0.clone(), index.index());
        }
        // let a = g.add_node(());
        // let b = g.add_node(());
        // let c = g.add_node(());

        // g.add_edge(a, b, ());
        // g.add_edge(b, c, ());
        // g.add_edge(c, a, ());

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

pub fn to_input_graph<N: Clone, E: Clone, Ty: EdgeType>(
    g: &StableGraph<N, E, Ty>,
) -> Graph<N, E, Ty> {
    transform(g, projects_node_transform, default_edge_transform)
}

pub fn projects_node_transform<N: Clone>(idx: NodeIndex, data: &N) -> Node<N> {
    let loc = random_location(DEFAULT_SPAWN_SIZE);
    Node::new(loc, data.clone()).with_label(idx.index().to_string())
}

impl Default for ConfigAnalyzer {
    fn default() -> Self {
        Self {
            url: "git.eon-cds.de/".to_owned(),
            group: "23994".to_owned(),
            password: std::env::var("GITLAB_TOKEN").unwrap_or_default(),
            gitlab_client: None,
            data: BTreeMap::new(),
            graph: None,
        }
    }
}

impl eframe::App for ConfigAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls").show(ctx, |ui: &mut egui::Ui| {
            let url_label = ui.label("Hostname: ");
            ui.vertical(|ui| {
                ui.heading("Heading 1");
                ui.text_edit_singleline(&mut self.url)
                    .labelled_by(url_label.id);
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

                ui.text_edit_singleline(&mut self.group)
                    .labelled_by("Group".into());

                // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
                if ui.button("Connect to Gitlab").clicked() {
                    let res = gitlab::Gitlab::new(&self.url, &self.password);
                    match &res {
                        Ok(_) => log::info!("Connected to Gitlab"),
                        Err(e) => log::error!("Failed to connect to Gitlab {e}"),
                    }
                    self.gitlab_client = res.ok();
                }
                ui.label(format!("Current Url {}", self.url));

                if let Some(client) = &self.gitlab_client {
                    if ui.button("Load Projects in Group").clicked() {
                        // https://docs.gitlab.com/ee/api/projects.html
                        let projects_request = GroupBuilder::default()
                            .group(self.group.clone())
                            .with_projects(true)
                            .build()
                            .unwrap();
                        let projects_response: Result<Group, ApiError<_>> =
                            projects_request.query(client);

                        match projects_response {
                            Ok(g) => {
                                log::info!("Group: {:?}", &g);

                                self.data
                                    .extend(g.projects.unwrap().into_iter().map(|project| {
                                        (project.id.to_string().into_boxed_str(), project)
                                    }));
                            }
                            Err(e) => {
                                // ui.label(format!("Error: {}", e));
                                log::error!("Error: {}", e);
                            }
                        }
                    }

                    if ui.button("Load Graph Input Data").clicked() {
                        // https://docs.gitlab.com/ee/api/repositories.html#list-repository-tree
                        for project in self.data.iter() {
                            let tree_object: Option<Vec<gitlab::types::RepoTreeObject>> =
                                TreeBuilder::default()
                                    // .path("main.tf")
                                    .ref_("main")
                                    .project(project.0.to_string())
                                    .recursive(true)
                                    .build()
                                    .unwrap()
                                    .query(client)
                                    .ok();
                            if let Some(tos) = tree_object {
                                for tree_object in tos.iter().filter(|o| {
                                    o.path.starts_with("cli-config-") && o.path.ends_with(".json")
                                }) {
                                    log::info!(
                                        "Json File: {:?} in project {}",
                                        &tree_object,
                                        &project.1.name
                                    );
                                }
                            }
                        }

                        self.generate_graph();
                    }
                }

                ui.label(format!("Projects: {}", &self.data.len()));
                for project in &self.data {
                    ui.label(format!(
                        "Project Name: {}, Project ID: {}",
                        project.1.name, project.1.id
                    ));
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            if let Some(mut graph) = self.graph.as_mut() {
                let interaction_settings = &SettingsInteraction::new()
                    .with_dragging_enabled(true)
                    .with_clicking_enabled(true)
                    .with_folding_enabled(true)
                    .with_selection_enabled(true)
                    .with_selection_multi_enabled(true)
                    .with_selection_depth(i32::MAX)
                    .with_folding_depth(usize::MAX);
                ui.add(&mut GraphView::new(&mut graph).with_interactions(interaction_settings));
            }
        });
    }
}
