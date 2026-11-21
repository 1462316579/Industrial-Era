//! Home's Mods tab in the glass look (the mockup's `InstanceMods`, `launcher-views.tsx`): "N mods
//! in {instance}" and + Find mods over one `ROW` per mod, its icon, name and "version · source".
//! The launcher's own controls fit in: the filter and Update all in the head, a mod's update
//! and its on/off `Toggle` at the row's end. Dark appearance only.
use super::*;
use crate::glass_ui::{self as ui, white};
use glass_home_lists::Listener;
use industrial_era_theme::glass::radius;
use servers_page::Field;

impl Launcher {
    /// Home's Mods tab: mods are removed; show empty state.
    pub(super) fn glass_instance_mods(&mut self, _cx: &mut Context<Self>) -> Div {
        ui::empty().child(tr("Mods removed"))
    }
}
