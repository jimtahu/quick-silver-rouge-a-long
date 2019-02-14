use quicksilver::{
    geom::{Shape, Vector},
    graphics::{Background::Img, Color, Font, FontStyle, Image},
    lifecycle::{run, Asset, Settings, State, Window},
    Future, Result
};

struct Game {
    title: Asset<Image>,
    square_font_info: Asset<Image>,
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
       Ok(Self{ title, square_font_info, })
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
