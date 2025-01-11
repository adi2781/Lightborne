use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use enum_map::EnumMap;

use super::{
    sensor::{HitByLight, LightSensor},
    LightColor, LightRaySource, LIGHT_SPEED,
};
use crate::shared::GroupLabel;

#[derive(Component)]
pub struct LightSegment {
    start: Vec2,
    end: Vec2,
    pub color: LightColor,
}

#[derive(Resource, Default)]
pub struct LightSegmentCache {
    table: EnumMap<LightColor, Vec<Entity>>,
}

pub fn simulate_light_sources(
    mut commands: Commands,
    q_light_sources: Query<&LightRaySource>,
    mut q_rapier: Query<&mut RapierContext>,
    q_light_sensor: Query<&LightSensor>,
    mut segment_cache: ResMut<LightSegmentCache>,
) {
    let Ok(rapier_context) = q_rapier.get_single_mut() else {
        return;
    };

    for source in q_light_sources.iter() {
        let mut ray_pos = source.start_pos;
        let mut ray_dir = source.start_dir;
        let collision_groups = match source.color {
            LightColor::White => CollisionGroups::new(
                GroupLabel::WHITE_RAY,
                GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR,
            ),
            _ => CollisionGroups::new(
                GroupLabel::LIGHT_RAY,
                GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::WHITE_RAY,
            ),
        };

        let mut ray_qry = QueryFilter::new().groups(collision_groups);

        let mut pts: Vec<Vec2> = vec![ray_pos];
        let mut remaining_time = source.time_traveled;
        for _ in 0..source.color.num_bounces() + 1 {
            let Some((entity, intersection)) = rapier_context.cast_ray_and_get_normal(
                ray_pos,
                ray_dir,
                remaining_time,
                true,
                ray_qry,
            ) else {
                let final_point = ray_pos + ray_dir * remaining_time;
                pts.push(final_point);
                break;
            };

            if intersection.time_of_impact < 0.01 {
                break;
            }

            remaining_time -= intersection.time_of_impact;

            pts.push(intersection.point);

            if q_light_sensor.contains(entity) {
                commands.entity(entity).insert(HitByLight);
            };

            ray_pos = intersection.point;
            ray_dir = ray_dir.reflect(intersection.normal);
            ray_qry = ray_qry.exclude_collider(entity);
        }

        while segment_cache.table[source.color].len() < pts.len() - 1 {
            segment_cache.table[source.color].push(commands.spawn(()).id());
        }

        for (i, pair) in pts.windows(2).enumerate() {
            let segment = LightSegment {
                start: pair[0],
                end: pair[1],
                color: source.color,
            };

            let midpoint = segment.start.midpoint(segment.end).extend(1.0);
            let scale = Vec3::new(segment.start.distance(segment.end), 1., 1.);
            let rotation = (segment.end - segment.start).to_angle();
            let transform = Transform::from_translation(midpoint)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_z(rotation));

            commands
                .entity(segment_cache.table[source.color][i])
                .insert(segment)
                .insert(transform);

            if source.color == LightColor::White {
                commands
                    .entity(segment_cache.table[source.color][i])
                    .insert((
                        Collider::cuboid(0.5, 0.5),
                        Sensor,
                        CollisionGroups::new(
                            GroupLabel::WHITE_RAY,
                            GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::LIGHT_RAY,
                        ),
                    ));
            }
        }
    }
}

pub fn tick_light_sources(mut q_light_sources: Query<&mut LightRaySource>) {
    for mut source in q_light_sources.iter_mut() {
        source.time_traveled += LIGHT_SPEED;
    }
}
