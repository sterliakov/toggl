use iced::widget::text;
use iced_aw::badge;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectId(u64);

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub active: bool,
    pub color: String,
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Project {
    pub fn parsed_color(&self) -> iced::Color {
        iced::Color::parse(&self.color).expect("Project color must be valid")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaybeProject {
    Some(Project),
    None,
}

impl std::fmt::Display for MaybeProject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeProject::Some(p) => p.fmt(f),
            MaybeProject::None => f.write_str("---"),
        }
    }
}

impl From<Project> for MaybeProject {
    fn from(value: Project) -> Self {
        Self::Some(value)
    }
}

impl From<Option<Project>> for MaybeProject {
    fn from(value: Option<Project>) -> Self {
        match value {
            Some(p) => Self::Some(p),
            None => Self::None,
        }
    }
}

impl From<MaybeProject> for Option<Project> {
    fn from(val: MaybeProject) -> Self {
        match val {
            MaybeProject::Some(p) => Some(p),
            MaybeProject::None => None,
        }
    }
}

impl MaybeProject {
    pub fn project_badge<'a, T>(
        &self,
    ) -> badge::Badge<'a, T, iced::Theme, iced::Renderer> {
        if let Self::Some(project) = self {
            let color = project.parsed_color();
            badge(text(project.name.clone()).size(10).line_height(1.0)).style(
                move |_, _| badge::Style {
                    background: color.into(),
                    ..badge::Style::default()
                },
            )
        } else {
            badge(text("No project".to_string()).size(10).line_height(1.0))
                .style(iced_aw::style::badge::light)
        }
        .height(22)
    }
}
