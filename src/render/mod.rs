use std::marker::PhantomData;

use bevy::{
    app::Update,
    asset::{load_internal_asset, AssetApp},
    core_pipeline::core_2d::Transparent2d,
    prelude::{Handle, IntoSystemConfigs, Plugin, Shader},
    render::{
        mesh::MeshVertexAttribute,
        render_phase::AddRenderCommand,
        render_resource::{SpecializedRenderPipelines, VertexFormat},
        ExtractSchedule, Render, RenderApp, RenderSet,
    },
};

use crate::render::{
    binding::{TilemapBindGroupLayouts, TilemapBindGroups},
    buffer::{TilemapStorageBuffers, TilemapUniformBuffer},
    chunk::{ChunkUnload, RenderChunkStorage, UnloadRenderChunk},
    culling::FrustumCulling,
    draw::DrawTilemap,
    material::StandardTilemapMaterialSingleton,
    pipeline::EntiTilesPipeline,
    resources::TilemapInstances,
    texture::TilemapTexturesStorage,
};

use self::material::TilemapMaterial;

pub mod binding;
pub mod buffer;
pub mod chunk;
pub mod culling;
pub mod draw;
pub mod extract;
pub mod material;
pub mod pipeline;
pub mod prepare;
pub mod queue;
pub mod resources;
pub mod texture;

pub const SQUARE: Handle<Shader> = Handle::weak_from_u128(54311635145631);
pub const ISOMETRIC: Handle<Shader> = Handle::weak_from_u128(45522415151365135);
pub const HEXAGONAL: Handle<Shader> = Handle::weak_from_u128(341658413214563135);
pub const COMMON: Handle<Shader> = Handle::weak_from_u128(1321023135616351);
pub const TILEMAP_SHADER: Handle<Shader> = Handle::weak_from_u128(89646584153215);

pub const TILEMAP_MESH_ATTR_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("GridIndex", 14513156146, VertexFormat::Sint32x4);
pub const TILEMAP_MESH_ATTR_COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Color", 85415341854, VertexFormat::Float32x4);
pub const TILEMAP_MESH_ATTR_TEX_INDICES: MeshVertexAttribute =
    MeshVertexAttribute::new("TextureIndex", 186541653135, VertexFormat::Sint32x4);
pub const TILEMAP_MESH_ATTR_FLIP: MeshVertexAttribute =
    MeshVertexAttribute::new("Flip", 7365156123161, VertexFormat::Uint32x4);

#[derive(Default)]
pub struct EntiTilesRendererPlugin<M: TilemapMaterial>(PhantomData<M>);

impl<M: TilemapMaterial> Plugin for EntiTilesRendererPlugin<M> {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, SQUARE, "shaders/square.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, ISOMETRIC, "shaders/isometric.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, HEXAGONAL, "shaders/hexagonal.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, COMMON, "shaders/common.wgsl", Shader::from_wgsl);

        load_internal_asset!(
            app,
            TILEMAP_SHADER,
            "shaders/tilemap.wgsl",
            Shader::from_wgsl
        );

        app.add_systems(
            Update,
            (
                culling::cull_tilemaps,
                texture::set_texture_usage,
                material::standard_material_register,
            ),
        );

        app.init_resource::<FrustumCulling>()
            .init_resource::<StandardTilemapMaterialSingleton>();
        app.init_asset::<M>();

        app.register_type::<UnloadRenderChunk>();
        app.add_event::<ChunkUnload>();

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app
            .add_systems(
                ExtractSchedule,
                (
                    extract::extract_tilemaps,
                    extract::extract_changed_tilemaps::<M>,
                    extract::extract_tiles,
                    extract::extract_view,
                    extract::extract_materials::<M>,
                    extract::extract_unloaded_chunks,
                    extract::extract_resources,
                    extract::extract_despawned_tilemaps,
                    extract::extract_despawned_tiles,
                ),
            )
            .add_systems(
                Render,
                (
                    prepare::prepare_tilemaps::<M>,
                    prepare::prepare_tiles::<M>,
                    prepare::prepare_unloaded_chunks::<M>,
                    prepare::prepare_despawned_tilemaps::<M>,
                    prepare::prepare_despawned_tiles::<M>,
                    culling::cull_chunks::<M>,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue::queue::<M>.in_set(RenderSet::Queue));

        render_app
            .init_resource::<RenderChunkStorage<M>>()
            .init_resource::<TilemapTexturesStorage>()
            .init_resource::<TilemapUniformBuffer<M>>()
            .init_resource::<TilemapStorageBuffers>()
            .init_resource::<TilemapBindGroups<M>>()
            .init_resource::<TilemapInstances<M>>();

        render_app.add_render_command::<Transparent2d, DrawTilemap<M>>();
    }

    fn finish(&self, app: &mut bevy::prelude::App) {
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app
            .init_resource::<TilemapBindGroupLayouts>()
            .init_resource::<EntiTilesPipeline<M>>()
            .init_resource::<SpecializedRenderPipelines<EntiTilesPipeline<M>>>();
    }
}
