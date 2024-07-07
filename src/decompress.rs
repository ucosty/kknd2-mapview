// kknd2-mapview
// Copyright (c) 2024 Matthew Costa <ucosty@gmail.com>
//
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Seek, Write};
use std::path::PathBuf;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16, Box<dyn Error>> {
    Ok(u16::from_le_bytes(buffer[offset..offset + 2].try_into()?))
}

fn decompress_data(input: &[u8], output_size: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut input_cursor: usize = 0;
    let mut output_cursor: usize = 0;
    let mut counter: u32 = 0;
    let mut code_bits = 0;
    let mut output: Vec<u8> = vec![0; output_size];

    while input_cursor < input.len() {
        if counter == 0 {
            let word = read_u16(input, input_cursor)?;

            code_bits = code_bits & !0xffff | word;
            input_cursor += 2;
            counter = 16;
        }

        if (code_bits & 1) == 1 {
            let source_copy_cursor =
                ((((input[input_cursor] as u16) << 4) & !0xff) | (input[input_cursor + 1] as u16)) as usize;

            let pattern_size = (input[input_cursor] & 0x0f) + 1;
            if source_copy_cursor > output_cursor {
                return Err("source_copy_cursor > output_cursor".into());
            }
            let mut copy_cursor = output_cursor - source_copy_cursor;

            for _ in 0..pattern_size {
                output[output_cursor] = output[copy_cursor];
                output_cursor += 1;
                copy_cursor += 1;
            }

            input_cursor += 2;
        } else {
            if output_cursor >= output.len() || input_cursor >= input.len() {
                return Err("Index out of bounds".into());
            }

            output[output_cursor] = input[input_cursor];
            output_cursor += 1;
            input_cursor += 1;
        }

        code_bits >>= 1;
        counter -= 1;
    }

    Ok(output)
}

fn decompress_block(output_size: usize, input: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    if output_size == input.len() {
        return Ok(input.clone());
    }

    decompress_data(input, output_size)
}

fn decompress_part<R: Read>(
    reader: &mut BufReader<R>,
    big_endian: bool,
) -> Result<Vec<u8>, Box<dyn Error>>
where
    R: Seek,
{
    let mut decompressed_bytes: u32 = 0;

    let part_uncompressed_size = if big_endian {
        reader.read_u32::<BigEndian>()?
    } else {
        reader.read_u32::<LittleEndian>()?
    };

    reader.seek_relative(4)?;

    let mut output: Vec<u8> = vec![];

    while decompressed_bytes < part_uncompressed_size {
        let chunk_uncompressed_size = reader.read_u32::<LittleEndian>()?;
        let chunk_compressed_size = reader.read_u32::<LittleEndian>()?;

        let mut chunk_buffer: Vec<u8> = vec![0; chunk_compressed_size as usize];
        reader.read_exact(&mut chunk_buffer)?;

        let decompressed_chunk = decompress_block(chunk_uncompressed_size as usize, &chunk_buffer)?;

        decompressed_bytes += output.write(decompressed_chunk.as_slice())? as u32;
    }

    Ok(output)
}

pub struct DecompressedFile {
    pub archive: Vec<u8>,
    pub _metadata: Vec<u8>,
}

pub fn decompress(path: &PathBuf) -> Result<DecompressedFile, Box<dyn Error>> {
    let file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;

    let mut reader = BufReader::new(file);

    let _magic = reader.read_u32::<LittleEndian>()?;
    reader.seek_relative(4)?;

    let archive = decompress_part(&mut reader, true)?;
    let _metadata = decompress_part(&mut reader, false)?;

    Ok(DecompressedFile{ archive, _metadata })
}
