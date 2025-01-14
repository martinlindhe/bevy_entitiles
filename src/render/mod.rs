use bevy::{
    app::{App, Update},
    asset::load_internal_asset,
    prelude::{Handle, Plugin, Shader},
    render::{
        mesh::MeshVertexAttribute, render_resource::VertexFormat, ExtractSchedule, RenderApp,
    },
};

use crate::render::{
    binding::TilemapBindGroupLayouts,
    buffer::TilemapStorageBuffers,
    chunk::{ChunkUnload, RenderChunkStorage, UnloadRenderChunk},
    culling::FrustumCulling,
    material::StandardTilemapMaterialSingleton,
    texture::TilemapTexturesStorage,
};

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
pub struct EntiTilesRendererPlugin;

impl Plugin for EntiTilesRendererPlugin {
    fn build(&self, app: &mut App) {
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

        app.register_type::<UnloadRenderChunk>();
        app.add_event::<ChunkUnload>();

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app.add_systems(
            ExtractSchedule,
            (
                extract::extract_tilemaps,
                extract::extract_tiles,
                extract::extract_view,
                extract::extract_unloaded_chunks,
                extract::extract_resources,
                extract::extract_despawned_tilemaps,
                extract::extract_despawned_tiles,
            ),
        );

        render_app
            .init_resource::<TilemapTexturesStorage>()
            .init_resource::<TilemapStorageBuffers>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        render_app.init_resource::<TilemapBindGroupLayouts>();
    }
}
