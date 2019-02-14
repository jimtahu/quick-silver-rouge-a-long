use quicksilver::{
    geom::{Shape, Vector},
    graphics::{Background::Img, Color, Font, FontStyle, Image},
    lifecycle::{run, Asset, Settings, State, Window},
    Future, Result
};

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
                color: Color::BLACK,
            };
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
}

impl State for Game {
   ///Init game / load assests
   fn new() -> Result<Self> {
       let font_square = "square.ttf";
       let title = Asset::new(Font::load(font_square).and_then(|font| {
           font.render("QuickSilver Roguelike", &FontStyle::new(20.0, Color::BLACK))
       }));
       let square_font_info = Asset::new(Font::load(font_square).and_then(|font| {
           font.render("Square font by Wouter Van Oortmerssen, terms: CC BY 3.0", &FontStyle::new(12.0, Color::BLACK))
       }));
       let game_glyphs = "#@g.%";

       let map_size = Vector::new( 40, 40 );
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
       })
   }

   /// Process input / update game state
   fn update(&mut self, window: &mut Window) -> Result<()> {
       Ok(())
   }
   /// Draw stuff
   fn draw(&mut self, window: &mut Window) -> Result<()> {
       window.clear(Color::WHITE)?;

       self.title.execute(|image| {
           window.draw(
               &image.area().with_center((window.screen_size().x as i32 / 2, 40)),
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

       Ok(())
   }
}

fn main() {
    std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");
    let settings = Settings {
        scale: quicksilver::graphics::ImageScaleStrategy::Blur,
        ..Default::default()
    };
    run::<Game>( "QuickSilver Rougelike", Vector::new(800,600), settings );
}
