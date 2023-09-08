#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use eframe::egui::{self};
use egui_graphs::{to_input_graph, Graph, GraphView, SettingsInteraction};
use gitlab::{
    api::{groups::GroupBuilder, projects::repository::TreeBuilder, ApiError, Query},
    GroupId, GroupStatistics, Project, VisibilityLevel,
};
use petgraph::{stable_graph::StableGraph, Directed};
use serde::{Deserialize, Serialize};

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
    data: Vec<Project>,
    g: Option<Graph<(), (), Directed>>,
}

impl ConfigAnalyzer {
    fn generate_graph(&mut self) {
        // TODO implement dependency graph
        let mut g: StableGraph<(), ()> = StableGraph::new();

        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());

        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());

        self.g = Some(to_input_graph(&g))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Group {
    /// The ID of the group.
    pub id: GroupId,
    /// The name of the group.
    pub name: String,
    /// The path to the group.
    pub path: String,
    /// The description of the group.
    pub description: Option<String>,
    /// Whether the project is public, internal, or private.
    pub visibility: VisibilityLevel,
    /// Whether LFS is enabled for the group.
    pub lfs_enabled: bool,
    /// The URL to the group avatar.
    pub avatar_url: Option<String>,
    /// The URL to the group's profile page.
    pub web_url: String,
    /// Whether membership requests are allowed for the group.
    pub request_access_enabled: bool,
    pub full_name: String,
    pub full_path: String,
    pub parent_id: Option<GroupId>,
    /// Statistics about the group.
    pub statistics: Option<GroupStatistics>,

    pub projects: Option<Vec<Project>>, // TODO: create MR for gitlab crate to add this field
}

impl Default for ConfigAnalyzer {
    fn default() -> Self {
        Self {
            url: "".to_owned(),
            group: "".to_owned(),
            password: std::env::var("GITLAB_TOKEN").unwrap_or_default(),
            gitlab_client: None,
            data: Vec::new(),
            g: None,
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
                                self.data.extend(g.projects.unwrap().into_iter());
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
                                    .project(project.id.value())
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
                                        &project.name
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
                        project.name, project.id
                    ));
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            if let Some(mut graph) = self.g.as_mut() {
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
