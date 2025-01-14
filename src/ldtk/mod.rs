use std::path::Path;

use bevy::{
    app::{Plugin, Startup, Update},
    asset::{load_internal_asset, AssetServer, Assets, Handle},
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::{Added, With},
        system::{Commands, NonSend, ParallelCommands, Query, Res, ResMut},
    },
    math::{UVec2, Vec2},
    render::{mesh::Mesh, render_resource::Shader},
    sprite::{Material2dPlugin, Sprite, SpriteBundle, TextureAtlasLayout},
    transform::components::Transform,
};

use crate::{
    ldtk::{
        components::{LayerIid, LdtkLoader, LdtkLoaderMode, LdtkUnloader, WorldIid},
        json::{
            field::FieldInstance,
            level::{EntityInstance, ImagePosition, Neighbour, TileInstance},
            EntityRef, GridPoint, LdtkColor, Toc, World,
        },
        resources::{
            LdtkAdditionalLayers, LdtkAssets, LdtkGlobalEntityRegistry, LdtkPatterns, LdtkTocs,
        },
        sprite::{AtlasRect, NineSliceBorders, SpriteMesh},
    },
    tilemap::map::TilemapStorage,
};

use self::{
    components::{
        EntityIid, GlobalEntity, LdtkLoadedLevel, LdtkTempTransform, LdtkUnloadLayer, LevelIid,
    },
    events::{LdtkEvent, LevelEvent},
    json::{
        definitions::LayerType,
        level::{LayerInstance, Level},
        LdtkJson, WorldLayout,
    },
    layer::{LdtkLayers, PackedLdtkEntity},
    resources::{LdtkLevelManager, LdtkLoadConfig},
    sprite::LdtkEntityMaterial,
    traits::{LdtkEntityRegistry, LdtkEntityTagRegistry},
};

pub mod app_ext;
pub mod components;
pub mod events;
pub mod json;
pub mod layer;
pub mod resources;
pub mod sprite;
pub mod traits;

pub const ENTITY_SPRITE_SHADER: Handle<Shader> = Handle::weak_from_u128(89874656485416351634163551);

pub struct EntiTilesLdtkPlugin;

impl Plugin for EntiTilesLdtkPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            ENTITY_SPRITE_SHADER,
            "entity_sprite.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(Material2dPlugin::<LdtkEntityMaterial>::default());

        app.add_systems(Startup, parse_ldtk_json);
        app.add_systems(
            Update,
            (
                load_ldtk_json,
                unload_ldtk_level,
                unload_ldtk_layer,
                global_entity_registerer,
                ldtk_temp_tranform_applier,
            ),
        );

        app.insert_non_send_resource(LdtkEntityRegistry::default());

        app.init_resource::<LdtkLevelManager>()
            .init_resource::<LdtkLoadConfig>()
            .init_resource::<LdtkAdditionalLayers>()
            .init_resource::<LdtkAssets>()
            .init_resource::<LdtkPatterns>()
            .init_resource::<LdtkTocs>()
            .init_resource::<LdtkGlobalEntityRegistry>();

        app.add_event::<LdtkEvent>();

        app.register_type::<LdtkLoadedLevel>()
            .register_type::<GlobalEntity>()
            .register_type::<EntityIid>()
            .register_type::<LayerIid>()
            .register_type::<LevelIid>()
            .register_type::<WorldIid>()
            .register_type::<LevelEvent>()
            .register_type::<LdtkLoader>()
            .register_type::<LdtkUnloader>()
            .register_type::<LdtkLoaderMode>()
            .register_type::<AtlasRect>()
            .register_type::<LdtkEntityMaterial>()
            .register_type::<NineSliceBorders>()
            .register_type::<SpriteMesh>();

        app.register_type::<FieldInstance>()
            .register_type::<Level>()
            .register_type::<ImagePosition>()
            .register_type::<Neighbour>()
            .register_type::<LayerInstance>()
            .register_type::<TileInstance>()
            .register_type::<EntityInstance>()
            .register_type::<LdtkColor>()
            .register_type::<LdtkJson>()
            .register_type::<Toc>()
            .register_type::<World>()
            .register_type::<EntityRef>()
            .register_type::<GridPoint>();

        app.register_type::<LdtkLevelManager>()
            .register_type::<LdtkLoadConfig>()
            .register_type::<LdtkAdditionalLayers>()
            .register_type::<LdtkAssets>()
            .register_type::<LdtkPatterns>()
            .register_type::<LdtkGlobalEntityRegistry>();

        #[cfg(feature = "algorithm")]
        {
            app.init_resource::<resources::LdtkWfcManager>();

            app.register_type::<resources::LdtkWfcManager>();
        }

        #[cfg(feature = "physics")]
        {
            app.register_type::<layer::physics::LdtkPhysicsLayer>();
        }
    }
}

fn parse_ldtk_json(mut manager: ResMut<LdtkLevelManager>, config: Res<LdtkLoadConfig>) {
    manager.reload_json(&config);
}

fn global_entity_registerer(
    mut registry: ResMut<LdtkGlobalEntityRegistry>,
    query: Query<(Entity, &EntityIid), Added<GlobalEntity>>,
) {
    query.iter().for_each(|(entity, iid)| {
        registry.register(iid.clone(), entity);
    });
}

fn ldtk_temp_tranform_applier(
    commands: ParallelCommands,
    mut entities_query: Query<(Entity, &mut Transform, &LdtkTempTransform)>,
) {
    entities_query
        .par_iter_mut()
        .for_each(|(entity, mut transform, ldtk_temp)| {
            transform.translation += ldtk_temp.level_translation.extend(ldtk_temp.z_index);
            commands.command_scope(|mut c| {
                c.entity(entity).remove::<LdtkTempTransform>();
            });
        });
}

pub fn unload_ldtk_level(
    mut commands: Commands,
    mut query: Query<(Entity, &LdtkLoadedLevel, &LevelIid), With<LdtkUnloader>>,
    mut ldtk_events: EventWriter<LdtkEvent>,
    global_entities: Res<LdtkGlobalEntityRegistry>,
) {
    query.iter_mut().for_each(|(entity, level, iid)| {
        ldtk_events.send(LdtkEvent::LevelUnloaded(LevelEvent {
            identifier: level.identifier.clone(),
            iid: iid.0.clone(),
        }));
        level.unload(&mut commands, &global_entities);
        commands.entity(entity).despawn();
    });
}

#[cfg(not(feature = "physics"))]
pub fn unload_ldtk_layer(
    mut commands: Commands,
    mut query: Query<&mut TilemapStorage, With<LdtkUnloadLayer>>,
) {
    query.iter_mut().for_each(|mut storage| {
        storage.despawn(&mut commands);
    });
}

#[cfg(feature = "physics")]
pub fn unload_ldtk_layer(
    mut commands: Commands,
    mut query: Query<
        (
            &mut TilemapStorage,
            Option<&mut crate::tilemap::physics::PhysicsTilemap>,
        ),
        With<LdtkUnloadLayer>,
    >,
) {
    query.iter_mut().for_each(|(mut storage, physics)| {
        if let Some(mut physics) = physics {
            physics.remove_all(&mut commands);
        }
        storage.despawn(&mut commands);
    });
}

pub fn load_ldtk_json(
    mut commands: Commands,
    loader_query: Query<(Entity, &LdtkLoader)>,
    asset_server: Res<AssetServer>,
    entity_registry: Option<NonSend<LdtkEntityRegistry>>,
    entity_tag_registry: Option<NonSend<LdtkEntityTagRegistry>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut ldtk_events: EventWriter<LdtkEvent>,
    config: Res<LdtkLoadConfig>,
    mut manager: ResMut<LdtkLevelManager>,
    addi_layers: Res<LdtkAdditionalLayers>,
    mut ldtk_assets: ResMut<LdtkAssets>,
    mut entity_material_assets: ResMut<Assets<LdtkEntityMaterial>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut patterns: ResMut<LdtkPatterns>,
    global_entities: Res<LdtkGlobalEntityRegistry>,
) {
    for (entity, loader) in loader_query.iter() {
        let entity_registry = entity_registry.as_ref().map(|r| &**r);
        let entity_tag_registry = entity_tag_registry.as_ref().map(|r| &**r);

        ldtk_assets.initialize(
            &config,
            &manager,
            &asset_server,
            &mut atlas_layouts,
            &mut entity_material_assets,
            &mut mesh_assets,
        );

        load_levels(
            &mut commands,
            &config,
            &mut manager,
            &addi_layers,
            loader,
            &asset_server,
            &entity_registry.unwrap_or(&LdtkEntityRegistry::default()),
            &entity_tag_registry.unwrap_or(&LdtkEntityTagRegistry::default()),
            entity,
            &mut ldtk_events,
            &mut ldtk_assets,
            &mut patterns,
            &global_entities,
        );

        commands.entity(entity).remove::<LdtkLoader>();
    }
}

fn load_levels(
    commands: &mut Commands,
    config: &LdtkLoadConfig,
    manager: &mut LdtkLevelManager,
    addi_layers: &LdtkAdditionalLayers,
    loader: &LdtkLoader,
    asset_server: &AssetServer,
    entity_registry: &LdtkEntityRegistry,
    entity_tag_registry: &LdtkEntityTagRegistry,
    level_entity: Entity,
    ldtk_events: &mut EventWriter<LdtkEvent>,
    ldtk_assets: &mut LdtkAssets,
    patterns: &mut LdtkPatterns,
    global_entities: &LdtkGlobalEntityRegistry,
) {
    let ldtk_data = manager.get_cached_data();

    let Some((level_index, level)) = ldtk_data
        .levels
        .iter()
        .enumerate()
        .find(|(_, level)| level.identifier == loader.level)
    else {
        return;
    };

    let translation = loader
        .trans_ovrd
        .unwrap_or_else(|| get_level_translation(&ldtk_data, level_index));

    let level_px = UVec2 {
        x: level.px_wid as u32,
        y: level.px_hei as u32,
    };

    let background = load_background(level, translation, level_px, asset_server, config);

    let mut ldtk_layers = LdtkLayers::new(
        level_entity,
        level.layer_instances.len(),
        &ldtk_assets,
        translation,
        config.z_index,
        loader.mode,
        background,
    );

    for (layer_index, layer) in level.layer_instances.iter().enumerate() {
        #[cfg(feature = "algorithm")]
        if let Some(path) = addi_layers.path_layer.as_ref() {
            if layer.identifier == path.identifier {
                ldtk_layers
                    .assign_path_layer(path.clone(), layer::path::analyze_path_layer(layer, path));
                continue;
            }
        }

        #[cfg(feature = "physics")]
        if let Some(phy) = addi_layers.physics_layer.as_ref() {
            if layer.identifier == phy.identifier {
                ldtk_layers.assign_physics_layer(
                    phy.clone(),
                    layer.int_grid_csv.clone(),
                    UVec2 {
                        x: layer.c_wid as u32,
                        y: layer.c_hei as u32,
                    },
                );
                continue;
            }
        }

        load_layer(
            layer_index,
            layer,
            &mut ldtk_layers,
            translation,
            config,
            &global_entities,
            patterns,
            loader,
        );
    }

    ldtk_layers.apply_all(
        commands,
        patterns,
        level,
        entity_registry,
        entity_tag_registry,
        config,
        ldtk_assets,
        asset_server,
    );

    ldtk_events.send(LdtkEvent::LevelLoaded(LevelEvent {
        identifier: level.identifier.clone(),
        iid: level.iid.clone(),
    }));
}

fn load_background(
    level: &Level,
    translation: Vec2,
    level_px: UVec2,
    asset_server: &AssetServer,
    config: &LdtkLoadConfig,
) -> SpriteBundle {
    let texture = level
        .bg_rel_path
        .as_ref()
        .map(|path| asset_server.load(Path::new(&config.asset_path_prefix).join(path)));

    SpriteBundle {
        sprite: Sprite {
            color: level.bg_color.into(),
            custom_size: Some(level_px.as_vec2()),
            ..Default::default()
        },
        texture: texture.unwrap_or_default(),
        transform: Transform::from_xyz(
            level_px.x as f32 / 2. + translation.x,
            -(level_px.y as f32) / 2. + translation.y,
            config.z_index as f32 - level.layer_instances.len() as f32 - 1.,
        ),
        ..Default::default()
    }
}

fn load_layer(
    layer_index: usize,
    layer: &LayerInstance,
    ldtk_layers: &mut LdtkLayers,
    translation: Vec2,
    config: &LdtkLoadConfig,
    global_entities: &LdtkGlobalEntityRegistry,
    patterns: &LdtkPatterns,
    loader: &LdtkLoader,
) {
    match layer.ty {
        LayerType::IntGrid | LayerType::AutoLayer => {
            layer.auto_layer_tiles.iter().for_each(|tile| {
                ldtk_layers.set_tile(layer_index, layer, tile, config, patterns, &loader.mode);
            });
        }
        LayerType::Entities => {
            for (order, entity_instance) in layer.entity_instances.iter().enumerate() {
                let iid = EntityIid(entity_instance.iid.clone());
                if global_entities.contains(&iid) {
                    continue;
                }

                let fields = entity_instance
                    .field_instances
                    .iter()
                    .map(|field| (field.identifier.clone(), field.clone()))
                    .collect();
                let packed_entity = PackedLdtkEntity {
                    instance: entity_instance.clone(),
                    fields,
                    iid,
                    transform: LdtkTempTransform {
                        level_translation: translation,
                        z_index: config.z_index as f32
                            - layer_index as f32
                            - (1. - (order as f32 / layer.entity_instances.len() as f32)),
                    },
                };
                ldtk_layers.set_entity(packed_entity);
            }
        }
        LayerType::Tiles => {
            layer.grid_tiles.iter().for_each(|tile| {
                ldtk_layers.set_tile(layer_index, layer, tile, config, patterns, &loader.mode);
            });
        }
    }
}

fn get_level_translation(ldtk_data: &LdtkJson, index: usize) -> Vec2 {
    let level = &ldtk_data.levels[index];
    match ldtk_data.world_layout.unwrap() {
        WorldLayout::GridVania | WorldLayout::Free => Vec2 {
            x: level.world_x as f32,
            y: -level.world_y as f32,
        },
        WorldLayout::LinearHorizontal | WorldLayout::LinearVertical => Vec2::ZERO,
    }
}
