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
// gui constants
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;
const INVENTORY_WIDTH: i32 = 50;
// map size
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
// object placement
const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;
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

const HEAL_AMOUNT: i32 = 4;

const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;

const CONFUSE_NUM_TURNS: i32 = 10;
const CONFUSE_RANGE: i32 = 8;

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index!=second_index);
    let split_at_index = cmp::max(first_index,second_index);
    let (first_slice,second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index],&mut second_slice[0])
    } else {
        (&mut second_slice[0],&mut first_slice[second_index])
    }
}

#[derive(Clone,Copy,Debug,PartialEq)]
enum DeathCallback {
    Player,
    Monster,
}

#[derive(Clone,Copy,Debug,PartialEq)]
enum Item {
    Heal,
    Lightning,
    Confuse,
}

enum  UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Clone,Copy,Debug,PartialEq)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback,
}

#[derive(Clone,Debug,PartialEq)]
enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
}

impl Object {
    pub fn new( x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool ) -> Self {
        Object{
            x,
            y,
            char,
            color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x,self.y)
    }

    pub fn set_pos( &mut self, x: i32, y: i32 ) {
        self.x = x;
        self.y = y;
    }

    pub fn draw( &self, con: &mut dyn Console ) {
        con.set_default_foreground(self.color);
        con.put_char(self.x,self.y,self.char,BackgroundFlag::None);
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2)+dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game){
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self,game);
            }
        }
    }

    pub fn heal( &mut self, amount: i32 ) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game){
        let damage = self.fighter.map_or(0, |f| f.power)-target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            game.messages.add(
                format!( "{} attacks {} for {}.",
                self.name, target.name, damage
                ), WHITE );
            target.take_damage(damage,game);
        } else {
            game.messages.add(
                format!(
                "{} attacks {} but has no effect.",
                self.name, target.name
                ), WHITE );
        }
    }
}

impl DeathCallback {
    fn callback(self, object: &mut Object, game: &mut Game){
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object,game);
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

fn is_blocked( x: i32, y: i32, map: &Map, objects: &[Object] ) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    objects.iter().any(
        |object| object.blocks && object.pos() == (x,y) )
}

fn move_by( id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object] ) {
    let (x,y) = objects[id].pos();
    if !is_blocked(x+dx, y+dy, map, objects) {
        objects[id].set_pos(x+dx, y+dy);
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

fn create_orc( x: i32, y: i32 ) -> Object {
    let mut orc = Object::new(x,y,'o', "orc", DESATURATED_GREEN, true);
    orc.fighter = Some( Fighter {
        max_hp: 10,
        hp: 10,
        defense: 0,
        power: 3,
        on_death: DeathCallback::Monster,
    });
    orc.ai = Some( Ai::Basic );
    orc.alive = true;
    orc
}

fn create_troll( x: i32, y: i32 ) -> Object {
    let mut troll = Object::new(x,y,'T', "troll", DARKER_GREEN, true);
    troll.fighter = Some(Fighter {
        max_hp: 16,
        hp: 16,
        defense: 1,
        power: 4,
        on_death: DeathCallback::Monster,
    });
    troll.ai = Some( Ai::Basic );
    troll.alive = true;
    troll
}

fn place_objects( room: Rect, map: &Map, objects: &mut Vec<Object> ){
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS+1);
    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1+1,room.x2);
        let y = rand::thread_rng().gen_range(room.y1+1,room.y2);
        if !is_blocked(x, y, map, objects) {
            let monster = if rand::random::<f32>() < 0.8 {
                create_orc(x, y)
            } else {
                create_troll(x, y)
            };
            objects.push(monster);
        }
    }

    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS+1);
    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1+1,room.x2);
        let y = rand::thread_rng().gen_range(room.y1+1,room.y2);
        if !is_blocked(x, y, map, objects) {
            let dice = rand::random::<f32>();
            let item = if dice < 0.7 {
                let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
                object.item = Some(Item::Heal);
                object
            } else if dice < 0.7 + 0.1 {
                let mut object = Object::new(x, y, '#', "scroll of lightning bolt", LIGHT_AZURE, false);
                object.item = Some(Item::Lightning);
                object
            } else {
                let mut object = Object::new(x, y, '#', "scroll of confusion", LIGHT_YELLOW, false);
                object.item = Some(Item::Confuse);
                object
            };
            objects.push(item);
        }
    }
}

struct Messages {
    messages: Vec<(String,Color)>,
}

impl Messages {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }
    pub fn add<T: Into<String>>( &mut self, messages: T, color: Color ){
        self.messages.push((messages.into(),color));
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String,Color)> {
        self.messages.iter()
    }
}

struct Game {
    map: Map,
    messages: Messages,
    inventory: Vec<Object>,
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
            place_objects(new_room, &map, objects);
            rooms.push(new_room);
        }
    }
    map
}

fn move_towards( id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object] ) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2)+dy.pow(2)) as f32).sqrt();
    let dx = (dx as f32 / distance ).round() as i32;
    let dy = (dy as f32 / distance ).round() as i32;
    move_by(id, dx, dy, map, objects);
}

fn player_move_or_attack( dx: i32, dy: i32, game: &mut Game, objects: &mut [Object] ) {
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;
    let target_id = objects.iter().position(
        |object| object.fighter.is_some() && object.pos() == (x,y)
    );
    match target_id {
        Some(target_id) => {
            let (player,target) = mut_two(PLAYER,target_id,objects);
            player.attack(target, game);
        }
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}

fn closest_monster( tcod: &Tcod, objects: &[Object], max_range: i32 ) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range+1) as f32;
    for (id,object) in objects.iter().enumerate() {
        if (id != PLAYER)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x,object.y)
            {
                let dist = objects[PLAYER].distance_to(object);
                if dist < closest_dist {
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}

fn pick_item_up( object_id: usize, game: &mut Game, objects: &mut Vec<Object> ){
    if game.inventory.len() >= 26 {
        game.messages.add(format!(
        "Inventory full, cannot pickup {}.", objects[object_id].name
        ), RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.messages.add(format!(
        "You picked up a {}.", item.name
        ), GREEN);
        game.inventory.push(item);
    }
}

fn cast_heal(
    _inventory_id: usize,
    _tcode: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object],
) -> UseResult {
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health.",RED);
        }
        objects[PLAYER].heal(HEAL_AMOUNT);
        game.messages.add("Your wounds start to heal!", LIGHT_VIOLET);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object],
) -> UseResult {
    let monster_id = closest_monster( tcod, objects, LIGHTNING_RANGE );
    if let Some(monster_id) = monster_id {
        objects[monster_id].take_damage(LIGHTNING_DAMAGE,game);
        game.messages.add(
            format!("A lightning bolt strikes the {} for {} damage.",objects[monster_id].name,LIGHTNING_DAMAGE),
            LIGHTER_BLUE
            );
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is in range.",RED);
        UseResult::Cancelled
    }
}

fn cast_confuse(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object],
) -> UseResult {
    let monster_id = closest_monster( tcod, objects, CONFUSE_RANGE );
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.messages.add(
            format!("The {} begins to stumble around!", objects[monster_id].name), LIGHT_GREEN,
        );
        UseResult::UsedUp
    } else {
        game.messages.add("No enemy is in range.",RED);
        UseResult::Cancelled
    }
}

fn player_death( player: &mut Object, game: &mut Game ) {
    game.messages.add("You dead!",RED);
    player.char='%';
    player.color=DARK_RED;
}

fn monster_death( monster: &mut Object, game: &mut Game ) {
    game.messages.add(format!("{} dies!", monster.name), ORANGE);
    monster.char='%';
    monster.color=DARK_RED;
    monster.blocks=false;
    monster.fighter=None;
    monster.ai=None;
    monster.name=format!("remains of {}", monster.name);
}

fn render_all( tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool ) {
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(player.x,player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    //draw the objects
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter( |o| tcod.fov.is_in_fov(o.x,o.y) )
        .collect();
    to_draw.sort_by(|o1,o2| { o1.blocks.cmp(&o2.blocks) });
    for object in &to_draw {
        object.draw(&mut tcod.con);
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

    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, LIGHT_RED, DARK_RED);
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }
    blit(
        &tcod.panel,
        (0,0),(SCREEN_WIDTH,SCREEN_HEIGHT),
        &mut tcod.root,
        (0,PANEL_Y), 1.0, 1.0,
    );
}

struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
}

#[derive(Clone,Copy,Debug,PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn ai_basic( monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object] ) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x,player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0 ) {
            let (monster,player) = mut_two(monster_id,PLAYER,objects);
            monster.attack(player,game);
        }
    }
    Ai::Basic
}

fn ai_confused(
    monster_id: usize,
    game: &mut Game,
    objects: &mut [Object],
    previous_ai: Box<Ai>,
    num_turns: i32
) -> Ai {
    if num_turns >= 0 {
        move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            &game.map,
            objects
        );
        Ai::Confused { previous_ai: previous_ai, num_turns: num_turns-1 }
    } else {
        game.messages.add(format!("The {} is no longer confused!", objects[monster_id].name ), RED );
        *previous_ai
    }
}

fn ai_take_turn( monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object] ) {
    use Ai::*;
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, objects),
            Confused { previous_ai, num_turns } => ai_confused(monster_id, game, objects, previous_ai, num_turns),
        };
        objects[monster_id].ai = Some(new_ai);
    }
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root ) -> Option<usize> {
    assert!( options.len() <= 26, "Cannot have menu with more than 26 options.");
    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;
    let mut window = Offscreen::new(width, height);
    window.set_default_foreground(WHITE);
    window.print_rect_ex( 0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header );
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {})", menu_letter, option_text.as_ref());
        window.print_ex( 0, header_height+index as i32, BackgroundFlag::None, TextAlignment::Left, text );
    }
    let x = SCREEN_WIDTH/2 - width/2;
    let y = SCREEN_HEIGHT/2 - height/2;
    blit(&window, (0,0), (width,height), root, (x,y), 1.0, 0.7);
    root.flush();
    let key = root.wait_for_keypress(true);
    if key.printable.is_alphanumeric() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu( inventory: &[Object], header: &str, root: &mut Root ) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };
    let inventory_index = menu(header,&options,INVENTORY_WIDTH,root);
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn use_item( inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    use Item::*;
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
        };
        match on_use( inventory_id, tcod, game, objects ) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.messages.add("Cancelled",WHITE);
            }
        }
    } else {
        game.messages.add(
            format!("The {} cannot be used.", game.inventory[inventory_id].name),
            WHITE);
    }
}

fn handle_keys( tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object> ) -> PlayerAction
{
    use PlayerAction::*;
    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match ( key, key.text(), player_alive ) {
        ( Key { code: Escape, .. }, _, _ ) => Exit,
        ( Key { printable: 'q', .. }, _, _ ) => Exit,
        ( Key { code: Enter, alt: true, .. }, _, _ ) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }

        ( Key { code: Up, .. }, _, true ) => {
            player_move_or_attack(0,-1,game,objects);
            TookTurn
        },
        ( Key { code: Down, .. }, _, true ) => {
            player_move_or_attack(0,1,game,objects);
            TookTurn
        }
        ( Key { code: Left, .. }, _, true ) => {
            player_move_or_attack(-1,0,game,objects);
            TookTurn
        }
        ( Key { code: Right, .. }, _, true ) => {
            player_move_or_attack(1,0,game,objects);
            TookTurn
        }
        ( Key { printable: 'g', .. }, _, true ) => {
            let item_id = objects.iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some() );
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        ( Key { printable: 'i', .. }, _, true ) => {
            let inventory_index = inventory_menu(&game.inventory, "Select an item to use it, or any other to cancel.\n", &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, objects);
            }
            TookTurn
        }

        _ => DidntTakeTurn
    }
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
) {
    let bar_width = ( value as f32 / maximum as f32 * total_width as f32 ) as i32;
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);
    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }
    panel.set_default_foreground(WHITE);
    panel.print_ex(x+total_width/2, y,BackgroundFlag::None, TextAlignment::Center,
        &format!("{}: {}/{}",name, value, maximum) );
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
        panel: Offscreen::new(SCREEN_WIDTH,PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH,MAP_HEIGHT),
     };
    tcod::system::set_fps(LIMIT_FPS);

    let mut player = Object::new( 0, 0, '@', "player", WHITE, true );
    player.alive = true;
    player.fighter = Some( Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: DeathCallback::Player,
    });
    let mut objects = vec![ player ];
    let mut game = Game {
        map: make_map( &mut objects ),
        messages: Messages::new(),
        inventory: vec![],
    };
    game.messages.add(
        "Welcome, stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        RED,
    );
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
        let player_action = handle_keys( &mut tcod, &mut game, &mut objects );
        if player_action == PlayerAction::Exit { break; }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut objects);
                }
            }
        }
    }
}
