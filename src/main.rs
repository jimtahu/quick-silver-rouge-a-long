use quicksilver::{
    geom::{ Rectangle, Shape, Vector},
    graphics::{Background::Blended, Background::Img, Color, Font, FontStyle, Image},
    input::Key,
    lifecycle::{run, Asset, Settings, State, Window},
    Future, Result
};

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct Tile {
    pos: Vector,
    glyph: char,
    color: Color,
}

#[derive(Clone, Debug, PartialEq)]
struct Entity {
    pos: Vector,
    glyph: char,
    color: Color,
    hp: i32,
    max_hp: i32,
}

fn generate_entities( ) -> Vec<Entity> {
    vec![
        Entity {
            pos: Vector::new(9, 6),
            glyph: 'g',
            color: Color::RED,
            hp: 1,
            max_hp: 1,
        },
        Entity {
            pos: Vector::new(2, 4),
            glyph: 'g',
            color: Color::RED,
            hp: 1,
            max_hp: 1,
        },
        Entity {
            pos: Vector::new(7, 5),
            glyph: '%',
            color: Color::PURPLE,
            hp: 0,
            max_hp: 0,
        },
        Entity {
            pos: Vector::new(4, 8),
            glyph: '%',
            color: Color::PURPLE,
            hp: 0,
            max_hp: 0,
        },
    ]
}

fn generate_map( size: Vector ) -> Vec<Tile> {
    let width = size.x as usize;
    let height = size.y as usize;
    let mut map = Vec::with_capacity( width * height );
    for x in 0..width {
        for y in 0..height {
            let mut tile = Tile {
                pos: Vector::new( x as f32, y as f32 ),
                glyph: '.',
                color: Color::WHITE,
            };
            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                tile.glyph = '#';
            }
            map.push(tile);
        }
    }
    map
}

struct Game {
    title: Asset<Image>,
    square_font_info: Asset<Image>,
    map_size: Vector,
    map: Vec<Tile>,
    entities: Vec<Entity>,
    player_id: usize,
    tileset: Asset<HashMap<char, Image>>,
    tile_size_px: Vector,
}

impl State for Game {
   ///Init game / load assests
   fn new() -> Result<Self> {
       let font_square = "square.ttf";
       let title = Asset::new(Font::load(font_square).and_then(|font| {
           font.render("QuickSilver Roguelike", &FontStyle::new(20.0, Color::RED))
       }));
       let square_font_info = Asset::new(Font::load(font_square).and_then(|font| {
           font.render("Square font by Wouter Van Oortmerssen, terms: CC BY 3.0", &FontStyle::new(12.0, Color::GREEN))
       }));
       let game_glyphs = "#@g.%";
       let tile_size_px = Vector::new(24, 24);
       let tileset = Asset::new(Font::load(font_square).and_then(move |text| {
           let tiles = text.render( game_glyphs, &FontStyle::new(tile_size_px.y, Color::WHITE) )
               .expect("Could not render the font tileset.");
           let mut tileset = HashMap::new();
           for ( index, glyph ) in game_glyphs.chars().enumerate() {
               let pos = ( index as i32 * tile_size_px.x as i32, 0 );
               let tile = tiles.subimage( Rectangle::new(pos, tile_size_px) );
               tileset.insert( glyph, tile );
           }
           Ok(tileset)
       }));


       let map_size = Vector::new( 20, 20 );
       let map = generate_map( map_size );
       let mut entities = generate_entities();
       let player_id = entities.len();

       entities.push(Entity {
           pos: Vector::new(5, 3),
           glyph: '@',
           color: Color::BLUE,
           hp: 3,
           max_hp: 5,
       });

       Ok(Self{
           title,
           square_font_info,
           map_size,
           map,
           entities,
           player_id,
           tileset,
           tile_size_px,
       })
   }

   /// Process input / update game state
   fn update(&mut self, window: &mut Window) -> Result<()> {
       use quicksilver::input::ButtonState::*;

       let player = &mut self.entities[self.player_id];
       if window.keyboard()[Key::H] == Pressed {
           player.pos.x -= 1.0;
       }
       if window.keyboard()[Key::L] == Pressed {
           player.pos.x += 1.0;
       }
       if window.keyboard()[Key::K] == Pressed {
           player.pos.y -= 1.0;
       }
       if window.keyboard()[Key::J] == Pressed {
           player.pos.y += 1.0;
       }
       if window.keyboard()[Key::Q].is_down() {
           window.close();
       }
       Ok(())
   }
   /// Draw stuff
   fn draw(&mut self, window: &mut Window) -> Result<()> {
       window.clear(Color::BLACK)?;

       self.title.execute(|image| {
           window.draw(
               &image.area().with_center((window.screen_size().x as i32 / 2, 30)),
               Img(&image),
           );
           Ok(())
       })?;

       self.square_font_info.execute(|image| {
           window.draw(
               &image.area().translate((4, window.screen_size().y as i32 - 60)),
               Img(&image),
           );
           Ok(())
       })?;

       let tile_size_px = self.tile_size_px;
       let ( tileset, map ) = (&mut self.tileset, &self.map );
       tileset.execute(|tileset| {
           let offset_px = Vector::new(20, 50);
           for tile in map.iter() {
               if let Some(image) = tileset.get(&tile.glyph) {
                   let pos_px = tile.pos.times(tile_size_px);
                   window.draw(
                       &Rectangle::new(offset_px + pos_px, image.area().size()),
                       Blended( &image, tile.color ),
                   );
               }
           }
           Ok(())
       })?;
       let ( tileset, entities ) = (&mut self.tileset, &self.entities );
       tileset.execute(|tileset| {
           let offset_px = Vector::new(20, 50);
           for entity in entities.iter() {
               if let Some(image) = tileset.get(&entity.glyph) {
                   let pos_px = entity.pos.times(tile_size_px);
                   window.draw(
                       &Rectangle::new(offset_px + pos_px, image.area().size()),
                       Blended( &image, entity.color ),
                   );
               }
           }
           Ok(())
       })?;

       Ok(())
   }
}

fn main() {
    std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");
    let settings = Settings {
        resize: quicksilver::graphics::ResizeStrategy::Fill,
        scale: quicksilver::graphics::ImageScaleStrategy::Blur,
        ..Default::default()
    };
    run::<Game>( "QuickSilver Rougelike", Vector::new(800,600), settings );
}
