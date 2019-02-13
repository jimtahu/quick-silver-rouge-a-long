use quicksilver::{
    geom::Vector,
    lifecycle::{run, Settings, State, Window},
    Result
};

struct Game;

impl State for Game {
   ///Init game / load assests
   fn new() -> Result<Self> {
       Ok(Self)
   }

   /// Process input / update game state
   fn update(&mut self, window: &mut Window) -> Result<()> {
       Ok(())
   }
   /// Draw stuff
   fn draw(&mut self, window: &mut Window) -> Result<()> {
       Ok(())
   }
}

fn main() {
    let settings = Settings {
        ..Default::default()
    };
    run::<Game>( "QuickSilver Rougelike", Vector::new(800,600), settings );
}
