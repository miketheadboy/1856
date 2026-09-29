//! Cut-out rigs: a man built from parts on a skeleton, so his limbs do what
//! the sim says. The rifle arm finds its target (your sights, at the gate),
//! the other hand holds the barrel, the feet stay where they're planted and
//! the knees take up the difference, a ball knocks back the part it hits,
//! and the dead go down in a heap. Old wounds show: a stiff gun arm droops
//! and shakes, a bad leg stands crooked. Clothes come off the outfit.
//!
//! Placeholder art: every part is a rounded slab in the outfit's colors. The
//! skeleton, the solvers and the motion are the real thing; drawn parts drop
//! in over the same bones later (docs/PLAN.md, the art pipeline).
//!
//! Screen space throughout: stage pixels, y down, angles by `atan2(dy, dx)`.

use bevy::prelude::*;
use bleeding_kansas::sim::psyche::Limb;
use bleeding_kansas::sim::wardrobe::{ITEMS, Slot};
use bleeding_kansas::sim::world::{NpcId, World, is_woman};

/// Nodes kept for one figure. Unused ones hide.
pub const PARTS: usize = 22;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part(pub usize);

/// One drawn piece: a capsule from `a` to `b`, `thick` across.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub a: Vec2,
    pub b: Vec2,
    pub thick: f32,
    pub color: Color,
    /// Round ends (limbs) or square (hat brim, belt).
    pub round: bool,
}

impl Piece {
    fn new(a: Vec2, b: Vec2, thick: f32, color: Color) -> Self {
        Piece {
            a,
            b,
            thick,
            color,
            round: true,
        }
    }

    fn square(mut self) -> Self {
        self.round = false;
        self
    }
}

/// Bone lengths for a grown man about 150 stage pixels tall.
pub struct Build {
    pub spine: f32,
    pub neck: f32,
    pub head: f32,
    pub shoulder: f32,
    pub hip: f32,
    pub upper_arm: f32,
    pub forearm: f32,
    pub thigh: f32,
    pub shin: f32,
    pub rifle: f32,
}

pub const MAN: Build = Build {
    spine: 40.0,
    neck: 6.0,
    head: 11.0,
    shoulder: 13.0,
    hip: 7.0,
    upper_arm: 24.0,
    forearm: 22.0,
    thigh: 30.0,
    shin: 30.0,
    rifle: 50.0,
};

/// What the figure wears and carries, read off the sim.
#[derive(Clone, Copy, Debug)]
pub struct Dress {
    pub hat: Option<(Color, f32, f32)>,
    pub coat: Option<Color>,
    pub shirt: Color,
    pub legs: Color,
    pub feet: Color,
    pub skirt: bool,
    pub knife: bool,
    pub belt: bool,
    pub gun: bool,
}

const SKIN: Color = Color::srgb(0.84, 0.71, 0.58);
const IRON: Color = Color::srgb(0.12, 0.11, 0.10);
const WOOD: Color = Color::srgb(0.38, 0.24, 0.14);

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::srgb(r, g, b)
}

/// Colors for a piece of clothing by name. Placeholder palette, period dyes.
fn dye(name: &str) -> Color {
    match name {
        "red flannel shirt" => rgb(0.62, 0.16, 0.12),
        "hickory shirt" => rgb(0.36, 0.41, 0.52),
        "boiled white shirt" => rgb(0.90, 0.88, 0.82),
        "linsey shirt" | "rags" => rgb(0.52, 0.47, 0.38),
        "calico dress" => rgb(0.58, 0.36, 0.34),
        "silk dress" => rgb(0.32, 0.20, 0.42),
        "sack coat" | "patched coat" => rgb(0.36, 0.27, 0.19),
        "frock coat" | "black frock" => rgb(0.11, 0.10, 0.10),
        "buffalo coat" => rgb(0.27, 0.19, 0.13),
        "blanket capote" => rgb(0.80, 0.75, 0.62),
        "fringed hunting shirt" | "linen duster" => rgb(0.62, 0.52, 0.36),
        "army greatcoat" => rgb(0.30, 0.34, 0.44),
        "wool shawl" => rgb(0.42, 0.30, 0.28),
        "straw hat" => rgb(0.80, 0.70, 0.45),
        "sunbonnet" | "bonnet with ribbons" => rgb(0.86, 0.82, 0.70),
        "forage cap" => rgb(0.18, 0.22, 0.36),
        "coonskin cap" => rgb(0.40, 0.31, 0.22),
        "beaver hat" => rgb(0.30, 0.27, 0.24),
        "moccasins" => rgb(0.55, 0.42, 0.28),
        "bare feet" => SKIN,
        _ => rgb(0.16, 0.13, 0.11),
    }
}

/// Hat shape by name: (brim width, crown height).
fn hat_shape(name: &str) -> (f32, f32) {
    match name {
        "stovepipe hat" => (24.0, 20.0),
        "straw hat" | "preacher's hat" => (34.0, 7.0),
        "forage cap" | "coonskin cap" => (20.0, 8.0),
        "sunbonnet" | "bonnet with ribbons" => (26.0, 12.0),
        _ => (30.0, 9.0),
    }
}

pub fn dress_of(world: &World, id: NpcId) -> Dress {
    let n = world.npc(id);
    let mut d = Dress {
        hat: None,
        coat: None,
        shirt: rgb(0.52, 0.47, 0.38),
        legs: rgb(0.45, 0.38, 0.25),
        feet: rgb(0.16, 0.13, 0.11),
        skirt: is_woman(&n.name),
        knife: false,
        belt: false,
        gun: world.families[n.family as usize].stores.arms.at_hand() > 0,
    };
    for w in n.outfit.on.iter().flatten() {
        let it = &ITEMS[w.item as usize];
        match it.slot {
            Slot::Hat => {
                let (brim, crown) = hat_shape(it.name);
                d.hat = Some((dye(it.name), brim, crown));
            }
            Slot::Coat => d.coat = Some(dye(it.name)),
            Slot::Shirt => d.shirt = dye(it.name),
            Slot::Feet => d.feet = dye(it.name),
            Slot::Kit => match it.name {
                "Bowie knife" => d.knife = true,
                "revolver belt" => d.belt = true,
                _ => {}
            },
        }
    }
    if d.skirt {
        d.legs = d.shirt;
    }
    d
}

/// Everything that moves him this frame.
#[derive(Clone, Copy, Debug)]
pub struct Drive {
    /// Where his feet are planted (between them), stage px.
    pub feet: Vec2,
    pub t: f32,
    /// 0 calm .. 1 shaking.
    pub fear: f32,
    /// Where the rifle points, and how far up it is (0 hanging .. 1 aimed).
    pub aim: Vec2,
    pub raise: f32,
    /// Hit knock-back, radians, by where it landed.
    pub knock: Knock,
    /// 0 standing .. 1 down in a heap.
    pub fall: f32,
    /// 0 standing .. 1 on his knees.
    pub kneel: f32,
    /// His old wounds and new: the gun arm, a leg.
    pub bad_arm: bool,
    pub bad_leg: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Knock {
    pub head: f32,
    pub chest: f32,
    pub arm: f32,
    pub leg: f32,
    /// Pushed back bodily, px.
    pub push: f32,
}

/// Two bones reaching for a target: returns (elbow, hand). `bend` picks
/// which way the joint folds (+1 or -1). Out of reach, the limb points
/// straight at it.
pub fn reach(base: Vec2, target: Vec2, l1: f32, l2: f32, bend: f32) -> (Vec2, Vec2) {
    let to = target - base;
    let d = to.length().clamp(1e-3, l1 + l2 - 1e-3);
    let dir = to.normalize_or(Vec2::X);
    let cos_a = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let a = cos_a.acos() * bend;
    let (s, c) = a.sin_cos();
    let upper = Vec2::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
    let elbow = base + upper * l1;
    let hand = elbow + (base + dir * d - elbow).normalize_or(dir) * l2;
    (elbow, hand)
}

fn at(p: Vec2, angle: f32, len: f32) -> Vec2 {
    p + Vec2::from_angle(angle) * len
}

/// Straight down, in screen space.
const DOWN: f32 = std::f32::consts::FRAC_PI_2;
const UP: f32 = -std::f32::consts::FRAC_PI_2;

/// Lay out the figure. Pure: the same drive gives the same pieces.
pub fn pose(b: &Build, dr: &Drive, dress: &Dress) -> Vec<Piece> {
    let breath = (dr.t * 1.7).sin();
    let tremble = dr.fear * (dr.t * 23.0).sin() * 0.6;
    let stand = 1.0 - dr.fall.max(dr.kneel);
    // The hips ride over the feet; knees take up what's left.
    let leg_len = b.thigh + b.shin;
    let hip_h = leg_len * (0.97 - 0.45 * dr.kneel) + breath * 0.8;
    let lean = if dr.bad_leg { 4.0 } else { 0.0 };
    let mut pelvis = dr.feet + Vec2::new(dr.knock.push + tremble + lean, -hip_h);
    // Down in a heap: the hips hit the ground and slide back.
    pelvis = pelvis.lerp(dr.feet + Vec2::new(18.0, -8.0), dr.fall);
    let spine_a = UP + 0.03 * breath - 0.35 * dr.knock.chest - 0.25 * dr.kneel
        + dr.fall * 1.45
        + if dr.bad_leg { 0.06 } else { 0.0 };
    let neck = at(pelvis, spine_a, b.spine);
    let head_a = spine_a - 0.6 * dr.knock.head + 0.25 * dr.fall;
    let head_c = at(neck, head_a, b.neck + b.head);
    let across = spine_a + DOWN; // perpendicular to the spine, to his right (screen left)
    let shoulder_r =
        neck - Vec2::from_angle(across) * b.shoulder + Vec2::from_angle(spine_a) * -4.0;
    let shoulder_l =
        neck + Vec2::from_angle(across) * b.shoulder + Vec2::from_angle(spine_a) * -4.0;
    let hip_r = pelvis - Vec2::from_angle(across) * b.hip;
    let hip_l = pelvis + Vec2::from_angle(across) * b.hip;

    // Arms. Hanging, the hands fall by the thighs; raised, the gun hand
    // goes to the shoulder line and the rifle lays toward the target.
    let droop = if dr.bad_arm { 0.35 } else { 0.0 };
    let raise = dr.raise * stand * (1.0 - droop * 0.5);
    let hang_r = shoulder_r + Vec2::new(-4.0, b.upper_arm + b.forearm - 4.0);
    let hang_l = shoulder_l + Vec2::new(4.0, b.upper_arm + b.forearm - 4.0);
    let aim_dir = (dr.aim - shoulder_r).normalize_or(Vec2::X);
    let shake = if dr.bad_arm {
        Vec2::new(0.0, (dr.t * 17.0).sin() * 3.0)
    } else {
        Vec2::ZERO
    };
    let grip_up = shoulder_r + aim_dir * 12.0 + Vec2::new(0.0, 8.0) + shake;
    let grip = hang_r.lerp(grip_up, raise);
    let knock_arm = Vec2::new(dr.knock.arm * 30.0, dr.knock.arm * 10.0);
    let (elbow_r, hand_r) = reach(shoulder_r, grip + knock_arm, b.upper_arm, b.forearm, -1.0);
    // The rifle: along the aim when raised, muzzle down when hanging.
    let rifle_dir = Vec2::from_angle(
        DOWN + 0.25 + (aim_dir.to_angle() - (DOWN + 0.25)) * raise + dr.fall * 0.8,
    );
    let muzzle = hand_r + rifle_dir * (b.rifle * 0.75);
    let butt = hand_r - rifle_dir * (b.rifle * 0.25);
    // The other hand holds the barrel when it's up.
    let hold = hang_l.lerp(hand_r + rifle_dir * (b.rifle * 0.4), raise);
    let (elbow_l, hand_l) = reach(shoulder_l, hold, b.upper_arm, b.forearm, 1.0);

    // Legs: feet planted a stride apart; knees bend toward the viewer's
    // side a little. Kneeling, the shins lie along the ground.
    let foot_r = dr.feet + Vec2::new(-10.0, 0.0);
    let foot_l = dr.feet + Vec2::new(10.0, 0.0);
    let kick = Vec2::new(dr.knock.leg * 14.0, -dr.knock.leg * 6.0);
    let (knee_r, ankle_r) = reach(hip_r, foot_r + kick, b.thigh, b.shin, 1.0);
    let (knee_l, ankle_l) = reach(hip_l, foot_l, b.thigh, b.shin, -1.0);

    let coat = dress.coat.unwrap_or(dress.shirt);
    let mut v = Vec::with_capacity(PARTS);
    // Back arm, legs, body, front arm: draw order is list order.
    v.push(Piece::new(shoulder_l, elbow_l, 9.0, coat));
    v.push(Piece::new(elbow_l, hand_l, 8.0, coat));
    v.push(Piece::new(
        hand_l,
        hand_l + (hand_l - elbow_l).normalize_or(Vec2::Y) * 3.0,
        6.0,
        SKIN,
    ));
    if dress.skirt {
        // A dress hangs from the hips in a bell; boots show under it.
        let hem = (ankle_r + ankle_l) / 2.0 + Vec2::new(0.0, -6.0);
        v.push(Piece::new(pelvis, hem, 34.0, dress.legs).square());
    } else {
        v.push(Piece::new(hip_r, knee_r, 12.0, dress.legs));
        v.push(Piece::new(knee_r, ankle_r, 10.0, dress.legs));
        v.push(Piece::new(hip_l, knee_l, 12.0, dress.legs));
        v.push(Piece::new(knee_l, ankle_l, 10.0, dress.legs));
    }
    v.push(Piece::new(
        ankle_r,
        ankle_r + Vec2::new(-8.0, 0.0),
        7.0,
        dress.feet,
    ));
    v.push(Piece::new(
        ankle_l,
        ankle_l + Vec2::new(8.0, 0.0),
        7.0,
        dress.feet,
    ));
    // Torso: the coat over the shirt; a strip of shirt down the front.
    v.push(Piece::new(pelvis, neck, 30.0, coat));
    if dress.coat.is_some() {
        v.push(Piece::new(
            at(pelvis, spine_a, 6.0),
            at(pelvis, spine_a, b.spine - 3.0),
            9.0,
            dress.shirt,
        ));
    }
    if dress.belt {
        v.push(
            Piece::new(
                hip_r - Vec2::new(3.0, 2.0),
                hip_l + Vec2::new(3.0, -2.0),
                5.0,
                IRON,
            )
            .square(),
        );
        v.push(Piece::new(
            hip_r + Vec2::new(-6.0, 0.0),
            hip_r + Vec2::new(-6.0, 14.0),
            6.0,
            IRON,
        ));
    }
    if dress.knife {
        v.push(Piece::new(
            hip_l + Vec2::new(4.0, 0.0),
            hip_l + Vec2::new(7.0, 13.0),
            3.0,
            rgb(0.75, 0.75, 0.72),
        ));
    }
    // Head and hat.
    v.push(Piece::new(neck, at(neck, head_a, b.neck), 8.0, SKIN));
    v.push(Piece::new(
        head_c,
        head_c + Vec2::new(0.0, 0.01),
        b.head * 2.0,
        SKIN,
    ));
    if let Some((c, brim, crown)) = dress.hat {
        let top = at(head_c, head_a, b.head * 0.7);
        let side = Vec2::from_angle(head_a + DOWN);
        v.push(Piece::new(top - side * brim / 2.0, top + side * brim / 2.0, 4.0, c).square());
        v.push(Piece::new(top, at(top, head_a, crown), 18.0, c).square());
    }
    // The rifle, then the front arm over it.
    if dress.gun {
        v.push(Piece::new(butt, hand_r, 6.0, WOOD).square());
        v.push(Piece::new(hand_r, muzzle, 3.5, IRON).square());
    }
    v.push(Piece::new(shoulder_r, elbow_r, 9.0, coat));
    v.push(Piece::new(elbow_r, hand_r, 8.0, coat));
    v.push(Piece::new(
        hand_r,
        hand_r + (hand_r - elbow_r).normalize_or(Vec2::Y) * 3.0,
        6.0,
        SKIN,
    ));
    v.truncate(PARTS);
    v
}

/// Knock-back springs and the fall, carried frame to frame.
#[derive(Resource, Default)]
pub struct Motion {
    pub knock: Knock,
    vel: Knock,
    pub fall: f32,
    pub kneel: f32,
    /// How far his rifle was up when the moment ended; it stays there.
    pub raise: f32,
    /// The shot that's been answered, so it's felt once.
    hit_seen: Option<u32>,
}

impl Motion {
    /// A ball arrives at `y` (0 top of the figure .. 1 feet).
    pub fn hit(&mut self, y: f32, id: u32) {
        if self.hit_seen == Some(id) {
            return;
        }
        self.hit_seen = Some(id);
        if y < 0.18 {
            self.vel.head += 9.0;
        } else if y < 0.45 {
            self.vel.chest += 7.0;
            self.vel.push += 90.0;
        } else if y < 0.6 {
            self.vel.arm += 8.0;
            self.vel.push += 40.0;
        } else {
            self.vel.leg += 9.0;
        }
    }

    /// Springs pull the knocks back toward zero, damped.
    pub fn step(&mut self, dt: f32, dead: bool, down: bool) {
        let k = 60.0;
        let c = 9.0;
        let spring = |x: &mut f32, v: &mut f32| {
            *v += (-k * *x - c * *v) * dt;
            *x += *v * dt;
        };
        spring(&mut self.knock.head, &mut self.vel.head);
        spring(&mut self.knock.chest, &mut self.vel.chest);
        spring(&mut self.knock.arm, &mut self.vel.arm);
        spring(&mut self.knock.leg, &mut self.vel.leg);
        spring(&mut self.knock.push, &mut self.vel.push);
        let to = |x: &mut f32, on: bool, rate: f32| {
            *x = if on {
                (*x + dt * rate).min(1.0)
            } else {
                (*x - dt * rate).max(0.0)
            };
        };
        to(&mut self.fall, dead, 1.4);
        to(&mut self.kneel, down && !dead, 1.8);
    }

    pub fn reset(&mut self) {
        *self = Motion::default();
    }
}

/// Is his gun arm or his leg bad, fresh or old?
pub fn wounds(world: &World, id: NpcId) -> (bool, bool) {
    let n = world.npc(id);
    let has = |l: Limb| n.hurt == Some(l) || n.scars & l.bit() != 0;
    (has(Limb::GunArm), has(Limb::Leg))
}

/// Write pieces into the UI nodes: each node is a capsule centered on its
/// piece, rotated to lie along it.
pub fn paint(
    pieces: &[Piece],
    parts: &mut Query<(&Part, &mut Node, &mut BackgroundColor, &mut UiTransform)>,
) {
    for (p, mut node, mut bg, mut tf) in parts.iter_mut() {
        let Some(pc) = pieces.get(p.0) else {
            node.display = Display::None;
            continue;
        };
        let d = pc.b - pc.a;
        let len = d.length() + if pc.round { pc.thick } else { 0.0 };
        let c = (pc.a + pc.b) / 2.0;
        node.display = Display::Flex;
        node.left = Val::Px(c.x - len / 2.0);
        node.top = Val::Px(c.y - pc.thick / 2.0);
        node.width = Val::Px(len.max(1.0));
        node.height = Val::Px(pc.thick);
        node.border_radius = if pc.round {
            BorderRadius::all(Val::Px(pc.thick / 2.0))
        } else {
            BorderRadius::all(Val::Px(1.0))
        };
        bg.0 = pc.color;
        tf.rotation = Rot2::radians(d.y.atan2(d.x));
    }
}

/// Spawn the nodes for one figure under a stage.
pub fn spawn(s: &mut ChildSpawnerCommands) {
    for i in 0..PARTS {
        s.spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            Part(i),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limb_reaches_what_it_can() {
        let base = Vec2::new(0.0, 0.0);
        let t = Vec2::new(30.0, 20.0);
        let (elbow, hand) = reach(base, t, 24.0, 22.0, 1.0);
        assert!((hand - t).length() < 0.5, "{hand:?}");
        assert!(((elbow - base).length() - 24.0).abs() < 0.01);
        assert!(((hand - elbow).length() - 22.0).abs() < 0.01);
    }

    #[test]
    fn out_of_reach_it_points() {
        let (_, hand) = reach(Vec2::ZERO, Vec2::new(100.0, 0.0), 24.0, 22.0, 1.0);
        assert!((hand.y).abs() < 0.5 && hand.x > 45.0);
    }

    #[test]
    fn the_bend_picks_the_side() {
        let t = Vec2::new(0.0, 40.0);
        let (e1, _) = reach(Vec2::ZERO, t, 24.0, 22.0, 1.0);
        let (e2, _) = reach(Vec2::ZERO, t, 24.0, 22.0, -1.0);
        assert!(e1.x * e2.x < 0.0);
    }

    fn drive() -> Drive {
        Drive {
            feet: Vec2::new(560.0, 268.0),
            t: 0.0,
            fear: 0.0,
            aim: Vec2::new(400.0, 300.0),
            raise: 0.0,
            knock: Knock::default(),
            fall: 0.0,
            kneel: 0.0,
            bad_arm: false,
            bad_leg: false,
        }
    }

    fn dress() -> Dress {
        Dress {
            hat: Some((Color::BLACK, 30.0, 9.0)),
            coat: Some(Color::BLACK),
            shirt: Color::WHITE,
            legs: Color::BLACK,
            feet: Color::BLACK,
            skirt: false,
            knife: false,
            belt: false,
            gun: true,
        }
    }

    #[test]
    fn the_feet_stay_planted() {
        let mut d = drive();
        let a = pose(&MAN, &d, &dress());
        d.t = 1.3;
        d.knock.push = 12.0;
        let b = pose(&MAN, &d, &dress());
        // The boots are pieces 7 and 8 (after three back-arm pieces and four
        // leg pieces).
        for i in [7, 8] {
            assert!((a[i].a - b[i].a).length() < 1.0, "boot {i} slid");
        }
    }

    #[test]
    fn raised_the_muzzle_points_at_the_target() {
        let mut d = drive();
        d.raise = 1.0;
        let p = pose(&MAN, &d, &dress());
        let barrel = p.iter().find(|x| x.thick == 3.5).expect("the barrel");
        let dir = (barrel.b - barrel.a).normalize();
        let want = (d.aim - barrel.a).normalize();
        assert!(dir.dot(want) > 0.9, "{dir:?} vs {want:?}");
    }

    #[test]
    fn the_dead_lie_down() {
        let mut d = drive();
        let up = pose(&MAN, &d, &dress());
        d.fall = 1.0;
        let down = pose(&MAN, &d, &dress());
        let top = |p: &[Piece]| p.iter().map(|x| x.a.y.min(x.b.y)).fold(f32::MAX, f32::min);
        assert!(top(&down) > top(&up) + 60.0);
    }

    #[test]
    fn a_hit_knocks_and_settles() {
        let mut m = Motion::default();
        m.hit(0.3, 1);
        m.step(0.05, false, false);
        assert!(m.knock.chest > 0.0);
        for _ in 0..200 {
            m.step(0.016, false, false);
        }
        assert!(m.knock.chest.abs() < 0.02);
        // The same shot twice is felt once.
        m.hit(0.3, 1);
        m.step(0.016, false, false);
        assert!(m.knock.chest.abs() < 0.05);
    }
}
