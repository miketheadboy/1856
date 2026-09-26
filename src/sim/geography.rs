//! Douglas County, Kansas Territory, 1855, at half a mile to the tile.
//!
//! The map runs 32 miles east-west and 20 north-south, from Big Springs in
//! the west past Eudora to the edge of Shawnee land, and from the Delaware
//! lands north of the Kaw down across the Santa Fe Trail. Feature positions
//! follow period maps closely where sources agree, and are marked
//! approximate where they don't.
//!
//! Terrain is not a backdrop (§7). Timber belts follow the rivers; everywhere
//! else is tallgrass. Timber is where you cut logs and where nobody sees you
//! coming; prairie is where fire runs and everybody sees everything.

use super::nations::NationId;

pub const MILES_PER_TILE: f32 = 0.5;
pub const WIDTH: i32 = 64;
pub const HEIGHT: i32 = 40;

/// Timber stands within this many miles of a river or creek.
const TIMBER_BELT_MILES: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Prairie,
    Timber,
    River,
    Road,
    Town,
    /// Treaty land.
    Reserve(NationId),
}

impl Terrain {
    /// How likely a witness is to see who did it, relative to open ground (§7.4).
    pub fn visibility(self) -> f32 {
        match self {
            Terrain::Prairie | Terrain::Road => 1.2,
            Terrain::Town => 1.4,
            Terrain::Timber | Terrain::River => 0.5,
            Terrain::Reserve(_) => 0.8,
        }
    }

    /// How readily fire spreads through it (§12). Tallgrass is tinder.
    pub fn fuel(self) -> f32 {
        match self {
            Terrain::Prairie => 1.4,
            Terrain::Timber => 0.7,
            Terrain::Road | Terrain::Town => 0.5,
            Terrain::River => 0.0,
            Terrain::Reserve(_) => 1.0,
        }
    }

    pub fn glyph(self) -> char {
        match self {
            Terrain::Prairie => '.',
            Terrain::Timber => '♣',
            Terrain::River => '~',
            Terrain::Road => '=',
            Terrain::Town => '#',
            Terrain::Reserve(_) => ',',
        }
    }
}

/// Positions in miles from the map's northwest corner: x east, y south.
pub type Miles = (f32, f32);

pub struct Place {
    pub name: &'static str,
    pub at: Miles,
    pub approximate: bool,
}

/// The Kansas (Kaw) River, west to east. The county's north line.
pub const KAW: &[Miles] = &[
    (0.0, 2.6),
    (4.0, 2.2),
    (8.0, 2.9),
    (12.0, 3.4),
    (16.0, 3.1),
    (19.5, 3.6),
    (24.0, 3.0),
    (28.0, 3.3),
    (32.0, 3.0),
];

/// The Wakarusa, flowing east through the middle of the county to the Kaw
/// near Eudora.
pub const WAKARUSA: &[Miles] = &[
    (0.0, 11.5),
    (5.0, 10.6),
    (9.0, 10.0),
    (12.0, 9.5),
    (15.0, 8.6),
    (18.0, 7.8),
    (20.5, 7.3),
    (24.0, 6.8),
    (26.5, 5.4),
    (28.4, 3.3),
];

/// Creeks feeding the Wakarusa from the south: more timber, more cover.
pub const CREEKS: &[&[Miles]] = &[
    // Rock Creek, toward Clinton (approximate)
    &[(10.5, 15.0), (11.5, 12.5), (12.0, 9.6)],
    // Washington Creek (approximate)
    &[(15.0, 14.0), (15.5, 11.0), (15.8, 8.7)],
    // Captain's Creek, near Black Jack (approximate)
    &[(24.5, 16.5), (25.0, 12.0), (25.5, 7.5)],
];

/// The Santa Fe Trail across the south of the county.
pub const SANTA_FE_TRAIL: &[Miles] = &[
    (32.0, 15.2),
    (26.0, 15.6),
    (22.0, 15.5),
    (19.0, 15.0),
    (14.0, 16.2),
    (8.0, 17.4),
    (0.0, 18.6),
];

/// The California Road: up from the Wakarusa crossing to Lawrence, then west
/// along the ridge south of the Kaw toward Big Springs and Topeka.
pub const CALIFORNIA_ROAD: &[Miles] = &[
    (32.0, 8.8),
    (27.0, 8.2),
    (23.5, 6.0),
    (20.0, 4.6),
    (14.0, 4.6),
    (9.0, 4.2),
    (2.0, 4.7),
    (0.0, 5.0),
];

/// The road from Lawrence south past Blanton's Bridge to Palmyra.
pub const PALMYRA_ROAD: &[Miles] = &[(20.0, 4.6), (20.5, 7.3), (19.8, 11.0), (19.0, 15.0)];

pub const PLACES: &[Place] = &[
    Place {
        name: "Lawrence",
        at: (20.0, 4.5),
        approximate: false,
    },
    Place {
        name: "Lecompton",
        at: (9.0, 3.4),
        approximate: false,
    },
    Place {
        name: "Big Springs",
        at: (2.0, 4.6),
        approximate: false,
    },
    Place {
        name: "Franklin",
        at: (23.5, 6.0),
        approximate: false,
    },
    Place {
        name: "Blanton's Bridge",
        at: (20.5, 7.3),
        approximate: false,
    },
    Place {
        name: "Eudora",
        at: (28.5, 3.9),
        approximate: false,
    },
    Place {
        name: "Clinton",
        at: (12.0, 9.8),
        approximate: true,
    },
    Place {
        name: "Palmyra",
        at: (19.0, 15.0),
        approximate: false,
    },
    Place {
        name: "Black Jack",
        at: (22.0, 15.6),
        approximate: false,
    },
    Place {
        name: "Hickory Point",
        at: (15.0, 12.0),
        approximate: true,
    },
    Place {
        name: "Mount Oread",
        at: (19.4, 5.0),
        approximate: false,
    },
];

pub fn place(name: &str) -> Miles {
    PLACES
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.at)
        .unwrap_or((16.0, 10.0))
}

pub fn to_tile(m: Miles) -> (i32, i32) {
    (
        (m.0 / MILES_PER_TILE).round() as i32,
        (m.1 / MILES_PER_TILE).round() as i32,
    )
}

pub fn to_miles(t: (i32, i32)) -> Miles {
    (t.0 as f32 * MILES_PER_TILE, t.1 as f32 * MILES_PER_TILE)
}

fn segment_distance(p: Miles, a: Miles, b: Miles) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

/// Distance in miles from a point to a polyline.
pub fn distance_to(p: Miles, line: &[Miles]) -> f32 {
    line.windows(2)
        .map(|w| segment_distance(p, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

/// Latitude of the Kaw at a given x, for north/south-of-river tests.
fn kaw_y(x: f32) -> f32 {
    for w in KAW.windows(2) {
        if x >= w[0].0 && x <= w[1].0 {
            let t = (x - w[0].0) / (w[1].0 - w[0].0);
            return w[0].1 + t * (w[1].1 - w[0].1);
        }
    }
    KAW.last().map_or(3.0, |p| p.1)
}

#[derive(Clone, Debug)]
pub struct Map {
    tiles: Vec<Terrain>,
}

impl Map {
    pub fn county() -> Self {
        let mut tiles = Vec::with_capacity((WIDTH * HEIGHT) as usize);
        for ty in 0..HEIGHT {
            for tx in 0..WIDTH {
                tiles.push(classify(to_miles((tx, ty))));
            }
        }
        Self { tiles }
    }

    pub fn at(&self, t: (i32, i32)) -> Terrain {
        let x = t.0.clamp(0, WIDTH - 1);
        let y = t.1.clamp(0, HEIGHT - 1);
        self.tiles[(y * WIDTH + x) as usize]
    }

    /// Miles from a tile to the nearest timber you could cut without crossing
    /// onto treaty land.
    pub fn miles_to_timber(&self, from: (i32, i32)) -> f32 {
        let mut best = f32::INFINITY;
        for ty in 0..HEIGHT {
            for tx in 0..WIDTH {
                if self.at((tx, ty)) == Terrain::Timber {
                    let d = (((tx - from.0).pow(2) + (ty - from.1).pow(2)) as f32).sqrt();
                    best = best.min(d * MILES_PER_TILE);
                }
            }
        }
        best
    }

    /// The whole county as text, with named places and farms marked.
    pub fn render(&self, marks: &[((i32, i32), char)]) -> String {
        let mut out = String::new();
        for ty in 0..HEIGHT {
            for tx in 0..WIDTH {
                let c = marks
                    .iter()
                    .find(|(t, _)| *t == (tx, ty))
                    .map(|(_, c)| *c)
                    .unwrap_or_else(|| self.at((tx, ty)).glyph());
                out.push(c);
            }
            out.push('\n');
        }
        out
    }
}

fn classify(p: Miles) -> Terrain {
    let river = distance_to(p, KAW).min(distance_to(p, WAKARUSA));
    if river < 0.3 {
        return Terrain::River;
    }
    if PLACES
        .iter()
        .filter(|pl| !pl.approximate && pl.name != "Mount Oread")
        .any(|pl| ((p.0 - pl.at.0).powi(2) + (p.1 - pl.at.1).powi(2)).sqrt() < 0.5)
    {
        return Terrain::Town;
    }
    let road = [SANTA_FE_TRAIL, CALIFORNIA_ROAD, PALMYRA_ROAD]
        .iter()
        .map(|r| distance_to(p, r))
        .fold(f32::INFINITY, f32::min);
    if road < 0.25 {
        return Terrain::Road;
    }
    // North of the Kaw: Delaware trust lands. East edge: Shawnee lands.
    if p.1 < kaw_y(p.0) {
        return Terrain::Reserve(NationId::Delaware);
    }
    if p.0 > 30.5 {
        return Terrain::Reserve(NationId::Shawnee);
    }
    let creek = CREEKS
        .iter()
        .map(|c| distance_to(p, c))
        .fold(f32::INFINITY, f32::min);
    if river < TIMBER_BELT_MILES || creek < TIMBER_BELT_MILES * 0.6 {
        return Terrain::Timber;
    }
    Terrain::Prairie
}

/// Homesteads by family, in miles: Free-State claims around Lawrence, Clinton
/// and Palmyra; Pro-Slavery claims around Lecompton and Franklin. Your claim
/// sits on the prairie between Lawrence and the Wakarusa.
pub const CLAIMS_FREE_STATE: &[Miles] = &[
    (17.5, 6.8),
    (13.0, 10.8),
    (21.5, 12.5),
    (18.0, 13.8),
    (22.0, 9.0),
];

pub const CLAIMS_PRO_SLAVERY: &[Miles] = &[
    (10.5, 5.2),
    (7.5, 6.5),
    (25.0, 7.2),
    (13.5, 6.2),
    (26.5, 10.0),
];

pub const PLAYER_CLAIM: Miles = (19.5, 8.6);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rivers_and_towns_land_where_they_should() {
        let map = Map::county();
        assert_eq!(map.at(to_tile(place("Lawrence"))), Terrain::Town);
        assert_eq!(map.at(to_tile(place("Lecompton"))), Terrain::Town);
        assert_eq!(map.at(to_tile((20.0, 3.6))), Terrain::River);
        // North of the Kaw is Delaware land.
        assert_eq!(
            map.at(to_tile((12.0, 1.0))),
            Terrain::Reserve(NationId::Delaware)
        );
    }

    #[test]
    fn timber_follows_the_rivers() {
        let map = Map::county();
        // Along the Wakarusa there's timber within a mile; out on the high
        // prairie between the rivers there isn't.
        assert!(map.miles_to_timber(to_tile((15.0, 9.4))) < 1.0);
        assert!(map.miles_to_timber(to_tile((6.0, 15.0))) > 1.0);
    }

    #[test]
    fn claims_are_on_the_map_and_on_land() {
        let map = Map::county();
        for c in CLAIMS_FREE_STATE
            .iter()
            .chain(CLAIMS_PRO_SLAVERY)
            .chain([&PLAYER_CLAIM])
        {
            let t = to_tile(*c);
            assert!(t.0 >= 0 && t.0 < WIDTH && t.1 >= 0 && t.1 < HEIGHT);
            assert_ne!(map.at(t), Terrain::River, "{c:?} is in a river");
        }
    }
}
