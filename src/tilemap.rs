// src/tilemap.rs

use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::camera::Camera;
use crate::tile_properties::TileTable;

pub const TILE_SIZE: u32 = 16;
pub const TILE_SCALE: u32 = 3;
pub const TILE_DRAW_SIZE: u32 = TILE_SIZE * TILE_SCALE;

/// Type des indices de tuiles — u32 pour supporter 256+ tuiles
pub type TileIndex = u32;

/// Valeur sentinelle : case vide dans Tiled (gid=0)
pub const EMPTY_TILE: TileIndex = u32::MAX;

pub struct Tilemap {
    pub tiles: Vec<Vec<TileIndex>>,
    pub rows: usize,
    pub cols: usize,
}

impl Tilemap {
    pub fn new(tiles: Vec<Vec<TileIndex>>) -> Self {
        let rows = tiles.len();
        let cols = if rows > 0 { tiles[0].len() } else { 0 };
        Tilemap { tiles, rows, cols }
    }

    pub fn pixel_width(&self) -> f32 {
        (self.cols as u32 * TILE_DRAW_SIZE) as f32
    }

    pub fn pixel_height(&self) -> f32 {
        (self.rows as u32 * TILE_DRAW_SIZE) as f32
    }

    pub fn get_tile(&self, col: i32, row: i32) -> Option<TileIndex> {
        if col < 0 || row < 0
            || row >= self.rows as i32
            || col >= self.cols as i32
        {
            return None;
        }
        Some(self.tiles[row as usize][col as usize])
    }

    pub fn is_solid(&self, col: i32, row: i32, table: &TileTable) -> bool {
        match self.get_tile(col, row) {
            None                  => true,
            Some(EMPTY_TILE)      => false, // case vide = traversable
            Some(idx)             => table.is_blocking(idx),
        }
    }

    pub fn is_solid_at(&self, world_x: f32, world_y: f32, table: &TileTable) -> bool {
        let col = (world_x / TILE_DRAW_SIZE as f32).floor() as i32;
        let row = (world_y / TILE_DRAW_SIZE as f32).floor() as i32;
        self.is_solid(col, row, table)
    }

    fn src_rect(&self, tile_index: TileIndex, tiles_per_row: u32) -> Rect {
        let col_in_sheet = tile_index % tiles_per_row;
        let row_in_sheet = tile_index / tiles_per_row;
        Rect::new(
            (col_in_sheet * TILE_SIZE) as i32,
            (row_in_sheet * TILE_SIZE) as i32,
            TILE_SIZE,
            TILE_SIZE,
        )
    }

    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        tileset: &Texture,
        camera: &Camera,
        tiles_per_row: u32,
    ) -> Result<(), String> {
        let start_col = (camera.x / TILE_DRAW_SIZE as f32).floor() as i32;
        let start_row = (camera.y / TILE_DRAW_SIZE as f32).floor() as i32;
        let end_col   = start_col + (camera.width  / TILE_DRAW_SIZE) as i32 + 2;
        let end_row   = start_row + (camera.height / TILE_DRAW_SIZE) as i32 + 2;

        for row in start_row..end_row {
            for col in start_col..end_col {
                if let Some(tile_index) = self.get_tile(col, row) {
                    if tile_index == EMPTY_TILE { continue; }

                    let src = self.src_rect(tile_index, tiles_per_row);
                    let (sx, sy) = camera.world_to_screen(
                        (col as u32 * TILE_DRAW_SIZE) as f32,
                        (row as u32 * TILE_DRAW_SIZE) as f32,
                    );
                    canvas.copy(
                        tileset,
                        Some(src),
                        Some(Rect::new(sx, sy, TILE_DRAW_SIZE, TILE_DRAW_SIZE)),
                    )?;
                }
            }
        }
        Ok(())
    }
}