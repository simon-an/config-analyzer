#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::BTreeMap;

use egui_graphs::{Graph, GraphView, SettingsInteraction};
use petgraph::Directed;
use serde::{Deserialize, Serialize};

// hide console window on Windows in release
use eframe::egui::{self};
use gitlab::{
    api::{groups::GroupBuilder, projects::repository::TreeBuilder, ApiError, Query},
    Project, RestError,
};
// use egui_graphs::{default_edge_transform, Graph, GraphView, Node, SettingsInteraction, to_graph_custom};
// use petgraph::{prelude::*, EdgeType};

mod configuration_schema;
mod gitlab_file;
mod gitlab_group;
mod graph;
use gitlab_file::File;
use gitlab_group::Group;

use crate::configuration_schema::VariableShareConfig;
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
        Box::new(|_cc: &eframe::CreationContext<'_>| Box::<ConfigAnalyzer>::default()),
    )
}

struct ConfigAnalyzer {
    url: String,
    groups: Vec<String>,
    password: String,
    gitlab_client: Option<gitlab::Gitlab>,
    data: BTreeMap<Box<str>, Project>,
    project_configs: BTreeMap<Box<str>, VariableShareConfig>,
    project_dependencies: Vec<(Box<str>, Box<str>)>,
    graph: Option<Graph<ProjectNode, (), Directed>>,
}

impl Default for ConfigAnalyzer {
    fn default() -> Self {
        Self {
            url: "".to_owned(),
            groups: vec!["32365".to_owned(), "32366".to_owned(), "32364".to_owned(), "25429".to_owned()],
            password: std::env::var("GITLAB_TOKEN").unwrap_or_default(),
            gitlab_client: None,
            data: BTreeMap::new(),
            project_configs: BTreeMap::new(),
            project_dependencies: Vec::new(),
            graph: None,
        }
    }
}

#[derive(Clone, Hash, std::fmt::Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProjectNode {
    id: Box<str>,
    name: Box<str>,
    group: Box<str>,
}
impl From<Project> for ProjectNode {
    fn from(value: Project) -> Self {
        Self {
            id: value.id.to_string().into_boxed_str(),
            name: value.name.into_boxed_str(),
            group: value.namespace.name.into_boxed_str(),
        }
    }
}

impl eframe::App for ConfigAnalyzer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls").show(ctx, |ui: &mut egui::Ui| {
            // let url_label = ui.label("Hostname: ");
            ui.vertical(|ui| {
                // ui.heading("Heading 1");
                // ui.text_edit_singleline(&mut self.url)
                //     .labelled_by(url_label.id);
                // ui.add(egui::TextEdit::singleline(&mut self.password).password(true));

                // ui.text_edit_singleline(&mut self.group)
                //     .labelled_by("Group".into());

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
                    if ui.button("Load Projects in Groups").clicked() {
                        // https://docs.gitlab.com/ee/api/projects.html
                        for group in &self.groups {
                            let projects_request = GroupBuilder::default()
                                .group(group.clone())
                                .with_projects(true)
                                .build()
                                .unwrap();
                            let projects_response: Result<Group, ApiError<_>> =
                                projects_request.query(client);

                            match projects_response {
                                Ok(g) => {
                                    log::debug!("Group: {:?}", &g);

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
                            //    tos.iter().for_each(|o| {
                            //         log::info!(
                            //             "Json File: {:?} in project {}",
                            //             &o,
                            //             &project.1.name
                            //         );
                            //     });
                                for tree_object in tos.iter().filter(|o| {
                                    o.name.starts_with("cli-config-") || o.name.starts_with("redis") && o.name.ends_with(".json") // TODO yaml
                                }) {
                                    log::info!(
                                        "Json File: {:?} in project {}",
                                        &tree_object.path,
                                        &project.1.name
                                    );
                                    let file_content: Result<File, ApiError<RestError>> =
                                        gitlab::api::projects::repository::files::FileBuilder::default()
                                            .project(project.0.to_string())
                                            .file_path(tree_object.path.clone())
                                            .ref_("main")
                                            .build()
                                            .unwrap()
                                            .query(client);
                                    match file_content {
                                        Err(error) => {
                                            log::error!(
                                                "Failed to load file content for {:?} {}",
                                                &tree_object,
                                                error
                                            );
                                        }
                                        Ok(content) => {
                                            // log::info!("File Content: {}", &content);
                                            let config: VariableShareConfig =
                                                serde_json::from_str(&content.get_content()).expect("content of file is not valid json");
                                            log::debug!("Config: {:?}", &config);

                                            self.project_configs.insert(project.0.clone(), config);
                                        }
                                    }
                                }
                            }
                        }
                        self.update_project_dependencies();
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
                    .with_selection_enabled(true)
                    .with_selection_multi_enabled(true);
                ui.add(&mut GraphView::new(&mut graph).with_interactions(interaction_settings));
            }
        });
    }
}
