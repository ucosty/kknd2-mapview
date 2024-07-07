// kknd2-mapview
// Copyright (c) 2024 Matthew Costa <ucosty@gmail.com>
//
// SPDX-License-Identifier: MIT

use std::error::Error;
use std::mem::size_of;
use std::usize;

struct TableEntry {
    pub kind: u32,
    pub table_offset: u32,
}

#[derive(Debug)]
pub struct FileEntry {
    pub kind: u32,
    pub offset: u32,
    pub size: u32,
}

fn parse_table_of_contents_entry(data: &[u8]) -> Result<TableEntry, Box<dyn Error>> {
    Ok(TableEntry {
        kind: u32::from_le_bytes(data[0..4].try_into()?),
        table_offset: u32::from_le_bytes(data[4..8].try_into()?),
    })
}

fn get_file_offset(data: &[u8], entry: u32) -> Result<u32, Box<dyn Error>> {
    let entry_offset = (entry * 4) as usize;
    Ok(u32::from_le_bytes(data[entry_offset..entry_offset + 4].try_into()?))
}

pub fn unpack(archive_data: &Vec<u8>) -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let mut files: Vec<FileEntry> = Vec::new();

    let table_of_contents_offset = u32::from_le_bytes(archive_data[0..4].try_into()?);
    let table_entry_size = size_of::<TableEntry>() as u32;
    for i in 0..7 {
        let entry_offset = table_of_contents_offset + (i * table_entry_size);
        let entry = parse_table_of_contents_entry(&archive_data[entry_offset as usize..])?;

        if entry.kind == 0 {
            break;
        }

        let next_entry_offset = entry_offset + table_entry_size;
        let next_entry = parse_table_of_contents_entry(&archive_data[next_entry_offset as usize..])?;
        let entry_end_offset = if next_entry.table_offset == 0 {
            table_of_contents_offset
        } else {
            next_entry.table_offset
        };

        let file_table_size = entry_end_offset - entry.table_offset;

        for j in 0..file_table_size / 4 {
            let offset = get_file_offset(&archive_data[entry.table_offset as usize..], j as u32)?;
            if offset == 0 {
                break;
            }

            files.push(FileEntry { kind: entry.kind, offset, size: 0 });
        }
    }

    for i in 0..files.len() {
        files[i].size = if i == files.len() - 1 {
             table_of_contents_offset - files[i].offset
        } else {
            files[i + 1].offset - files[i].offset
        }
    }

    Ok(files)
}

pub fn extract_file(archive_data: &Vec<u8>, entry: &FileEntry) -> Result<Vec<u8>, Box<dyn Error>> {
    let start  = entry.offset as usize;
    let end = start + entry.size as usize;

    Ok(archive_data[start..end].to_vec())
}
