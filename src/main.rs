// kknd2-mapview
// Copyright (c) 2024 Matthew Costa <ucosty@gmail.com>
//
// SPDX-License-Identifier: MIT

use std::error::Error;

use speedy2d::dimen::UVec2;
use speedy2d::font::Font;
use speedy2d::window::{WindowCreationOptions, WindowPosition, WindowSize};
use speedy2d::Window;

use crate::viewer::{MapView, MapViewEvent};

mod map;
mod viewer;
mod decompress;
mod unpack;

fn main() -> Result<(), Box<dyn Error>> {
    // Enforce x11 mode for now
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    // Load the font
    let bytes = include_bytes!("../assets/NotoSans-Regular.ttf");
    let font = Font::new(bytes).unwrap();

    let window = Window::<MapViewEvent>::new_with_user_events(
        "KKnD 2 Map Viewer",
        WindowCreationOptions::new_windowed(
            WindowSize::PhysicalPixels(UVec2::from((1024, 768))),
            Option::from(WindowPosition::Center),
        ),
    )?;

    let event_sender = window.create_user_event_sender();

    let map_view = MapView::new(font, event_sender);

    window.run_loop(map_view)
}
