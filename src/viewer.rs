// kknd2-mapview
// Copyright (c) 2024 Matthew Costa <ucosty@gmail.com>
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::env;

use rfd::FileDialog;
use speedy2d::color::Color;
use speedy2d::font::{Font, TextLayout, TextOptions};
use speedy2d::Graphics2D;
use speedy2d::image::{ImageDataType, ImageHandle, ImageSmoothingMode};
use speedy2d::window::{KeyScancode, UserEventSender, VirtualKeyCode, WindowHandler, WindowHelper};

use crate::map::{load_map, Map};

pub struct MapView {
    tiles: HashMap<u32, ImageHandle>,
    images_loaded: bool,
    map: Option<Map>,
    pan_up: bool,
    pan_down: bool,
    pan_left: bool,
    pan_right: bool,
    offset_x: u32,
    offset_y: u32,
    font: Font,
    event_sender: UserEventSender<MapViewEvent>
}

#[derive(Debug)]
pub enum MapViewEvent {
    OpenMap
}

impl MapView {
    pub fn new(font: Font, event_sender: UserEventSender<MapViewEvent>) -> MapView {
        MapView {
            tiles: Default::default(),

            images_loaded: false,
            map: None,
            pan_up: false,
            pan_down: false,
            pan_left: false,
            pan_right: false,
            offset_x: 0,
            offset_y: 0,
            font,
            event_sender
        }
    }

    fn on_draw_map(&mut self, helper: &mut WindowHelper<MapViewEvent>, graphics: &mut Graphics2D) {
        let map = &mut self.map.as_ref().unwrap();

        if !self.images_loaded {
            for index in map.layers[0].tiles.keys() {
                let data = &map.layers[0].tiles.get(index).unwrap().pixels;
                let tile = graphics
                    .create_image_from_raw_pixels(
                        ImageDataType::RGBA,
                        ImageSmoothingMode::NearestNeighbor,
                        (32, 32),
                        data.as_slice(),
                    )
                    .unwrap();
                self.tiles.insert(*index, tile);
            }

            for index in map.layers[1].tiles.keys() {
                let data = &map.layers[1].tiles.get(index).unwrap().pixels;
                let tile = graphics
                    .create_image_from_raw_pixels(
                        ImageDataType::RGBA,
                        ImageSmoothingMode::NearestNeighbor,
                        (32, 32),
                        data.as_slice(),
                    )
                    .unwrap();
                self.tiles.insert(*index, tile);
            }

            self.images_loaded = true;
        }

        let mut require_redraw = false;

        let window_size = helper.get_size_pixels();

        let tile_width = map.layers[0].tile_width;
        let tile_height = map.layers[0].tile_height;

        let map_width_pixels = map.layers[0].map_width * tile_width;
        let map_height_pixels = map.layers[0].map_height * tile_height;

        // TODO: probably need to figure out the panning speed based on framerate
        let pan_speed = 16;
        if self.pan_up && self.offset_y > pan_speed {
            self.offset_y = self.offset_y - pan_speed;
            require_redraw = true;
        }

        if self.pan_down && (self.offset_y + window_size.y < map_height_pixels) {
            self.offset_y = self.offset_y + pan_speed;
            require_redraw = true;
        }

        if self.pan_left && self.offset_x > pan_speed {
            self.offset_x = self.offset_x - pan_speed;
            require_redraw = true;
        }

        if self.pan_right && (self.offset_x + window_size.x < map_width_pixels) {
            self.offset_x = self.offset_x + pan_speed;
            require_redraw = true;
        }

        // Calculate the starting tile
        let tile_offset_x = self.offset_x / tile_width;
        let tile_offset_y = self.offset_y / tile_height;
        let pixel_offset_x = self.offset_x % tile_width;
        let pixel_offset_y = self.offset_y % tile_height;

        // Calculate the screen width in tiles
        let screen_width_tiles = window_size.x / tile_width;
        let screen_width_tiles = if pixel_offset_x > 0
            && (tile_offset_x + screen_width_tiles < map.layers[0].map_width)
        {
            (window_size.x / tile_width) + 1
        } else {
            window_size.x / tile_width
        };

        let screen_height_tiles = window_size.y / tile_height;
        let screen_height_tiles = if pixel_offset_y > 0
            && (tile_offset_y + screen_height_tiles < map.layers[0].map_height)
        {
            (window_size.y / tile_height) + 1
        } else {
            window_size.y / tile_height
        };

        graphics.clear_screen(Color::BLACK);

        for y in 0..screen_height_tiles {
            for x in 0..screen_width_tiles {
                for l in 0..map.layers.len() {
                    let tile_x = tile_offset_x + x;
                    let tile_y = tile_offset_y + y;

                    let position = (tile_x + (tile_y * map.layers[l].map_width)) as usize;
                    let tile_index = map.layers[l].tile_map[position];

                    let tile_width = map.layers[l].tile_width;
                    let tile_height = map.layers[l].tile_height;

                    if tile_index == 0 {
                        continue;
                    }

                    if let Some(tile) = self.tiles.get(&tile_index) {
                        graphics.draw_image(
                            (
                                (x * tile_width) as f32 - pixel_offset_x as f32,
                                (y * tile_height) as f32 - pixel_offset_y as f32,
                            ),
                            tile,
                        );
                    }
                }
            }
        }

        if require_redraw {
            helper.request_redraw();
        }
    }

    fn on_draw_no_map(&mut self, _helper: &mut WindowHelper<MapViewEvent>, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::from_rgb(0.8, 0.8, 0.8));
        let message = self.font.layout_text("KKnD 2 Map Viewer\nPress 'O' to open a map file\n\nSupports KKnD 2 LPS, LPC, LPM, and extracted MAPD files", 32.0, TextOptions::new());
        graphics.draw_text((50.0, 50.0), Color::BLACK, &message);
    }
}

impl WindowHandler<MapViewEvent> for MapView {
    fn on_user_event(&mut self, _helper: &mut WindowHelper<MapViewEvent>, event: MapViewEvent) {
        match event {
            MapViewEvent::OpenMap => {
                let path = env::current_dir().unwrap();
                let file = FileDialog::new()
                    .add_filter("Level Archives", &["lps", "lpc", "lpm", "MAPD"])
                    .set_directory(path)
                    .pick_file();

                if let Some(path) = file {
                    self.map = Option::from(load_map(&path).unwrap());
                    self.tiles.clear();
                    self.images_loaded = false;
                }
            }
        }
    }

    fn on_draw(&mut self, helper: &mut WindowHelper<MapViewEvent>, graphics: &mut Graphics2D) {
        match self.map {
            None => self.on_draw_no_map(helper, graphics),
            Some(_) => self.on_draw_map(helper, graphics),
        }
    }

    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<MapViewEvent>,
        virtual_key_code: Option<VirtualKeyCode>,
        _scancode: KeyScancode,
    ) {
        if let Some(key) = virtual_key_code {
            match key {
                VirtualKeyCode::Up => self.pan_up = true,
                VirtualKeyCode::Down => self.pan_down = true,
                VirtualKeyCode::Left => self.pan_left = true,
                VirtualKeyCode::Right => self.pan_right = true,
                VirtualKeyCode::O => {
                    self.event_sender.send_event(MapViewEvent::OpenMap).unwrap();
                }
                _ => {}
            }
        }
        helper.request_redraw();
    }

    fn on_key_up(
        &mut self,
        helper: &mut WindowHelper<MapViewEvent>,
        virtual_key_code: Option<VirtualKeyCode>,
        _scancode: KeyScancode,
    ) {
        if let Some(key) = virtual_key_code {
            match key {
                VirtualKeyCode::Up => self.pan_up = false,
                VirtualKeyCode::Down => self.pan_down = false,
                VirtualKeyCode::Left => self.pan_left = false,
                VirtualKeyCode::Right => self.pan_right = false,
                _ => {}
            }
        }
        helper.request_redraw();
    }
}
