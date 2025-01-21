use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::player::{PlayerBundle, PlayerMarker};
use activatable::ActivatablePlugin;
use crystal::CrystalPlugin;
use misc::{ButtonBundle, StartFlag};
use setup::LevelSetupPlugin;
use walls::{spawn_wall_collision, WallBundle};

pub mod activatable;
mod crystal;
mod entity;
pub mod interactable;
pub mod misc;
mod setup;
mod walls;

/// [`Plugin`] that handles everything related to the level.
pub struct LevelManagementPlugin;

impl Plugin for LevelManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .add_plugins(LevelSetupPlugin)
            .add_plugins(ActivatablePlugin)
            .add_plugins(CrystalPlugin)
            .add_event::<LevelSwitchEvent>()
            .init_resource::<CurrentLevel>()
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                    load_level_neighbors: true,
                },
                ..default()
            })
            .register_ldtk_entity::<PlayerBundle>("Lyra")
            .register_ldtk_entity::<ButtonBundle>("Button")
            .register_ldtk_entity::<StartFlag>("Start")
            .register_ldtk_int_cell::<WallBundle>(1)
            .add_systems(Update, spawn_wall_collision)
            .add_systems(Update, switch_level);
    }
}

/// [`Resource`] that holds the `level_iid` of the current level.
#[derive(Default, Resource)]
pub struct CurrentLevel {
    pub level_iid: String,
    pub world_box: Rect,
}

/// [`Event`] that will be sent to inform other systems that the level is switching and should be
/// reinitialized.
#[derive(Event)]
pub struct LevelSwitchEvent;

/// [`System`] that will run on [`Update`] to check if the Player has moved to another level. If
/// the player has, then a [`LevelSwitchEvent`] will be sent out to notify other systems.
fn switch_level(
    q_player: Query<(&Transform, &EntityInstance), With<PlayerMarker>>,
    q_level: Query<&LevelIid>,
    mut level_selection: ResMut<LevelSelection>,
    ldtk_projects: Query<&LdtkProjectHandle>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
    mut ev_level_switch: EventWriter<LevelSwitchEvent>,
    mut current_level: ResMut<CurrentLevel>,
) {
    let Ok((transform, instance)) = q_player.get_single() else {
        return;
    };
    for level_iid in q_level.iter() {
        let ldtk_project = ldtk_project_assets
            .get(ldtk_projects.single())
            .expect("Project should be loaded if level has spawned");

        let level = ldtk_project
            .get_raw_level_by_iid(&level_iid.to_string())
            .expect("Spawned level should exist in Ldtk project");

        let world_box = Rect::new(
            level.world_x as f32,
            level.world_y as f32,
            (level.world_x + level.px_wid) as f32,
            (level.world_y - level.px_hei) as f32,
        );

        let player_box = Rect::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.x + instance.width as f32,
            transform.translation.y - instance.height as f32,
        );

        if world_box.contains(player_box.center()) {
            // ev_move_camera.send(MoveCameraEvent(world_box.center()));
            if current_level.level_iid != level_iid.as_str() {
                ev_level_switch.send(LevelSwitchEvent);
                *current_level = CurrentLevel {
                    level_iid: level_iid.to_string(),
                    world_box,
                };
                *level_selection = LevelSelection::iid(level_iid.to_string());
            }
            break;
        }
    }
}
