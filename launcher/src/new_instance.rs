//! New instance's kinds (docs/design-system.md, "Night"; plan R0, screen 4): IndustrialEra, Vanilla
//! (Minecraft: Java Edition as Mojang ships it) and Modded (with Fabric or Quilt). A Java kind picks
//! its version from Mojang's public list (`minecraft_versions`), fetched once when the dialog first
//! needs it; the game itself comes with the first Play, for an account that owns it.
use super::*;
use crate::plate::alpha;
use crate::shell::{c, Action};
use industrial_era_launcher::loaders::Kind as Loader;
use industrial_era_launcher::minecraft_versions::{Kind as VersionKind, Versions};
use servers_page::Field;
use theme::night;

/// Rows the version list shows at most: the search finds the others.
const ROWS: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NewKind {
    IndustrialEra,
    Vanilla,
}

/// What a new Java instance plays.
#[derive(Clone, Debug)]
pub(super) struct JavaChoice {
    pub(super) kind: NewKind,
    pub(super) loader: Loader,
    /// None: the newest release.
    pub(super) version: Option<String>,
    pub(super) snapshots: bool,
    pub(super) old: bool,
}

impl Default for JavaChoice {
    fn default() -> Self {
        JavaChoice {
            kind: NewKind::IndustrialEra,
            loader: Loader::Fabric,
            version: None,
            snapshots: false,
            old: false,
        }
    }
}

impl JavaChoice {
    /// The Minecraft version it makes: the chosen one, else the newest release.
    pub(super) fn minecraft(&self, versions: Option<&Versions>) -> Option<String> {
        self.version
            .clone()
            .or_else(|| versions.map(|v| v.latest_release.clone()))
    }

    /// The name it gets when none is typed: "Vanilla 1.21.4", "Fabric 1.21.4".
    pub(super) fn default_name(&self, minecraft: &str) -> String {
        format!("Vanilla {minecraft}")
    }
}

/// Mojang's list as the dialog has it.
#[derive(Default)]
pub(super) enum VersionList {
    #[default]
    NotAsked,
    Loading,
    Ready(Versions),
    Failed(String),
}

impl Launcher {
    fn java_choice(&mut self) -> Option<&mut JavaChoice> {
        self.instances.dialog.as_mut().map(|d| &mut d.java)
    }

    /// The name a new Java instance gets for what it plays now.
    fn default_name(&self) -> Option<String> {
        let j = &self.instances.dialog.as_ref()?.java;
        let versions = match &self.instances.versions {
            VersionList::Ready(v) => Some(v),
            _ => None,
        };
        (j.kind != NewKind::IndustrialEra)
            .then(|| j.minecraft(versions))
            .flatten()
            .map(|mc| j.default_name(&mc))
    }

    /// Changes what the new instance plays. Its name follows while it is the one the dialog gave
    /// (or none), never one the player typed.
    pub(super) fn change(&mut self, f: impl FnOnce(&mut JavaChoice)) {
        let before = self.default_name();
        if let Some(j) = self.java_choice() {
            f(j);
        }
        let typed = self.servers.value(Field::InstanceName).trim().to_string();
        if typed.is_empty() || Some(&typed) == before.as_ref() {
            let after = self.default_name().unwrap_or_default();
            self.servers.text(Field::InstanceName).set(after);
        }
    }

    /// Picks the dialog's kind; a Java one fetches Mojang's list the first time.
    pub(super) fn pick_kind(&mut self, kind: NewKind, cx: &mut Context<Self>) {
        self.change(|j| j.kind = kind);
        if kind == NewKind::IndustrialEra
            || !matches!(self.instances.versions, VersionList::NotAsked)
        {
            return;
        }
        self.instances.versions = VersionList::Loading;
        let task = cx
            .background_executor()
            .spawn(async { industrial_era_launcher::minecraft_versions::load() });
        cx.spawn(async move |this, cx| {
            let list = task.await;
            let _ = this.update(cx, |this, cx| {
                let before = this.servers.value(Field::InstanceName).trim().is_empty();
                this.instances.versions = match list {
                    Ok(v) => VersionList::Ready(v),
                    Err(e) => VersionList::Failed(e),
                };
                // The newest release is known now: so is the name.
                if before {
                    this.change(|_| {});
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The tabs over a new instance's dialog: IndustrialEra, Vanilla.
    pub(super) fn kind_tabs(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let now = self.instances.dialog.as_ref().map(|d| d.java.kind);
        div().flex().gap(px(4.)).children(
            [
                (NewKind::IndustrialEra, "IndustrialEra"),
                (NewKind::Vanilla, "Vanilla"),
            ]
            .map(|(kind, label)| {
                shell::tab(
                    format!("new-kind-{label}"),
                    tr(label),
                    now == Some(kind),
                    &t,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.pick_kind(kind, cx);
                    cx.notify();
                }))
            }),
        )
    }

    /// A Java kind's part of the dialog: the version (search, kinds, list) and where the game
    /// comes from.
    pub(super) fn java_choice_rows(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let n = t.night;
        let Some(choice) = self.instances.dialog.as_ref().map(|d| d.java.clone()) else {
            return div();
        };
        let heading = |s: &str| {
            shell::text(night::CAPTION, c(n.muted))
                .font_weight(FontWeight::SEMIBOLD)
                .child(tr(s).to_string())
        };
        let toggle = |id: &'static str, label: &'static str, on: bool| {
            shell::action(id, if on { Action::Raised } else { Action::Quiet }, &t)
                .h(px(26.))
                .px(px(10.))
                .child(shell::label(tr(label).to_string()))
        };
        let search = self
            .servers
            .value(Field::InstanceVersion)
            .trim()
            .to_lowercase();
        let chosen = choice.minecraft(match &self.instances.versions {
            VersionList::Ready(v) => Some(v),
            _ => None,
        });
        let list: AnyElement = match &self.instances.versions {
            VersionList::NotAsked | VersionList::Loading => shell::text(night::UI, c(n.muted))
                .child(tr("Asking Mojang for its versions…").to_string())
                .into_any_element(),
            VersionList::Failed(e) => shell::text(night::UI, c(n.danger))
                .child(e.clone())
                .into_any_element(),
            VersionList::Ready(versions) => {
                let rows: Vec<_> = versions
                    .of(choice.snapshots, choice.old)
                    .filter(|v| search.is_empty() || v.id.to_lowercase().contains(&search))
                    .take(ROWS)
                    .map(|v| {
                        let selected = chosen.as_deref() == Some(v.id.as_str());
                        let id = v.id.clone();
                        let kind = match v.kind {
                            VersionKind::Release => "Release",
                            VersionKind::Snapshot => "Snapshot",
                            VersionKind::OldBeta => "Beta",
                            VersionKind::OldAlpha => "Alpha",
                        };
                        shell::row(format!("mc-{}", v.id), selected, &t)
                            .flex_none()
                            .h(px(32.))
                            .px(px(12.))
                            .child(
                                shell::text(night::UI, c(n.ink))
                                    .relative()
                                    .w(px(120.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(v.id.clone()),
                            )
                            .child(shell::text(night::CAPTION, c(n.muted)).relative().child(
                                format!("{} · {}", tr(kind), v.released.get(..10).unwrap_or("")),
                            ))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.change(|j| j.version = Some(id.clone()));
                                cx.notify();
                            }))
                    })
                    .collect();
                if rows.is_empty() {
                    shell::text(night::UI, c(n.muted))
                        .child(tr("No version matches.").to_string())
                        .into_any_element()
                } else {
                    div()
                        .id("mc-versions")
                        .flex()
                        .flex_col()
                        .h(px(6. * 32. + 8.))
                        .p(px(4.))
                        .overflow_y_scroll()
                        .rounded(px(night::RADIUS_CONTROL))
                        .border_1()
                        .border_color(alpha(n.stroke, n.stroke_alpha))
                        .children(rows)
                        .into_any_element()
                }
            }
        };
        let filters = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(self.field(Field::InstanceVersion, tr("Search versions"), 240., cx))
            .child(
                toggle("mc-snapshots", "Snapshots", choice.snapshots).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.change(|j| j.snapshots = !j.snapshots);
                        cx.notify();
                    },
                )),
            )
            .child(toggle("mc-old", "Old", choice.old).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.change(|j| j.old = !j.old);
                    cx.notify();
                },
            )));
        let rows = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(heading("Minecraft version"))
            .child(filters)
            .child(list);
        rows.child(shell::text(night::CAPTION, c(n.muted)).child(
            tr("Play downloads the game from Mojang for your account the first time, with the Java it needs.")
                .to_string(),
        ))
    }
}

/// Makes a Java instance: `name`, playing `minecraft`.
pub(super) fn create_java(
    _choice: &JavaChoice,
    name: &str,
    minecraft: &str,
) -> Result<instances::Instance, String> {
    instances::create_java(name, minecraft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_java_instance_is_named_after_what_it_plays() {
        let choice = JavaChoice {
            kind: NewKind::Vanilla,
            ..Default::default()
        };
        assert_eq!(choice.default_name("1.21.4"), "Vanilla 1.21.4");
        assert_eq!(choice.minecraft(None), None);
    }
}
