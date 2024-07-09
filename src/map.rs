// kknd2-mapview
// Copyright (c) 2024 Matthew Costa <ucosty@gmail.com>
//
// SPDX-License-Identifier: MIT

use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;
use crate::decompress::decompress;
use crate::unpack;
use crate::unpack::{FileEntry, unpack};

const DATA_HEADER_SIZE: u32 = 8;

struct Colour {
    r: u8,
    g: u8,
    b: u8,
}

pub struct Tile {
    pub pixels: Vec<u8>,
}

pub struct MapLayer {
    pub map_width: u32,
    pub map_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tile_map: Vec<u32>,
    pub tiles: HashMap<u32, Tile>,
}

pub struct Map {
    pub layers: Vec<MapLayer>,
}

fn read_raw_tile<R: Read + Seek>(
    reader: &mut BufReader<R>,
    offset: u64,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let saved_stream_position = reader.stream_position()?;
    reader.seek(SeekFrom::Start(offset))?;

    let size = (width * height) as usize;
    let mut buffer = Vec::<u8>::with_capacity(size);
    buffer.resize(size, 0);
    reader.read_exact(buffer.as_mut_slice())?;
    reader.seek(SeekFrom::Start(saved_stream_position))?;
    Ok(buffer)
}

fn create_tile_from_raw(data: &Vec<u8>, palette: &Vec<Colour>) -> Result<Tile, Box<dyn Error>> {
    let mut pixels = Vec::<u8>::with_capacity(data.len());

    for i in 0..data.len() {
        let palette_index = data[i] as usize;

        if palette_index == 0 {
            pixels.push(0);
            pixels.push(0);
            pixels.push(0);
            pixels.push(0);
            continue;
        }

        pixels.push(palette[palette_index].r);
        pixels.push(palette[palette_index].g);
        pixels.push(palette[palette_index].b);
        pixels.push(0xff);
    }

    Ok(Tile { pixels })
}

fn read_layer<R: Read + Seek>(
    reader: &mut BufReader<R>,
    file_offsets: u32,
    palette: &Vec<Colour>,
) -> Result<MapLayer, Box<dyn Error>> {
    let tile_width = reader.read_u32::<LittleEndian>()?;
    let tile_height = reader.read_u32::<LittleEndian>()?;
    let map_width = reader.read_u32::<LittleEndian>()?;
    let map_height = reader.read_u32::<LittleEndian>()?;

    // Skip some unknown data
    // FIXME: not unknown now
    // it is layer_width_pixels, layer_height_pixels, then something unknown
    reader.seek_relative(12)?;

    let map_size = (map_width * map_height) as usize;
    let mut tile_map: Vec<u32> = Vec::with_capacity(map_size);

    let mut tiles = HashMap::<u32, Tile>::new();

    for _i in 0..map_size {
        let tile_id = reader.read_u32::<LittleEndian>()?;
        tile_map.push(tile_id - (tile_id % 4));

        let offset = tile_id - (tile_id % 4);

        if offset == 0 {
            continue;
        }

        if !tiles.contains_key(&offset) {
            let raw_tile = read_raw_tile(
                &mut *reader,
                (offset + DATA_HEADER_SIZE - file_offsets) as u64,
                tile_width,
                tile_height,
            )?;
            let tile = create_tile_from_raw(&raw_tile, &palette)?;
            tiles.insert(offset, tile);
        }
    }

    Ok(MapLayer {
        map_width,
        map_height,
        tile_width,
        tile_height,
        tile_map,
        tiles,
    })
}

pub fn parse_map<R: Read + Seek>(
    reader: &mut BufReader<R>,
    file_offsets: u32,
) -> Result<Map, Box<dyn Error>> {
    // Skip some unknown data (probably a version number)
    reader.seek_relative(4)?;
    let layers = reader.read_u32::<LittleEndian>()?;

    let mut layer_offsets = Vec::<u64>::new();
    for _i in 0..layers {
        let layer_offset = reader.read_u32::<LittleEndian>()?;
        layer_offsets.push(layer_offset as u64);
    }

    let palette_size = reader.read_u32::<LittleEndian>()?;

    let mut palette: Vec<Colour> = Vec::with_capacity(palette_size as usize);
    for _i in 0..palette_size as usize {
        let colour_packed = reader.read_u16::<LittleEndian>()?;
        let colour = Colour {
            r: (((colour_packed & 0x7c00) >> 7) & 0xff) as u8,
            g: (((colour_packed & 0x03e0) >> 2) & 0xff) as u8,
            b: (((colour_packed & 0x001f) << 3) & 0xff) as u8,
        };
        palette.push(colour);
    }

    let mut map_layers = Vec::<MapLayer>::new();

    for i in 0..layers as usize {
        reader.seek(SeekFrom::Start(layer_offsets[i] + DATA_HEADER_SIZE as u64 - file_offsets as u64))?;

        let layer_magic = reader.read_u32::<LittleEndian>()?;
        if layer_magic != 0x5343524c {
            return Err(format!("Layer {}: Invalid magic {:#x} at offset {:?}", i, layer_magic, reader.stream_position()).into());
        }

        let layer = read_layer(&mut *reader, file_offsets, &palette)?;
        map_layers.push(layer);
    }

    Ok(Map { layers: map_layers })
}

pub fn load_map(path: &PathBuf) -> Result<Map, Box<dyn Error>> {
    let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader = BufReader::new(file);

    let magic = reader.read_u32::<LittleEndian>()?;

    match magic {
        0xdeadc0de => {
            let file_offsets = reader.read_u32::<LittleEndian>()?;
            parse_map(&mut reader, file_offsets)
        }
        _ => {
            let decompressed_data = decompress(&path)?;
            let files = unpack(&decompressed_data.archive)?;

            let mut map_file: Option<FileEntry> = Option::None;

            for file in files {
                if file.kind == 0x4450414D {
                    map_file = Option::from(file);
                    break;
                }
            }

            match map_file {
                None => Err(format!("No MAPD data found in file: {:?}", path).into()),
                Some(entry) => {
                    let mut padding = Vec::<u8>::new();
                    padding.resize(8, 0);

                    let data = [padding,
                        unpack::extract_file(&decompressed_data.archive, &entry)?].concat();

                    let cursor = Cursor::new(data);
                    let mut cursor_reader = BufReader::new(cursor);

                    cursor_reader.seek_relative(8)?;
                    parse_map(&mut cursor_reader, entry.offset)
                }
            }
        }
    }
}
