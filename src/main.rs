use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::input::Key;
use tcod::input::KeyCode::*;
use tcod::map::{FovAlgorithm, Map as FovMap};

// window size
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 60;
const LIMIT_FPS: i32 = 20;

// map size
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
// object placement
const MAX_ROOM_MONSTERS: i32 = 3;
const PLAYER: usize = 0;
// map colors
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };
// Field of view
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new( x: i32, y: i32, char: char, color: Color ) -> Self {
        Object{ x, y, char, color }
    }

    pub fn move_by( &mut self, dx: i32, dy: i32, game: &Game ) {
        if !game.map[(self.x+dx) as usize][(self.y+dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    pub fn draw( &self, con: &mut dyn Console ) {
        con.set_default_foreground(self.color);
        con.put_char(self.x,self.y,self.char,BackgroundFlag::None);
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            explored: false,
        }
    }
    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false,
        }
    }
}

type Map = Vec<Vec<Tile>>;

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new( x: i32, y: i32, w: i32, h: i32 ) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x+w,
            y2: y+h,
        }
    }
    pub fn center(&self) -> (i32,i32) {
        let center_x = (self.x1+self.x2)/2;
        let center_y = (self.y1+self.y2)/2;
        (center_x,center_y)
    }
    pub fn intersects_with( &self, other: &Rect ) -> bool {
        (self.x1 <= other.x2)
        && (self.x2 >= other.x1)
        && (self.y1 <= other.y2)
        && (self.y2 >= other.y1)
    }
}

fn create_room( room: Rect, map: &mut Map ) {
    for x in (room.x1+1)..room.x2 {
        for y in (room.y1+1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel( x1: i32, x2: i32, y: i32, map: &mut Map ) {
    for x in cmp::min(x1,x2)..(cmp::max(x1,x2)+1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel( y1: i32, y2: i32, x: i32, map: &mut Map ) {
    for y in cmp::min(y1,y2)..(cmp::max(y1,y2)+1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn place_objects( room: Rect, objects: &mut Vec<Object> ){
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS+1);
    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1+1,room.x2);
        let y = rand::thread_rng().gen_range(room.y1+1,room.y2);
        let monster = if rand::random::<f32>() < 0.8 {
            Object::new(x,y,'o',DESATURATED_GREEN)
        } else {
            Object::new(x,y,'T',DARKER_GREEN)
        };
        objects.push(monster);
    }
}

struct Game {
    map: Map,
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    // fill with empty tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize ];
    // create rooms
    let mut rooms = vec![];
    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE,ROOM_MAX_SIZE+1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE,ROOM_MAX_SIZE+1);
        let x = rand::thread_rng().gen_range(0,MAP_WIDTH-w);
        let y = rand::thread_rng().gen_range(0,MAP_HEIGHT-h);
        let new_room = Rect::new(x, y, w, h);
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));
        if !failed {
            create_room(new_room, &mut map);
            let (new_x,new_y) = new_room.center();
            if rooms.is_empty() {
                // make this the starting room
                objects[PLAYER].x = new_x;
                objects[PLAYER].y = new_y;
            } else {
                // connect to previous room, coin toss on h-v or v-h
                let (prev_x,prev_y) = rooms[rooms.len()-1].center();
                if rand::random() {
                    create_h_tunnel( prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel( prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel( prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel( prev_x, new_x, new_y, &mut map);
                }
            }
            place_objects(new_room, objects);
            rooms.push(new_room);
        }
    }
    map
}

fn render_all( tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool ) {
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x,player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    //draw the objects
    for object in objects {
        if tcod.fov.is_in_fov( object.x, object.y ){
            object.draw(&mut tcod.con);
        }
    }
    //draw the map tiles as background color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let color = match ( visible, wall ) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                *explored = true;
            }
            if *explored {
                tcod.con.set_char_background( x, y, color, BackgroundFlag::Set );
            }
        }
    }
    blit(
        &tcod.con,
        ( 0, 0 ),
        ( SCREEN_WIDTH, SCREEN_HEIGHT ),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

fn handle_keys( tcod: &mut Tcod, game: &Game, player: &mut Object ) -> bool
{
    let key = tcod.root.wait_for_keypress(true);
    match key {
        Key { code: Escape, .. } => return true,
        Key { code: Enter, alt: true, .. } => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        }

        Key { code: Up, .. } => player.move_by(0,-1,game),
        Key { code: Down, .. } => player.move_by(0,1,game),
        Key { code: Left, .. } => player.move_by(-1,0,game),
        Key { code: Right, .. } => player.move_by(1,0,game),

        _ => {}
    }
    false
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH,SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();
    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH,MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH,MAP_HEIGHT),
     };
    tcod::system::set_fps(LIMIT_FPS);

    let player = Object::new( 0, 0, '@', WHITE );
    let mut objects = vec![ player ];
    let mut game = Game {
        map: make_map( &mut objects ),
    };
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x, y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked
            );
        }
    }

    let mut previous_player_position = ( -1, -1 );
    while !tcod.root.window_closed() {
        tcod.con.clear();
        let fov_recompute = previous_player_position != (objects[PLAYER].x,objects[PLAYER].y);
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        tcod.root.flush();
        let player = &mut objects[PLAYER];
        previous_player_position = ( player.x, player.y );
        let exit = handle_keys( &mut tcod, &game, player );
        if exit { break; }
    }
}
